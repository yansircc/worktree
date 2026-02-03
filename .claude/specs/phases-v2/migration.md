# 迁移计划

## 概述

从 Hooks 系统迁移到 Phases 系统，分 5 个阶段实施。

**重要文档：** 详细的文件清单见 [files.md](./files.md)，记录了每个文件的处置方式（保留/修改/重写/删除/新增）。

## 改动统计

| 处置 | src/ (64 files) | tests/ (24 files) |
|------|-----------------|-------------------|
| ✅ 保留 | 8 | 3 |
| 🔧 修改 | 30 | 17 |
| 🔴 重写 | 11 | 2 |
| ❌ 删除 | 15 | 2 |
| ➕ 新增 | 14 | 3 |

**目标文件数：** src/ 63 files, tests/ 25 files

详见 [files.md](./files.md)（包含每个文件的详细处置说明和验收检查清单）

## 当前状态

### 现有文件结构

```
src/
├── models/
│   ├── config.rs         # HooksConfig, Step, HookDef (711 行) → 重写
│   ├── status.rs         # TaskStatus, TaskPhase (594 行) → 重写
│   ├── store.rs          # TaskStore (741 行) → 重写
│   ├── agent_step.rs     # AgentStep → 删除
│   ├── builtin_pipelines.rs → 删除
│   └── ...
├── services/
│   └── hooks/            # Hooks 引擎 (5 文件) → 删除，改为 executor/
│   ├── config_ops.rs     → 删除
│   ├── status_ops.rs     → 删除
│   └── notify.rs         → 删除
├── commands/
│   ├── run.rs            → 删除
│   ├── review.rs         → 删除
│   ├── complete.rs       → 删除
│   ├── pause.rs          → 删除
│   ├── resume.rs         → 删除
│   ├── hooks_cmd.rs      → 删除
│   ├── pipeline_cmd.rs   → 删除
│   └── ...
```

## 迁移阶段

### Phase 1: 核心模型

**目标：** 定义新的数据结构

**任务：**

1. 新增 `src/models/step.rs`
   - Step struct（execute, input, output, observe, verify）
   - StepState enum
   - StepResult struct

2. 新增 `src/models/workflow.rs`
   - Workflow struct（steps, execution, context）
   - WorkflowState enum

3. 新增 `src/models/phase.rs`
   - Phase struct（on_enter, on_exit, resources）
   - PhaseState enum

4. 新增 `src/models/project.rs`
   - Project struct（phases, workflows, concurrency）
   - ProjectStatus struct

5. 扩展 `src/models/task.rs`
   - 添加 phases override 支持

6. 更新 `src/models/config.rs`
   - 添加 phases, workflows 字段
   - 实现配置合并逻辑

**测试：**
- 单元测试：各模型序列化/反序列化
- 单元测试：状态派生逻辑
- 单元测试：配置合并逻辑

---

### Phase 2: 执行引擎

**目标：** 实现新的执行引擎

**任务：**

1. 新增 `src/services/executor/step.rs`
   - StepExecutor
   - 执行 run/agent
   - 处理 input/output
   - 处理 observe（terminal/log）
   - 处理 verify（self/script/agent/human/schema）

2. 新增 `src/services/executor/workflow.rs`
   - WorkflowExecutor
   - 执行模式（sequential/parallel/dag）
   - 错误处理
   - 上下文传递

3. 新增 `src/services/executor/phase.rs`
   - PhaseTransition
   - 资源转换（none ↔ full）
   - on_enter/on_exit 执行
   - 自动阶段推进

4. 新增 `src/services/observer/`
   - terminal.rs（tmux/zellij 观测）
   - log.rs（日志记录）

**测试：**
- 单元测试：StepExecutor
- 单元测试：WorkflowExecutor
- 集成测试：完整工作流

---

### Phase 3: 状态管理

**目标：** 实现状态派生链

**任务：**

1. 新增 `src/models/state.rs`
   - StepState 派生逻辑
   - WorkflowState 派生逻辑
   - PhaseState 派生逻辑
   - TaskStatus 派生逻辑
   - ProjectStatus 派生逻辑

2. 更新 `src/models/status.rs`
   - 使用新的状态模型
   - 状态持久化

3. 实现状态观测
   - 资源检测（worktree/branch/window）
   - 进程检测

**测试：**
- 单元测试：各级状态派生
- 集成测试：状态与资源一致性

---

### Phase 4: 命令重写

**目标：** 实现新命令

**任务：**

1. 新增 `src/commands/step.rs`
   - `wt step done`
   - `wt step block`
   - `wt step fail`

2. 新增 `src/commands/next.rs`
   - 强制推进阶段

3. 新增 `src/commands/prev.rs`
   - 强制回退阶段

4. 更新 `src/commands/stop.rs`
   - 停止进程，保持状态

5. 更新 CLI 定义
   - 添加新命令
   - 删除旧命令

6. 删除旧命令
   - run.rs
   - review.rs
   - complete.rs
   - pause.rs
   - resume.rs

**测试：**
- CLI 测试：所有新命令
- 集成测试：完整流程

---

### Phase 5: 清理

**目标：** 删除旧代码，更新文档

**任务：**

1. 删除旧代码
   - `src/services/hooks/` 目录
   - `src/models/builtin_pipelines.rs`
   - 旧命令文件

2. 更新配置
   - 删除 hooks 字段
   - 只保留 phases 相关

3. 更新文档
   - README.md
   - CLAUDE.md

4. 清理测试
   - 删除旧测试
   - 确保新测试覆盖

**测试：**
- 全量测试通过
- 手动测试完整流程

---

## 模块结构（目标）

```
src/
├── models/
│   ├── mod.rs
│   ├── project.rs        # Project
│   ├── task.rs           # Task
│   ├── phase.rs          # Phase
│   ├── workflow.rs       # Workflow
│   ├── step.rs           # Step
│   ├── state.rs          # 状态派生
│   ├── config.rs         # 配置
│   └── store.rs          # 存储
│
├── services/
│   ├── mod.rs
│   ├── git.rs
│   ├── multiplexer/
│   ├── executor/
│   │   ├── mod.rs
│   │   ├── step.rs
│   │   ├── workflow.rs
│   │   └── phase.rs
│   └── observer/
│       ├── mod.rs
│       ├── terminal.rs
│       └── log.rs
│
├── commands/
│   ├── mod.rs
│   ├── step.rs           # wt step
│   ├── next.rs           # wt next
│   ├── prev.rs           # wt prev
│   ├── stop.rs           # wt stop
│   ├── reset.rs
│   ├── delete.rs
│   ├── list.rs
│   └── status/
│
└── tui/
```

---

## 测试策略

### 每个阶段的测试要求

| 阶段 | 单元测试 | 集成测试 | CLI 测试 |
|------|----------|----------|----------|
| Phase 1 | 模型、派生 | - | - |
| Phase 2 | 执行器 | 工作流 | - |
| Phase 3 | 状态派生 | 状态一致性 | - |
| Phase 4 | - | 阶段转换 | 所有命令 |
| Phase 5 | - | 完整流程 | 全量回归 |

### 回归测试

每个阶段完成后：
```bash
cargo test --lib
cargo test --test cli
```

---

## 时间线建议

| 阶段 | 预估 | 依赖 |
|------|------|------|
| Phase 1 | 2 sessions | 无 |
| Phase 2 | 2 sessions | Phase 1 |
| Phase 3 | 1 session | Phase 2 |
| Phase 4 | 2 sessions | Phase 3 |
| Phase 5 | 1 session | Phase 4 |

总计：约 8 个 session
