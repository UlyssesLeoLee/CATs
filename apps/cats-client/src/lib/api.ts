// 客户端 API 封装（通过 Tauri invoke 调 Rust command）
//
// 不在前端直接持有 reqwest, 所有 HTTP 经 Rust 端走 BFF。

import { invoke } from "@tauri-apps/api/core";

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