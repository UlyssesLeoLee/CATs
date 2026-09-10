//! User factory
//!
//! 引用: 设计书 §4.1.1
//!
//! User 对象: id / username / email / password_hash / is_active / created_at
//!
//! 提供:
//! - 默认 (随机 username/email, 密码 hash 是 fake argon2-like 字符串, 不真验)
//! - 固定 id (用于幂等测试)
//! - 批量 (默认 count=1)
//! - 边界: inactive / 重复 username / 长 username

use super::{now_utc, random_past_within_days, Factory};
use fake::faker::internet::en::Username;
use fake::Fake;
use serde::{Deserialize, Serialize};

/// 生成随机 email (不依赖 fake crate 的 faker 模块, 自己造, 跨 fake 版本稳定)
fn random_email() -> String {
    use rand::Rng;
    let user = (0..8)
        .map(|_| rand::thread_rng().gen_range(b'a'..=b'z') as char)
        .collect::<String>();
    let domains = ["example.com", "test.com", "mock.org", "cats.local"];
    let domain = domains[rand::thread_rng().gen_range(0..domains.len())];
    format!("{user}@{domain}")
}

/// User 数据 (与 user-service 模型字段对齐)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// 主键 (UUID v4)
    pub id: uuid::Uuid,
    /// 用户名 (unique, 3-32 字符)
    pub username: String,
    /// email (RFC 5322 简化校验)
    pub email: String,
    /// argon2id hash (mock: 形如 `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`)
    pub password_hash: String,
    /// 是否激活 (false = 已禁用)
    pub is_active: bool,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// User 工厂配置
#[derive(Debug, Clone)]
pub struct UserFactory {
    fixed_id: Option<uuid::Uuid>,
    is_active: Option<bool>,
    /// 自定义 username (None = 随机生成)
    username: Option<String>,
    /// 自定义 email (None = 随机生成)
    email: Option<String>,
    /// 自定义 password_hash (None = 形如 argon2 字符串占位)
    password_hash: Option<String>,
    /// 创建时间偏移天数 (None = 随机过去 365 天内)
    created_within_days: Option<i64>,
    /// 批量构造的 count
    count: usize,
}

impl UserFactory {
    /// 默认配置 (随机 + 激活 + 默认 count=1)
    pub fn new() -> Self {
        Self {
            fixed_id: None,
            is_active: Some(true),
            username: None,
            email: None,
            password_hash: None,
            created_within_days: None,
            count: 1,
        }
    }

    /// 固定 id (幂等测试)
    pub fn fixed_id(mut self, id: uuid::Uuid) -> Self {
        self.fixed_id = Some(id);
        self
    }

    /// 强制 inactive
    pub fn inactive(mut self) -> Self {
        self.is_active = Some(false);
        self
    }

    /// 自定义 username
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// 自定义 email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// 自定义 password_hash
    pub fn with_password_hash(mut self, hash: impl Into<String>) -> Self {
        self.password_hash = Some(hash.into());
        self
    }

    /// 创建时间在最近 N 天内
    pub fn created_within_days(mut self, days: i64) -> Self {
        self.created_within_days = Some(days);
        self
    }

    /// 批量构造的 count
    pub fn count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }

    /// 构造一个
    fn build_one(&self) -> User {
        let id = self.fixed_id.unwrap_or_else(uuid::Uuid::new_v4);
        let username = self
            .username
            .clone()
            .unwrap_or_else(|| Username().fake::<String>());
        let email = self
            .email
            .clone()
            .unwrap_or_else(random_email);
        let password_hash = self.password_hash.clone().unwrap_or_else(|| {
            // 形如 argon2 占位 (不实际算, 测试不验证 hash 内容)
            format!("$argon2id$v=19$m=19456,t=2,p=1${}${}", "fakesalt0123456789", "fakehash0123456789")
        });
        let is_active = self.is_active.unwrap_or(true);
        let created_at = self
            .created_within_days
            .map(random_past_within_days)
            .unwrap_or_else(now_utc);

        User {
            id,
            username,
            email,
            password_hash,
            is_active,
            created_at,
            updated_at: created_at,
        }
    }

    /// 默认 count (供 lib.rs 回归测试用)
    pub fn default_count() -> usize { 1 }
}

impl Default for UserFactory {
    fn default() -> Self { Self::new() }
}

impl Factory for UserFactory {
    type Output = User;

    fn build(&self) -> User { self.build_one() }

    fn build_many(&self, count: usize) -> Vec<User> {
        (0..count).map(|_| self.build_one()).collect()
    }
}

impl User {
    /// DB 行 (与 user-service.UserCredential 字段对齐)
    pub fn as_db_row(&self) -> UserDbRow {
        UserDbRow {
            id: self.id,
            username: self.username.clone(),
            email: Some(self.email.clone()),
            password_hash: self.password_hash.clone(),
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// DB row (与 user-service.UserCredential 字段对齐)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserDbRow {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::invalid_emails;

    #[test]
    fn default_user_has_unique_id() {
        let u1 = UserFactory::new().build();
        let u2 = UserFactory::new().build();
        assert_ne!(u1.id, u2.id, "每个 User 必须独立 UUID");
        assert!(u1.is_active);
        assert!(!u1.username.is_empty());
        assert!(u1.email.contains('@'));
    }

    #[test]
    fn fixed_id_is_honored() {
        let id = uuid::Uuid::new_v4();
        let u = UserFactory::new().fixed_id(id).build();
        assert_eq!(u.id, id);
    }

    #[test]
    fn inactive_user() {
        let u = UserFactory::new().inactive().build();
        assert!(!u.is_active);
    }

    #[test]
    fn custom_username_and_email() {
        let u = UserFactory::new()
            .with_username("alice")
            .with_email("alice@example.com")
            .build();
        assert_eq!(u.username, "alice");
        assert_eq!(u.email, "alice@example.com");
    }

    #[test]
    fn batch_creates_count_users() {
        let users = UserFactory::new().build_many(10);
        assert_eq!(users.len(), 10);
        // 10 个独立 id
        let mut ids: Vec<_> = users.iter().map(|u| u.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "id 必须唯一");
    }

    #[test]
    fn created_within_days_respected() {
        let u = UserFactory::new().created_within_days(7).build();
        let now = now_utc();
        let delta = now - u.created_at;
        assert!(delta.num_seconds() <= 7 * 86_400 + 1);
        assert!(delta.num_seconds() >= 0);
    }

    #[test]
    fn invalid_emails_can_be_constructed_via_with_email() {
        for e in invalid_emails() {
            let u = UserFactory::new().with_email(*e).build();
            assert_eq!(u.email, *e);
        }
    }

    #[test]
    fn db_row_round_trip() {
        let u = UserFactory::new().build();
        let row = u.as_db_row();
        assert_eq!(row.id, u.id);
        assert_eq!(row.username, u.username);
    }

    #[test]
    fn default_count_is_one() {
        assert_eq!(UserFactory::default_count(), 1);
    }
}
