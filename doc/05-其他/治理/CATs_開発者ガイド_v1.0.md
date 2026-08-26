# CATs 開発者ガイド v1.1

> **文档编号**：CATs-IMPL-054  
> **フェーズ**：54 コーディング + 编码规范 / Git 工作流 / 本地开发  
> **关联任务**：150 任务 #54、#56（CR）、#57（Build）  
> **版本**：v1.1（增 §5.5 UI/UX 规约，对齐 `CATs_UI-UX设计书_v1.1`）  
> **基线**：v1.0（2026-08-20）  
> **创建日**：2026-08-20  
> **更新**：v1.0 → v1.1（2026-08-26 增 §5.5）  
> **作者**：架构师 + Rust Lead + 前端 Lead（worker 代签 per DEC-008）

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| 架构师 | ☐ | — |
| Rust Lead | ☐ | — |
| 前端 Lead | ☐ | — |
| QA | ☐ | — |
| PM | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-20 | 架构师 | 评审前草稿：上手 + 编码 + Git + 本地 |
| **v1.1** | **2026-08-26** | **架构师 + Rust Lead + 前端 Lead** | **增 §5.5 UI/UX 规约（Design Token / 折叠屏 / 动态主题 / WCAG AAA / 极简速度 / PR 验收清单），对齐 `CATs_UI-UX设计书_v1.1`** |

---

## 1. 目的

为 CATs 项目开发者提供**统一的上手指南、编码规范、Git 工作流、本地开发流程**，作为：

- 新人入职（M1 起）的必读
- 代码评审（任务 56）的依据
- 编码一致性的基线
- 团队知识库的核心

---

## 2. 读者

- 后端开发者（Rust）
- 前端开发者（TypeScript / Next.js / Tauri）
- DBA
- QA（需读懂代码）
- SRE（需部署和调试）
- 新人（首日必读）

---

## 3. 上手指南（首日）

### 3.1 Day 1 检查清单

- [ ] 收到 GitHub Enterprise 账号
- [ ] 加入 cats-org org
- [ ] 克隆 `cats` 仓库
- [ ] 安装开发工具（Rust / Node / Docker / IDE）
- [ ] 运行 `bootstrap.sh`
- [ ] 启动本地 K8s（kind）
- [ ] 部署 dev 环境
- [ ] 启动服务
- [ ] 跑通 UT（`cargo test` + `pnpm test`）
- [ ] 提交第一个 PR（修改 README typo）
- [ ] 阅读本指南 §4-§10

### 3.2 必装工具

| 工具 | 版本 | 安装命令 |
|------|------|----------|
| Git | 2.42+ | https://git-scm.com |
| Rust | **最新 stable** | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`（跟随官方，2026-08-20 决议 QA-024） |
| cargo | 随 Rust | 随 Rust |
| Node.js | 20 LTS | nvm |
| pnpm | 9+ | `npm i -g pnpm` |
| Docker | 24+ | Docker Desktop |
| kind | latest | `brew install kind` |
| kubectl | 1.28+ | `brew install kubectl` |
| k9s | latest | `brew install k9s` |
| Sops | latest | `brew install sops` |

### 3.3 必读文档

1. 本指南（你正在读）
2. `CATs_微服务架构设计书 v1.0`
3. `CATs_接口设计书 v2.0`
4. `CATs_数据库设计书 v2.0`
5. `CATs_Rust 技术选型书 v1.0`
6. `CATs_CI_CD 構築運用手順書 v1.0`
7. `CATs_品質ゲート運用手順書 v1.0`

---

## 4. 编码规范（Rust）

### 4.1 风格

- **格式化**：`cargo fmt`（默认 rustfmt）
- **Lint**：`cargo clippy --all-targets --all-features -- -D warnings`
- **命名**：
  - 模块：`snake_case`
  - 类型：`PascalCase`
  - 函数：`snake_case`
  - 常量：`SCREAMING_SNAKE_CASE`
  - 枚举：`PascalCase`，变体 `PascalCase`
  - 特征：`PascalCase`
- **导入**：分组（std / 第三方 / 内部）
- **错误**：使用 `thiserror` / `anyhow`
- **日志**：使用 `tracing`（结构化）
- **文档**：`///` 公共 API 必须

### 4.2 异步

- 运行时：Tokio（QA-021 决议，唯一）
- 避免阻塞：用 `tokio::task::spawn_blocking`
- 取消：用 `tokio_util::sync::CancellationToken`
- 超时：所有 IO 调用必加 `.timeout()`

### 4.3 错误处理

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    
    #[error("invalid input: {0}")]
    InvalidInput(String),
    
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;
```

### 4.4 测试

- 单元测试：每个 module 一个 `#[cfg(test)] mod tests`
- 集成测试：`tests/` 目录
- Mock：`mockall`
- 覆盖率：≥ 80%
- 命名：`test_<func>_<scenario>_<expected>`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_input_valid_returns_ok() {
        let result = parse_input("hello");
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_fetch_user_not_found_returns_err() {
        let result = fetch_user("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
```

### 4.5 注释

- `///` 公共 API
- `//` 复杂逻辑
- `// TODO:` 待办
- `// FIXME:` 已知 bug
- `// SAFETY:` unsafe 块
- 避免无意义注释

### 4.6 安全

- 禁止 `unsafe`（除特定 FFI / 性能关键）
- 禁止 `unwrap()`（除测试）
- 禁止明文密钥
- 禁止 SQL 拼接
- 强制 `#[must_use]`

---

## 5. 编码规范（TypeScript / 前端）

### 5.1 风格

- **格式化**：`prettier`
- **Lint**：`eslint`（@typescript-eslint）
- **命名**：
  - 变量 / 函数：`camelCase`
  - 类 / 类型 / 接口：`PascalCase`
  - 常量：`SCREAMING_SNAKE_CASE`（模块级）
  - 文件：`kebab-case.ts`
- **导入**：绝对路径（`@/components/...`）
- **类型**：禁用 `any`（必要时 `unknown` + 类型守卫）
- **null 检查**：可选链 `?.` + 空合并 `??`

### 5.2 React 规范

- 组件：函数组件 + Hooks
- 状态：优先 `useState`，复杂用 `useReducer`
- 全局：Zustand / Jotai（避免 Redux）
- 数据获取：TanStack Query
- 样式：Tailwind CSS + shadcn/ui
- 表单：react-hook-form + zod
- 测试：Vitest + Testing Library

### 5.3 命名文件

```
components/
  Button/
    Button.tsx
    Button.test.tsx
    index.ts
pages/        (or app/)
hooks/
utils/
types/
```

### 5.4 注释

- JSDoc 公共 API
- 复杂组件加 README.md
- 避免无意义注释

### 5.5 UI/UX 规约（v1.1 增 / 对齐 `CATs_UI-UX设计书_v1.1`）

> 完整设计语言见 `doc/02-基础设计/UI/CATs_UI-UX设计书_v1.1`。本节只列**开发落地必须遵循的硬规约**。

#### 5.5.1 Design Token 三层（强制）

- **禁止**在组件内硬编码颜色 / 间距 / 字号；
- **必须**通过 `tokens.ts` 引用：`color.text` 而非 `#1F2328`、`space.md` 而非 `12px`；
- 新增 token 必须先在 `packages/tokens/primitives.ts` 定义，再由 semantic / component 层引用；
- Figma → Token 同步管道由前端 Lead 维护（OI-2）。

#### 5.5.2 折叠屏适配（强制）

- 所有页面**必须**基于 `@container` + CSS Grid 实现响应式；
- 折叠屏展开态（≥ 900 px）默认进入**左：列表 / 右：详情**分栏；
- 桌面 Web（≥ 1400 px）必须支持**三栏布局**（项目 / 文档 / 协同面板）；
- 折叠 / 展开切换**禁止刷新页面**，用 `matchMedia` + 状态机实现 0.2s 过渡；
- 跨应用拖拽（浏览器 → Tauri）HTML5 Drag-and-Drop 必须支持，禁用 `event.preventDefault()` 阻断。

#### 5.5.3 动态主题（强制）

- **禁止**在 CSS 中写 `color: #000` / `color: #FFFFFF` 等绝对值；
- 深色模式背景**禁止**使用纯黑 `#000000`（光晕效应），用 `#0F1115`（已在 token）；
- 主题切换 < 200ms 完成，**禁止触发网络请求**；
- 用户偏好持久化到 `user.preferences.theme`（DB），跨设备同步；
- 长任务（≥ 30min）自动启用护眼暖色滤镜，用户可手动覆盖。

#### 5.5.4 WCAG 2.1 AAA（强制）

- 正文文本对比度 ≥ **7:1**（AAA），大文本 ≥ 4.5:1；
- **禁止仅靠颜色传达信息**：TM 命中、QA 警告必须图标 + 文字 + 颜色三冗余；
- **禁止**用 `outline: none` 替代焦点环，焦点环必须 2px 实线 + 4px offset；
- 协同自动保存、远端光标、LLM 流式输出必须 `aria-live="polite"` ；
- CI 卡点：axe-core + pa11y-ci + Lighthouse a11y ≥ 95，违例阻断 merge。

#### 5.5.5 极简速度预算（强制）

- **冷启动 → 编辑器可输入** ≤ 1.0s（P99）；
- **首页查词 → 释义显示** ≤ 0.5s（借鉴有道：1.5s 提速到 0.5s）；
- **TM 召回（debounce 200ms）** ≤ 200ms；
- 包体预算：Tauri 主包 ≤ 80MB；Web 首屏 JS ≤ 200KB gzip；LLM wasm ≤ 30MB；
- 性能数据实时上报 telemetry，P99 劣化 > 20% 持续 5min 触发 P2 告警；
- 核心功能区 / 拓展功能区**严格分层**（桌面 Web 3 Tab、Tauri 4 Tab、Chrome 扩展 1 Popup），禁止堆砌。

#### 5.5.6 验收清单（PR 必勾）

- [ ] 组件通过 Storybook a11y addon
- [ ] axe-core / pa11y-ci 0 违例
- [ ] Lighthouse a11y ≥ 95
- [ ] 包体增量 ≤ 阈值（Tauri 1MB / Web 10KB gzip）
- [ ] 折叠屏 / 桌面 Web 布局在 Playwright device emulation 截图通过
- [ ] 主题切换无 FOUC（Flash of Unstyled Content）
- [ ] Token 引用覆盖率 ≥ 95%（无硬编码）

#### 5.5.7 引用

- `CATs_UI-UX设计书_v1.1` §10-§14（有道设计语言 4 原则）
- `CATs_開発者ガイド_v1.1`（本节）
- `CATs_P1假设层决议_v1.1` §5（admin UI 段同步引用）

---

## 6. 编码规范（SQL）

### 6.1 风格

- 关键字大写
- 表/字段小写 snake_case
- 显式 JOIN
- 缩进 2 空格
- 注释：表与字段必加

### 6.2 必备

- 所有表：`created_at`, `updated_at`, `created_by`, `updated_by`
- 软删除：`deleted_at`
- 主键：UUID v7
- 时间戳：`TIMESTAMPTZ`
- JSON：`JSONB`
- 金额：`NUMERIC(precision, scale)`（不用 `FLOAT`）

### 6.3 安全

- 全部参数化查询
- 禁止字符串拼接
- 强制 RLS

### 6.4 性能

- 索引覆盖 WHERE / JOIN / ORDER BY
- 避免 SELECT *
- 避免 OFFSET 大分页（用 keyset）
- EXPLAIN 检查

---

## 7. Git 工作流

### 7.1 分支模型（Trunk-Based）

```
main ─────●──────●──────●──────●─────→ (production)
            \    /        \    /
             \  /          \  /
feat/xxx ────●            \/
                            ●
release/v1.0 ──────────────●──────● (release branch)
```

| 分支 | 用途 | 保护 | 命名 |
|------|------|------|------|
| `main` | 主干 | 强保护 | 固定 |
| `feat/*` | 功能分支 | 弱保护 | `feat/bff-add-cache` |
| `fix/*` | 修复分支 | 弱保护 | `fix/bff-null-deref` |
| `chore/*` | 杂项 | 弱保护 | `chore/deps-bump` |
| `release/*` | 发布分支 | 强保护 | `release/v1.2.0` |
| `hotfix/*` | 紧急修复 | 强保护 | `hotfix/bff-cve-2026-001` |

### 7.2 提交规范（Conventional Commits）

```
<type>(<scope>): <subject>

<body>

<footer>
```

| type | 含义 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(bff): add TM match endpoint` |
| `fix` | 修复 | `fix(tm): handle empty embedding` |
| `refactor` | 重构 | `refactor(tm): extract match service` |
| `perf` | 性能 | `perf(tm): add HNSW index` |
| `test` | 测试 | `test(bff): add cache tests` |
| `docs` | 文档 | `docs(readme): update setup` |
| `chore` | 杂项 | `chore(deps): bump tokio to 1.40` |
| `style` | 风格 | `style(bff): format code` |
| `ci` | CI | `ci(actions): add cache` |

### 7.3 PR 流程

1. 创建分支（`feat/xxx`）
2. 提交（多次小提交）
3. 推送
4. 创建 PR（用模板）
5. CI 自动跑（Lint / Test / SAST / Build）
6. 1+ 评审通过
7. Squash Merge 到 main
8. 删除分支

### 7.4 PR 模板

```markdown
## 目的
<!-- 为什么要做这个改动 -->

## 改动
<!-- 详细说明 -->

## 测试
- [ ] 单元测试通过
- [ ] 集成测试通过
- [ ] 手动测试完成

## 截图（如适用）

## 关联
- Issue: #123
- 文档: link
```

### 7.5 Code Review 清单

- [ ] 符合编码规范
- [ ] 无新增 lint 警告
- [ ] 测试覆盖率达标
- [ ] 无新增 SAST 警告
- [ ] 无密钥泄露
- [ ] 错误处理完整
- [ ] 日志规范
- [ ] 文档更新
- [ ] 性能可接受
- [ ] 无破坏性变更（或已沟通）

---

## 8. 本地开发

### 8.1 仓库结构

```
cats/
├── apps/
│   ├── bff/                  # BFF 服务
│   ├── browser-ext/          # 浏览器扩展
│   └── web/                  # Web 控制台
├── services/                 # 15 微服务
│   ├── tm-service/
│   ├── term-service/
│   ├── mt-service/
│   ├── ...
│   └── audit-svc/
├── libs/                     # 共享库
│   ├── cats-common/          # 通用工具
│   ├── cats-error/           # 错误类型
│   ├── cats-grpc/            # gRPC 桩
│   └── cats-db/              # 数据库封装
├── migrations/               # 数据库迁移
├── proto/                    # gRPC proto
├── openapi/                  # OpenAPI 规范
├── k8s/                      # K8s 清单
├── scripts/                  # 工具脚本
├── docs/                     # 文档
└── tests/                    # E2E 测试
```

### 8.2 启动开发环境

```bash
# 1. 克隆
git clone https://github.com/cats-org/cats.git
cd cats

# 2. 安装依赖
./scripts/bootstrap.sh

# 3. 启动本地 K8s + 部署
make dev-up

# 4. 跑迁移
make migrate

# 5. 启动单个服务（开发模式）
cargo run -p bff

# 6. 启动前端
pnpm --filter web dev
```

### 8.3 调试技巧

#### Rust

```bash
# 单元测试
cargo test -p bff

# 集成测试
cargo test -p bff --test '*'

# 带日志
RUST_LOG=debug cargo run -p bff

# 远程调试
cargo build && lldb target/debug/bff
```

#### 前端

```bash
# 开发模式
pnpm --filter web dev

# 构建
pnpm --filter web build

# 类型检查
pnpm --filter web typecheck

# 测试
pnpm --filter web test
```

#### 数据库

```bash
# 连接
make db-shell

# 查看慢查询
make db-slow

# 重建
make db-reset
```

### 8.4 常见问题

| 问题 | 答案 |
|------|------|
| 端口被占用 | `lsof -i :8080` + kill |
| Docker 慢 | 检查资源分配（≥ 4 CPU / 8G） |
| 编译慢 | `cargo clean` + 重新构建 |
| 迁移失败 | 检查 `migrations/_journal` + 重置 |
| K8s 起不来 | `kind delete cluster` + 重建 |

---

## 9. 测试规范

### 9.1 单元测试（UT）

- 与代码同文件
- 覆盖率 ≥ 80%
- 必须覆盖：正常路径 + 异常路径 + 边界
- 命名：`test_<func>_<scenario>_<expected>`

### 9.2 集成测试（IT）

- `tests/` 目录
- 真实数据库（testcontainers）
- 真实消息队列
- 覆盖：服务间调用 + 数据库 + 外部 API

### 9.3 E2E 测试

- Playwright
- 浏览器自动化
- 覆盖：用户场景

### 9.4 性能测试

- k6 / wrk
- CI 跑冒烟
- ST 阶段跑完整

### 9.5 安全测试

- SAST（clippy / semgrep）
- DAST（OWASP ZAP）
- SCA（trivy / snyk）
- 渗透测试（ST 末）

---

## 10. 调试与排错

### 10.1 日志

- 结构化日志（tracing / pino）
- 必含字段：timestamp / level / service / trace_id / span_id
- 级别：ERROR / WARN / INFO / DEBUG / TRACE
- 生产：INFO（默认）/ 排错时 DEBUG
- 关键事件：登录 / 权限 / 数据 CRUD / 错误

### 10.2 Tracing

- OpenTelemetry
- Jaeger / Tempo
- 必含：trace_id 贯穿全链路
- 采样率：dev 100% / prod 10%

### 10.3 指标

- Prometheus
- 必含：QPS / 延迟 / 错误率 / 饱和度
- RED 方法：Rate / Error / Duration
- USE 方法：Utilization / Saturation / Errors

### 10.4 排错流程

1. 看监控（指标异常？）
2. 看 tracing（慢在哪？）
3. 看日志（错误信息？）
4. 本地复现
5. 修复 + 测试
6. 回归

---

## 11. 发布流程

### 11.1 功能发布

```
feat 分支 → PR → main → 自动部署 staging
                              ↓
                          测试通过
                              ↓
                       release 分支
                              ↓
                  ArgoCD 部署 pre-prod
                              ↓
                       测试通过 + CAB
                              ↓
                  ArgoCD 部署 prod（Canary）
                              ↓
                       监控正常
                              ↓
                          100% 流量
```

### 11.2 紧急修复

```
hotfix 分支 → PR（强评审）→ main
                              ↓
                  ArgoCD 部署 prod（立即）
                              ↓
                       监控 + 回滚预案
```

详见 `CATs_CI_CD_構築運用手順書_v1.0.md` §8。

---

## 12. 安全开发

- 禁止 `unsafe`（除特批）
- 禁止 `unwrap()`（除测试）
- 禁止明文密钥入库
- 强制参数化查询
- 强制 SAST
- 强制依赖审计（cargo audit / pnpm audit）
- 强制密钥扫描（gitleaks）
- 强制 CODEOWNERS

---

## 13. 知识库

| 主题 | 位置 |
|------|------|
| 架构 | `docs/02-基础设计\` |
| 设计 | `docs/03-详细设计\` |
| 测试 | `docs/04-测试\` |
| 部署 | `docs\05-其他\治理\` |
| Runbook | 即将建（M3） |
| ADR | 即将建（M1） |

---

## 14. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_CI_CD 構築運用手順書 v1.0 | `05-其他\治理\` |
| CATs_品質ゲート運用手順書 v1.0 | `05-其他\治理\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_接口设计书 v2.0 | `03-详细设计\接口设计\` |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` |
| CATs_Rust 技术选型书 v1.0 | `02-基础设计\技术选型\` |
| CATs_安全要件定义书 v1.0 | `05-其他\安全\` |
| CATs_SQL 设计一览 v1.0 | `03-详细设计\SQL\` |

---

## 15. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | CODEOWNERS 文件 | 架构师 | M1-S0 |
| OI-2 | Pre-commit hooks 配置 | Rust Lead | M1-S0 |
| OI-3 | 本地测试 seed data | DBA | M1-S0 |
| OI-4 | IDE 配置同步（settings.json / .vscode） | 前端 Lead | M1-S0 |
| OI-5 | 知识库 wiki 平台选型 | PM | M1-S0 |
| **OI-6** | **§5.5 UI/UX 规约 6 项 PR 验收清单（Storybook a11y / axe / Lighthouse / 包体 / 折叠屏 / 主题）落地** | **前端 Lead + QA** | **M1-S1 末** |
| **OI-7** | **§5.5.2 折叠屏 Playwright device emulation 截图测试** | **前端 Lead** | **M1-S1 末** |
| **OI-8** | **§5.5.5 包体预算 CI 卡点（≤ 80MB Tauri / ≤ 200KB Web JS）** | **SRE + 前端 Lead** | **M1-S0 末** |
| **OI-9** | **§5.5.4 WCAG AAA axe-core / pa11y-ci CI 集成** | **QA + 前端** | **M1-S2 末** |

---

**文档结束（v1.1）**
