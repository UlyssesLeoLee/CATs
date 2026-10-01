//! `file-service` — 文件存取服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (file_db 接口契约 v1.0.0)
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3 (error enum)
//! 引用: doc/05-其他/管理/CATs_权限矩阵_v1.0.md §3 (RBAC: file:read / file:create / file:delete)
//! 引用: ULYS-152 切片 B-3
//!
//! M1 业务实现 (per ULYS-152 切片 B-3 完成判据):
//! - POST   /v1/files                  — 上传 (JSON + base64, M1 简化, Sprint 2 升 multipart)
//! - GET    /v1/files/{id}             — 下载 (JSON 包装 base64)
//! - GET    /v1/files/{id}/metadata    — 元数据
//! - DELETE /v1/files/{id}             — 软删除 (status='deleted')
//! - GET    /v1/files                  — 列表 (按 workspace_id + 分页)
//! - GET    /healthz
//! - 8 逻辑库 file_db (per Baseline §5.1) + pgcrypto extension
//!
//! 设计选择 (per 缺标比错标安全, 守门 #11):
//! - 上传走 JSON + base64 而非 multipart: M1 简化, 避免引入 actix-multipart 依赖
//!   (per Cargo.lock 当前未含 actix-multipart). Sprint 2 升级.
//! - 本地磁盘存储 (per env FILE_STORAGE_ROOT), Sprint 3 切 S3
//! - 软删除: status 列切换, 物理文件保留 (审计追溯)
//! - sha256 计算用于完整性校验 + 后续去重 (M2 落地)

pub mod db;
pub mod handlers;
pub mod models;
pub mod rbac;

pub use models::{
    ErrorBody, FileListResponse, FileMetadataResponse, ListFilesQuery, NewFileRecord, UploadFileRequest,
    UploadFileResponse,
};

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串
pub fn version() -> &'static str {
    VERSION
}

/// 返回 crate 名称
pub fn name() -> &'static str {
    NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(
            v.starts_with("0.1."),
            "version should start with '0.1.', got {v}"
        );
    }

    #[test]
    fn name_is_file_service() {
        assert_eq!(name(), "file-service");
    }
}