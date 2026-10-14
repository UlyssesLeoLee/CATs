# ULYS-151 SSE 首帧 bug 重写 spec

**任务**: ULYS-151 (task-service 切片 B-2 SSE)
**目标**: 按 ULYS-45 `14e35be` 修复版重写 SSE 首帧处理, 消除首帧丢失 bug
**基线**: 当前 `agent/minimaxm3/d11b0bce579f` 分支的 task-service SSE 实现 (per bb3f957)

## 问题 (per reviewer cf3de69f 9/21 + 9/23 复审)

task-service 当前 SSE 实现照 ULYS-45 **修复前**的 `af8abb6` 写了, 漏了 **`14e35be`** 修复版本的关键修复. 症状是 SSE 客户端订阅时第一帧丢失或延迟, 任务进度第二行接不到.

## 当前 SSE 实现位置

`crates/task-service/src/handlers.rs` (handler `task_event_stream` 或类似名, 路径 `/v1/tasks/{id}/events`).

## 14e35be 修复内容 (per reviewer 提示)

`14e35be` 是 ULYS-45 的 SSE 首帧修复 commit. 关键修复点:
1. **订阅立即发送 heartbeat/comment 帧**: 客户端建立连接时, 服务端必须先发一帧 SSE 注释或心跳, 避免客户端 SSE parser 等待第一帧超时
2. **broadcast::Receiver::recv 改 try_recv + 立即 yield**: 订阅建立后先 yield 一次空状态, 让 actix-web 的 stream 调度起来
3. **History frame ordering**: 用 broadcast channel 的 seq number 保证客户端按序处理, 不依赖 wall-clock

## 任务清单

1. 在自己的 worktree 里 (不要污染主 worktree) 创建分支:
   ```
   cd C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/.repos/d83598db-3bfe-4386-b86d-ca266aa77e9f/github.com+UlyssesLeoLee+CATs.git
   git worktree add "C:/Users/leo19/multica_workspaces_desktop-api.multica.ai/ulysses-ca266aa77e9f/ulys-125-sse-rewrite/workdir/CATs" -b agent/minimaxm3/sse-rewrite agent/minimaxm3/d11b0bce579f
   ```
2. 改 crates/task-service/src/handlers.rs SSE handler, 订阅 broadcast 后立即 yield 一帧 heartbeat/comment + 用 try_recv 替代 recv
3. 加 integration test: SSE 客户端订阅时第一帧 < 100ms 内收到 (用 reqwest-eventsource)
4. commit author = `架构师(Mavis 接手 agent per DEC-008) <architecture@mavis.local>`
5. push origin + 报 commit hash

## 验收

- cargo check -p task-service exit 0
- cargo test -p task-service --lib 全过 (含新加 first-frame test)
- commit + push 成功

## 守门合规

#1 禁回溯 / #5 不动 rust-toolchain.toml / #9 v19 explicit path / #14 v3 代签 / L11 cargo 1 次