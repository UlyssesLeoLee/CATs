<script lang="ts">
  import { onMount } from "svelte";
  import {
    fetchTranslationLookup,
    runtimeMode,
    type TmMatch,
  } from "../lib/api";

  // 表单
  let projectId = $state("demo-project-001");
  let sourceText = $state("");
  let sourceLang = $state("zh");
  let targetLang = $state("en");
  let threshold = $state(0.7);

  // 命中
  let matches = $state<TmMatch[]>([]);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let fromCache = $state(false);
  let lastLookupAt = $state<string | null>(null);

  // 分桶: 100% (exact) / 模糊 / 无命中
  let exactMatches = $derived(matches.filter((m) => m.is_exact));
  let fuzzyMatches = $derived(
    matches.filter((m) => !m.is_exact && m.similarity >= threshold),
  );
  let belowThreshold = $derived(
    matches.filter((m) => !m.is_exact && m.similarity < threshold),
  );

  onMount(async () => {
    // 启动时如已有 sourceText 自动查
  });

  async function doLookup() {
    if (!sourceText.trim()) return;
    busy = true;
    errorMsg = null;
    try {
      const r = await fetchTranslationLookup({
        source: sourceText,
        project_id: projectId,
        source_lang: sourceLang,
        target_lang: targetLang,
        threshold,
      });
      matches = r.matches;
      fromCache = r.from_cache;
      lastLookupAt = new Date().toISOString().slice(11, 19);
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }

  function simColor(sim: number): string {
    if (sim >= 0.95) return "#16a34a";
    if (sim >= 0.8) return "#65a30d";
    if (sim >= 0.6) return "#ca8a04";
    return "#94a3b8";
  }

  function applyToEditor(t: TmMatch) {
    // MVP: 把命中目标文本写回 sourceText 下方预览栏 (用户复制走)
    // 真接 TranslatePage 留 M2
    selectedTarget = t.target_text;
  }
  let selectedTarget = $state<string | null>(null);
</script>

<section class="tm-page">
  <header class="bar">
    <h1>TM 浏览 (切片 D)</h1>
    <div class="nav-links">
      <a href="#/translate">翻译</a>
      <a href="#/tasks">任务</a>
      <a href="#/glossary">术语</a>
      <a href="#/projects">项目</a>
    </div>
  </header>

  <p class="notice">
    ⓘ 当前实现: TM 匹配通过 <code>fetchTranslationLookup</code> → BFF
    <code>/v1/translate/lookup</code> (POST /v1/tasks dispatch 已可用, BFF
    未暴露 TM browse endpoint, 后端 M2 升级). 此页面复用 lookup 渲染 100% /
    模糊可视化, 选定条目暂存到 <strong>{runtimeMode.isTauri() ? "本地 SQLite 镜像" : "会话状态"}</strong>.
  </p>

  <div class="lookup-bar">
    <input bind:value={projectId} placeholder="项目 ID" class="grow" />
    <input bind:value={sourceLang} maxlength="8" placeholder="源" class="lang" />
    <input bind:value={targetLang} maxlength="8" placeholder="目标" class="lang" />
    <label class="thr">
      阈值
      <input
        type="number"
        bind:value={threshold}
        min="0"
        max="1"
        step="0.05"
      />
    </label>
  </div>

  <textarea
    bind:value={sourceText}
    placeholder="输入待查 TM 的源文本…"
    rows="3"
  ></textarea>
  <div class="action-row">
    <button onclick={doLookup} disabled={busy || !sourceText.trim()}>
      {busy ? "查询中…" : "查 TM"}
    </button>
    {#if lastLookupAt}
      <span class="meta">最近查询 {lastLookupAt} · 命中 {matches.length} 条</span>
    {/if}
    {#if fromCache}
      <span class="cache">⚠ 命中本地缓存</span>
    {/if}
  </div>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <!-- 100% 命中 -->
  <section class="bucket">
    <h2>100% 命中 (exact) — {exactMatches.length}</h2>
    {#if exactMatches.length === 0}
      <p class="placeholder">无</p>
    {:else}
      <ul class="cards">
        {#each exactMatches as m (m.tm_id)}
          <li class="card exact">
            <div class="src">{m.source_text}</div>
            <div class="tgt">{m.target_text}</div>
            <div class="meta-line">
              <span class="sim" style="color: {simColor(m.similarity)}">
                相似度 {(m.similarity * 100).toFixed(1)}%
              </span>
              <button class="pick" onclick={() => applyToEditor(m)}>选用</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <!-- 模糊命中 -->
  <section class="bucket">
    <h2>模糊命中 (≥ {(threshold * 100).toFixed(0)}%) — {fuzzyMatches.length}</h2>
    {#if fuzzyMatches.length === 0}
      <p class="placeholder">无</p>
    {:else}
      <ul class="cards">
        {#each fuzzyMatches as m (m.tm_id)}
          <li class="card fuzzy">
            <div class="src">{m.source_text}</div>
            <div class="tgt">{m.target_text}</div>
            <div class="meta-line">
              <span class="sim" style="color: {simColor(m.similarity)}">
                {(m.similarity * 100).toFixed(1)}%
              </span>
              <button class="pick" onclick={() => applyToEditor(m)}>选用</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <!-- 低于阈值 -->
  {#if belowThreshold.length > 0}
    <section class="bucket dim">
      <h2>低于阈值 (低于 {(threshold * 100).toFixed(0)}%) — {belowThreshold.length}</h2>
      <ul class="cards compact">
        {#each belowThreshold.slice(0, 5) as m (m.tm_id)}
          <li class="card">
            <div class="src">{m.source_text}</div>
            <div class="tgt">{m.target_text}</div>
            <span class="sim" style="color: {simColor(m.similarity)}">
              {(m.similarity * 100).toFixed(1)}%
            </span>
          </li>
        {/each}
      </ul>
      {#if belowThreshold.length > 5}
        <p class="placeholder">… 还有 {belowThreshold.length - 5} 条</p>
      {/if}
    </section>
  {/if}

  {#if selectedTarget}
    <aside class="selected">
      <h3>已选用译文</h3>
      <p>{selectedTarget}</p>
      <button onclick={() => (selectedTarget = null)}>清空</button>
    </aside>
  {/if}
</section>

<style>
  .tm-page {
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
  .lookup-bar {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr auto;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .lookup-bar input {
    padding: 0.5rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .lookup-bar .grow {
    grid-column: span 1;
  }
  .thr {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.8rem;
    color: #475569;
  }
  .thr input {
    width: 4rem;
  }
  textarea {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font: inherit;
    font-size: 0.9rem;
    box-sizing: border-box;
    resize: vertical;
  }
  .action-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin: 0.75rem 0;
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
  .meta {
    color: #64748b;
    font-size: 0.8rem;
  }
  .cache {
    color: #b45309;
    font-size: 0.8rem;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    background: #fee2e2;
    padding: 0.5rem;
    border-radius: 4px;
  }
  .bucket {
    margin-top: 1.5rem;
  }
  .bucket.dim h2 {
    color: #94a3b8;
  }
  .bucket h2 {
    font-size: 1rem;
    margin: 0 0 0.5rem 0;
    color: #334155;
  }
  .placeholder {
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .cards.compact {
    gap: 0.25rem;
  }
  .card {
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 0.75rem;
    background: #fff;
  }
  .card.exact {
    border-color: #16a34a;
    background: #f0fdf4;
  }
  .card.fuzzy {
    border-color: #fde68a;
    background: #fffbeb;
  }
  .cards.compact .card {
    padding: 0.4rem 0.6rem;
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 0.5rem;
    align-items: center;
  }
  .src {
    font-size: 0.8rem;
    color: #64748b;
  }
  .tgt {
    font-size: 0.95rem;
    margin: 0.25rem 0;
  }
  .cards.compact .tgt {
    margin: 0;
  }
  .meta-line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.4rem;
  }
  .sim {
    font-weight: 600;
    font-size: 0.85rem;
  }
  .pick {
    padding: 0.25rem 0.6rem;
    font-size: 0.75rem;
    background: #16a34a;
  }
  .selected {
    margin-top: 1.5rem;
    padding: 1rem;
    background: #dcfce7;
    border: 1px solid #16a34a;
    border-radius: 4px;
  }
  .selected h3 {
    margin: 0 0 0.5rem 0;
    font-size: 0.9rem;
    color: #166534;
  }
  .selected p {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
  }
</style>
