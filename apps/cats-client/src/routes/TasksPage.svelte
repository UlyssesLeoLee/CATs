<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    listLocalTasks,
    listTaskEvents,
    runtimeMode,
    updateLocalTaskStatus,
    type LocalTask,
    type LocalTaskEvent,
  } from "../lib/api";

  // 列表
  let tasks = $state<LocalTask[]>([]);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  // 选中任务详情
  let selectedTaskId = $state<string | null>(null);
  let selectedEvents = $state<LocalTaskEvent[]>([]);
  let loadingEvents = $state(false);

  // 自动刷新: 每 5 秒拉一次本地任务列表 (SSE 真推送待 BFF 升级, 见 notice)
  let pollTimer: number | null = null;

  // 状态机: pending → running → completed / failed / cancelled
  const STATUS_FLOW = ["pending", "running", "completed", "failed", "cancelled"] as const;
  type Status = (typeof STATUS_FLOW)[number];

  function statusColor(s: string): string {
    switch (s) {
      case "running":
        return "#1d4ed8";
      case "completed":
      case "succeeded":
        return "#16a34a";
      case "failed":
        return "#b91c1c";
      case "cancelled":
        return "#64748b";
      default:
        return "#ca8a04";
    }
  }

  onMount(async () => {
    await refresh();
    pollTimer = window.setInterval(refresh, 5000);
  });

  onDestroy(() => {
    if (pollTimer !== null) window.clearInterval(pollTimer);
  });

  async function refresh() {
    busy = true;
    errorMsg = null;
    try {
      const r = await listLocalTasks({ limit: 100 });
      tasks = r.tasks;
      // 重新选中的任务详情
      if (selectedTaskId) {
        await loadEvents(selectedTaskId);
      }
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function loadEvents(taskId: string) {
    loadingEvents = true;
    try {
      const r = await listTaskEvents({ task_id: taskId });
      selectedEvents = r.events;
    } catch (e: unknown) {
      // 不覆盖主列表错误
      console.warn("listTaskEvents failed:", e);
    } finally {
      loadingEvents = false;
    }
  }

  function selectTask(t: LocalTask) {
    selectedTaskId = t.task_id;
    void loadEvents(t.task_id);
  }

  async function markStatus(taskId: string, newStatus: Status) {
    try {
      await updateLocalTaskStatus({ task_id: taskId, new_status: newStatus });
      await refresh();
    } catch (e: unknown) {
      errorMsg = String(e);
    }
  }

  function truncate(s: string, n: number): string {
    return s.length > n ? s.slice(0, n) + "…" : s;
  }

  function parsePayload(json?: string | null): Record<string, unknown> | null {
    if (!json) return null;
    try {
      return JSON.parse(json);
    } catch {
      return null;
    }
  }
</script>

<section class="tasks-page">
  <header class="bar">
    <h1>任务列表 (切片 D)</h1>
    <div class="nav-links">
      <a href="#/translate">翻译</a>
      <a href="#/tm">TM</a>
      <a href="#/glossary">术语</a>
      <a href="#/projects">项目</a>
    </div>
  </header>

  <p class="notice">
    ⓘ 当前实现: <code>POST /v1/tasks</code> dispatch 成功后写入
    <strong>{runtimeMode.isTauri() ? "本地 SQLite" : "会话状态"}</strong>。
    真 SSE 推送由 task-service 提供, 但 BFF 未代理 (M2 范畴) — 进度靠本地
    5 秒轮询 + 手动 <code>markStatus</code> 模拟. 等 BFF 升级后自动接入.
  </p>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <div class="grid">
    <!-- 左: 任务列表 -->
    <div class="task-list">
      <div class="list-header">
        <h2>已派发 ({tasks.length})</h2>
        <button onclick={refresh} disabled={busy}>{busy ? "…" : "刷新"}</button>
      </div>
      {#if tasks.length === 0}
        <p class="placeholder">尚未派发任何任务 — 去 <a href="#/translate">翻译页</a> 发起</p>
      {:else}
        <ul>
          {#each tasks as t (t.task_id)}
            <li>
              <button
                class="task-row"
                class:active={selectedTaskId === t.task_id}
                onclick={() => selectTask(t)}
              >
                <div class="row-1">
                  <span class="status" style="background: {statusColor(t.status)}">
                    {t.status}
                  </span>
                  <code class="tid">{truncate(t.task_id, 12)}</code>
                </div>
                <div class="row-2">{truncate(t.source_text, 60)}</div>
                <div class="row-3">
                  <span class="proj">{truncate(t.project_id, 16)}</span>
                  <span class="time">{t.updated_at}</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- 右: 详情 + 事件流 -->
    <div class="task-detail">
      {#if !selectedTaskId}
        <p class="placeholder">点击左侧任务查看详情与事件流</p>
      {:else}
        {@const selected = tasks.find((t) => t.task_id === selectedTaskId)}
        {#if selected}
          <div class="detail-header">
            <h2>任务详情</h2>
            <code>{selected.task_id}</code>
          </div>
          <dl>
            <dt>状态</dt>
            <dd>
              <span class="status" style="background: {statusColor(selected.status)}">
                {selected.status}
              </span>
            </dd>
            <dt>项目</dt>
            <dd><code>{selected.project_id}</code></dd>
            <dt>源文</dt>
            <dd class="src">{selected.source_text}</dd>
            {#if selected.target_text}
              <dt>译文</dt>
              <dd class="src">{selected.target_text}</dd>
            {/if}
            <dt>更新时间</dt>
            <dd>{selected.updated_at}</dd>
          </dl>

          <div class="actions">
            <span class="label">手动推进状态:</span>
            {#each STATUS_FLOW as s (s)}
              <button
                class="action"
                disabled={selected.status === s}
                onclick={() => markStatus(selected.task_id, s)}
              >
                → {s}
              </button>
            {/each}
          </div>

          <h3>事件流 ({selectedEvents.length})</h3>
          {#if loadingEvents}
            <p class="placeholder">载入事件…</p>
          {:else if selectedEvents.length === 0}
            <p class="placeholder">无事件</p>
          {:else}
            <ol class="events">
              {#each selectedEvents as e (e.event_id)}
                {@const p = parsePayload(e.payload_json)}
                <li>
                  <span class="ev-type" style="color: {statusColor(e.event_type)}">
                    {e.event_type}
                  </span>
                  <span class="ev-time">{e.recorded_at}</span>
                  {#if p}
                    <pre class="ev-payload">{JSON.stringify(p, null, 2)}</pre>
                  {/if}
                </li>
              {/each}
            </ol>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
</section>

<style>
  .tasks-page {
    padding: 1.5rem;
    max-width: 1400px;
    margin: 0 auto;
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  h1 {
    margin: 0;
    font-size: 1.4rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  h3 {
    font-size: 0.95rem;
    margin: 1rem 0 0.5rem 0;
  }
  .nav-links {
    display: flex;
    gap: 1rem;
    font-size: 0.85rem;
  }
  .nav-links a {
    color: #0f172a;
    text-decoration: none;
    padding: 0.25rem 0.5rem;
    border-radius: 3px;
  }
  .nav-links a:hover {
    background: #e2e8f0;
  }
  .notice {
    background: #fef3c7;
    border: 1px solid #fde68a;
    padding: 0.75rem 1rem;
    border-radius: 4px;
    font-size: 0.8rem;
    color: #78350f;
    margin: 0 0 1rem 0;
  }
  .notice code {
    background: #fde68a;
    padding: 0.05rem 0.3rem;
    border-radius: 2px;
    font-size: 0.75rem;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    background: #fee2e2;
    padding: 0.5rem;
    border-radius: 4px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.5fr;
    gap: 1.5rem;
  }
  .task-list,
  .task-detail {
    background: #fff;
    padding: 1rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
  }
  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .task-row {
    width: 100%;
    text-align: left;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    cursor: pointer;
    font: inherit;
  }
  .task-row:hover {
    background: #f1f5f9;
  }
  .task-row.active {
    background: #dbeafe;
    border-color: #1d4ed8;
  }
  .row-1 {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.2rem;
  }
  .row-2 {
    font-size: 0.85rem;
    color: #334155;
    margin-bottom: 0.2rem;
  }
  .row-3 {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: #64748b;
  }
  .status {
    color: #fff;
    padding: 0.1rem 0.5rem;
    border-radius: 3px;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .tid {
    font-size: 0.78rem;
    color: #475569;
  }
  .placeholder {
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .placeholder a {
    color: #1d4ed8;
  }
  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .detail-header code {
    font-size: 0.78rem;
    color: #475569;
  }
  dl {
    display: grid;
    grid-template-columns: 6rem 1fr;
    gap: 0.4rem 0.75rem;
    font-size: 0.85rem;
    margin: 0;
  }
  dt {
    color: #64748b;
  }
  dd {
    margin: 0;
    color: #1e293b;
  }
  .src {
    white-space: pre-wrap;
    background: #f8fafc;
    padding: 0.5rem;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.82rem;
  }
  .actions {
    margin: 1rem 0;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .label {
    color: #64748b;
    font-size: 0.8rem;
    margin-right: 0.4rem;
  }
  .action {
    padding: 0.3rem 0.7rem;
    font-size: 0.78rem;
    background: #fff;
    color: #1e293b;
    border: 1px solid #cbd5e1;
  }
  .action:hover:not(:disabled) {
    background: #f1f5f9;
  }
  .action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .events {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .events li {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 3px;
    padding: 0.5rem;
    font-size: 0.82rem;
  }
  .ev-type {
    font-weight: 600;
    margin-right: 0.5rem;
  }
  .ev-time {
    color: #64748b;
    font-size: 0.75rem;
  }
  .ev-payload {
    margin: 0.4rem 0 0 0;
    padding: 0.4rem;
    background: #fff;
    border-radius: 2px;
    font-size: 0.75rem;
    overflow-x: auto;
  }
  button {
    padding: 0.4rem 0.8rem;
    background: #0f172a;
    color: #fff;
    border: 0;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
