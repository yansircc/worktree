# Dead Code Cleanup Spec

## 目标

删除所有未使用的代码，简化项目结构，消除编译警告。

## 当前状态

- 39 个死代码警告
- 两套并行的状态系统 (v1 实际使用, v2 部分定义但未集成)
- 大量为未来功能预留但从未使用的代码

## 清理原则

1. **删除所有 `cargo build` 报告的死代码**
2. **保留测试中使用的代码**（如果只在测试中用，考虑删除测试）
3. **不破坏现有功能**（所有测试必须通过）
4. **简化为单一状态系统**

## 清理清单

### Phase 1: 删除未使用的错误变体

**文件**: `src/error.rs`

删除:
- [ ] `DependencyNotCompleted`
- [ ] `AlreadyActive`
- [ ] `Script`
- [ ] `HookFailed`

### Phase 2: 清理 models/project.rs

**删除整个结构体/枚举**:
- [ ] `MultiplexerType` (config.rs 已有同名实现)
- [ ] `ResourceConfig`
- [ ] `ResourceLimits`
- [ ] `NotifyBackend`
- [ ] `NotificationConfig`
- [ ] `Project`

**保留** (被 config.rs re-export 并使用):
- `ProjectStatus` - 被 store.rs 使用
- `ConcurrencyConfig` - 被 config.rs re-export
- `PhasesConfig` - 被 config.rs re-export
- `ProjectObserve` - 被 config.rs re-export

### Phase 3: 清理 models/state.rs

**删除函数**:
- [ ] `derive_workflow_state()` - 未使用
- [ ] `derive_phase_state()` - 未使用
- [ ] `derive_project_status()` - 未使用
- [ ] `derive_task_status()` - 未使用
- [ ] `derive_task_status_from_steps()` - 未使用

**保留**:
- `DerivedTaskStatus` - 被 status.rs 和 store.rs 使用
- `TaskRuntimeState` - 被 next.rs, prev.rs, executor/phase.rs 使用

### Phase 4: 清理 models/step.rs

**删除**:
- [ ] `VerifyType` enum
- [ ] `StepExecute` enum
- [ ] 相关的 impl 方法

**保留**:
- `Step` struct (被 workflow.rs 使用)
- `StepState` enum (被 executor 使用)
- `StepResult` struct (被 executor 使用)
- `ObserveMode`, `ObserveConfig` (被 next.rs 使用)

### Phase 5: 清理 models/workflow.rs

**检查并删除未使用的**:
- [ ] `Workflow::new()` 如果未使用
- [ ] `Workflow::from_scripts()` 如果未使用
- [ ] 其他未使用的方法

### Phase 6: 清理 models/status.rs

**删除未使用的方法**:
- [ ] `TaskState::to_derived_status()`
- [ ] `TaskState::from_runtime_state()`
- [ ] `StatusStore::get_derived_status()`
- [ ] `StatusStore::all_derived_statuses()`
- [ ] `StatusStore::count_by_derived_status()`

### Phase 7: 清理 models/store.rs

**删除未使用的方法**:
- [ ] `TaskStore::get_derived_status()`
- [ ] `TaskStore::project_status()`
- [ ] `TaskStore::full_project_status()`
- [ ] `TaskStore::is_phases_v2_enabled()`
- [ ] `TaskStore::phase_sequence()`
- [ ] `TaskContext` 相关未使用方法

### Phase 8: 清理 models/config.rs

**删除未使用的方法**:
- [ ] `WtConfig::is_phases_v2()`
- [ ] `WtConfig::max_active_tasks()`
- [ ] `WtConfig::max_agents()`

### Phase 9: 清理 models/phase.rs

**删除未使用的方法**:
- [ ] `Phase::display_name()`
- [ ] `Phase::needs_resources()`
- [ ] `Phase::with_on_enter()`
- [ ] `Phase::with_on_exit()`
- [ ] `PhaseResources::needs_worktree()`
- [ ] `PhaseResources::needs_branch()`
- [ ] `PhaseResources::needs_window()`
- [ ] `PhaseState::is_terminal()`
- [ ] `PhaseState::is_success()`
- [ ] `PhaseState::icon()`
- [ ] `default_phases()`

### Phase 10: 清理 services/executor/

**删除未使用的**:
- [ ] `StepExecutor` 如果未使用
- [ ] `WorkflowExecutor` / `WorkflowResult` 如果未使用
- [ ] `PhaseTransitionResult` 如果未使用
- [ ] `TerminalObserver` / `TerminalSettings` 如果未使用
- [ ] `LogObserver` / `create_workflow_log_entry` 如果未使用

### Phase 11: 清理 services/multiplexer/

**检查**:
- [ ] `branch_name()` 函数

### Phase 12: 清理 services/task_context.rs

**删除未使用的方法**:
- [ ] `require_not_scratch()`
- [ ] `require_status()`
- [ ] `require_instance()`
- [ ] `require_worktree()`
- [ ] `validate_transition()`
- [ ] `build_hook_context()`

### Phase 13: 清理 services/observer/

**删除**:
- [ ] `LogObserver::write()`
- [ ] `LogObserver::writeln()`
- [ ] `LogObserver::load_workflow_context()`
- [ ] `LogObserver::read_step_log()`
- [ ] `LogObserver::list_step_logs()`
- [ ] `StepLogEntry` struct

### Phase 14: 修复 Clippy 警告

- [ ] `commands/list.rs:238` - 重构 `print_tree_node` 减少参数
- [ ] `commands/step.rs:30` - 实现 `FromStr` trait
- [ ] `commands/next.rs:120-121` - 使用 `is_some_and`/`is_none_or`

### Phase 15: 清理未使用的导入

- [ ] `src/services/executor/step.rs` - `crate::error::Result`
- [ ] `src/services/executor/phase.rs` - `crate::models::step::Step`
- [ ] `src/tui/mod.rs` - 未使用的 `worktree` 变量

### Phase 16: 更新 models/mod.rs 导出

删除不再存在的导出。

## 验收标准

1. `cargo build 2>&1 | grep warning` 输出为空
2. `cargo clippy` 无警告
3. `cargo test` 全部通过 (lib + cli + integration)
4. 代码行数显著减少

## 执行顺序

1. 先删除明显独立的死代码（error variants, 独立函数）
2. 删除未使用的结构体/枚举
3. 删除未使用的方法
4. 清理导入和导出
5. 修复 Clippy 警告
6. 运行测试确认

## 风险

- 某些代码可能只在测试中使用，需要同时删除测试
- 某些看似未使用的 pub 方法可能是 API 预留，但作为个人项目可以删除
