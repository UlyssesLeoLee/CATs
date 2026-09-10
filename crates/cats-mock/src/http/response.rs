//! 预制 JSON 响应 (HTTP 状态码 + ErrorBody 格式)
//!
//! 引用: 设计书 §4.4.2
//!
//! ErrorBody 字段对齐 auth-service::models::ErrorBody

use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 错误响应 (统一格式 per 实施前QA §3.4)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ErrorBody {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self { error: error.into(), message: message.into(), detail: None }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// 转 actix HttpResponse
    pub fn to_response(&self, status: u16) -> HttpResponse {
        let mut builder = HttpResponse::build(
            actix_web::http::StatusCode::from_u16(status)
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        );
        builder.json(self)
    }
}

impl fmt::Display for ErrorBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

impl std::error::Error for ErrorBody {}

/// 预制错误响应
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    InternalServerError,
    ServiceUnavailable,
}

impl MockError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::InternalServerError => "internal_error",
            Self::ServiceUnavailable => "service_unavailable",
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::InternalServerError => 500,
            Self::ServiceUnavailable => 503,
        }
    }

    pub fn default_message(&self) -> &'static str {
        match self {
            Self::BadRequest => "请求参数无效",
            Self::Unauthorized => "未认证或 token 失效",
            Self::Forbidden => "无权限访问",
            Self::NotFound => "资源不存在",
            Self::Conflict => "资源冲突",
            Self::InternalServerError => "服务器内部错误",
            Self::ServiceUnavailable => "服务暂不可用",
        }
    }

    /// 转 ErrorBody
    pub fn to_body(&self) -> ErrorBody {
        ErrorBody::new(self.code(), self.default_message())
    }
}

/// ResponseBuilder: 快速构造测试用响应
pub struct ResponseBuilder;

impl ResponseBuilder {
    /// 200 OK + JSON body
    pub fn ok<T: Serialize>(body: T) -> HttpResponse {
        HttpResponse::Ok().json(body)
    }

    /// 201 Created + JSON body
    pub fn created<T: Serialize>(body: T) -> HttpResponse {
        HttpResponse::Created().json(body)
    }

    /// 204 No Content
    pub fn no_content() -> HttpResponse {
        HttpResponse::NoContent().finish()
    }

    /// 错误响应 (用 MockError)
    pub fn err(e: MockError) -> HttpResponse {
        e.to_body().to_response(e.status())
    }

    /// 错误响应 + 自定义 message
    pub fn err_with_msg(e: MockError, msg: impl Into<String>) -> HttpResponse {
        ErrorBody::new(e.code(), msg).to_response(e.status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_serializes_to_expected_json() {
        let eb = ErrorBody::new("not_found", "user not found");
        let json = serde_json::to_string(&eb).unwrap();
        assert!(json.contains("\"error\":\"not_found\""));
        assert!(json.contains("\"message\":\"user not found\""));
        // detail 是 None, 应被 skip
        assert!(!json.contains("detail"));
    }

    #[test]
    fn error_body_with_detail_includes_field() {
        let eb = ErrorBody::new("bad", "msg").with_detail("trace=abc");
        let json = serde_json::to_string(&eb).unwrap();
        assert!(json.contains("detail"));
    }

    #[test]
    fn mock_error_status_codes() {
        assert_eq!(MockError::BadRequest.status(), 400);
        assert_eq!(MockError::Unauthorized.status(), 401);
        assert_eq!(MockError::Forbidden.status(), 403);
        assert_eq!(MockError::NotFound.status(), 404);
        assert_eq!(MockError::Conflict.status(), 409);
        assert_eq!(MockError::InternalServerError.status(), 500);
        assert_eq!(MockError::ServiceUnavailable.status(), 503);
    }

    #[test]
    fn mock_error_codes_are_snake_case() {
        for e in [
            MockError::BadRequest,
            MockError::Unauthorized,
            MockError::Forbidden,
            MockError::NotFound,
            MockError::Conflict,
            MockError::InternalServerError,
            MockError::ServiceUnavailable,
        ] {
            assert!(!e.code().is_empty());
            assert!(!e.code().contains(' '));
        }
    }

    #[test]
    fn response_builder_ok_works() {
        let r = ResponseBuilder::ok(serde_json::json!({"ok": true}));
        assert_eq!(r.status().as_u16(), 200);
    }

    #[test]
    fn response_builder_created_works() {
        let r = ResponseBuilder::created(serde_json::json!({"id": "1"}));
        assert_eq!(r.status().as_u16(), 201);
    }

    #[test]
    fn response_builder_no_content_works() {
        let r = ResponseBuilder::no_content();
        assert_eq!(r.status().as_u16(), 204);
    }

    #[test]
    fn response_builder_err_uses_correct_status() {
        let r = ResponseBuilder::err(MockError::NotFound);
        assert_eq!(r.status().as_u16(), 404);
    }
}
