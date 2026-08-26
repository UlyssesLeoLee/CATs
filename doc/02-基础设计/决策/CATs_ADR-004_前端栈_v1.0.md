# CATs ADR-004：前端技术栈选型

> **文档编号**：CATs-ADR-004
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师 + 前端 Lead
> **状态**：已接受
> **取代**：—

---

## 1. 背景

CATs 是浏览器优先的 SaaS，前端需要：

- 富文本编辑器（CAT 核心 UI）
- 实时协同（Yjs CRDT）
- 复杂状态管理（多面板 + 多视图）
- 高频交互（TM 召回建议、QA 标记、术语高亮）
- Chrome 扩展（CAT 浏览器工作台的核心形态）

## 2. 选项

| 框架 | 优势 | 劣势 | 决策 |
|------|------|------|------|
| **React 18 + TypeScript** ✅ | 生态最广、Tiptap/ProseMirror 适配、富文本方案成熟 | Bundle 体积、并发模式复杂度 | 采纳 |
| Vue 3 | 学习曲线平缓 | 富文本与协同生态较弱 | 否决 |
| Svelte / SvelteKit | 体积小、性能好 | 生态不成熟、招聘难 | 否决 |
| Solid | 性能极佳 | 生态极小 | 否决 |

**配套选型**：

| 维度 | 选型 | 备注 |
|------|------|------|
| 富文本内核 | **Tiptap**（基于 ProseMirror） | 与 Yjs 原生集成 |
| 状态管理 | **Zustand + TanStack Query** | 局部态 / 服务态分离 |
| 路由 | **TanStack Router** | 类型安全 |
| 构建 | **Vite 5** | 快速 HMR、ESM 原生 |
| 测试 | **Vitest + Testing Library + Playwright** | 单测/E2E |
| Chrome 扩展 | **WXT（Web Extension Framework）** | 现代化框架 |
| UI 库 | **Radix UI + 自研样式** | 无样式可访问性 + 设计系统 |

## 3. 决策

**React 18 + TypeScript + Vite + Tiptap + Yjs + WXT**。

前端 Monorepo 结构：

```
apps/
  web/         # 主 SaaS Web
  ext/         # Chrome 扩展（WXT）
packages/
  ui/          # 共享组件（Radix 包装）
  editor/      # Tiptap 扩展 + 协同绑定
  protocol/    # 与 BFF 通信的类型化客户端
  i18n/        # 翻译工作台的语言资源
```

## 4. 影响

- **正面**：React + Tiptap + Yjs 是 CAT 工具事实标准（Trados/MemoQ 路线），招聘与生态友好
- **负面**：
  - 需维护 Monorepo（pnpm workspaces + Turborepo）
  - Chrome 扩展与 Web 共享代码需谨慎（DOM 抽象）
  - 富文本 + 协同的复杂度高，需专人对接
- **风险**：
  - 前端 Lead 招聘窗口（QA-009）→ 已在 PMO 跟踪

## 5. 关联

- 上游：CATs_需求规格说明书_v2.0 §UI
- 下游：CATs_模块设计书_v2.0 §前端、CATs_接口设计书_v2.0
- 阻塞项：QA-009
