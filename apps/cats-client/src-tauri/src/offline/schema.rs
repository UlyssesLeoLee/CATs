//! SQL schema 常量（编译期 include_str! 注入 schema.sql）
//!
//! 引用: DB 三分类横展开原则（9/1 18:30 JST）
//!   Work / Transaction / Master 三类分门别类, 不允许只列一类合并

/// schema.sql 全文
pub const SCHEMA_SQL: &str = include_str!("schema.sql");