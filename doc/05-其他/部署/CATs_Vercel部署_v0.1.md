# CATs Vercel 部署 v0.1 (cats-client 静态预览)

| 项 | 值 |
|---|---|
| 文档版本 | v0.1 |
| 日期 | 2026-09-20 |
| 作者 | 架构师 (MinimaxM3 per ULYS-110) |
| 状态 | 🟡 代码就绪 + 配置就绪 + **真实部署待用户 run** |
| Issue | [ULYS-110](https://multica.ai/issues/ULYS-110) |

---

## §1 真实状态 (诚实披露)

### §1.1 已落地 ✅ (本轮 commit)

- `apps/cats-client` Svelte 5 + Vite 5 静态构建可生产就绪 (本机实测 `npm run build` 937 ms,产物 `dist/index.html` 406 B + JS 47.67 KB + CSS 5.64 KB)
- `apps/cats-client/src/lib/api.ts` 改造: Tauri 运行时调用真实 `invoke()`, 浏览器/Vercel 走 `webInvoke()` mock (返回空状态而不是 ReferenceError)
- `apps/cats-client/src/App.svelte` header 加 `web preview` 橙色 badge, 明确告知用户当前是浏览器预览不是 Tauri 桌面
- `apps/cats-client/package.json` 加 `@tauri-apps/api ^2.0.0` 为 `optionalDependencies` (避免 web 构建缺包)
- 仓根 `vercel.json`: buildCommand/outputDirectory/rewrites(全 SPA hash router)/cache headers
- 仓根 `.vercelignore`: 排除 Rust target / node_modules / .git
- `.github/workflows/deploy-vercel.yml`: push-to-main 自动 production 部署 / 其他分支 preview 部署 / 手动 `workflow_dispatch`

### §1.2 仍需用户手动操作 ⚠️

1. **首次创建 Vercel 项目** — 在 https://vercel.com/new 选 CATs repo, framework 选 "Other", Root Directory 留空, Build/Output 命令用 `vercel.json` 里的默认
2. **填 GitHub Actions secret** (或者改用 Vercel GitHub App 自动):
   - `VERCEL_TOKEN` — Personal Token from https://vercel.com/account/tokens
   - `VERCEL_ORG_ID` — `vercel teams ls` 或 Vercel Settings → General
   - `VERCEL_PROJECT_ID` — 创建项目后从 `.vercel/project.json` 读
3. **推 trigger 分支** — 当前 agent 分支 `agent/minimaxm3/ce1e3d60f6b2` 不在 trigger 列表, merge 到 `main` 或 `dev` 后 CI 才会跑

### §1.3 不能上 Vercel 的部分 ❌ (诚实披露)

- **Rust 后端 12 service + cats-bff + cats-ai-gateway** — Vercel 是 serverless/edge, 不能跑长连接 Rust 服务. 当前 binary 编译又卡在 rustc 1.98 metadata bug (per BACKEND_STATUS_v0.1 §2). 即便编译过, 也只能走 [Fly.io / Render / Railway / 自托管 K3s](doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md §ADR-15 §ADR-16 §ADR-18).
- **PostgreSQL 18.6 + Kafka 3.7.1** — Vercel 无 Postgres/Kafka. 仍走自托管 K3s + CloudNativePG Operator.
- **apps/cats-client 的真实功能** — 在 Vercel 上是 **静态预览壳**, `get_offline_status` / `auth_login` / `list_projects` / `fetch_translation_lookup` / `create_project` / `enqueue_offline_action` / `sync_offline_queue` 全部返回 mock 空状态或抛 "requires Tauri runtime". 仅 LoginPage / ProjectsPage / TranslatePage UI 框架 + hash router 真实运行.

---

## §2 部署架构图

```
[GitHub repo CATs]
      │
      ├── push to main/dev  ──┐
      │                       │
      └── workflow_dispatch ──┤
                              ▼
            [GitHub Actions: deploy-vercel.yml]
              ubuntu-latest, Node 22
                              │
              ┌───────────────┴────────────────┐
              │  npm ci @ apps/cats-client     │
              │  npm run build → dist/         │
              └────────────────┬───────────────┘
                               ▼
                  [vercel deploy --prod]
                               │
                               ▼
            [Vercel CDN: cats-client-web-preview.vercel.app]
                  静态 SPA + 缓存 (1y immutable)
                  真实功能 = 0, 仅 UI shell
```

---

## §3 手工 (不用 CI) 部署步骤

> 给用户做一次手动部署演练用.

```bash
# 0. 前置: 安装 vercel CLI + 登录
npm install -g vercel@latest
vercel login --token <VERCEL_TOKEN>   # 或浏览器交互登录

# 1. 在仓根拉一次 project info
vercel pull --yes --environment=production

# 2. 部署 production
vercel deploy --yes --prod

# 3. 部署 preview (任意 feature branch)
vercel deploy --yes
```

URL 形如 `https://cats-client-web-preview-<hash>-<team>.vercel.app`.

---

## §4 链接 / 引用

- [CATs_技术选型书_v2.0 §ADR-15](doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md) — Web 控制台/BFF 用 Next.js (未落地), 明确说 "自托管不依赖 Vercel 云服务"
- [CATs_技术基线_v1.0 §1](doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md) — Tauri 2.x + Svelte 5 客户端
- [CATs_微服务架构设计书_v1.0 §4.1](doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md) — 16 服务清单 (含 cats-bff)
- [BACKEND_STATUS_v0.1 §2](deploy/BACKEND_STATUS_v0.1.md) — 12 service binary 因 rustc 1.98 metadata bug 未编译
- [V1.1-PLAN §6.9](V1.1-PLAN.md) — 接口设计书 v2.0 升版路径 (Sprint 3 排期)

---

## §5 升版触发 (per §8.1 PMO 守门)

- [ ] Next.js Web 控制台落地 → 切换 Vercel build target 到 `apps/web-console/` (需新建项目)
- [ ] Rust 后端 binary 编译通过 → 后端服务走 Fly.io/Render, 不再依赖 Vercel
- [ ] `cats-bff` 暴露 HTTP gateway → Vercel preview 可在浏览器中代理 invoke() 到真后端 (消除 mock)

---

> Permanent commit per ULYS-110 决策 (待 Ulysses 拍板 token). 守门 #14 v3 + 诚实披露约束.
