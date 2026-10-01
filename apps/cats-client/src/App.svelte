<script lang="ts">
  import { onMount } from "svelte";
  import { route } from "./lib/router.svelte";
  import { runtimeMode } from "./lib/api";
  import LoginPage from "./routes/LoginPage.svelte";
  import TranslatePage from "./routes/TranslatePage.svelte";
  import ProjectsPage from "./routes/ProjectsPage.svelte";

  let { children } = $props();

  // 本地 $state + $effect: hashchange → 写入 routeName, 模板读 routeName.
  // 备用保险: navVersion 计数, 每次 hashchange ++ 用于 {#key} 强制重渲,
  // 兜底 Svelte 5 production build 下 $state→{#if} reactive 漂移的极端情况.
  let routeName = $state<string>("login");
  let navVersion = $state(0);

  onMount(() => {
    const update = () => {
      const cleaned = location.hash.startsWith("#")
        ? location.hash.slice(1)
        : location.hash;
      const [path] = cleaned.split("?");
      routeName = path || "login";
      navVersion += 1;
      route.current = {
        name: routeName,
        params: new URLSearchParams(),
      };
    };
    window.addEventListener("hashchange", update);
    update(); // 初始化
    return () => window.removeEventListener("hashchange", update);
  });
</script>

<div class="cats-shell">
  <header>
    <span class="brand">CATs</span>
    <nav>
      <a href="#/projects">项目</a>
      <a href="#/translate">翻译</a>
      <a href="#/login">登录</a>
    </nav>
    {#if !runtimeMode.isTauri()}
      <span class="badge-web" title="静态预览 — 无 Tauri runtime, 命令走 mock">
        web preview
      </span>
    {/if}
  </header>

  <main>
    {#key navVersion}
      {#if routeName === "translate"}
        <TranslatePage />
      {:else if routeName === "projects"}
        <ProjectsPage />
      {:else}
        <LoginPage />
      {/if}
    {/key}
  </main>
</div>

<style>
  .cats-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: -apple-system, "Segoe UI", "PingFang SC", sans-serif;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    height: 48px;
    background: #0f172a;
    color: #e2e8f0;
  }
  .brand {
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  nav {
    display: flex;
    gap: 1rem;
  }
  nav a {
    color: #cbd5e1;
    text-decoration: none;
  }
  nav a:hover {
    color: #fff;
  }
  .badge-web {
    margin-left: auto;
    background: #f59e0b;
    color: #0f172a;
    font-size: 0.7rem;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  main {
    flex: 1;
    overflow: auto;
    background: #f8fafc;
  }
</style>