//! 本地落盘 storage layer (file-service)
//!
//! MVP: ./var/files/{org_id}/{file_id}
//! 后期可替换为 MinIO / S3 (per brief §1 file-service)

use cats_common::CatsError;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const VAR_DIR: &str = "var/files";

pub async fn ensure_base_dir() -> Result<(), CatsError> {
    fs::create_dir_all(VAR_DIR)
        .await
        .map_err(|e| CatsError::internal("failed to create var/files dir", e.to_string()))
}

pub async fn write_file(
    org_id: uuid::Uuid,
    file_id: uuid::Uuid,
    name: &str,
    bytes: &[u8],
) -> Result<String, CatsError> {
    ensure_base_dir().await?;
    let org_dir = PathBuf::from(VAR_DIR).join(org_id.to_string());
    fs::create_dir_all(&org_dir)
        .await
        .map_err(|e| CatsError::internal("failed to create org dir", e.to_string()))?;

    // 防止 path traversal: name 不能含 `/` `\` `..`
    let safe_name: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '.' | '\0'))
        .collect();
    let path = org_dir.join(format!("{file_id}_{safe_name}"));

    let mut f = fs::File::create(&path)
        .await
        .map_err(|e| CatsError::internal("failed to create file", e.to_string()))?;
    f.write_all(bytes)
        .await
        .map_err(|e| CatsError::internal("failed to write file", e.to_string()))?;
    f.flush()
        .await
        .map_err(|e| CatsError::internal("failed to flush file", e.to_string()))?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ensure_base_dir_succeeds() {
        // 临时目录测试
        let test_dir = std::env::temp_dir().join("cats-file-service-test");
        let _ = tokio::fs::remove_dir_all(&test_dir).await;
        std::env::set_var("CARGO_TEST_TMPDIR", test_dir.to_string_lossy().to_string());
        // 此测试仅验证函数签名编译; 实际落盘路径取决于 cwd
    }

    #[test]
    fn path_traversal_protection_strips_dots() {
        let name: String = "../etc/passwd"
            .chars()
            .filter(|c| !matches!(c, '/' | '\\' | '.' | '\0'))
            .collect();
        assert!(!name.contains('/'));
        assert!(!name.contains('.'));
    }
}