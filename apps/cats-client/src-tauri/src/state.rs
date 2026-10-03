//! 应用全局状态
//!
//! 持有:
//! - HTTP client (reqwest) + BFF base URL
//! - 当前 JWT access/refresh token（mutex 保护）
//! - SQLite 离线数据库连接池句柄（Option，启动后懒初始化）
//! - 在线/离线标志
//!
//! 设计原则（MVP 阶段）:
//! - 全局单实例（Arc），通过 tauri::State 注入到各 command
//! - 状态变更走 std::sync::Mutex（MVP 简化；后续可用 tokio::sync::RwLock）
//! - 离线判定: 最近一次 HTTP 请求 5xx/网络错误后切到 offline 模式

use std::path::PathBuf;
use std::sync::Mutex;

use reqwest::Client;
use tauri::Manager;

use crate::offline::OfflineDb;

/// 应用全局状态
pub struct AppState {
    /// HTTP client（共享 reqwest 连接池）
    pub http: Client,

    /// BFF base URL（默认 https://api.cats.internal，env `CATS_BFF_URL` 可覆盖）
    pub bff_base: String,

    /// 当前 JWT
    pub token: Mutex<Option<TokenPair>>,

    /// 在线/离线标志
    pub online: Mutex<bool>,

    /// SQLite 离线数据库（懒初始化）
    pub offline_db: Mutex<Option<OfflineDb>>,
}

/// JWT access + refresh token 对
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl AppState {
    /// 创建默认 AppState（HTTP client 已就绪；DB 懒初始化）
    pub fn new() -> anyhow::Result<Self> {
        let bff_base = std::env::var("CATS_BFF_URL")
            .unwrap_or_else(|_| "https://api.cats.internal".to_string());

        let http = Client::builder()
            .user_agent(concat!("cats-client/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()?;

        Ok(Self {
            http,
            bff_base,
            token: Mutex::new(None),
            online: Mutex::new(true),
            offline_db: Mutex::new(None),
        })
    }

    /// 懒初始化 SQLite 离线数据库
    ///
    /// 路径: `<app_local_data_dir>/offline.db`
    /// 失败时不抛错，记 warn，由 caller 决定是否继续纯在线模式。
    pub fn ensure_db(&self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        let mut guard = self.offline_db.lock().expect("offline_db mutex poisoned");
        if guard.is_some() {
            return Ok(());
        }

        let dir: PathBuf = app
            .path()
            .app_local_data_dir()
            .map_err(|e| anyhow::anyhow!("无法获取 app_local_data_dir: {e}"))?;
        std::fs::create_dir_all(&dir)?;
        let db_path = dir.join("offline.db");

        let db = OfflineDb::open(&db_path)?;
        *guard = Some(db);
        Ok(())
    }

    /// 当前 access_token 快照（clone）
    pub fn access_token(&self) -> Option<String> {
        self.token
            .lock()
            .expect("token mutex poisoned")
            .as_ref()
            .map(|t| t.access_token.clone())
    }

    /// 写入新的 token 对
    pub fn set_token(&self, pair: TokenPair) {
        *self.token.lock().expect("token mutex poisoned") = Some(pair);
    }

    /// 清除 token（登出）
    pub fn clear_token(&self) {
        *self.token.lock().expect("token mutex poisoned") = None;
    }

    /// 标记离线
    pub fn mark_offline(&self) {
        *self.online.lock().expect("online mutex poisoned") = false;
    }

    /// 标记在线
    pub fn mark_online(&self) {
        *self.online.lock().expect("online mutex poisoned") = true;
    }

    /// 当前在线状态
    pub fn is_online(&self) -> bool {
        *self.online.lock().expect("online mutex poisoned")
    }
}