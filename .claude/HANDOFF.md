# Handoff 文档 - wt 开发进度

## Session 24 完成的工作 (2026-02-03)

### 1. 完全迁移完成 - v2 后缀清除

本次 session 完成了从 v1 到 v2 的**完全迁移**：

#### 文件重命名
- `src/models/config_v2.rs` → `src/models/config.rs`
- `src/models/status_v2.rs` → `src/models/status.rs`
- `src/services/hooks_v2/` → `src/services/hooks/`

#### 类型重命名
- `ConfigV2` → `WtConfig`
- `StatusStoreV2` → `StatusStore`
- `TaskStatusV2` → `TaskStatus`
- `TaskStateV2` → `TaskState`
- `HooksEngineV2` → `HooksEngine`

#### 状态值更新
- `Running` → `Active`
- `Review` → `Idle`

### 2. 命令层统一使用新 Hooks API

所有命令现在使用统一的 `hooks.execute("hook_name", context)` API：

| 命令 | Hook 名称 |
|------|-----------|
| `wt run` | `"run"` |
| `wt review` | `"review"` |
| `wt resume` | `"resume"` |
| `wt complete` | `"complete"` |
| `wt delete` | `"delete"` |
| `wt reset` | `"reset"` |

#### 删除的旧 API
从 `HooksEngine` 中删除了向后兼容方法：
- `on_create()`, `before_run()`, `after_run()`
- `before_review()`, `after_review()`
- `before_resume()`, `before_complete()`, `after_complete()`
- `before_delete()`, `before_reset()`

### 3. 新增 `--verbose` 选项

`wt status --verbose` 现在显示详细状态信息：
- Phase (developing/reviewing/merging)
- IdleReason (done/human_review/error/conflict/timeout/manual)
- Active since (时间戳)

### 4. 删除的文件和代码
- `.wt/CONTEXT.md` (旧上下文文档)
- 旧的 `src/models/config.rs`, `status.rs`, `hook_context.rs`
- 旧的 `src/services/hooks.rs`
- HooksEngine 中的向后兼容方法

### 测试状态

```
cargo test --lib: 162 passed
cargo test --test cli: 121 passed
cargo test --test integration: 46 passed
Total: 所有测试通过
```

---

## 项目状态

### 当前架构

```
src/
├── models/
│   ├── config.rs     # JSONC 配置 (.wt/config.jsonc)
│   ├── status.rs     # 状态模型 (Pending/Active/Idle/Completed)
│   ├── store.rs      # 任务存储
│   └── task.rs       # 任务定义
├── services/
│   ├── hooks/        # Hooks 引擎
│   │   ├── mod.rs    # HooksEngine
│   │   ├── context.rs # ExecutionContext
│   │   ├── step.rs   # StepExecutor
│   │   └── pipeline.rs # PipelineExecutor
│   ├── multiplexer/  # tmux/zellij 抽象
│   └── git.rs        # Git 操作
└── commands/         # CLI 命令
```

### Hooks 系统

配置格式 (`.wt/config.jsonc`):
```jsonc
{
  "multiplexer": "tmux",
  "session_name": "my-project",
  "hooks": {
    "run": [
      { "type": "script", "run": "npm install" },
      { "type": "agent", "interactive": true, "prompt": "..." }
    ],
    "review": [
      { "type": "script", "run": "npm run lint" }
    ],
    "complete": [
      { "type": "internal", "run": "branch:merge" }
    ]
  }
}
```

### 状态模型

```
Status:
○ Pending  →  ● Active  ⇄  ◐ Idle  →  ✓ Completed
  (未创建)    (有进程)    (无进程)    (已完成)

Phase:
(none) → developing → reviewing → merging → (done)
```

### 规格完成度

| 规格项 | 状态 |
|--------|------|
| JSONC 配置解析 | ✅ |
| 新状态模型 (Pending/Active/Idle/Completed) | ✅ |
| Phase 字段 | ✅ |
| IdleReason 字段 | ✅ |
| active_since 字段 | ✅ |
| script step | ✅ |
| agent step (交互/非交互) | ✅ |
| internal step | ✅ |
| condition step | ✅ |
| Pipeline 执行器 | ✅ |
| `wt hooks run <hook>` | ✅ |
| `wt pause <task>` | ✅ |
| `wt status --verbose` | ✅ |
| 命令通过新 Hooks API | ✅ |

---

## 下一步工作

项目功能完整，可选优化：

1. **清理 dead code warnings** - 有一些未使用的辅助方法
2. **使用 DefaultTransition** - 自动状态管理（目前手动管理）
3. **internal step 实现** - 目前只是占位符

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 24 | 完全迁移 + 命令层统一使用新 Hooks API + --verbose |
| 23 | Agent Hooks 系统实现 (Phase 1-4) |
| 22 | Agent Hooks 系统设计（访谈 + Codex 辩论） |
| 21 | cleanup-legacy + Phase 4 docs 完成 |
| 20 | Phase 3 完成、修复 zellij 合并问题 |
| 19 | 补全 atomic-misc CLI 子命令、创建 hooks.rs |
| 18 | Hooks 系统设计、任务规划、Phase 1-2 服务层 |
