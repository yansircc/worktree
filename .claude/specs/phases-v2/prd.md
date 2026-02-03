# PRD: Phases System v2

## 背景

wt 是一个多 agent 并行开发任务管理 CLI 工具。当前使用 Hooks 系统管理任务生命周期，存在以下问题：

1. **语义不清晰**：Hooks 暗示"钩入事件"，但实际是"达成阶段目标"
2. **命令与阶段耦合**：run/review/complete 硬编码，无法自定义阶段
3. **Step 类型复杂**：script/agent/internal/condition 四种类型
4. **状态管理混乱**：状态在不同层级，缺乏统一模型

## 产品目标

### 主要目标

1. **完整概念层级**：Project → Task → Phase → Workflow → Step
2. **状态从 Step 派生**：一切状态从最小执行单元开始
3. **执行/观测/验证分离**：Step 三个正交维度
4. **自动阶段推进**：workflow 成功则自动进入下一阶段
5. **配置分层覆盖**：Project → Task → Phase 逐层覆盖

### 非目标

1. 不改变 git worktree 的使用方式
2. 不改变 multiplexer (tmux/zellij) 的集成方式
3. 不添加新的外部依赖

## 概念层级

```
Project (项目)
  └─ Task (任务)
       └─ Phase (阶段)
            └─ Workflow (工作流)
                 └─ Step (步骤)
                      ├─ execute   # 执行器
                      ├─ observe   # 观测器
                      └─ verify    # 验证器
```

## 功能需求

### F1: Step - 最小执行单元

Step 包含三个正交维度：

| 维度 | 说明 | 工具 |
|------|------|------|
| Execute | 做什么 | run (script) / agent |
| Observe | 如何观测 | terminal / log |
| Verify | 如何验证 | self / script / agent / human / schema |

Step 状态：pending, running, success, failed, blocked, timeout, skipped

### F2: Workflow - Step 编排

Workflow 定义如何协调多个 Step：

- 执行模式：sequential / parallel / dag
- 错误处理：abort / continue / skip_dependents
- 上下文传递：steps 之间共享数据

### F3: Phase - 任务阶段

Phase 是任务生命周期的一个阶段：

- 资源定义：none / full
- Timing：on_enter / on_exit
- 前置条件：dependencies / phase / condition
- 自动推进：workflow success 时自动进入下一阶段

### F4: Task - 工作单元

Task 是可独立完成的工作单元：

- 对应一个 git 分支和 worktree
- 可依赖其他 tasks
- 可覆盖 phase 配置

### F5: Project - 顶层管理

Project 是 wt 管理的顶层概念：

- 全局配置：phases / workflows
- 并行策略：max_active_tasks / max_agents
- 观测通知：dashboard / slack

### F6: 状态派生链

```
StepState → WorkflowState → PhaseState → TaskStatus → ProjectStatus
```

所有状态从 Step 逐级向上派生，保证一致性。

### F7: Agent 标记命令

Agent 通过命令标记当前 Step 状态：

- `wt step done` - 标记成功
- `wt step block "原因"` - 标记阻塞
- `wt step fail "原因"` - 标记失败

### F8: Human 强制命令

Human 通过命令强制操作：

- `wt next` - 强制推进阶段
- `wt prev` - 强制回退阶段
- `wt stop` - 强制停止进程
- `wt reset` - 重置任务
- `wt delete` - 删除任务

## 成功指标

1. **概念清晰**：5 层概念，每层职责明确
2. **状态一致**：所有状态从 Step 派生
3. **自动化**：正常流程无需用户手动操作
4. **可扩展**：支持自定义阶段、workflow、验证器

## 时间线

| 阶段 | 内容 |
|------|------|
| Phase 1 | 核心模型（Step, Workflow, Phase, Task, Project） |
| Phase 2 | 执行引擎（StepExecutor, WorkflowExecutor） |
| Phase 3 | 状态管理（状态派生链） |
| Phase 4 | 命令重写（step, next, prev, stop） |
| Phase 5 | 清理（删除旧代码、更新文档） |
