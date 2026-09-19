// CATs 桌面客户端 — Svelte 5 入口
//
// 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//
// MVP 阶段使用自写轻量 hash router (避免引入 SvelteKit 的复杂度)。
// 路由表:
//   #/login        — 登录
//   #/translate    — 翻译工作台
//   #/projects     — 项目列表 + 新建

import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("mount target #app not found");
}

const app = mount(App, { target });
export default app;