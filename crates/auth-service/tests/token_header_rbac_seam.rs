//! 跨 crate 接缝：`issue_jwt` 签出的 token → 边缘注入的头 → 下游 RBAC 取值
//!
//! # 为什么需要这个文件（2026-10-07）
//!
//! 整条边缘鉴权链有四段：
//!
//! 1. `auth-service::issue_jwt` 把角色写进 token（`roles` 数组 + `roles_csv` 字符串）
//! 2. envoy `jwt_authn` 的 `claim_to_headers` 把 claim **原样**拷成请求头
//! 3. 下游服务调 `cats_rbac::service_helpers::extract_user_id_and_roles` 解析这两个头
//! 4. `require_roles` 拿解析出的 `Vec<Role>` 判权限
//!
//! 每一段单独都有测试，但**接缝没有**：
//! - 段 1 有 `issued_token_carries_roles_and_matching_roles_csv`（在
//!   `integration_auth.rs`），只验证 `roles_csv == roles.join(",")`
//! - 段 3 有 `parse_role` 的单测，只验证字符串能被解析
//!
//! 于是一个**只有接缝才会暴露**的缺陷可以完全静默地活着：比如
//! `Role` 变体改名、`parse_role` 不再接受 PascalCase、或者哪天有人把
//! `claim_to_headers` 映回 `roles` 数组 —— 每一段自己的测试都还是绿的，
//! 而线上表现为**全员 403**。这类"全线绿但功能全废"的失败，正是本文件要挡的。
//!
//! # 这不是 envoy 运行时验收
//!
//! 本文件只覆盖上面 4 段中的 **1 → 2 → 3**（按 envoy `claim_to_headers`
//! 的字符串语义在本地复现头注入）。它**不能**证明：
//! - envoy 真的验签通过
//! - `request_headers_to_remove` 真的剥离了客户端自带的头
//! - 整条 8080 链路真的通
//!
//! 那三项需要真 envoy + 真 Docker，当前受阻（per BACKEND_STATUS §4.1z）。

use actix_web::test::TestRequest;
use auth_service::auth::{issue_jwt, verify_jwt};
use cats_rbac::service_helpers::extract_user_id_and_roles;
use cats_rbac::Role;
use uuid::Uuid;

fn setup_jwt_secret() {
    std::env::set_var(
        "JWT_SECRET",
        "test_secret_at_least_32_bytes_long_for_hs256_xx",
    );
    std::env::set_var("JWT_EXPIRY_SECS", "3600");
    std::env::set_var("JWT_REFRESH_EXPIRY_SECS", "86400");
}

/// 复现 envoy `claim_to_headers` 对 string claim 的行为：**原样拷贝**。
///
/// 真实的 `deploy/envoy-mvp.yaml` / `deploy/k3s/cats-edge/envoy-deployment.yaml` 里是：
/// ```yaml
/// claim_to_headers:
///   - header_name: x-cats-user-id
///     claim_name: sub
///   - header_name: x-cats-roles
///     claim_name: roles_csv
/// ```
fn request_as_envoy_would_build(sub: &str, roles_csv: &str) -> actix_web::HttpRequest {
    TestRequest::default()
        .insert_header(("x-cats-user-id", sub.to_string()))
        .insert_header(("x-cats-roles", roles_csv.to_string()))
        .to_http_request()
}

// === 核心接缝 ===

/// 登录实际发出的角色（`handlers.rs::default_roles_for`）必须能被下游解析回同样的角色。
///
/// `default_roles_for` 的两条分支是 `["User"]` 与 `["User", "Sponsor"]`，
/// 这里是它们的**字面量镜像**。如果哪天 `default_roles_for` 改了而这里没改，
/// 本用例会先红 —— 这正是它要的效果：把"签发端写了什么"和"消费端认得什么"
/// 绑在同一个测试里，而不是各自断言各自的常量。
#[test]
fn roles_actually_issued_at_login_parse_into_the_roles_downstream_uses() {
    setup_jwt_secret();

    // default_roles_for 的两条真实分支
    for issued in [
        vec!["User".to_string()],
        vec!["User".to_string(), "Sponsor".to_string()],
    ] {
        let uid = Uuid::new_v4();
        let (token, _) = issue_jwt(uid, "someone", "access", issued.clone()).expect("issue_jwt");
        let claims = verify_jwt(&token).expect("verify_jwt");

        let req = request_as_envoy_would_build(&claims.sub, &claims.roles_csv);
        let (got_uid, got_roles) =
            extract_user_id_and_roles(&req).expect("边缘注入的头应被下游接受");

        assert_eq!(
            got_uid, uid,
            "X-Cats-User-Id 来自 sub，必须等于签发时的 user_id"
        );

        let expected: Vec<Role> = issued
            .iter()
            .map(|r| cats_rbac::service_helpers::parse_role(r).expect("签发的角色必须可解析"))
            .collect();
        assert_eq!(
            got_roles, expected,
            "roles_csv={:?} 解析出的角色与签发时不一致 —— 全线各自测试都是绿的，\
             但线上会全员 403",
            claims.roles_csv
        );
    }
}

/// `sub` 必须是 UUID —— 签发端传进来的 user_id 就是 UUID。
#[test]
fn sub_claim_round_trips_into_a_uuid() {
    setup_jwt_secret();
    let uid = Uuid::new_v4();
    let (token, _) =
        issue_jwt(uid, "someone", "access", vec!["User".to_string()]).expect("issue_jwt");
    let claims = verify_jwt(&token).expect("verify_jwt");

    let req = request_as_envoy_would_build(&claims.sub, &claims.roles_csv);
    let (got_uid, _roles) = extract_user_id_and_roles(&req).expect("应解析成功");
    assert_eq!(got_uid, uid);
}

/// 角色顺序必须保真 —— `require_roles` 会按序逐个判，顺序影响判定过程。
#[test]
fn role_order_is_preserved_through_the_csv_round_trip() {
    setup_jwt_secret();
    let issued = vec![
        "Sponsor".to_string(),
        "User".to_string(),
        "QualityLead".to_string(),
    ];
    let (token, _) =
        issue_jwt(Uuid::new_v4(), "someone", "access", issued.clone()).expect("issue_jwt");
    let claims = verify_jwt(&token).expect("verify_jwt");
    assert_eq!(claims.roles_csv, "Sponsor,User,QualityLead");

    let req = request_as_envoy_would_build(&claims.sub, &claims.roles_csv);
    let (_uid, got) = extract_user_id_and_roles(&req).expect("应解析成功");
    assert_eq!(
        got,
        vec![Role::Sponsor, Role::User, Role::QualityLead],
        "逗号串切分后顺序不得变化"
    );
}

// === 反向用例：钉住"为什么必须有 roles_csv" ===

/// **如果** envoy 的 `claim_to_headers` 映到 `roles` 数组而不是 `roles_csv`，
/// envoy 会把数组「序列化 JSON 再 Base64」——得到的头值长这样。
///
/// 这条用例把这个失败形态钉成**可执行的反例**：它必须解析失败（401），
/// 而不是解析出恰好正确的角色。这样一来，谁把 `claim_name` 改回 `roles`，
/// 就会在 CI 上立刻看到失败，而不是上线后看到全员 401。
#[test]
fn array_claim_form_is_rejected_rather_than_silently_mis_parsed() {
    // ["User","Sponsor"] 的 JSON 再 Base64（envoy 对 array claim 的处理）
    let as_envoy_would_encode_array = "WyJVc2VyIiwiU3BvbnNvciJd";
    let req =
        request_as_envoy_would_build(&Uuid::new_v4().to_string(), as_envoy_would_encode_array);

    let res = extract_user_id_and_roles(&req);
    assert!(
        res.is_err(),
        "数组形态的头值不应被解析成角色 —— 若这里通过，说明 `parse_role` 变得过宽，\
         会把 Base64 噪声当成合法角色名"
    );
}

/// 缺 `X-Cats-Roles` 必须是 401，不能被当成"零角色但已登录"。
#[test]
fn missing_roles_header_is_rejected() {
    let req = TestRequest::default()
        .insert_header(("x-cats-user-id", Uuid::new_v4().to_string()))
        .to_http_request();
    assert!(
        extract_user_id_and_roles(&req).is_err(),
        "只有 user_id 没有 roles 不足以通过边缘鉴权"
    );
}

/// 客户端自填的身份头在**边缘尚未剥离**时会被下游照单全收 —— 这条用例把
/// 风险写明，提醒 `request_headers_to_remove` 是不可省的第二道防线。
///
/// 注意它断言的是"会被接受"，不是"应该被接受"：下游**有意**不重复验签
/// （per 接口设计 v1.5 §1.2），所以唯一能拦住自填头的就是边缘的剥离。
/// 边缘那侧的保证由 lint 规则 16 静态守住（每条路由都必须有
/// `request_headers_to_remove`），运行时的实证仍受阻于 Docker。
#[test]
fn client_supplied_headers_are_trusted_by_downstream_by_design() {
    let req = request_as_envoy_would_build(&Uuid::new_v4().to_string(), "Sponsor");
    let (_uid, roles) = extract_user_id_and_roles(&req).expect("下游信任边缘注入的头");
    assert_eq!(
        roles,
        vec![Role::Sponsor],
        "这条用例记录的是**当前设计**：下游不验签，只信头。\
         拦住客户端自填头的责任全在 envoy 的 request_headers_to_remove 上。"
    );
}
