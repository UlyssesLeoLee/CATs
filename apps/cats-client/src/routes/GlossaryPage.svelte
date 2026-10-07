<script lang="ts">
  import { onMount } from "svelte";
  import {
    addLocalGlossaryEntry,
    listLocalGlossaryEntries,
    runtimeMode,
    type LocalGlossaryEntry,
  } from "../lib/api";
  import { navigate } from "../lib/router.svelte";

  // 表单状态
  let sourceTerm = $state("");
  let targetTerm = $state("");
  let domain = $state("");
  let notes = $state("");
  let projectId = $state(""); // 可空 = 全局

  // 列表状态
  let entries = $state<LocalGlossaryEntry[]>([]);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let infoMsg = $state<string | null>(null);

  // 后端未暴露状态: 客户端先把术语落到本地 SQLite, BFF / translation-core 升级后
  //   再批量同步 (per ULYS-154 §honest scope)
  const BACKEND_NOT_READY = !runtimeMode.isTauri() ? "web preview" : "本地 SQLite";

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    busy = true;
    errorMsg = null;
    try {
      const r = await listLocalGlossaryEntries({ limit: 100 });
      entries = r.entries;
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function submitAdd(e: SubmitEvent) {
    e.preventDefault();
    if (!sourceTerm.trim() || !targetTerm.trim()) return;
    busy = true;
    errorMsg = null;
    infoMsg = null;
    try {
      const r = await addLocalGlossaryEntry({
        source_term: sourceTerm.trim(),
        target_term: targetTerm.trim(),
        domain: domain.trim() || null,
        notes: notes.trim() || null,
        project_id: projectId.trim() || null,
      });
      infoMsg = `已添加: ${r.entry.source_term} → ${r.entry.target_term}`;
      // 清空表单 (保留 domain/notes 方便连续录入)
      sourceTerm = "";
      targetTerm = "";
      await refresh();
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  function sourceProjectLabel(e: LocalGlossaryEntry): string {
    return e.project_id ? `项目 ${e.project_id}` : "全局";
  }
</script>

<section class="glossary">
  <header class="bar">
    <h1>术语库</h1>
    <div class="nav-links">
      <a href="#/translate">翻译</a>
      <a href="#/tasks">任务</a>
      <a href="#/tm">TM</a>
      <a href="#/projects">项目</a>
    </div>
  </header>

  <p class="notice">
    ⓘ 当前实现: 术语条目写入 <strong>{BACKEND_NOT_READY}</strong>。
    后端 BFF / translation-core 升级暴露 glossary browse 后,
    自动批量同步 (per 切片 D §honest scope).
  </p>

  <div class="grid">
    <!-- 左: 新增表单 -->
    <form class="add-form" onsubmit={submitAdd}>
      <h2>新增术语</h2>
      <label>
        <span>源术语 <em>*</em></span>
        <input bind:value={sourceTerm} required disabled={busy} />
      </label>
      <label>
        <span>目标术语 <em>*</em></span>
        <input bind:value={targetTerm} required disabled={busy} />
      </label>
      <label>
        <span>领域 (可选)</span>
        <input
          bind:value={domain}
          placeholder="产品名 / 品牌 / 法律 / ..."
          disabled={busy}
        />
      </label>
      <label>
        <span>备注 (可选)</span>
        <textarea bind:value={notes} rows="2" disabled={busy}></textarea>
      </label>
      <label>
        <span>项目 ID (留空 = 全局)</span>
        <input bind:value={projectId} disabled={busy} />
      </label>
      <button type="submit" disabled={busy || !sourceTerm.trim() || !targetTerm.trim()}>
        {busy ? "保存中…" : "保存"}
      </button>
    </form>

    <!-- 右: 列表 -->
    <div class="list">
      <div class="list-header">
        <h2>已有术语 ({entries.length})</h2>
        <button onclick={refresh} disabled={busy}>刷新</button>
      </div>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}
      {#if infoMsg}
        <p class="info">{infoMsg}</p>
      {/if}

      {#if entries.length === 0}
        <p class="placeholder">尚未录入任何术语</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>源</th>
              <th>目标</th>
              <th>领域</th>
              <th>作用域</th>
              <th>录入时间</th>
            </tr>
          </thead>
          <tbody>
            {#each entries as e (e.entry_id)}
              <tr>
                <td>{e.source_term}</td>
                <td>{e.target_term}</td>
                <td class="muted">{e.domain ?? "—"}</td>
                <td>
                  <span class="scope" class:project={!!e.project_id}>
                    {sourceProjectLabel(e)}
                  </span>
                </td>
                <td class="muted">{e.created_at}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
</section>

<style>
  .glossary {
    padding: 1.5rem;
    max-width: 1200px;
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
    font-size: 0.85rem;
    color: #78350f;
    margin: 0 0 1rem 0;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.6fr;
    gap: 1.5rem;
  }
  .add-form {
    background: #fff;
    padding: 1rem;
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  h2 {
    font-size: 1rem;
    margin: 0 0 0.5rem 0;
  }
  label {
    display: flex;
    flex-direction: column;
    font-size: 0.8rem;
    color: #475569;
    gap: 0.3rem;
  }
  label em {
    color: #b91c1c;
    font-style: normal;
  }
  input,
  textarea {
    padding: 0.5rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font: inherit;
    font-size: 0.85rem;
  }
  button {
    padding: 0.5rem 1rem;
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
  .list {
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
  .placeholder {
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    background: #fee2e2;
    padding: 0.5rem;
    border-radius: 4px;
  }
  .info {
    color: #166534;
    font-size: 0.85rem;
    background: #dcfce7;
    padding: 0.5rem;
    border-radius: 4px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid #e2e8f0;
  }
  th {
    background: #f8fafc;
    font-weight: 600;
    color: #334155;
  }
  .muted {
    color: #94a3b8;
    font-size: 0.78rem;
  }
  .scope {
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.75rem;
    background: #e0e7ff;
    color: #3730a3;
  }
  .scope.project {
    background: #fef3c7;
    color: #78350f;
  }
</style>
