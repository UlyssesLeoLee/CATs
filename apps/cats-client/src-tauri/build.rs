// Tauri 2.x build script
//
// 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（客户端：Tauri 2.x + Svelte 5）
//
// 负责读取 tauri.conf.json + 生成平台特定资源（Windows 资源文件 / macOS Info.plist / Linux .desktop）

fn main() {
    tauri_build::build()
}