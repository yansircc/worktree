# 核心概念

## 概念层级

```
Project (项目)
  └─ Task (任务)
       └─ Phase (阶段)
            └─ Workflow (工作流)
                 └─ Step (步骤)
```

| 概念 | 说明 |
|------|------|
| Project | 顶层管理，包含全局配置和所有任务 |
| Task | 可独立完成的工作单元，对应一个 git 分支和 worktree |
| Phase | 任务生命周期的一个阶段 (pending → developing → reviewing → completed) |
| Workflow | Step 的编排，定义如何协调多个步骤 |
| Step | 最小执行单元，包含执行器(run/agent)、观测器、验证器 |

## 状态派生链

状态从最底层的 Step 逐级向上派生：

```
StepState → WorkflowState → PhaseState → TaskStatus → ProjectStatus
```

### StepState

```
pending → running → success/failed/blocked/timeout/skipped
```

### TaskStatus

两个维度描述：
- **Status** - 资源状态：Pending / Active / Idle / Completed
- **Phase** - 业务阶段：(none) / developing / reviewing / (done)

| Status | Phase | 场景 |
|--------|-------|------|
| Pending | (none) | 任务已定义，未创建资源 |
| Active | developing | agent 正在开发 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review 进行中 |
| Completed | (none) | 任务完成 |

### IdleReason

| 原因 | 说明 |
|------|------|
| `done` | 当前阶段正常完成 |
| `human_review` | 等待人工审核 |
| `error` | 命令执行出错 |
| `conflict` | 合并冲突待解决 |
| `manual` | 用户手动暂停 |

## 依赖规则

- 任务只能在**所有依赖都 Completed** 后才能启动
- `wt validate` 检测循环依赖
- `wt reset` 在清理前备份代码到 `.wt/backups/`

## 设计原则

1. **状态从 Step 派生** - 一切状态从最小执行单元开始
2. **自动阶段推进** - workflow 成功则自动进入下一阶段
3. **命令是强制标记** - 正常流程无需用户操作，命令只用于手动干预（如跳过失败、手动修复后推进）
4. **执行/观测/验证分离** - Step 三个正交维度
5. **配置分层覆盖** - 内置默认 → Project → Task 逐层覆盖
