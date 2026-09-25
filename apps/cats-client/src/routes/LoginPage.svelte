<script lang="ts">
  import { authLogin } from "../lib/api";
  import { navigate } from "../lib/router.svelte";

  let username = $state("");
  let password = $state("");
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    errorMsg = null;
    try {
      await authLogin(username, password);
      navigate("projects");
    } catch (e: unknown) {
      errorMsg = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="login">
  <div class="card">
    <h1>登录 CATs</h1>
    <p class="hint">全媒体 AI 辅助翻译 SaaS 平台</p>

    <form onsubmit={submit}>
      <label>
        <span>用户名</span>
        <input type="text" bind:value={username} required disabled={busy} />
      </label>
      <label>
        <span>密码</span>
        <input type="password" bind:value={password} required disabled={busy} />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" disabled={busy}>
        {busy ? "登录中…" : "登录"}
      </button>
    </form>

    <p class="footnote">
      MVP 客户端: 仅 JWT + refresh（OAuth/SSO 待 M1 阶段集成, per apps/cats-client/TODO.md）
    </p>
  </div>
</section>

<style>
  .login {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 2rem;
  }
  .card {
    width: 360px;
    background: #fff;
    padding: 2rem;
    border-radius: 8px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
  }
  h1 {
    margin: 0 0 0.25rem;
    font-size: 1.4rem;
  }
  .hint {
    margin: 0 0 1.5rem;
    color: #64748b;
    font-size: 0.85rem;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.85rem;
    color: #334155;
  }
  input {
    padding: 0.6rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    font-size: 0.95rem;
  }
  button {
    padding: 0.7rem;
    background: #0f172a;
    color: #fff;
    border: 0;
    border-radius: 4px;
    font-size: 0.95rem;
  }
  button:disabled {
    opacity: 0.6;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    background: #fee2e2;
    padding: 0.5rem;
    border-radius: 4px;
  }
  .footnote {
    margin-top: 1.5rem;
    font-size: 0.75rem;
    color: #94a3b8;
  }
</style>