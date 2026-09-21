<script lang="ts">
  import {
    dispatchTranslationTask,
    fetchTranslationLookup,
    runtimeMode,
    type TmMatch,
  } from "../lib/api";

  // MVP 默认项目 ID（前端只读 lookup；真实值由项目页路由参数传入）
  let projectId = $state("demo-project-001");
  let sourceText = $state("");
  let sourceLang = $state("zh");
  let targetLang = $state("en");

  let matches = $state<TmMatch[]>([]);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let fromCache = $state(false);

  // 切片 D: 派发任务状态
  let dispatchBusy = $state(false);
  let dispatchInfo = $state<string | null>(null);

  // 切片 D: 标签保护 (placeholder / inline tag 静态检测)
  // 检测规则 (per OFCAT F5 + i18next ICU MessageFormat):
  //   {var}                  - simple placeholder
  //   {var, plural, ...}     - ICU plural
  //   {var, select, ...}     - ICU select
  //   <tag>...</tag>         - HTML-like
  //   [[var]]                - custom bracket
  //   %var% / $var           - sprintf / shell
  let detectedTags = $derived(detectInlineTags(sourceText));
  let hasProtectedTags = $derived(detectedTags.length > 0);

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

  async function doDispatch() {
    if (!sourceText.trim()) return;
    if (!confirm(`派发 ${sourceText.length} 字符为翻译任务?`)) return;
    dispatchBusy = true;
    dispatchInfo = null;
    errorMsg = null;
    try {
      // file_id 用 sourceText 的 SHA-like 短 hash (前端轻量)
      const fileId = simpleHash(sourceText).toString(16).padStart(8, "0");
      const r = await dispatchTranslationTask({
        project_id: projectId,
        file_id: fileId,
        media_type: "TEXT",
        source_text: sourceText,
      });
      dispatchInfo = `已派发 task_id=${r.task_id}, status=${r.status} — 去任务页追踪`;
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      dispatchBusy = false;
    }
  }

  function goTasks() {
    location.hash = "#/tasks";
  }

  function detectInlineTags(s: string): string[] {
    const out: string[] = [];
    // ICU / i18next placeholder
    const reIcu = /\{[a-zA-Z0-9_]+(?:,\s*(?:plural|select|number|date|time)\b[^}]*)?\}/g;
    for (const m of s.matchAll(reIcu)) out.push(m[0]);
    // HTML-like
    const reHtml = /<\/?[a-zA-Z][a-zA-Z0-9-]*>/g;
    for (const m of s.matchAll(reHtml)) out.push(m[0]);
    // Bracket placeholders
    const reBracket = /\[\[[a-zA-Z0-9_]+\]\]/g;
    for (const m of s.matchAll(reBracket)) out.push(m[0]);
    // sprintf
    const reSprintf = /%[sdif]/g;
    for (const m of s.matchAll(reSprintf)) out.push(m[0]);
    // shell
    const reShell = /\$\{[a-zA-Z_][a-zA-Z0-9_]*\}/g;
    for (const m of s.matchAll(reShell)) out.push(m[0]);
    return Array.from(new Set(out));
  }

  // 32-bit FNV-1a hash (前端轻量替代 crypto.subtle.digest)
  function simpleHash(s: string): number {
    let h = 0x811c9dc5;
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = (h * 0x01000193) >>> 0;
    }
    return h;
  }
</script>

<section class="workbench">
  <!-- 左侧：源文编辑 -->
  <div class="pane source">
    <h2>源文（{sourceLang}）</h2>
    <textarea bind:value={sourceText} placeholder="输入待翻译文本…"></textarea>

    <!-- 切片 D: 标签保护提示 -->
    {#if hasProtectedTags}
      <div class="tag-protection">
        <h3>🛡 标签保护 (F5)</h3>
        <p>检测到 {detectedTags.length} 处内联标签, 翻译时应保留原样:</p>
        <div class="tags">
          {#each detectedTags as t (t)}
            <code>{t}</code>
          {/each}
        </div>
      </div>
    {/if}

    <div class="meta">
      <label>项目 ID <input bind:value={projectId} /></label>
      <label>源语 <input bind:value={sourceLang} maxlength="8" /></label>
      <label>目标 <input bind:value={targetLang} maxlength="8" /></label>
      <button onclick={doLookup} disabled={busy || !sourceText.trim()}>
        {busy ? "查询中…" : "查 TM"}
      </button>
    </div>

    <!-- 切片 D: 派发任务按钮 -->
    <div class="dispatch-row">
      <button
        class="dispatch"
        onclick={doDispatch}
        disabled={dispatchBusy || !sourceText.trim()}
      >
        {dispatchBusy ? "派发中…" : "派发为翻译任务"}
      </button>
      {#if dispatchInfo}
        <p class="dispatch-info">
          ✅ {dispatchInfo}
          <button class="link" onclick={goTasks}>→ 任务页</button>
        </p>
      {/if}
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

  <!-- 右侧：术语 / 标签 -->
  <div class="pane hints">
    <h2>术语 / 标签</h2>
    <div class="hint-block">
      <h3>TM 命中</h3>
      <p>{matches.length} 条候选 · <a href="#/tm">→ 浏览 TM 页</a></p>
    </div>
    <div class="hint-block">
      <h3>标签保护</h3>
      <p>
        {#if hasProtectedTags}
          🛡 已检测 {detectedTags.length} 处 (见左下方)
        {:else}
          当前源文无内联标签 (F5 MVP 静态检测)
        {/if}
      </p>
    </div>
    <div class="hint-block">
      <h3>术语库</h3>
      <p><a href="#/glossary">→ 打开术语页</a></p>
    </div>
    <div class="hint-block">
      <h3>任务追踪</h3>
      <p><a href="#/tasks">→ 任务列表</a></p>
    </div>
    {#if !runtimeMode.isTauri()}
      <div class="hint-block">
        <h3>运行时</h3>
        <p>web preview · 派发功能不可用</p>
      </div>
    {/if}
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
    min-height: 6rem;
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
  .tag-protection {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: #ecfeff;
    border: 1px solid #67e8f9;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .tag-protection h3 {
    margin: 0 0 0.3rem 0;
    font-size: 0.82rem;
    color: #155e75;
  }
  .tag-protection p {
    margin: 0 0 0.4rem 0;
    color: #0e7490;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .tags code {
    background: #cffafe;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.75rem;
    color: #155e75;
  }
  .dispatch-row {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px dashed #e2e8f0;
  }
  .dispatch {
    width: 100%;
    padding: 0.6rem 1rem;
    background: #16a34a;
    color: #fff;
    border: 0;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }
  .dispatch:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .dispatch-info {
    margin: 0.4rem 0 0 0;
    font-size: 0.8rem;
    color: #166534;
    background: #dcfce7;
    padding: 0.4rem 0.6rem;
    border-radius: 3px;
  }
  .link {
    background: none;
    color: #1d4ed8;
    border: 0;
    padding: 0 0 0 0.5rem;
    text-decoration: underline;
    cursor: pointer;
    font-size: 0.8rem;
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
  .hint-block a {
    color: #1d4ed8;
    text-decoration: none;
  }
  .hint-block a:hover {
    text-decoration: underline;
  }
</style>
