// 客户端 API 封装 (通过 Tauri invoke 调 Rust command)
//
// 不在前端直接持有 reqwest, 所有 HTTP 经 Rust 端走 BFF。
//
// 运行时分支:
//   - Tauri 2.x desktop (window.__TAURI_INTERNALS__ 存在):
//       调 invoke() → Rust command → cats-bff → 微服务
//   - Web 浏览器 (Vercel / 普通浏览器 / 静态预览):
//       返回 web-mode 模拟值, 让 LoginPage / ProjectsPage / TranslatePage
//       渲染出 "未连接后端" 的空状态, 不抛 ReferenceError.
//
// 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//       doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md §ADR-15 (Web 控制台/BFF)

// 运行时检测 Tauri (避免静态 import 失败 — 该模块在 web 模式下根本不存在)
declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// 模拟 invoke — 仅在 web 模式被调用, 不污染 Tauri 模式行为
async function webInvoke<T>(cmd: string, args?: unknown): Promise<T> {
  console.warn(
    `[cats-client] web-mode: Tauri runtime not detected, command "${cmd}" returns mock.`,
  );
  // 简单调度: 让每个命令返回类型安全的占位
  switch (cmd) {
    case "get_offline_status":
      return {
        online: false,
        queue_length: 0,
        last_sync_at: null,
        db_path: "(web preview, no local db)",
      } as unknown as T;
    case "list_projects":
      return { projects: [], total: 0 } as unknown as T;
    case "auth_login":
      throw new Error(
        "登录功能仅在 Tauri 桌面客户端可用 (per §ADR-15, 浏览器控制台请用 Next.js Web Console)",
      );
    case "fetch_translation_lookup":
      return { matches: [], from_cache: false } as unknown as T;
    case "create_project":
    case "enqueue_offline_action":
    case "sync_offline_queue":
      throw new Error(`web preview: command "${cmd}" requires Tauri runtime`);
    default:
      throw new Error(`unknown command: ${cmd}`);
  }
}

// 真实 invoke — 仅在 Tauri 模式被调用. 用 dynamic import 让 Rollup 在
// web 模式下完全不会尝试解析 @tauri-apps/api/core
let realInvokeCache: ((cmd: string, args?: unknown) => Promise<unknown>) | null =
  null;
async function realInvoke(cmd: string, args?: unknown): Promise<unknown> {
  if (!realInvokeCache) {
    const mod = await import("@tauri-apps/api/core");
    realInvokeCache = (c: string, a?: unknown) => mod.invoke(c, a as Record<string, unknown>);
  }
  return realInvokeCache(cmd, args);
}

async function invoke<T>(cmd: string, args?: unknown): Promise<T> {
  if (isTauri()) {
    return realInvoke(cmd, args) as Promise<T>;
  }
  return webInvoke<T>(cmd, args);
}

export interface TmMatch {
  tm_id: string;
  source_text: string;
  target_text: string;
  similarity: number;
  is_exact: boolean;
}

export interface Project {
  id: string;
  name: string;
  source_lang?: string;
  target_lang?: string;
  status?: string;
}

export interface OfflineStatus {
  online: boolean;
  queue_length: number;
  last_sync_at: string | null;
  db_path: string;
}

export interface AuthLoginResponse {
  user_id?: string;
  org_id?: string;
  roles: string[];
  access_token_expires_at: string;
}

export const runtimeMode = {
  isTauri: () => isTauri(),
  mode: isTauri() ? "tauri" : "web",
} as const;

export async function getOfflineStatus(): Promise<OfflineStatus> {
  return invoke<OfflineStatus>("get_offline_status");
}

export async function authLogin(
  username: string,
  password: string,
): Promise<AuthLoginResponse> {
  return invoke<AuthLoginResponse>("auth_login", { username, password });
}

export async function fetchTranslationLookup(args: {
  source: string;
  project_id: string;
  source_lang: string;
  target_lang: string;
  threshold?: number;
}): Promise<{ matches: TmMatch[]; from_cache: boolean }> {
  return invoke("fetch_translation_lookup", { ...args });
}

export async function listProjects(): Promise<{ projects: Project[]; total?: number }> {
  return invoke("list_projects");
}

export async function createProject(args: {
  name: string;
  source_lang: string;
  target_lang: string;
}): Promise<Project> {
  return invoke("create_project", { ...args });
}

export async function enqueueOfflineAction(args: {
  action_type: string;
  payload: unknown;
}): Promise<{ seq: number }> {
  return invoke("enqueue_offline_action", { request: args });
}

export async function syncOfflineQueue(): Promise<{ total: number; succeeded: number; failed: number }> {
  return invoke("sync_offline_queue");
}
