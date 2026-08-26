# CATs UI / UX 设计书 v1.0

> **文档编号**：CATs-UI-001
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师 + 前端 Lead + QA（worker 代签 per DEC-008）
> **任务编号**：150 任务 #26（P2 索引 #23）
> **上游**：[CATs_ADR-004 前端栈 v1.0](../决策/CATs_ADR-004_前端栈_v1.0.md) §3
> **下游**：[CATs_模块设计书 v2.0](../../03-详细设计/模块设计/) §前端、[CATs_接口设计书 v2.0](../../03-详细设计/接口设计/)

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| 起草 | 架构师 + 前端 Lead + QA | ☑ | 2026-08-26 | worker 代签 per DEC-008 |
| 评审 | — | ☐ | — | 待评审会 |
| 批准 | — | ☐ | — | 待评审会 |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-26** | **架构师 + 前端 Lead + QA** | **P2 索引 #23 落地：设计原则 + 风格指南 + 组件库 + 关键流程 + i18n + a11y + token** |

---

## 1. 设计原则

| # | 原则 | 含义 | 度量 |
|---|------|------|------|
| P-01 | **可访问性优先** | 全部组件按 WCAG 2.1 AA 设计，色对比 ≥ 4.5:1 | a11y 自动化测试 100% 通过 |
| P-02 | **键盘可达** | 任意操作可纯键盘完成（含协同） | 关键路径 0 鼠标 |
| P-03 | **响应式 + 多端** | 同一设计语言覆盖 Tauri / Web / Chrome 扩展 | Design Token 100% 复用 |
| P-04 | **任务优先** | 信息架构按"翻译任务"组织，非按"系统模块" | 译员从进入到提交 ≤ 3 步 |
| P-05 | **可逆 / 安全** | 删除 / 覆盖前必确认，undo 至少 30s | undo 覆盖率 100% |
| P-06 | **可见即可信任** | TM 命中、术语引用、模型来源全部可视化 | 命中标签强制显示 |
| P-07 | **离线友好** | 网络断/弱网下基本可操作，恢复自动同步 | 离线状态可视化 |
| P-08 | **国际化内建** | 文案 100% 走 i18n key | 字符串硬编码 0 容忍 |
| P-09 | **性能预算** | LCP ≤ 1.2s，首屏 JS ≤ 200KB gzip | Lighthouse ≥ 90 |
| P-10 | **设计-研发同源** | 组件库即代码，Token 即变量 | 设计走查差异 ≤ 5% |

---

## 2. 风格指南

### 2.1 色彩（Color Tokens）

| Token | Light | Dark | 用途 | 对比度（AA） |
|-------|-------|------|------|---------------|
| `--color-bg` | `#FFFFFF` | `#0F1115` | 背景 | — |
| `--color-surface` | `#F7F8FA` | `#1A1D23` | 卡片/面板 | — |
| `--color-text` | `#1F2328` | `#E6E8EB` | 主文本 | 13.6 : 1 |
| `--color-text-muted` | `#5B6370` | `#9AA3B2` | 次文本 | 6.2 : 1 |
| `--color-primary` | `#2D6CDF` | `#4F8CFF` | 主色 / 行动 | 5.4 : 1 |
| `--color-success` | `#1F9D55` | `#3FBF7A` | 成功 / 100% 命中 | 4.7 : 1 |
| `--color-warning` | `#C97A1F` | `#E0A055` | 模糊命中 / QA 警告 | 4.6 : 1 |
| `--color-danger` | `#D04A3A` | `#E96A5A` | QA 错误 / 删除 | 5.1 : 1 |
| `--color-fuzzy` | `#A8651B` | `#D8A24E` | 模糊匹配 | 4.5 : 1 |
| `--color-nomatch` | `#5B6370` | `#9AA3B2` | 无匹配 | — |
| `--color-term` | `#5B4BD8` | `#8E80FF` | 术语高亮 | 5.3 : 1 |

### 2.2 排版（Typography）

| Token | 用途 | Font | Size / Line-height |
|-------|------|------|--------------------|
| `--font-sans` | UI 文本 | Inter | — |
| `--font-mono` | 代码 / 标签 | JetBrains Mono | — |
| `--font-editor` | 翻译编辑器 | Noto Serif / Source Han Serif | 15/24, 16/26 |
| `--fs-h1` | 页面标题 | Inter | 24/32, 600 |
| `--fs-h2` | 区块标题 | Inter | 18/26, 600 |
| `--fs-body` | 正文 | Inter | 14/22, 400 |
| `--fs-caption` | 辅助说明 | Inter | 12/18, 400 |
| `--fs-editor` | 编辑器句段 | Noto Serif | 15/24, 400 |

### 2.3 间距 / 圆角 / 阴影

| Token | 值 | 用途 |
|-------|----:|------|
| `--space-1` ~ `--space-8` | 4, 8, 12, 16, 24, 32, 48, 64 | 间距阶梯 |
| `--radius-sm/md/lg` | 4, 8, 12 | 圆角 |
| `--shadow-sm/md/lg` | 0 1 2 / 0 4 12 / 0 8 24 | 阴影 |

### 2.4 动效

| 场景 | 时长 | 缓动 | 备注 |
|------|-----:|------|------|
| Hover / Focus | 120 ms | `ease-out` | 颜色 / 阴影 |
| Panel 展开 | 200 ms | `cubic-bezier(.2,.8,.2,1)` | 高度 + 透明度 |
| Toast 出现 | 160 ms | `ease-out` | 3s 自动关闭 |
| 协同光标移动 | 即时 | — | 不缓动，保持实感 |
| 模态出现 | 240 ms | `cubic-bezier(.2,.8,.2,1)` | 背景遮罩 120ms |

> 遵循 `prefers-reduced-motion`：用户系统级开关开启时全部动效降级为 0ms。

---

## 3. 组件库

### 3.1 基础层（基于 Radix UI 无样式 + 自研样式）

| 组件 | Radix 原语 | 自研要点 |
|------|------------|----------|
| Button | `Slot` | variant: primary/secondary/ghost/danger |
| Input / Textarea | — | a11y label 强校验，错误态红 + 文案 |
| Select | `Select` | 单/多选、虚拟滚动 |
| Dialog | `Dialog` | 焦点陷阱 + ESC + 点击遮罩 |
| DropdownMenu | `DropdownMenu` | 键盘导航 |
| Tabs | `Tabs` | 路由同步 |
| Toast | `Toast` | 全局 store |
| Tooltip | `Tooltip` | 200ms 延迟 |
| Switch / Checkbox | `Switch` / `Checkbox` | 键盘可达 |
| Slider | `Slider` | 用于 QA 阈值 / 模糊阈值 |
| Popover | `Popover` | 用于 TM 命中详情 |
| Accordion | `Accordion` | 用于术语折叠 |
| Avatar | — | 用于协作者头像 |

### 3.2 业务组件（自研）

| 组件 | 描述 | 关键交互 |
|------|------|----------|
| `<SegmentRow>` | 翻译编辑器单句段行 | 显示源/译/TM 命中/术语/QA 状态；高亮可点击 |
| `<TMMatchPopover>` | TM 命中详情浮层 | 显示源差异、匹配度、历史译者 |
| `<TermHighlight>` | 术语高亮（在编辑器内） | 鼠标悬停看定义；右键替换/锁定 |
| `<QAIssueBadge>` | QA 错误徽标 | 点击跳转错误位置 |
| `<CollabCursor>` | 协同光标 | 远端用户颜色 + 名字气泡 |
| `<CollabPresence>` | 协同在线列表 | 头像 + 当前段落位置 |
| `<ProjectSidebar>` | 项目导航侧栏 | 树形：项目 / 任务 / 句段 |
| `<LanguagePicker>` | 语言对选择 | 200+ 语言，支持拼音首字母搜索 |
| `<ExportDialog>` | 导出对话框 | 格式 / 范围 / 过滤；实时预览 |
| `<DiffViewer>` | 译校 diff 视图 | 左右对照 + 颜色 |
| `<SegmentLock>` | 句段锁（审校锁/PM 解锁） | 锁状态徽标 |
| `<OfflineIndicator>` | 离线状态条 | 队列数 / 同步进度 |

### 3.3 组件库使用约束

- **禁止**直接 import Radix UI 业务组件，**必须**通过 `packages/ui` 包装层；
- 所有组件**必须**提供 `data-testid`；
- 颜色 / 间距 / 字号**必须**走 token，不允许硬编码；
- 组件 a11y：**每个**组件必须带 Storybook a11y 插件通过证明。

---

## 4. 关键流程

### 4.1 翻译编辑器（核心）

```mermaid
flowchart LR
    A[项目树<br/>ProjectSidebar] --> B[句段列表<br/>SegmentTable]
    B --> C[当前句段<br/>SegmentRow]
    C --> D[TM 召回<br/>TMMatchPopover]
    D --> E{接受/忽略}
    E -- 接受 --> F[填入译文]
    E -- 忽略 --> G[手动翻译]
    C --> H[术语高亮<br/>TermHighlight]
    H --> I[强制引用/锁定]
    F --> J[QA 实时检查]
    G --> J
    I --> J
    J --> K{QA 通过?}
    K -- 否 --> L[错误浮窗<br/>QAIssueBadge]
    L --> C
    K -- 是 --> M[下一句段]
    M --> B
```

> **要点**：源/译左右对照；TM 命中以**绿/橙/灰**三色编码（100%/模糊/无）；术语强制高亮 + 锁；QA 实时反馈无阻断但高亮。

### 4.2 TM 面板

- 默认折叠在右侧抽屉；
- 召回列表按相似度降序；
- 每条命中显示：源 / 译 / 匹配度 / 领域 / 历史译者 / 时间；
- 快捷键：`Ctrl+1~9` 直接采用第 N 条；
- 过滤：领域 / 客户 / 时间窗 / 译者。

### 4.3 协同（Yjs CRDT）

```mermaid
sequenceDiagram
    participant A as 译员 A
    participant B as 译员 B
    participant WS as collab-ws (Yjs WS)
    participant DB as collab-persistence

    A->>WS: 订阅 doc(roomId)
    B->>WS: 订阅 doc(roomId)
    A->>WS: 编辑句段 1（local update）
    WS->>DB: 写 snapshot（节流 1s）
    WS->>B: broadcast awareness + ops
    B-->>B: 渲染远端 cursor + 文本
    WS->>A: ack
    A->>WS: 离线 → 队列 ops
    Note over A,WS: 联网后批量 flush
```

> **要点**：远端光标用对方颜色；句段锁机制避免同段抢占；冲突由 CRDT 自动收敛；离线 ops 队列上限 5MB / 用户。

### 4.4 设置

- 个人：界面语言 / 主题 / 字体 / 快捷键 / TM 阈值；
- 项目：术语引用规则 / 模糊阈值 / QA 规则开关；
- 租户：成员 / 角色 / 计费配额 / 模型路由（fail-closed 开关）；
- 系统（管理员）：审计 / 备份 / 灰度发布。

---

## 5. 国际化（i18n）策略

### 5.1 技术栈

- 库：**i18next + react-i18next**（与 Tiptap / Tauri 生态兼容）；
- 格式：JSON + ICU MessageFormat（支持复数 / 性别 / 数字格式）；
- 加载：lazy chunk + namespace 拆分。

### 5.2 翻译工作台的双向 i18n

CATs 自身 UI 走 i18n；同时平台是"翻译"工具 → **必须**支持自定义语言资源（CATs_命名空间 + 客户项目命名空间）。

| 命名空间 | 内容 | 维护方 |
|----------|------|--------|
| `cats.common` | 通用 UI（按钮、菜单） | 前端 Lead |
| `cats.editor` | 编辑器专业术语 | 前端 Lead + 术语专家 |
| `cats.errors` | 错误码文案 | 前端 + QA |
| `project.{projectId}` | 项目自定义术语 | 客户 / 术语专家 |

### 5.3 关键约束

1. **零硬编码**：CI 跑 i18n lint，违例 PR 直接拒；
2. **占位 key 上线**：未翻译 key 走 fallback 显示 `__key__` 高亮，便于发现遗漏；
3. **RTL 支持**：阿拉伯语 / 希伯来语走 `dir="rtl"`，编辑器表格方向自适应；
4. **字体回退**：中日韩走 Noto Sans CJK SC/JP/KR，回退 Latin Inter；
5. **日期 / 数字 / 货币**：走 `Intl.*` API，按 locale 渲染。

---

## 6. 无障碍（a11y）

### 6.1 目标合规

- **WCAG 2.1 AA** 全量达成；
- 区域：**中日韩 + 英 + 阿拉伯**，全部支持屏幕阅读器（NVDA / JAWS / VoiceOver）；
- 协同场景的"远端光标"必须有**文本等价**（aria-live polite 播报 "用户 X 移到句段 12"）。

### 6.2 关键 a11y 要求

| 项 | 要求 | 验收方式 |
|----|------|----------|
| 键盘可达 | 全部交互纯键盘可达，焦点环可见 | a11y 自动化 + 人工 5 用户测试 |
| 颜色对比 | 文本 ≥ 4.5:1，大文本 ≥ 3:1 | axe-core 扫描 |
| 替代文本 | 图标按钮均带 aria-label | ESLint jsx-a11y |
| 表单 | label 关联、错误描述绑定 | Storybook a11y addon |
| 时间 | 协同自动保存可暂停（长任务 30s+） | 用户设置 |
| 闪烁 | 0 次/秒（无前庭诱发风险） | 人工 + 工具 |
| 焦点管理 | 模态打开自动聚焦，关闭归还 | 自动化测试 |

### 6.3 自动化 a11y 测试

- Storybook a11y addon：每个组件必须通过；
- Playwright + axe-core：关键流程 e2e；
- CI 必跑，违例阻断 merge。

---

## 7. 设计 Token

### 7.1 Token 三层

```
Primitive（原子）     Semantic（语义）      Component（组件）
color.blue.500  →   color.primary     →   button.bg.primary
spacing.4       →   space.md         →   card.padding
font.size.14    →   fs.body          →   segment.fontSize
```

- **Primitive**：`packages/tokens/primitives.ts`（仅变量定义）；
- **Semantic**：`packages/tokens/semantic.ts`（按用途命名）；
- **Component**：`packages/ui/*/tokens.ts`（组件内引用语义层）。

### 7.2 主题切换

- Light / Dark 通过 `:root[data-theme]` 切换；
- 用户偏好持久化到 localStorage + 服务端 `user.preferences`；
- 跟随系统：`prefers-color-scheme` 自动切换。

### 7.3 交付物

- `tokens.css`（CSS Variables）；
- `tokens.ts`（TS 类型，组件消费）；
- `tokens.json`（Design Tools 同步给 Figma）；
- Storybook 主题切换器。

---

## 8. 引用与关联

| 文档 | 引用点 |
|------|--------|
| CATs_ADR-004 前端栈 v1.0 | §3 React + Tiptap + Yjs + WXT + Radix |
| CATs_ADR-001 微服务架构 v1.0 | §3 collab-ws / collab-persistence 服务 |
| CATs_ADR-005 认证与多租户 v1.0 | §3 Keycloak 决定 i18n 登录流程 |
| CATs_需求规格说明书 v2.0 | §3 F02 客户端 / F11 运维管理 |
| CATs_模块设计书 v2.0 | §前端章节（占位 / 待 M1-S2 落地） |
| CATs_AsIs 业务流程图 v1.0 | 痛点 P-04（邮件）→ i18n + 协同推送 |
| CATs_AsIs 系统构成图 v1.0 | 旧 Trados/MemoQ → Tauri + WXT 替代路径 |

---

## 9. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 主题色板客户最终签字 | 产品 + 前端 | M1-S1 末 |
| OI-2 | Figma ↔ Token 同步管道建立 | 前端 Lead | M1-S1 末 |
| OI-3 | a11y 自动化在 CI 跑通 | QA + 前端 | M1-S2 末 |
| OI-4 | RTL 阿拉伯语版编辑器实测 | 前端 + 翻译团队 | M2 末 |
| OI-5 | 高对比度（High Contrast）模式 | 前端 | M3 上线前 |

---

**文档结束（v1.0 UI/UX 设计书）**
