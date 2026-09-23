<script lang="ts">
  import { fetchTranslationLookup, type TmMatch } from "../lib/api";

  // MVP 默认项目 ID（前端只读 lookup；真实值由项目页路由参数传入）
  let projectId = $state("demo-project-001");
  let sourceText = $state("");
  let sourceLang = $state("zh");
  let targetLang = $state("en");

  let matches = $state<TmMatch[]>([]);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let fromCache = $state(false);

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
      });
      matches = r.matches;
      fromCache = r.from_cache;
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="workbench">
  <!-- 左侧：源文编辑 -->
  <div class="pane source">
    <h2>源文（{sourceLang}）</h2>
    <textarea bind:value={sourceText} placeholder="输入待翻译文本…"></textarea>
    <div class="meta">
      <label>项目 ID <input bind:value={projectId} /></label>
      <label>源语 <input bind:value={sourceLang} maxlength="8" /></label>
      <label>目标 <input bind:value={targetLang} maxlength="8" /></label>
      <button onclick={doLookup} disabled={busy || !sourceText.trim()}>
        {busy ? "查询中…" : "查 TM"}
      </button>
    </div>
  </div>

  <!-- 中间：候选译文 -->
  <div class="pane candidates">
    <h2>候选译文（{targetLang}）</h2>
    {#if errorMsg}
      <p class="error">{errorMsg}</p>
    {:else if matches.length === 0}
      <p class="placeholder">尚未查询或无候选。</p>
    {:else}
      <ul>
        {#each matches as m, i (m.tm_id + i)}
          <li class:exact={m.is_exact}>
            <div class="src">{m.source_text}</div>
            <div class="tgt">{m.target_text}</div>
            <div class="meta-line">
              <span>相似度 {(m.similarity * 100).toFixed(1)}%</span>
              <span class="badge">{m.is_exact ? "EXACT" : "FUZZY"}</span>
            </div>
          </li>
        {/each}
      </ul>
      {#if fromCache}
        <p class="cache-note">⚠ 命中本地缓存（离线模式）</p>
      {/if}
    {/if}
  </div>

  <!-- 右侧：TM/术语/标签 -->
  <div class="pane hints">
    <h2>术语 / 标签</h2>
    <div class="hint-block">
      <h3>TM 命中</h3>
      <p>{matches.length} 条候选</p>
    </div>
    <div class="hint-block">
      <h3>QA 标签</h3>
      <p>MVP 阶段未实现（per apps/cats-client/TODO.md）</p>
    </div>
    <div class="hint-block">
      <h3>术语库</h3>
      <p>MVP 阶段未实现</p>
    </div>
  </div>
</section>

<style>
  .workbench {
    display: grid;
    grid-template-columns: 1fr 1.4fr 1fr;
    height: 100%;
    gap: 1px;
    background: #e2e8f0;
  }
  .pane {
    background: #fff;
    padding: 1rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  h2 {
    margin: 0 0 0.75rem;
    font-size: 0.95rem;
    color: #0f172a;
  }
  textarea {
    flex: 1;
    resize: none;
    padding: 0.75rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.95rem;
  }
  .meta {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr auto;
    gap: 0.5rem;
    align-items: end;
    margin-top: 0.75rem;
  }
  .meta label {
    display: flex;
    flex-direction: column;
    font-size: 0.75rem;
    color: #64748b;
  }
  .meta input {
    padding: 0.4rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .meta button {
    padding: 0.5rem 1rem;
    background: #0f172a;
    color: #fff;
    border: 0;
    border-radius: 4px;
  }
  .placeholder {
    color: #94a3b8;
    font-size: 0.85rem;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  li {
    border: 1px solid #e2e8f0;
    border-radius: 4px;
    padding: 0.75rem;
  }
  li.exact {
    border-color: #16a34a;
    background: #f0fdf4;
  }
  .src {
    font-size: 0.8rem;
    color: #64748b;
    margin-bottom: 0.25rem;
  }
  .tgt {
    font-size: 0.95rem;
    margin-bottom: 0.5rem;
  }
  .meta-line {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: #64748b;
  }
  .badge {
    background: #0f172a;
    color: #fff;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.65rem;
  }
  li.exact .badge {
    background: #16a34a;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
  }
  .cache-note {
    margin-top: 0.5rem;
    color: #b45309;
    font-size: 0.8rem;
  }
  .hint-block {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #e2e8f0;
  }
  .hint-block h3 {
    margin: 0 0 0.4rem;
    font-size: 0.85rem;
    color: #334155;
  }
  .hint-block p {
    margin: 0;
    font-size: 0.8rem;
    color: #64748b;
  }
</style>