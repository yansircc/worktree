# Handoff 文档 - wt 开发进度

## Session 32 完成的工作 (2026-02-03)

### Phase 3 + Phase 4a + Phase 4b 实现完成 ✅

#### Phase 3: 状态管理

| 文件 | 修改内容 |
|------|----------|
| `src/models/config.rs` | 添加 phases/concurrency/observe 字段，添加 v2 方法 |
| `src/models/status.rs` | 添加 v2 桥接 (to_derived_status, from_runtime_state) |
| `src/models/store.rs` | 添加 Project 支持 (project_status, full_project_status) |

#### Phase 4a: 新增命令

| 文件 | 内容 |
|------|------|
| `src/commands/step.rs` | `wt step done/block/fail` - Agent 标记当前 step 状态 |
| `src/commands/prev.rs` | `wt prev <task>` - 强制回退到上一阶段 |

#### Phase 4b: 命令重写

| 文件 | 内容 |
|------|------|
| `src/commands/next.rs` | 重写为 `wt next <task>` - 强制推进到下一阶段 |
| `src/commands/stop.rs` | 新增 `wt stop <task>` - 停止任务进程但保留资源 |
| `src/commands/reset.rs` | 添加 `--to` 参数支持重置到指定阶段 |
| `tests/cli/next.rs` | 重写测试适配新的 next 命令语义 |

---

## 项目状态

### 测试

```
cargo test --lib: 270 passed
cargo test --test cli: 116 passed
编译警告: dead_code (预期，新模型尚未被命令层完全使用)
```

### 新增/修改的命令

```bash
# Phase 4a (新增)
wt step done              # Agent 标记 step 完成
wt step block [message]   # Agent 标记 step 阻塞
wt step fail [message]    # Agent 标记 step 失败
wt prev <task>            # 强制回退到上一阶段

# Phase 4b (新增/修改)
wt next <task>            # 强制推进到下一阶段 (重写)
wt stop <task>            # 停止任务进程 (新增)
wt reset <task> --to <phase>  # 重置到指定阶段 (扩展)
```

---

## 下一步工作

### Phase 4c: 删除旧命令 (下一 Session)

删除以下文件和 cli.rs 中对应的定义：
- `src/commands/run.rs`
- `src/commands/review.rs`
- `src/commands/resume.rs`
- `src/commands/complete.rs`
- `src/commands/pause.rs`
- `src/commands/hooks_cmd.rs`
- `src/commands/pipeline_cmd.rs`

### Phase 5: 清理 (再下一 Session)

- 删除 `src/services/hooks/` 目录
- 删除 `src/models/agent_step.rs`
- 删除 `src/models/builtin_pipelines.rs`
- 删除旧的 services (config_ops, status_ops, notify)
- 更新 README.md 和 CLAUDE.md

### 参考文档

- **架构设计**: `.claude/specs/phases-v2/architecture.md`
- **API 定义**: `.claude/specs/phases-v2/api.md`
- **文件清单**: `.claude/specs/phases-v2/files.md`

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 32 | **Phase 3+4a+4b 完成** - 状态管理 + step/prev/next/stop/reset 命令 |
| 31 | **Phase 1+2 完成** - 核心模型 + 执行引擎 |
| 30 | Phases v2 文件清单 - 详细评估每个文件的处置方式 |
| 29 | 重构: TaskContext + task_parser + builtin_pipelines |
| 28 | Dead code 彻底清理 + 架构分析 + 重构规格 |
| 27 | AgentStep 重构 + ClaudeCommandBuilder |
| 26 | Pipeline 完善 + 预定义 pipelines |
| 25 | 术语统一 + Agent step CLI 参数对齐 |
| 24 | 命令层统一使用新 Hooks API |
| 23 | Agent Hooks 系统实现 |
