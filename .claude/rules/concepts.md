# 核心概念

## 概念层级

```
Project
  └─ Task                        # 独立工作单元，对应 git 分支 + worktree
       └─ Phase                  # 生命周期阶段 (pending → developing → reviewing → completed)
            ├─ on_enter Workflow  # 进入时执行
            └─ on_exit Workflow   # 退出时执行
                 └─ Step          # 最小执行单元 (脚本 or agent)
```

## 状态派生链

状态从底层向上派生，高层状态是低层状态的聚合：

```
StepState → WorkflowState → PhaseState → TaskStatus
```

### StepState (最小粒度)

```
pending → running → success / failed / blocked / timeout / skipped
```

### WorkflowState

从所有 Step 的状态聚合：

| 条件 | WorkflowState |
|------|---------------|
| 所有 step 成功 | Success |
| 任一 step 失败 | Failed |
| 任一 step 阻塞 | Blocked |
| 有 step 在运行 | Running |
| 全部待执行 | Pending |

### PhaseState

直接映射 WorkflowState (on_enter 的结果)。

### TaskStatus (用户可见)

两个正交维度：

- **Status** — 资源状态 (有没有进程在跑)
- **Phase** — 业务阶段 (开发到哪一步了)

```
Status:  ○ Pending  →  ● Active  ⇄  ◐ Idle  →  ✓ Completed
Phase:   (none)     →  developing → reviewing →  (done)
```

| Status | Phase | 场景 |
|--------|-------|------|
| Pending | (none) | 任务已定义，未创建资源 |
| Active | developing | agent 正在编码 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review 进行中 |
| Completed | (none) | 任务完成 |

### IdleReason

Idle 状态下的具体原因：

| 值 | 说明 |
|----|------|
| `done` | 当前阶段正常完成 |
| `human_review` | 等待人工审核 |
| `error` | 命令执行出错 |
| `conflict` | 合并冲突待解决 |
| `timeout` | 执行超时 |
| `manual` | 用户手动暂停 |

## 任务生命周期

### 正常流程 (自动推进)

```
wt next task
  → 创建 worktree/branch/window
  → 执行 on_enter workflow (启动 agent)
  → agent 工作中 (Active/developing)
  → agent 调用 wt step done
  → workflow 成功 → 自动推进到 reviewing
  → review 通过 → 推进到 completed
```

### 人工干预 (命令是强制标记)

正常流程无需用户操作。命令只用于手动干预：

| 命令 | 场景 |
|------|------|
| `wt next` | 跳过失败、手动推进 |
| `wt prev` | 回退到上一阶段重做 |
| `wt stop` | 暂停 agent 进程 |
| `wt reset` | 重来，备份代码后清理 |

## 依赖规则

- 任务只有在**所有依赖都 Completed** 后才能启动
- `wt validate` 检测循环依赖
- `wt reset` 在清理前备份代码到 `.wt/backups/`

## 设计原则

1. **状态从 Step 派生** — 一切状态从最小执行单元向上聚合
2. **自动阶段推进** — workflow 成功则自动进入下一阶段
3. **命令是强制标记** — 正常流程无需用户操作
4. **执行/观测/验证分离** — Step 的三个正交维度
5. **配置分层覆盖** — 内置默认 → 项目配置 → 任务覆盖
