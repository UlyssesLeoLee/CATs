//! DB fixture: PG 18.6 + pgvector 0.8 schema/seed 工厂
//!
//! 引用: 设计书 §4.2
//!
//! 三层抽象:
//! - [`schema`]: SQL DDL 字符串 (per 16 service 的 CREATE TABLE)
//! - [`seed`]: 数据插入 SQL (per 业务 factory 的批量 INSERT)
//! - [`fixture`]: 整合 + 提供 in-memory 或 testcontainers 切换
//!
//! 设计原则:
//! - **零外部依赖默认**: 不需要 docker, in-memory `DbFixture::in_memory()` 即可跑大多数测试
//! - **可选 testcontainers**: `DbFixture::pg_testcontainers()` 在 CI 有 docker 时启动真实 PG
//! - **schema 与 service 同步**: SQL DDL 是从 `crates/*/migrations/` 复制过来, 测试用
//!
//! ## 用法
//!
//! ```ignore
//! use cats_mock::db::{DbFixture, SchemaSet, SeedSet};
//!
//! // 1) 内存 fixture (默认, 无 docker 依赖)
//! let db = DbFixture::in_memory();
//! db.apply_schema(SchemaSet::all_common()).unwrap();
//! db.apply_seed(SeedSet::users_default()).unwrap();
//!
//! // 2) 真实 PG (CI)
//! let db = DbFixture::pg_testcontainers("postgres://...").await.unwrap();
//! ```

pub mod schema;
pub mod seed;
pub mod fixture;

pub use fixture::*;
pub use schema::*;
pub use seed::*;
