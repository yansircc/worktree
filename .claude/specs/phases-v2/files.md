# 文件清单

本文档记录重构涉及的所有文件及其处置方式。

## 图例

| 标记 | 含义 |
|------|------|
| ✅ 保留 | 无需修改，直接复用 |
| 🔧 修改 | 需要修改以适配新模型 |
| 🔴 重写 | 保留文件名，但内容大幅重写 |
| ❌ 删除 | 不再需要，迁移后删除 |
| ➕ 新增 | 需要新建的文件 |

---

## src/ 目录

### 顶层文件

| 文件 | 处置 | 说明 |
|------|------|------|
| `main.rs` | 🔧 修改 | 更新命令路由（删除旧命令，添加新命令） |
| `lib.rs` | 🔧 修改 | 更新导出 |
| `cli.rs` | 🔴 重写 | 删除 Run/Review/Resume/Complete/Pause/Hooks/Pipeline，新增 Step/Prev，修改 Next/Stop |
| `error.rs` | 🔧 修改 | 添加 StepError, WorkflowError, PhaseError |
| `display.rs` | 🔧 修改 | 添加新状态的颜色和图标 |
| `constants.rs` | 🔧 修改 | 更新日志路径结构（`.wt/logs/{task}/{phase}/`） |

---

### src/models/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 更新导出（添加 Project, Phase, Workflow, Step 等） |
| `task.rs` | 🔧 修改 | 添加 phases override 支持，TaskFrontmatter 添加 phases/metadata 字段 |
| `task_parser.rs` | 🔧 修改 | 适配新的 frontmatter 格式 |
| `status.rs` | 🔴 重写 | 删除 TaskStatus/TaskPhase/IdleReason，改为从 state.rs 导入派生状态 |
| `store.rs` | 🔴 重写 | 添加 Project 存储，状态从 state.rs 派生 |
| `config.rs` | 🔴 重写 | 删除 HooksConfig/HookDef/Step，改为 phases/workflows 定义 |
| `agent_step.rs` | ❌ 删除 | 合并到新的 step.rs |
| `builtin_pipelines.rs` | ❌ 删除 | 改为 phases 默认配置（内置在 config.rs） |
| ➕ `project.rs` | ➕ 新增 | Project struct, ProjectStatus, ProjectConfig |
| ➕ `phase.rs` | ➕ 新增 | Phase struct, PhaseState, resources/prerequisites/timing |
| ➕ `workflow.rs` | ➕ 新增 | Workflow struct, WorkflowState, execution mode (sequential/parallel/dag) |
| ➕ `step.rs` | ➕ 新增 | Step struct (execute/input/output/observe/verify), StepState, StepResult |
| ➕ `state.rs` | ➕ 新增 | 状态派生链逻辑 (StepState → WorkflowState → PhaseState → TaskStatus) |

**详细说明：**

- `config.rs` 当前 711 行，主要是 HooksConfig/Step 相关，需要删除这些定义，改为：
  - `PhasesConfig`: 阶段序列和定义
  - `WorkflowsConfig`: workflow 片段库
  - `ProjectConfig`: 项目级配置（并行策略、观测等）

- `status.rs` 当前 594 行，需要：
  - 删除 TaskStatus/TaskPhase/IdleReason 独立定义
  - 改为从 state.rs 导入派生后的状态

---

### src/services/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 删除 hooks 导出，添加 executor/observer 导出 |
| `git.rs` | ✅ 保留 | 完全复用 |
| `command.rs` | ✅ 保留 | 完全复用 |
| `workspace.rs` | ✅ 保留 | 完全复用 |
| `dependency.rs` | ✅ 保留 | 复用，逻辑相同 |
| `claude.rs` | 🔧 修改 | ClaudeCommandBuilder 基本复用，适配新的 Step 参数 |
| `transcript.rs` | 🔧 修改 | 适配新日志结构 `.wt/logs/{task}/{phase}/` |
| `files.rs` | ✅ 保留 | backup/clean 功能复用 |
| `task_context.rs` | 🔧 修改 | 适配新的 ExecutionContext（添加 step_index, exit_reason 等） |
| `config_ops.rs` | ❌ 删除 | 逻辑合并到 models/config.rs |
| `status_ops.rs` | ❌ 删除 | 逻辑合并到 models/state.rs |
| `notify.rs` | ❌ 删除 | 合并到 observer/ |

---

### src/services/multiplexer/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | ✅ 保留 | 完全复用 |
| `tmux.rs` | ✅ 保留 | 完全复用 |
| `zellij.rs` | ✅ 保留 | 完全复用 |

---

### src/services/hooks/ → src/services/executor/

整个目录需要重构为 `executor/`：

| 当前文件 | 处置 | 说明 |
|----------|------|------|
| `mod.rs` (92 行) | ❌ 删除 | HooksEngine 不再需要 |
| `step.rs` (401 行) | 🔴 重写 → `executor/step.rs` | StepExecutor 重构，添加 verify/observe |
| `context.rs` (251 行) | 🔧 修改 → `executor/context.rs` | ExecutionContext 添加新变量 |
| `pipeline.rs` | ❌ 删除 | 改为 WorkflowExecutor |
| `pipeline_store.rs` | ❌ 删除 | 后台 pipeline 管理不再需要 |

**新增文件：**

| 文件 | 说明 |
|------|------|
| ➕ `executor/mod.rs` | 入口，导出 StepExecutor, WorkflowExecutor, PhaseTransition |
| ➕ `executor/step.rs` | StepExecutor（从 hooks/step.rs 重构） |
| ➕ `executor/workflow.rs` | WorkflowExecutor（支持 sequential/parallel/dag） |
| ➕ `executor/phase.rs` | PhaseTransition（资源转换 + on_enter/on_exit） |
| ➕ `executor/context.rs` | ExecutionContext（从 hooks/context.rs 迁移） |

---

### src/services/observer/ (新增目录)

| 文件 | 说明 |
|------|------|
| ➕ `mod.rs` | 入口 |
| ➕ `terminal.rs` | 终端观测（tmux/zellij 输出） |
| ➕ `log.rs` | 日志记录（step 输出 → 文件） |

---

### src/commands/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 删除旧命令导出，添加新命令导出 |
| `init.rs` | 🔧 修改 | 生成新格式的默认配置（phases 而非 hooks） |
| `create.rs` | 🔧 修改 | 适配新的 task frontmatter（phases/metadata） |
| `validate.rs` | 🔧 修改 | 验证新配置格式 |
| `list.rs` | 🔧 修改 | 显示 phase/step 信息 |
| `next.rs` | 🔴 重写 | 从"显示就绪任务"改为"强制推进阶段" |
| `reset.rs` | 🔴 重写 | 适配新状态模型（phase=null） |
| `delete.rs` | 🔧 修改 | 适配新状态模型 |
| `tail.rs` | 🔧 修改 | 适配新日志结构 |
| `logs.rs` | 🔧 修改 | 适配新日志结构 |
| `new.rs` | 🔧 修改 | 适配新状态模型 |
| `completions.rs` | 🔧 修改 | 更新命令列表 |
| `run.rs` | ❌ 删除 | 被 `wt next`（从 pending 推进）替代 |
| `review.rs` | ❌ 删除 | 被自动阶段推进替代 |
| `complete.rs` | ❌ 删除 | 被自动阶段推进替代 |
| `pause.rs` | ❌ 删除 | 被 `wt stop` 替代 |
| `resume.rs` | ❌ 删除 | 被 `wt next`（从 idle 恢复）替代 |
| `hooks_cmd.rs` | ❌ 删除 | Hooks 系统被删除 |
| `pipeline_cmd.rs` | ❌ 删除 | Pipeline 系统被删除 |
| ➕ `step.rs` | ➕ 新增 | `wt step done/block/fail` - Agent 标记状态 |
| ➕ `prev.rs` | ➕ 新增 | `wt prev <task>` - 强制回退阶段 |
| ➕ `stop.rs` | ➕ 新增 | `wt stop <task>` - 停止进程（现在可能在 pause.rs，需要重命名） |

**注意：** 当前没有独立的 `stop.rs`，停止功能在 `pause.rs` 中。需要：
1. 删除 `pause.rs`
2. 新建 `stop.rs`（或重命名）

---

### src/commands/internal/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 适配新状态模型 |
| `git.rs` | ✅ 保留 | 基本复用 |
| `mux.rs` | ✅ 保留 | 基本复用 |
| `misc.rs` | 🔧 修改 | 适配新状态模型 |

---

### src/commands/status/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 适配新状态模型 |
| `types.rs` | 🔴 重写 | 显示派生状态（Step → Workflow → Phase → Task） |
| `display.rs` | 🔴 重写 | 渲染新的状态层级 |
| `actions.rs` | 🔧 修改 | 更新可用 actions（删除 review/resume，适配 next/prev/stop） |

---

### src/tui/

| 文件 | 处置 | 说明 |
|------|------|------|
| `mod.rs` | 🔧 修改 | 事件处理适配新命令 |
| `app.rs` | 🔴 重写 | 显示派生状态层级 |
| `ui.rs` | 🔴 重写 | 渲染新的状态信息 |

---

## tests/ 目录

### tests/cli/

| 文件 | 处置 | 说明 |
|------|------|------|
| `init.rs` | 🔧 修改 | 验证新配置格式 |
| `create.rs` | 🔧 修改 | 适配新 frontmatter |
| `validate.rs` | 🔧 修改 | 验证新配置 |
| `list.rs` | 🔧 修改 | 验证新状态显示 |
| `next.rs` | 🔴 重写 | 从"显示就绪"改为"强制推进" |
| `status.rs` | 🔧 修改 | 适配新状态模型 |
| `reset.rs` | 🔧 修改 | 适配新状态模型 |
| `delete.rs` | 🔧 修改 | 适配新状态模型 |
| `tail.rs` | 🔧 修改 | 适配新日志结构 |
| `logs.rs` | 🔧 修改 | 适配新日志结构 |
| `scratch.rs` | 🔧 修改 | 适配新状态模型 |
| `completions.rs` | ✅ 保留 | 基本复用 |
| `help.rs` | ✅ 保留 | 基本复用 |
| `no_config.rs` | 🔧 修改 | 验证新默认配置 |
| `review.rs` | ❌ 删除 | 命令已删除 |
| `resume.rs` | ❌ 删除 | 命令已删除 |
| ➕ `step.rs` | ➕ 新增 | 测试 `wt step done/block/fail` |
| ➕ `prev.rs` | ➕ 新增 | 测试 `wt prev` |
| ➕ `stop.rs` | ➕ 新增 | 测试 `wt stop` |

### tests/integration/

| 文件 | 处置 | 说明 |
|------|------|------|
| `cycle_detection.rs` | ✅ 保留 | 依赖检测逻辑不变 |
| `edge_cases.rs` | 🔧 修改 | 适配新状态模型 |
| `error_messages.rs` | 🔧 修改 | 更新错误消息断言 |
| `json_output.rs` | 🔧 修改 | 适配新 JSON 格式 |
| `task_store.rs` | 🔴 重写 | 适配新的 store/state 模型 |

### 其他测试文件

| 文件 | 处置 | 说明 |
|------|------|------|
| `cli.rs` | 🔧 修改 | 入口文件，更新模块导入 |
| `integration.rs` | 🔧 修改 | 入口文件，更新模块导入 |
| `common.rs` | 🔧 修改 | 测试辅助函数，适配新状态 |

---

## 统计

### src/ 文件处置统计

当前文件数：64

| 处置 | 数量 | 文件分布 |
|------|------|----------|
| ✅ 保留 | 8 | services: git.rs, command.rs, workspace.rs, dependency.rs, files.rs; multiplexer: mod.rs, tmux.rs, zellij.rs |
| 🔧 修改 | 30 | 顶层(5): main, lib, error, display, constants; models(3): mod, task, task_parser; services(4): mod, claude, transcript, task_context; hooks(1): context→executor/context; commands(10): mod, init, create, validate, list, delete, tail, logs, new, completions; internal(4): all; status(2): mod, actions; tui(1): mod |
| 🔴 重写 | 11 | 顶层(1): cli; models(3): status, store, config; hooks(1): step→executor/step; commands(2): next, reset; status(2): types, display; tui(2): app, ui |
| ❌ 删除 | 15 | models(2): agent_step, builtin_pipelines; services(3): config_ops, status_ops, notify; hooks(3): mod, pipeline, pipeline_store; commands(7): run, review, complete, pause, resume, hooks_cmd, pipeline_cmd |
| ➕ 新增 | 14 | models(5): project, phase, workflow, step, state; executor(3): mod, workflow, phase; observer(3): mod, terminal, log; commands(3): step, prev, stop |

**验证：** 8 + 30 + 11 + 15 = 64 ✓（当前）→ 64 - 15 + 14 = 63（目标，因为迁移的 2 个文件已在修改/重写中计算）

**迁移说明：**
- hooks/context.rs → executor/context.rs（算作修改，不重复计新增）
- hooks/step.rs → executor/step.rs（算作重写，不重复计新增）
- pause.rs 删除 → stop.rs 新增（功能相似但语义不同）

### tests/ 文件处置统计

当前文件数：24

| 处置 | 数量 | 说明 |
|------|------|------|
| ✅ 保留 | 3 | completions.rs, help.rs, cycle_detection.rs |
| 🔧 修改 | 17 | cli.rs, common.rs, integration.rs, init.rs, create.rs, validate.rs, list.rs, status.rs, reset.rs, delete.rs, tail.rs, logs.rs, scratch.rs, no_config.rs, edge_cases.rs, error_messages.rs, json_output.rs |
| 🔴 重写 | 2 | next.rs, task_store.rs |
| ❌ 删除 | 2 | review.rs, resume.rs |
| ➕ 新增 | 3 | step.rs, prev.rs, stop.rs |

**验证：** 3 + 17 + 2 + 2 = 24 ✓

---

## 依赖关系

新增文件的依赖关系（实施顺序）：

```
Phase 1: 核心模型
  step.rs (无依赖)
    ↓
  workflow.rs (依赖 step.rs)
    ↓
  phase.rs (依赖 workflow.rs)
    ↓
  project.rs (依赖 phase.rs)
    ↓
  state.rs (依赖以上所有)

Phase 2: 执行引擎
  executor/context.rs (从 hooks/context.rs 迁移)
    ↓
  executor/step.rs (依赖 context.rs)
    ↓
  executor/workflow.rs (依赖 step.rs)
    ↓
  executor/phase.rs (依赖 workflow.rs)

  observer/* (可并行开发)

Phase 3: 状态管理
  重写 status.rs, store.rs (依赖 state.rs)

Phase 4: 命令重写
  新增 step.rs, prev.rs, stop.rs
  重写 next.rs, reset.rs
  删除旧命令

Phase 5: 清理
  删除 hooks/*
  更新文档
```

---

## 迁移检查清单

### Phase 1 完成条件

- [ ] `src/models/step.rs` - Step, StepState, StepResult 定义
- [ ] `src/models/workflow.rs` - Workflow, WorkflowState 定义
- [ ] `src/models/phase.rs` - Phase, PhaseState 定义
- [ ] `src/models/project.rs` - Project, ProjectStatus 定义
- [ ] `src/models/state.rs` - 状态派生逻辑
- [ ] 单元测试通过

### Phase 2 完成条件

- [ ] `src/services/executor/mod.rs`
- [ ] `src/services/executor/context.rs`
- [ ] `src/services/executor/step.rs` - verify/observe 支持
- [ ] `src/services/executor/workflow.rs` - sequential/parallel/dag
- [ ] `src/services/executor/phase.rs` - on_enter/on_exit, 资源转换
- [ ] `src/services/observer/mod.rs`
- [ ] `src/services/observer/terminal.rs`
- [ ] `src/services/observer/log.rs`
- [ ] 单元测试通过

### Phase 3 完成条件

- [ ] `src/models/config.rs` 重写 - phases/workflows
- [ ] `src/models/status.rs` 重写 - 派生状态
- [ ] `src/models/store.rs` 重写 - 包含 Project
- [ ] 集成测试通过

### Phase 4 完成条件

- [ ] `src/cli.rs` 重写
- [ ] `src/commands/step.rs` 新增
- [ ] `src/commands/prev.rs` 新增
- [ ] `src/commands/stop.rs` 新增
- [ ] `src/commands/next.rs` 重写
- [ ] `src/commands/reset.rs` 重写
- [ ] 删除 run.rs, review.rs, complete.rs, pause.rs, resume.rs, hooks_cmd.rs, pipeline_cmd.rs
- [ ] CLI 测试通过

### Phase 5 完成条件

- [ ] 删除 `src/services/hooks/` 目录
- [ ] 删除 `src/models/agent_step.rs`
- [ ] 删除 `src/models/builtin_pipelines.rs`
- [ ] 删除 `src/services/config_ops.rs`
- [ ] 删除 `src/services/status_ops.rs`
- [ ] 删除 `src/services/notify.rs`
- [ ] 更新 README.md
- [ ] 更新 CLAUDE.md
- [ ] 全量测试通过
