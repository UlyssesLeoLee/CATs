//! 业务模型 + API DTO
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1 (file-service)
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (file_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - 接口设计书 v2.0 整份不存在 (per Sprint 1 §6.1 缺口)
//!   → DTO 基于微服务架构书 §4.1 + 本切片 B-3 任务描述
//! - ErrorBody 与 user-service / auth-service 错误码表 §3 枚举值一致
//! - storage_path 不暴露给客户端 (per 安全: BFF 透传)
//! - 上传走 multipart: M1 用 actix-multipart 接收 (可选); 简化版本直接 JSON {filename, content_base64} 落地

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DB 实体: file_db.files
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub owner_user_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub sha256: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 新建文件记录 (db 层接口, 不暴露给 HTTP)
#[derive(Debug, Clone)]
pub struct NewFileRecord {
    pub workspace_id: Uuid,
    pub owner_user_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub sha256: String,
}

// =====================================================================
// API DTO
// =====================================================================

/// GET /v1/files/{id}/metadata 响应 (per Baseline §5.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadataResponse {
    pub id: String,
    pub workspace_id: String,
    pub owner_user_id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<FileRecord> for FileMetadataResponse {
    fn from(r: FileRecord) -> Self {
        Self {
            id: r.id.to_string(),
            workspace_id: r.workspace_id.to_string(),
            owner_user_id: r.owner_user_id.to_string(),
            filename: r.filename,
            content_type: r.content_type,
            size_bytes: r.size_bytes,
            sha256: r.sha256,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// GET /v1/files 列表响应 (per Baseline §5.1 + 分页)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub items: Vec<FileMetadataResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// POST /v1/files 请求 (per 切片 B-3 简化: JSON + base64 内容)
///
/// 设计选择 (per 缺标比错标): M1 阶段走 JSON body + base64 content,
/// 避免引入 actix-multipart 依赖 (per Cargo.lock 当前没有 multipart).
/// Sprint 2 升级时切换到 multipart/form-data 透传.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileRequest {
    pub workspace_id: Uuid,
    pub filename: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// base64 编码的文件内容
    pub content_base64: String,
}

fn default_content_type() -> String {
    "application/octet-stream".to_string()
}

/// POST /v1/files 响应 (per Baseline §5.1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileResponse {
    pub file: FileMetadataResponse,
    /// 是否去重命中 (sha256 已存在时 created=false)
    pub deduplicated: bool,
}

/// GET /v1/files 列表查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesQuery {
    pub workspace_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// 错误响应 (与 user-service / auth-service 错误码表 §3 枚举值保持一致)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
