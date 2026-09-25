<script lang="ts">
  import { onMount } from "svelte";
  import { route } from "./lib/router.svelte";
  import LoginPage from "./routes/LoginPage.svelte";
  import TranslatePage from "./routes/TranslatePage.svelte";
  import ProjectsPage from "./routes/ProjectsPage.svelte";

  let { children } = $props();

  onMount(() => {
    // hash 变化监听: 自写 hash router
    const handler = () => {
      route.current = parseHash(location.hash);
    };
    window.addEventListener("hashchange", handler);
    handler(); // 初始化
    return () => window.removeEventListener("hashchange", handler);
  });

  function parseHash(hash: string): { name: string; params: URLSearchParams } {
    const cleaned = hash.startsWith("#") ? hash.slice(1) : hash;
    const [path, query] = cleaned.split("?");
    const name = path || "login";
    return { name, params: new URLSearchParams(query ?? "") };
  }
</script>

<div class="cats-shell">
  <header>
    <span class="brand">CATs</span>
    <nav>
      <a href="#/projects">项目</a>
      <a href="#/translate">翻译</a>
      <a href="#/login">登录</a>
    </nav>
  </header>

  <main>
    {#if route.current.name === "translate"}
      <TranslatePage />
    {:else if route.current.name === "projects"}
      <ProjectsPage />
    {:else}
      <LoginPage />
    {/if}
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
  main {
    flex: 1;
    overflow: auto;
    background: #f8fafc;
  }
</style>