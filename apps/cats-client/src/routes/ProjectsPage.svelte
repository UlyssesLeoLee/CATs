<script lang="ts">
  import { onMount } from "svelte";
  import {
    createProject,
    getOfflineStatus,
    listProjects,
    syncOfflineQueue,
    type OfflineStatus,
    type Project,
  } from "../lib/api";
  import { navigate } from "../lib/router.svelte";

  let projects = $state<Project[]>([]);
  let status = $state<OfflineStatus | null>(null);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  // 创建项目表单
  let newName = $state("");
  let newSource = $state("zh");
  let newTarget = $state("en");

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    busy = true;
    errorMsg = null;
    try {
      const [list, st] = await Promise.all([listProjects(), getOfflineStatus()]);
      projects = list.projects;
      status = st;
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function submitCreate(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    busy = true;
    errorMsg = null;
    try {
      if (status && !status.online) {
        // 离线 → 走队列
        const { enqueueOfflineAction } = await import("../lib/api");
        await enqueueOfflineAction({
          action_type: "create_project",
          payload: {
            name: newName,
            source_lang: newSource,
            target_lang: newTarget,
          },
        });
      } else {
        await createProject({
          name: newName,
          source_lang: newSource,
          target_lang: newTarget,
        });
      }
      newName = "";
      await refresh();
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function doSync() {
    busy = true;
    try {
      const r = await syncOfflineQueue();
      errorMsg = `同步完成: ${r.succeeded}/${r.total} 成功, ${r.failed} 失败`;
      await refresh();
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  function openTranslate(project: Project) {
    // MVP: 跳到翻译页 + 简单把项目名带到 URL query
    location.hash = `#/translate?project_id=${encodeURIComponent(project.id)}`;
    navigate("translate");
  }
</script>

<section class="projects">
  <div class="toolbar">
    <h1>项目</h1>
    <button onclick={refresh} disabled={busy}>刷新</button>
  </div>

  {#if status}
    <div class="status" class:offline={!status.online}>
      <span>{status.online ? "🟢 在线" : "🟡 离线"}</span>
      <span>队列: {status.queue_length} 条</span>
      {#if status.queue_length > 0}
        <button onclick={doSync} disabled={busy}>同步队列</button>
      {/if}
      {#if status.last_sync_at}
        <span class="last">上次同步: {status.last_sync_at}</span>
      {/if}
    </div>
  {/if}

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <div class="grid">
    <div class="list">
      <h2>项目列表（{projects.length}）</h2>
      {#if projects.length === 0}
        <p class="placeholder">尚未创建项目</p>
      {:else}
        <ul>
          {#each projects as p (p.id)}
            <li>
              <button class="proj-btn" onclick={() => openTranslate(p)}>
                <strong>{p.name}</strong>
                <span class="meta">
                  {p.source_lang ?? "?"} → {p.target_lang ?? "?"}
                  {#if p.status}· <span class="status-badge">{p.status}</span>{/if}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <form class="create" onsubmit={submitCreate}>
      <h2>新建项目</h2>
      <label>
        <span>名称</span>
        <input bind:value={newName} required disabled={busy} />
      </label>
      <label>
        <span>源语 (BCP-47)</span>
        <input bind:value={newSource} maxlength="8" disabled={busy} />
      </label>
      <label>
        <span>目标 (BCP-47)</span>
        <input bind:value={newTarget} maxlength="8" disabled={busy} />
      </label>
      <button type="submit" disabled={busy || !newName.trim()}>
        {status && !status.online ? "入队" : "创建"}
      </button>
    </form>
  </div>
</section>

<style>
  .projects {
    padding: 1.5rem;
    max-width: 1200px;
    margin: 0 auto;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }
  h1 {
    margin: 0;
    font-size: 1.4rem;
  }
  .status {
    display: flex;
    gap: 1rem;
    align-items: center;
    padding: 0.5rem 1rem;
    background: #ecfdf5;
    border: 1px solid #a7f3d0;
    border-radius: 4px;
    margin-bottom: 1rem;
    font-size: 0.85rem;
  }
  .status.offline {
    background: #fef3c7;
    border-color: #fde68a;
  }
  .status .last {
    color: #64748b;
    font-size: 0.75rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1.5rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0 0 0.75rem;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .proj-btn {
    width: 100%;
    text-align: left;
    padding: 0.75rem 1rem;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    cursor: pointer;
  }
  .proj-btn:hover {
    background: #f8fafc;
    border-color: #0f172a;
  }
  .meta {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.75rem;
    color: #64748b;
  }
  .status-badge {
    background: #e2e8f0;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
  }
  .create {
    background: #fff;
    padding: 1rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .create label {
    display: flex;
    flex-direction: column;
    font-size: 0.8rem;
    color: #475569;
    gap: 0.3rem;
  }
  .create input {
    padding: 0.5rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
  }
  .create button {
    padding: 0.6rem;
    background: #0f172a;
    color: #fff;
    border: 0;
    border-radius: 4px;
  }
  .placeholder {
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    background: #fee2e2;
    padding: 0.5rem 1rem;
    border-radius: 4px;
  }
  button {
    padding: 0.5rem 1rem;
    background: #0f172a;
    color: #fff;
    border: 0;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  button:disabled {
    opacity: 0.6;
  }
</style>