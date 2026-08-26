# CATs ADR-008：JWT 密钥轮换与刷新策略

> **文档编号**：CATs-ADR-008
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师(worker 代签 per DEC-008)
> **状态**：已接受
> **取代**：—

---

## 1. 背景

ADR-005 已落定认证走 Keycloak 多租户方案，但 JWT 的"密钥管理"与"刷新机制"未在 ADR-005 中细化。CATs 浏览器工作台场景下：

- 用户会话较长(翻译任务 / 文档编辑常驻 4-8 小时)
- 多租户隔离要求每个租户独立的签名密钥(避免单租户密钥泄露波及其他租户)
- 合规要求(等保 2.0 / GDPR)：定期轮换签名密钥，最长 24 小时
- 浏览器 BFF 入口：HTTP 端，不能让前端接触长期凭证

QA-073 已登记为 Open 项，要求 JWT 密钥轮换与刷新给出明确策略。要决策：

- **Token 寿命**：短 vs 长 vs 自适应
- **刷新机制**：静默刷新 vs 显式刷新 vs 滚动轮换
- **密钥轮换**：频率、租户隔离、Keycloak 配置
- **撤销语义**：泄露场景下如何快速废止已签发 token

## 2. 选项

| 选项 | 描述 | 优势 | 劣势 |
|------|------|------|------|
| **A. 短 Access(15min) + Refresh(7d, 滚动轮换)** ✅ | Access JWT 短寿命，Refresh Token HttpOnly cookie + 滚动轮换 | 泄露窗口小、撤销快、用户体验好、Keycloak 原生支持 | Refresh Token 状态需服务端持久化(Redis) |
| **B. 长 Access(24h) + 黑名单** | 单 token 长期有效，泄露后服务端拉黑 | 实现简单 | 泄露窗口 24h 不符合等保；黑名单同步延迟；Redis 单点 |
| **C. 混合：长短并存** | 内部服务长(1h)、外部 BFF 短(15min) | 内部调用免刷 | 内外不一致增加复杂度；运维需双策略 |

补充说明：

- 选项 C 看似灵活，但内部服务调用 gRPC 走 mTLS(参见 ADR-009)而非 JWT，JWT 仅在 BFF ↔ 浏览器层使用，混合策略失去意义
- Keycloak 26.x 原生支持 Refresh Token 旋转(Refresh Token Rotation)，符合选项 A 的"滚动轮换"

## 3. 决策

**采用 A：短 Access JWT(15 分钟) + Refresh Token(7 天，HttpOnly + Secure + SameSite=Strict cookie，滚动轮换)；Keycloak 管理签名密钥，每租户独立。**

实施细则：

1. **Access JWT**
   - 算法：RS256(非对称，公钥由 Keycloak JWKS endpoint 暴露)
   - 寿命：15 分钟
   - Payload：`sub` / `tenant_id` / `roles` / `iat` / `exp` / `jti`
   - BFF 校验后注入 `X-User-Id` / `X-Tenant-Id` 头传递给下游 gRPC(参见 ADR-002 / ADR-005)
2. **Refresh Token**
   - 寿命：7 天绝对过期(滑出窗口)
   - 存储：HttpOnly + Secure + SameSite=Strict cookie，键名 `__Host-cats_refresh`
   - **滚动轮换**：每次用 Refresh Token 换 Access 时，同时签发新 Refresh Token 并废弃旧值(rotation)
   - **重放检测**：若已轮换的旧 Refresh Token 再次出现 → 全会话撤销 + 告警
3. **签名密钥管理(Keycloak 侧)**
   - 每租户独立 realm，每 realm 独立签名密钥对
   - Keycloak 内置密钥轮换：默认 24 小时自动轮换(符合等保 2.0)
   - 旧公钥在 JWKS 中保留 1 个 grace period(供未过期 Access Token 校验)
4. **撤销策略**
   - 用户主动登出：撤销 Refresh Token 状态(Redis 黑名单 TTL = 剩余寿命)
   - 凭证泄露：管理员通过 admin 域触发 realm 级密钥轮换 + 全会话强制重登
   - Refresh Token 轮换重放：自动撤销 + 飞书告警到 SRE
5. **共享库**
   - `common-auth` 提供 `JwtVerifier` 与 `RefreshTokenStore` 抽象(参见 ADR-001 / ADR-005 共享库分层)

## 4. 影响

- **正面**：
  - 15 分钟泄露窗口远低于等保 2.0 上限，合规压力小
  - 滚动轮换 + 重放检测覆盖"Refresh Token 泄露"主路径
  - 签名密钥每租户独立 + 24h 轮换，租户隔离完备
  - 与 Keycloak 26.x 原生能力对齐，无自研轮换逻辑
- **负面**：
  - Refresh Token 状态写 Redis → 引入新依赖(Redis 已在技术选型 024 中决策)
  - BFF 每 15 分钟自动刷新一次前端无感，但弱网环境会感知延迟 → 需前端 loading 兜底
  - 密钥轮换期间 JWKS 双密钥并存，需客户端缓存容错(常见 JWT 库已支持)
- **风险**：
  - Redis 不可用时 Refresh 流程降级为"重新登录"→ 需在 `common-auth` 中实现降级开关
  - JWKS 端点被 DDoS → BFF 缓存公钥(TTL 5 分钟)+ 失败重试
  - 轮换 grace period 与 Access Token 寿命不匹配导致校验失败 → 已在决策 §3.3 规定 grace period ≥ Access Token 寿命

## 5. 关联

- **上游**：ADR-005(Keycloak + 多租户)、ADR-001(共享库 `common-auth`)
- **下游**：BFF 详细设计 v1.0(Refresh 端点契约)、可观测性平台设计 v1.0(JWT 验证 / 撤销指标)
- **阻塞项**：QA-073(JWT 密钥轮换)
