# 切片 D: 客户端 UI 深度功能（M2 范畴 / 本批不做）

**Slice**: D — 客户端 UI 补完
**目标**: `apps/cats-client/src/routes/*.svelte`
**优先级**: 🟡 M2（依赖切片 B 完成后才有 backend API）

## 不在本批子代理范围

本切片等切片 B（4 核心服务）落地后，backend API 稳定后再启动。

## 待办（仅作记录，不派工）

- `TranslatePage.svelte` — 真实翻译段落编辑 + 写回 + 标签保护 UI
- 新页 `GlossaryPage.svelte` — 术语条目浏览 + 新增
- 新页 `TmPage.svelte` — TM 条目浏览 + 100%/模糊匹配可视化
- 新页 `TasksPage.svelte` — 任务列表 + SSE 进度

## 引用

- 父: ULYS-125
- 审计: §4.4
- 客户端 README: `apps/cats-client/README.md`
