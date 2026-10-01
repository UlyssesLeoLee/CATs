# vercelserver — CATs Vercel 测试服务器 (独立可复用部署套件)

| 项 | 值 |
|---|---|
| 目录 | `vercelserver/` |
| 用途 | Vercel 测试服务器 — 静态部署 CATs 3 个 mock 应用 |
| 设计原则 | **独立可复用** — 可整目录复制到任何静态项目, 设 `SOURCE_ROOT` env 即可构建 |
| 不依赖 | Edge Functions / Serverless Functions / 任何 Vercel 特有能力 — 纯静态 |
| 平移目标 | Vercel / Cloudflare Pages / Netlify / GitHub Pages / S3+CloudFront |

---

## §1 目录结构

```
vercelserver/
├── README.md                  # 本文件
├── vercel.json                # Vercel 路由 + cache headers
├── .vercelignore              # 排除 build 产物
├── scripts/
│   └── build.sh               # 从 SOURCE_ROOT 拉取 + 构建 + 复制
├── .github/
│   └── workflows/
│       └── deploy-vercel.yml  # push trigger 自动 vercel deploy --prod
└── src/                       # Vercel outputDirectory
    ├── index.html             # landing page (含 3 app 入口卡片)
    ├── styles.css
    ├── cats-client/           # Svelte 5 + Vite 静态构建 (构建产物, gitignored)
    ├── web-console-mock/      # Next.js 控制台 mock (静态 HTML, 复制)
    └── media-pipeline-mock/   # 多模态预处理管道 mock (静态 HTML, 复制)
```

## §2 3 个子应用

| 路径 | 内容 | 来源 | 真实功能 |
|---|---|---|---|
| `/` (landing) | index.html + 3 卡片入口 | `vercelserver/src/` 自带 | 0 (静态) |
| `/cats-client/` | CATs 客户端 Svelte 5 | `apps/cats-client/` (Vite build) | 0 (mock) |
| `/web-console-mock/` | Next.js 控制台 mock | `deploy/web-console-mock/` | 0 (mock) |
| `/media-pipeline-mock/` | MMDPGE 多模态管道 mock | `vercelserver/src/media-pipeline-mock/` | 0 (mock) |

3 个 mock 都标了橙色 "MOCK" badge, 诚实地标识非真实功能。

## §3 本地开发

```bash
# 前置: 构建源 (从主项目拉取)
cd vercelserver
bash scripts/build.sh

# 起本地 HTTP 服务
cd src
python -m http.server 8080
# 访问:
#   http://127.0.0.1:8080/                    (landing)
#   http://127.0.0.1:8080/cats-client/       (客户端)
#   http://127.0.0.1:8080/web-console-mock/  (控制台 mock)
#   http://127.0.0.1:8080/media-pipeline-mock/ (管道 mock)
```

## §4 部署到 Vercel

### 4.1 首次 (手动)

```bash
# 安装 vercel CLI (一次性)
npm install -g vercel@latest

# 登录 (一次性, 或用 token)
vercel login --token <VERCEL_TOKEN>

# 在 vercelserver/ 目录下跑
cd vercelserver
vercel deploy --prod
```

### 4.2 持续集成 (push trigger)

`.github/workflows/deploy-vercel.yml` 在 `main` / `dev` push 时自动:

1. checkout repo
2. run `bash vercelserver/scripts/build.sh`
3. install vercel CLI
4. `vercel deploy --prod --token=$VERCEL_TOKEN`

需要在 GitHub repo 填 3 个 secret:
- `VERCEL_TOKEN`
- `VERCEL_ORG_ID`
- `VERCEL_PROJECT_ID`

### 4.3 复用指南 (移植到其他项目)

```bash
# 1. 复制整目录
cp -r path/to/cats/vercelserver path/to/your-project/vercelserver

# 2. 设置 source 指向新项目
cd path/to/your-project
SOURCE_ROOT=. bash vercelserver/scripts/build.sh

# 3. 部署
cd vercelserver
vercel deploy --prod
```

`build.sh` 通过 `SOURCE_ROOT` env 变量决定从哪拉源, 默认 `..` (父目录)。任意项目只要满足:

- `apps/cats-client/` 是 Svelte 5 + Vite 项目, 有 `npm run build` 脚本
- `deploy/web-console-mock/` 是静态 HTML+CSS 目录 (可选)
- `vercelserver/src/media-pipeline-mock/` 是静态 HTML+CSS 目录 (可选, 若无则跳过)

即可直接复用, **无需修改 vercelserver/ 内任何代码**。

## §5 路由表 (vercel.json rewrites)

| URL pattern | 落地文件 | 备注 |
|---|---|---|
| `/` | `src/index.html` | landing |
| `/cats-client/` | `src/cats-client/index.html` | SPA — 所有子路径都 fallback 到 index.html |
| `/cats-client/(.*)` | `src/cats-client/index.html` | hash router 走这里 |
| `/web-console-mock/` | `src/web-console-mock/index.html` | 静态 |
| `/web-console-mock/(.*)` | `src/web-console-mock/index.html` | 静态 |
| `/media-pipeline-mock/` | `src/media-pipeline-mock/index.html` | 静态 |
| `/media-pipeline-mock/(.*)` | `src/media-pipeline-mock/index.html` | 静态 |

## §6 cache headers

| 路径 | Cache-Control |
|---|---|
| `/(.*)/assets/(.*)` | `public, max-age=31536000, immutable` (1 年) |
| `/(.*)` | `X-Vercel-Test-Server: true` (标识用途) + `X-Frame-Options: SAMEORIGIN` |

## §7 不打算做的事 (诚实边界)

- ❌ 不会添加 Edge Functions / Serverless API — 纯静态, 平移无依赖
- ❌ 不会替换 cats-client 的真实 Tauri runtime — 仍是 web preview mock
- ❌ 不会接 Vercel 付费 features (Analytics / Speed Insights / Web Vitals) — 测试用不需要
- ❌ 不会进 monorepo Cargo workspace — vercelserver/ 与 Rust crates 完全解耦

## §8 升版触发 (per §8.1 PMO 守门)

- [ ] Next.js 控制台真实落地 → 替换 `web-console-mock/` 为真实 Next.js build 产物
- [ ] MMDPGE + VAD + 关键帧服务代码就绪 → 替换 `media-pipeline-mock/` 为可交互 demo
- [ ] cats-bff HTTP gateway 上线 → cats-client 移除 mock invoke

## §9 引用

- 仓根 `vercel.json` (本目录存在前, 由 ULYS-110 commit c0e99b0 创建) — 现已弃用, 由 `vercelserver/vercel.json` 取代
- `deploy/web-console-mock/` — Next.js 控制台静态 mock (per ULYS-110 commit b6d19db)
- `apps/cats-client/` — Svelte 5 客户端 (per ULYS-110 commit c0e99b0 + 07bb918)
- Issue: ULYS-110
- 守门 #14 v3 + 诚实披露约束生效中
