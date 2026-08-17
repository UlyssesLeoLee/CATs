# OFCAT 技术选型书

**系统名称:** OFCAT — AI 增强型 CAT 浏览器工作台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | OFCAT-TS-001 |
| 文档名 | 技术选型书（含选型决策记录 ADR） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-06-25 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [需求定义书 v1.1](../../01-需求/需求规格说明/OFCAT_需求定义书_v1.1.md)、[基础设计书 v1.0](../架构设计/OFCAT_基础设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | 初版。各层技术选型对比、评分与决策记录 |

---

## 1. 前言

### 1.1 目的
记录 OFCAT 各技术层的**候选方案对比、评分与最终决策**，为基础设计书的方式选定提供依据，并保留决策理由以便后续回溯（ADR）。

### 1.2 评估维度与权重
| 维度 | 权重 | 说明 |
|---|---|---|
| 适配性 | 30% | 与需求/约束（C1–C6）、生态的契合度 |
| 体验/性能 | 25% | 对分层延迟与极简体验的支撑 |
| 可维护性 | 20% | 长期维护、社区、稳定性 |
| 落地成本 | 15% | 开发/打包/运维成本 |
| 合规/安全 | 10% | 对数据合规与安全的支撑 |

> 评分 1–5（5 最佳）。综合分 = Σ(维度分 × 权重)。

---

## 2. 选型总览

| 层 | 决策 | 主要替代 | 综合分 |
|---|---|---|---|
| 部署形态 | 薄扩展 + 本地引擎 | 纯扩展 / 纯桌面端 | 4.6 |
| 扩展 UI | Svelte + TS（Vite/CRXJS） | React / 原生 | 4.3 |
| 引擎语言 | Python + FastAPI | Node + Fastify | 4.4 |
| 编排 | LangGraph | 自研状态机 | 4.2 |
| 存储 | SQLite + sqlite-vec | Qdrant / DuckDB | 4.5 |
| TM 匹配 | RapidFuzz + bge-m3 | 纯向量 / 纯编辑距离 | 4.5 |
| AI 网关 | LiteLLM + 自研合规路由 | 全自研 / OpenRouter | 4.3 |
| 本地模型 | Qwen 系（Ollama/vLLM） | Llama / DeepSeek 本地 | 4.4 |
| 嵌入模型 | bge-m3 | multilingual-e5 | 4.2 |
| 通信 | localhost HTTP+SSE+Token | Native Messaging / WebSocket | 4.3 |
| OCR（后续） | PaddleOCR | tesseract.js / 云 OCR | 4.1 |
| PDF（后续） | pdf.js | PDFium | 4.2 |
| 引擎打包 | PyInstaller + 托盘 | Tauri sidecar / Docker | 3.9 |

---

## 3. 选型决策记录（ADR）

### ADR-01 部署形态：薄扩展 + 本地引擎
- **背景**：需求要求 SQLite/本地 LLM/OCR/编排（C1），MV3 沙箱无法承载。
- **候选**：①纯扩展（WASM SQLite/tesseract）；②薄扩展+本地引擎；③纯桌面端。
- **决策**：②。扩展只做捕获/UI/写回，重逻辑下沉本地引擎。
- **理由**：一次性解决 SQLite/PaddleOCR/本地 LLM/LangGraph 落地；扩展轻、稳、可独立升级。
- **取舍**：需用户安装伴随程序（已在需求确认可接受）。

### ADR-02 扩展 UI：Svelte + TypeScript
- **候选**：React / Svelte / 原生 TS。
- **决策**：Svelte + TS，Vite + CRXJS 构建。
- **理由**：overlay 注入到任意页面，需极小体积与高渲染性能，Svelte 无虚拟 DOM、产物小；TS 保证契约。
- **取舍**：团队若更熟 React 可换；架构不依赖该选择。

### ADR-03 引擎语言：Python + FastAPI
- **候选**：Python(FastAPI) / Node(Fastify)。
- **决策**：Python + FastAPI。
- **理由**：PaddleOCR、LangGraph、sentence-transformers、sqlite-vec、RapidFuzz 均为 Python 一等公民；FastAPI 原生 async + SSE。
- **取舍**：单文件打包不如 Node 轻 → 用 PyInstaller + 托盘解决（ADR-13）。

### ADR-04 编排：LangGraph
- **候选**：LangGraph / 自研状态机。
- **决策**：LangGraph，MVP 即采用。
- **理由**：翻译管道本就是多节点有状态流程；后续多 Agent（OCR/投票/总结）可平滑扩展，避免重写。
- **取舍**：MVP 阶段略重 → 仅用其图与状态能力，节点保持轻量同步函数。

### ADR-05 存储：SQLite + sqlite-vec
- **候选**：SQLite(+sqlite-vec) / Qdrant / DuckDB。
- **决策**：SQLite + sqlite-vec。
- **理由**：本地权威副本（C4）、零运维、可备份迁移；sqlite-vec 在同一文件内提供向量 KNN，省去独立向量库。
- **取舍**：超大规模向量性能不及 Qdrant → 当前 TM 量级（O1 假设≤50 万）足够，超出再评估。

### ADR-06 TM 匹配：RapidFuzz（主）+ bge-m3 嵌入（辅）
- **候选**：纯编辑距离 / 纯向量 / 混合。
- **决策**：混合——RapidFuzz 算相似度为主排序，向量做语义召回补充。
- **理由**：CAT 的 fuzzy % 语义与编辑距离一致，用户预期可解释；纯向量会把"改了数字"的高相似句判低。
- **取舍**：维护两套索引 → 向量为可选增强，默认编辑距离即可用。

### ADR-07 AI 网关：LiteLLM + 自研合规路由
- **候选**：全自研 / LiteLLM / OpenRouter（云中转）。
- **决策**：LiteLLM 统一多家协议，外包一层自研合规路由。
- **理由**：LiteLLM 已支持 OpenAI/Claude/Gemini/DeepSeek 及本地 OpenAI 兼容端点，省去适配；合规（C3）是自有逻辑，必须自研并 fail-closed。
- **取舍**：OpenRouter 会把内容过第三方，违反合规，排除。

### ADR-08 本地模型：Qwen 系（Ollama/vLLM）
- **候选**：Qwen / Llama / DeepSeek 本地。
- **决策**：Qwen2.5/Qwen3 系；单机 Ollama，团队共享 GPU 用 vLLM。
- **理由**：中日英综合最强、指令遵循好（利于术语注入）；Ollama 部署最省事，vLLM 并发优。
- **取舍**：显存受限可选更小档位 Qwen；接口走 OpenAI 兼容，切换无成本。

### ADR-09 嵌入模型：bge-m3
- **决策**：bge-m3（多语种、长文本、检索强）。
- **替代**：multilingual-e5。二者均可，bge-m3 对 CJK 与混合语种表现更稳。

### ADR-10 通信：localhost HTTP + SSE + Bearer Token
- **候选**：Native Messaging / localhost HTTP / WebSocket。
- **决策**：localhost HTTP（SSE 流式）+ 随机令牌 + Origin 白名单。
- **理由**：实现/调试简单，SSE 天然支持流式首字；令牌+Origin 防任意网页越权与 DNS rebinding。
- **取舍**：Native Messaging 无端口更隐蔽但消息大小/流式受限、调试难 → 备选。

### ADR-11 OCR（后续）：PaddleOCR
- **决策**：PaddleOCR（中日英、表格/版面强）。**必须**在本地引擎（无法进浏览器），印证 ADR-01。
- **替代**：tesseract.js（精度不足）、云 OCR（合规风险）。

### ADR-12 PDF（后续）：pdf.js
- **决策**：pdf.js 渲染+文本层；版面坐标对齐为主要工作量。
- **替代**：PDFium（更重的集成）。

### ADR-13 引擎打包：PyInstaller + 系统托盘
- **决策**：PyInstaller 打包单可执行 + 托盘常驻 + 开机自启选项。
- **替代**：Tauri sidecar（额外 Rust 壳）、Docker（对内部非技术用户过重）。
- **取舍**：首启动较慢/体积大 → 可接受；后续评估 Tauri 做统一安装壳。

---

## 4. 被否决方案与理由

| 方案 | 否决理由 |
|---|---|
| 纯浏览器扩展（含 WASM SQLite/OCR） | MV3 沙箱无法稳定承载 OCR/本地 LLM/长任务，体验与能力受限 |
| Qdrant 向量库 | 独立服务、需运维，违背本地零运维（C4）；当前量级 sqlite-vec 足够 |
| OpenRouter 等云中转网关 | 内容经第三方，违反数据合规（C3） |
| 多模型投票进默认链 | 与极简/低延迟体验冲突，改为手动 L3 |
| 纯向量 TM 匹配 | fuzzy % 不可解释，"改数字"类高相似句被判低，违背 CAT 预期 |

---

## 5. 选型相关风险

| 风险 | 影响 | 对策 |
|---|---|---|
| R-01 富文本编辑器写回破坏文档 | 高 | 早期对 ProseMirror/CodeMirror 做写回 PoC（F8） |
| R-02 PyInstaller 包体大/杀软误报 | 中 | 代码签名；评估 Tauri 壳 |
| R-03 本地模型显存/性能不足 | 中 | 提供模型档位选择；vLLM 共享部署 |
| R-04 sqlite-vec 规模上限 | 中 | 监控规模，超阈值切分或迁移向量库 |
| R-05 合规误判（漏判敏感） | 高 | fail-closed + 域名清单可维护 + 审计（O6） |

---

## 6. 结论
以「薄扩展 + Python 本地引擎 + LangGraph 编排 + SQLite/sqlite-vec + LiteLLM 网关 + Qwen 本地模型」为基线技术栈，满足需求约束 C1–C6 与分层延迟目标，且各层均有平滑的演进/替换路径。详细版本号与依赖清单在详细设计阶段固化。
