//! 业务模型 + API DTO (file-service)
//!
//! 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.5
//! MVP: 接收上传 → 本地落盘 → 写 file_db 元数据

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct FileRow {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub project_id: Option<uuid::Uuid>,
    pub task_id: Option<uuid::Uuid>,
    pub name: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub storage_path: String,
    pub status: String,
    pub uploaded_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: uuid::Uuid,
    pub org_id: uuid::Uuid,
    pub project_id: Option<uuid::Uuid>,
    pub task_id: Option<uuid::Uuid>,
    pub name: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub status: String,
    pub uploaded_by: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<FileRow> for FileResponse {
    fn from(f: FileRow) -> Self {
        Self {
            id: f.id,
            org_id: f.org_id,
            project_id: f.project_id,
            task_id: f.task_id,
            name: f.name,
            content_type: f.content_type,
            size_bytes: f.size_bytes,
            sha256: f.sha256,
            status: f.status,
            uploaded_by: f.uploaded_by,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

/// multipart 上传元数据 (per §6.5 字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub org_id: uuid::Uuid,
    #[serde(default)]
    pub project_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub task_id: Option<uuid::Uuid>,
}

/// 内容响应 (仅 metadata, 不含实际 bytes — MVP 不实装下载)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub id: uuid::Uuid,
    pub name: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub storage_path: String,
    pub status: String,
    pub note: &'static str,
}