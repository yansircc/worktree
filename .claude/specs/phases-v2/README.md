# Phases System v2 - 规格文档

## 概述

wt 是一个多 agent 并行开发任务管理 CLI 工具。通过 git worktree 隔离工作区，让多个 AI agent 并行开发不同功能分支。

本规格定义了完整的概念体系和任务生命周期管理。

## 文档结构

| 文档 | 内容 | 读者 |
|------|------|------|
| [prd.md](./prd.md) | 产品需求、目标、非目标 | 产品/开发 |
| [stories.md](./stories.md) | 用户故事和验收标准 | 开发/测试 |
| [architecture.md](./architecture.md) | 技术架构、数据模型 | 开发 |
| [api.md](./api.md) | CLI 命令、配置格式 | 开发/用户 |
| [migration.md](./migration.md) | 迁移计划、阶段划分 | 开发 |
| [files.md](./files.md) | **文件清单**（每个文件的处置方式） | 开发 |
| [decisions.md](./decisions.md) | 设计决策记录 (ADR) | 开发 |

## 核心概念层级

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

## 状态派生链

```
StepState → WorkflowState → PhaseState → TaskStatus → ProjectStatus

状态从最底层的 Step 逐级向上派生，保证一致性。
```

## 核心设计原则

1. **状态从 Step 派生** - 一切状态从最小执行单元开始
2. **自动阶段推进** - workflow 成功则自动进入下一阶段
3. **执行/观测/验证分离** - Step 三个正交维度
4. **配置分层覆盖** - Project → Task → Phase 逐层覆盖

## 命令体系

```bash
# Agent 标记 Step 状态
wt step done              # 当前 step → success
wt step block "原因"       # 当前 step → blocked
wt step fail "原因"        # 当前 step → failed

# Human 强制操作
wt next <task>            # 强制推进到下一阶段
wt prev <task>            # 强制回退到上一阶段
wt stop <task>            # 强制停止当前进程
wt reset <task>           # 重置到 pending
wt delete <task>          # 彻底移除
```

## 快速开始（开发者）

1. 阅读 [prd.md](./prd.md) 了解产品目标
2. 阅读 [architecture.md](./architecture.md) 了解完整概念模型
3. 阅读 [stories.md](./stories.md) 了解具体需求
4. 按 [migration.md](./migration.md) 中的计划实施

## 状态

- [x] 规格设计
- [x] 文件清单（files.md）
- [ ] Phase 1: 核心模型
- [ ] Phase 2: 执行引擎
- [ ] Phase 3: 状态管理
- [ ] Phase 4: 命令重写
- [ ] Phase 5: 清理
