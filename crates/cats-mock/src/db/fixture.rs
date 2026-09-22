//! DbFixture: 整合 schema + seed, 提供 in-memory 与 testcontainers 两种模式
//!
//! 引用: 设计书 §4.2.3

use super::{SchemaSet, SeedParam, SeedSet};
use std::sync::Mutex;

/// DbFixture trait (统一接口)
///
/// 调用方按场景选实现:
/// - [`InMemoryDbFixture`]: 默认, 无外部依赖, 适合单测 + e2e 跑 actix routes
/// - [`PgDbFixture`]: 真实 PG via testcontainers, CI + 集成测试
#[async_trait::async_trait]
pub trait DbFixture: Send + Sync {
    /// 应用 schema (CREATE TABLE 等)
    async fn apply_schema(&self, schema: SchemaSet) -> anyhow::Result<()>;

    /// 应用 seed (INSERT 数据)
    async fn apply_seed(&self, seed: SeedSet) -> anyhow::Result<()>;

    /// 清空所有表 (per test 隔离)
    async fn truncate_all(&self) -> anyhow::Result<()>;

    /// 关闭 fixture
    async fn shutdown(self) -> anyhow::Result<()>;
}

// =====================================================================
// InMemoryDbFixture
// =====================================================================

/// 内存 fixture: 把所有表维护在 `Mutex<HashMap<TableName, Vec<Row>>>` 里
///
/// **不真用 SQL 解析**, 只接受 DDL 字符串 (用于 schema.apply_schema 兼容性)
/// 与 INSERT 调用 (按 table 名走预制 schema 的列名顺序)
///
/// 适用: 单测 (单进程, 不连 DB)
#[derive(Default)]
pub struct InMemoryDbFixture {
    tables: Mutex<std::collections::HashMap<String, Vec<Vec<String>>>>,
    applied_schemas: Mutex<Vec<String>>,
}

impl InMemoryDbFixture {
    /// 新建空 fixture
    pub fn new() -> Self { Self::default() }

    /// 当前表数量
    pub fn table_count(&self) -> usize { self.tables.lock().unwrap().len() }

    /// 某表的行数
    pub fn row_count(&self, table: &str) -> usize {
        self.tables
            .lock()
            .unwrap()
            .get(table)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// 所有表名
    pub fn tables(&self) -> Vec<String> {
        self.tables.lock().unwrap().keys().cloned().collect()
    }
}

#[async_trait::async_trait]
impl DbFixture for InMemoryDbFixture {
    async fn apply_schema(&self, schema: SchemaSet) -> anyhow::Result<()> {
        let mut tables = self.tables.lock().unwrap();
        let mut schemas = self.applied_schemas.lock().unwrap();
        for stmt in &schema.statements {
            if let Some(name) = extract_table_name(stmt) {
                tables.entry(name).or_default();
            }
            schemas.push(stmt.clone());
        }
        Ok(())
    }

    async fn apply_seed(&self, seed: SeedSet) -> anyhow::Result<()> {
        let mut tables = self.tables.lock().unwrap();
        for entry in &seed.entries {
            let rows = tables.entry(entry.table.clone()).or_default();
            // 把 SeedParam 序列化成字符串行 (调试用, 真实 e2e 不用)
            let row: Vec<String> = entry.params.iter().map(seed_param_to_string).collect();
            rows.push(row);
        }
        Ok(())
    }

    async fn truncate_all(&self) -> anyhow::Result<()> {
        let mut tables = self.tables.lock().unwrap();
        for rows in tables.values_mut() {
            rows.clear();
        }
        Ok(())
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        // InMemory 无外部资源, 直接 drop
        Ok(())
    }
}

// =====================================================================
// PgDbFixture (testcontainers stub)
// =====================================================================

/// 真实 PG fixture (via testcontainers)
///
/// 当前实现为 stub, **不**自动启动 docker; 真实启动由调用方在 CI 中按需启用
/// (避免单测默认依赖 docker, 影响本地开发体验)
///
/// 启用方法: 在 CI yaml 中加 `services: [postgres]` 显式启动, 然后:
/// ```ignore
/// let db = PgDbFixture::connect("postgres://...").await?;
/// db.apply_schema(SchemaSet::all_common()).await?;
/// db.apply_seed(SeedSet::users_default()).await?;
/// ```
pub struct PgDbFixture {
    /// 真实 PG 连接池 (可选; 启动后填充)
    pool: Option<sqlx::PgPool>,
    /// DSN (供后续 connect)
    pub dsn: String,
    /// 已应用的 schema
    applied_schemas: Vec<String>,
}

impl PgDbFixture {
    /// 准备 fixture (仅记录 DSN, 不连)
    pub fn prepare(dsn: impl Into<String>) -> Self {
        Self {
            pool: None,
            dsn: dsn.into(),
            applied_schemas: vec![],
        }
    }

    /// 已应用的 schema 数
    pub fn applied_schema_count(&self) -> usize { self.applied_schemas.len() }

    /// 是否已连接
    pub fn is_connected(&self) -> bool { self.pool.is_some() }
}

#[async_trait::async_trait]
impl DbFixture for PgDbFixture {
    async fn apply_schema(&self, _schema: SchemaSet) -> anyhow::Result<()> {
        // 真连 PG 时:
        //   let pool = self.pool.as_ref().context("not connected")?;
        //   for stmt in schema.statements { sqlx::query(&stmt).execute(pool).await?; }
        // 当前 stub: 仅记录, 不执行
        Err(anyhow::anyhow!(
            "PgDbFixture stub — call .connect(\"{}\") first, then enable impl when testcontainers runtime is wired in CI",
            self.dsn
        ))
    }

    async fn apply_seed(&self, _seed: SeedSet) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("PgDbFixture stub — see apply_schema"))
    }

    async fn truncate_all(&self) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("PgDbFixture stub"))
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        if let Some(pool) = self.pool {
            pool.close().await;
        }
        Ok(())
    }
}

// =====================================================================
// 工具函数
// =====================================================================

/// 从 "CREATE TABLE [IF NOT EXISTS] table_name (...)" 提取表名
///
/// 简单实现: 找 `TABLE` 后第一个**非** `IF/NOT/EXISTS` 的 token
fn extract_table_name(create_stmt: &str) -> Option<String> {
    let upper = create_stmt.to_uppercase();
    let idx = upper.find("CREATE TABLE")?;
    let rest = &create_stmt[idx + "CREATE TABLE".len()..];
    for token in rest.split(|c: char| c.is_whitespace() || c == '(') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        if t.eq_ignore_ascii_case("IF") || t.eq_ignore_ascii_case("NOT") || t.eq_ignore_ascii_case("EXISTS") {
            continue;
        }
        return Some(t.trim_end_matches('(').to_string());
    }
    None
}

fn seed_param_to_string(p: &SeedParam) -> String {
    // 简化: 用 Debug 形式
    format!("{:?}", p)
}

// =====================================================================
// 工厂入口
// =====================================================================

/// 新建 in-memory fixture (默认, 无依赖)
pub fn in_memory() -> InMemoryDbFixture { InMemoryDbFixture::new() }

/// 准备 PG fixture (stub, 调用方需补实现)
pub fn pg(dsn: impl Into<String>) -> PgDbFixture { PgDbFixture::prepare(dsn) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::SchemaSet;

    #[tokio::test]
    async fn in_memory_apply_schema_creates_tables() {
        let db = InMemoryDbFixture::new();
        db.apply_schema(SchemaSet::all_common()).await.unwrap();
        let tables = db.tables();
        // 5 表:users_credential / user_profile / project / task / audit_log
        assert!(tables.contains(&"users_credential".to_string()));
        assert!(tables.contains(&"user_profile".to_string()));
        assert!(tables.contains(&"project".to_string()));
        assert!(tables.contains(&"task".to_string()));
        assert!(tables.contains(&"audit_log".to_string()));
        assert_eq!(db.table_count(), 5);
    }

    #[tokio::test]
    async fn in_memory_apply_seed_increments_rows() {
        let db = InMemoryDbFixture::new();
        db.apply_schema(SchemaSet::all_common()).await.unwrap();
        db.apply_seed(crate::db::seed::users_default()).await.unwrap();
        assert_eq!(db.row_count("users_credential"), 1);
    }

    #[tokio::test]
    async fn in_memory_truncate_clears_rows() {
        let db = InMemoryDbFixture::new();
        db.apply_schema(SchemaSet::all_common()).await.unwrap();
        db.apply_seed(crate::db::seed::audit_events_default()).await.unwrap();
        assert_eq!(db.row_count("audit_log"), 1);
        db.truncate_all().await.unwrap();
        assert_eq!(db.row_count("audit_log"), 0);
        // 但表结构保留
        assert!(db.tables().contains(&"audit_log".to_string()));
    }

    #[test]
    fn extract_table_name_basic() {
        let s = "CREATE TABLE IF NOT EXISTS my_table (id INT);";
        assert_eq!(extract_table_name(s), Some("my_table".to_string()));
    }

    #[test]
    fn extract_table_name_no_if_not_exists() {
        let s = "CREATE TABLE foo (id INT);";
        assert_eq!(extract_table_name(s), Some("foo".to_string()));
    }

    #[test]
    fn pg_fixture_is_stub() {
        let pg = PgDbFixture::prepare("postgres://test");
        assert!(!pg.is_connected());
        assert_eq!(pg.dsn, "postgres://test");
        assert_eq!(pg.applied_schema_count(), 0);
    }
}
