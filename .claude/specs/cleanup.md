# 代码清理指南

基于资源粒度化 + 验证器统一重构后的全面代码清理。

## 1. 删除未使用的依赖

**Cargo.toml** — 删除以下两行：

```
sha2 = "0.10"   # line 24, 全局无引用
hex = "0.4"     # line 25, 全局无引用
```

> `atty` 仍在 `src/commands/status/mod.rs:27` 使用，保留。
> `serde_yaml` 在 `src/models/task.rs` 和 `src/models/task_parser.rs` 使用，保留。

**验证**: `cargo build` 后无编译错误。

---

## 2. 消除编译器 dead_code 警告

当前 `cargo check` 报 7 个 warning，按处理策略分类：

### 2.1 直接删除（无调用者）

| 文件 | 函数 | 原因 |
|------|------|------|
| `src/models/step.rs:105` | `StepResult::with_attempt()` | builder 方法，从未调用 |
| `src/models/step.rs:295` | `StepVerify::human_review()` | 工厂方法，从未调用 |

### 2.2 标记 `#[cfg(test)]`（仅测试使用）

| 文件 | 函数 | 测试调用位置 |
|------|------|------|
| `src/models/phase.rs:68` | `PhaseResources::none()` | phase.rs tests |
| `src/models/phase.rs:73` | `PhaseResources::full()` | phase.rs, next.rs, prev.rs tests |
| `src/models/phase.rs:268` | `Phase::new()` | phase.rs, executor/phase.rs tests |
| `src/models/phase.rs:284` | `Phase::with_resources()` | phase.rs, executor/phase.rs tests |
| `src/models/project.rs:64` | `PhasesConfig::sequence()` | project.rs tests |
| `src/models/state.rs:57` | `TaskRuntimeState::update_checkpoint()` | state.rs tests |
| `src/models/state.rs:63` | `TaskRuntimeState::resume_from()` | state.rs tests |
| `src/models/state.rs:68` | `TaskRuntimeState::can_resume()` | state.rs tests |
| `src/services/executor/workflow.rs:180` | `WorkflowExecutor::resume()` | workflow.rs tests |

> 注意：`Phase::new/with_resources` 和 `PhaseResources::none/full` 跨文件在 tests 中使用（`executor/phase.rs`、`next.rs`、`prev.rs` 的 `#[cfg(test)]` 模块），不能简单标记 `#[cfg(test)]`。有两种做法：
>
> **方案 A**：保留 pub，接受 warning（最简单）
> **方案 B**：在 lib 级别添加 `#[cfg(test)]` 的 test-helpers 模块统一导出（过度工程化，不推荐）
>
> 推荐方案 A，因为这些 warning 不影响正确性。但 `StepResult::with_attempt()` 和 `StepVerify::human_review()` 可以直接删除。

---

## 3. 清理 `#[allow(dead_code)]` 注解

逐个评估是否仍需要：

| 文件 | 位置 | 结论 |
|------|------|------|
| `src/services/executor/phase.rs:21` | `PhaseTransitionResult` struct | **保留** — 结构体字段被填充但不总是被读取，属正常 |
| `src/services/executor/workflow.rs:33` | `WorkflowResult.duration_ms` | **保留** — 同上 |
| `src/services/executor/condition/error.rs:9,13` | `UnexpectedChar(char)`, `InvalidNumber(String)` | **保留** — 用于 Debug trait 输出 |

---

## 4. 清理 TODO 注释

| 文件 | 行 | TODO 内容 | 处理方式 |
|------|------|------|------|
| `src/services/executor/step.rs:101` | `// TODO: collect artifacts` | **保留** — 属于正式的 roadmap 功能 |
| `src/services/executor/step.rs:102` | `// TODO: extract exports` | **保留** — 同上 |
| `src/services/executor/workflow.rs:435` | `// TODO: read actual output from file` | **保留** — 同上 |
| `src/services/executor/phase.rs:192` | `// TODO: Actually create worktree, branch, window` | **删除** — 资源分配已在 `commands/next.rs` 的 `allocate_resources()` 实现，executor 层此处是历史遗留 stub |

---

## 5. 更新文档以匹配代码

### 5.1 `.claude/rules/api.md`

**line 27**: `"verify": { "type": "self" }` → `"verify": { "run": "true" }`

完整的 verify 配置示例也需要更新为新的 struct 格式：
```jsonc
// 自验证（agent 通过 wt step done 标记）
"verify": { "run": "true" }

// 脚本验证
"verify": {
  "run": "npm test",
  "on_pass": "success",
  "on_fail": "blocked"
}
```

### 5.2 `.claude/rules/concepts.md`

**IdleReason 表格**（约 line 49-57）: 缺少 `Timeout` 值。添加：

```
| `timeout` | 执行超时 |
```

### 5.3 `.claude/CLAUDE.md`

目录结构部分已过时，需要更新 `src/models/` 和 `src/services/` 的文件列表：

**src/models/** 新增文件（未列出）：
- `agent_step.rs` — AgentStep 配置
- `workflow.rs` — Workflow, WorkflowState
- `phase.rs` — Phase, PhaseResources, PhaseState
- `step.rs` — Step, StepState, StepResult, StepVerify
- `project.rs` — Project 配置
- `schema.rs` — JSON Schema 生成
- `state.rs` — TaskRuntimeState

**src/services/** 缺少详情：
- `executor/` 子目录：`phase.rs`, `step.rs`, `workflow.rs`, `context.rs`, `condition/`
- `observer/` 子目录：`terminal.rs`, `log.rs`, `sync.rs`

**src/commands/** 缺少：
- `init/` 子目录：`mod.rs`, `config.rs`, `templates.rs`
- `internal/` 目录
- `new.rs` — wt new（scratch 环境）
- `tail.rs` — wt tail
- `logs.rs` — wt logs

---

## 6. 处理过时的 spec 文件

`.claude/specs/phases-v2/` 目录中的 spec 文件是设计文档，现在实现已经偏离了它们的描述。

有两个选择：

**方案 A（推荐）**: 整个删除 `.claude/specs/phases-v2/` 目录。这些文件是开发过程中的设计稿，不是活文档。保留它们只会误导后续的 agent。

**方案 B**: 逐文件更新（工作量大且容易遗漏）。需要更新的内容包括：
- `api.md`: `"resources": "none"/"full"` → struct 格式，`"verify": { "type": "self" }` → struct 格式
- `architecture.md`: 同上
- `stories.md`: `"resources": "full"` → struct 格式

> `.claude/specs/code-quality-improvements.md` 和 `.claude/specs/roadmap.md` 需要检查是否仍有参考价值，酌情保留或删除。

---

## 7. executor/phase.rs 中的 stub 逻辑

`src/services/executor/phase.rs` 的 `allocate_resources` 方法（line 188-196）是一个 stub：

```rust
fn allocate_resources(&self, resources: &PhaseResources) -> Result<bool> {
    if resources.is_empty() {
        Ok(false)
    } else {
        // TODO: Actually create worktree, branch, window
        Ok(true)
    }
}
```

实际的资源分配逻辑在 `src/commands/next.rs` 的 `allocate_resources()` 函数中。这个 executor 层的方法是早期设计的残留，实际上从未被生产代码路径调用（只在同文件的测试中使用）。

**处理方式**: 如果 executor 层的 `PhaseExecutor` 未来不会承担资源分配职责，可以删除这个 stub 方法和相关的 TODO。

---

## 实施检查清单

按顺序执行，每步后 `cargo check && cargo test`：

```
[ ] 1. 删除 Cargo.toml 中 sha2, hex 依赖
[ ] 2. 删除 StepResult::with_attempt() 和 StepVerify::human_review()
[ ] 3. 删除 executor/phase.rs 中的 allocate_resources stub 的 TODO 注释
[ ] 4. 更新 .claude/rules/api.md 中的 verify 格式
[ ] 5. 更新 .claude/rules/concepts.md 添加 timeout IdleReason
[ ] 6. 更新 .claude/CLAUDE.md 目录结构
[ ] 7. 删除或归档 .claude/specs/phases-v2/ 目录
[ ] 8. cargo check 确认 0 error（允许 test-only 的 dead_code warning）
[ ] 9. cargo test 确认全部通过
```
