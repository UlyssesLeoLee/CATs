//! translation-core 数据访问层 (project_db)
//!
//! MVP 简化:
//! - TM 查询走 pgvector 余弦相似度 + LIKE 精确回退
//! - Glossary 走 glossary_terms 表
//! - 真连 DB 留给 Sprint 末 (需要 project_db 在线)

use cats_common::{CatsError, ErrorCode};
use sqlx::PgPool;
use uuid::Uuid;

/// 一条 TM 行
#[derive(Debug, Clone)]
pub struct TmRow {
    pub tm_id: String,
    pub source_text: String,
    pub target_text: String,
    pub similarity: f32,
    pub is_exact: bool,
}

/// 一条 glossary 行
#[derive(Debug, Clone)]
pub struct TermRow {
    pub source_term: String,
    pub target_term: String,
    pub forbidden: bool,
}

/// 100% 精确匹配 (LIKE source_text = $1)
pub async fn tm_exact_match(
    pool: &PgPool,
    _tenant_id: Uuid,
    source_text: &str,
) -> Result<Vec<TmRow>, CatsError> {
    let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
        r#"SELECT tm_id, source_text, target_text
           FROM translation_memory
           WHERE source_text = $1 AND approved = true"#,
    )
    .bind(source_text)
    .fetch_all(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("tm exact fail: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(id, s, t)| TmRow {
            tm_id: id.to_string(),
            source_text: s,
            target_text: t,
            similarity: 1.0,
            is_exact: true,
        })
        .collect())
}

/// 模糊匹配 (pgvector 余弦相似度, threshold = 0.85 默认)
pub async fn tm_fuzzy_match(
    pool: &PgPool,
    _tenant_id: Uuid,
    source_embedding: Vec<f32>,
    threshold: f32,
    limit: i64,
) -> Result<Vec<TmRow>, CatsError> {
    // 注: embedding 维度需 project_db migration 跟 model 维度对齐
    // MVP: 直接用 pgvector <=> cosine distance
    let rows: Vec<(Uuid, String, String, f32)> = sqlx::query_as(
        r#"SELECT tm_id, source_text, target_text, 1 - (embedding <=> $1) AS similarity
           FROM translation_memory
           WHERE approved = true AND 1 - (embedding <=> $1) >= $2
           ORDER BY embedding <=> $1
           LIMIT $3"#,
    )
    .bind(&source_embedding)
    .bind(threshold)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("tm fuzzy fail: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(id, s, t, sim)| TmRow {
            tm_id: id.to_string(),
            source_text: s,
            target_text: t,
            similarity: sim,
            is_exact: sim >= 0.999,
        })
        .collect())
}

/// 写入一条 TM (后续 worker / 用户提交)
pub async fn tm_update(
    pool: &PgPool,
    tenant_id: Uuid,
    project_id: Uuid,
    source_text: &str,
    target_text: &str,
    embedding: Vec<f32>,
) -> Result<Uuid, CatsError> {
    let (id,): (Uuid,) = sqlx::query_as(
        r#"INSERT INTO translation_memory (tenant_id, project_id, source_text, target_text, embedding, approved)
           VALUES ($1, $2, $3, $4, $5, true)
           ON CONFLICT (tenant_id, source_text) DO UPDATE SET target_text = $4, updated_at = now()
           RETURNING tm_id"#,
    )
    .bind(tenant_id)
    .bind(project_id)
    .bind(source_text)
    .bind(target_text)
    .bind(&embedding)
    .fetch_one(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("tm update fail: {e}")))?;
    Ok(id)
}

/// 术语查询 (Match): 给出 source 包含的所有术语对
pub async fn glossary_match(
    pool: &PgPool,
    tenant_id: Uuid,
    source_text: &str,
) -> Result<Vec<TermRow>, CatsError> {
    let rows: Vec<(String, String, bool)> = sqlx::query_as(
        r#"SELECT source_term, target_term, forbidden
           FROM glossary_terms
           WHERE tenant_id = $1 AND position(source_term $3 in source_text) > 0"#,
    )
    .bind(tenant_id)
    .bind(0_i32) // placeholder for type safety
    .bind(source_text)
    .fetch_all(pool)
    .await
    .map_err(|e| CatsError::business(ErrorCode::InternalError, format!("glossary match fail: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|(s, t, f)| TermRow {
            source_term: s,
            target_term: t,
            forbidden: f,
        })
        .collect())
}