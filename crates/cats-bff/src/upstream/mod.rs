//! cats-bff 上游 service 客户端集合 (per 切片 A _slice_a_bff.md §交付清单 2)
//!
//! - `auth.rs`     — auth-service (login/refresh/logout/me)
//! - `projects.rs` — project-service (list/create)
//! - `tasks.rs`    — task-service (dispatch)

pub mod auth;
pub mod projects;
pub mod tasks;
