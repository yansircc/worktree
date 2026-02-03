# Agent Hooks 设计文档

## 概述

wt 的 hooks 系统和状态模型设计，支持多 agent 协作的现代开发工作流。

## 设计原则

1. **命令 = Hook** - 每个命令的行为完全由 hooks 定义
2. **全部 Hooks 化** - 没有特殊配置（如 `copy_files`、`start_args`），全部统一为 hooks
3. **状态 + 阶段分离** - 状态反映资源，阶段反映进度
4. **Pipeline 优先** - 多 agent 通过 stream-json 自动串联

## 状态模型

### 状态 (Status)

反映资源状态（进程是否在运行）：

```
Pending → Active ⇄ Idle → Completed
             │       │
             └───────┴── 当前阶段 (phase)
```

| 状态 | 含义 | 资源状态 |
|------|------|----------|
| Pending | 任务已定义，未创建资源 | 无 worktree，无分支 |
| Active | 有进程在运行 | worktree 存在，有 agent/script 在执行 |
| Idle | 资源存在但无进程 | worktree 存在，无 agent 运行，等待下一步 |
| Completed | 任务完成 | worktree 删除，分支合并 |

> 状态严格反映"是否有进程在运行"：
> - `wt run` 启动 agent → Active
> - agent 完成或用户暂停 → Idle
> - `wt review` 启动 pipeline → Active
> - pipeline 完成 → Idle
> - `wt complete` 执行合并 → Active → Completed

### 阶段 (Phase)

反映业务进度（预定义）：

| 阶段 | 说明 | 触发 |
|------|------|------|
| (none) | 未启动 | 任务创建后 |
| developing | 开发中 | `wt run` |
| reviewing | 审核中 | `wt review` |
| merging | 合并中 | `wt complete` |

### 状态 × 阶段 组合

| 状态 | 阶段 | 场景 |
|------|------|------|
| Pending | (none) | 任务已定义，依赖未完成 |
| Active | developing | agent 正在开发 |
| Idle | developing | agent 暂停，等待用户 |
| Active | reviewing | review pipeline 在运行 |
| Idle | reviewing | review 完成，等待下一步 |
| Active | merging | 合并/清理进行中 |
| Completed | (none) | 任务完成 |

### Idle 原因

当 `status: "idle"` 时，`idle_reason` 说明原因：

| 原因 | 说明 |
|------|------|
| `done` | 当前阶段正常完成，等待下一步 |
| `human_review` | 等待人工审核 |
| `error` | 命令/agent 执行出错（含中间崩溃） |
| `conflict` | 合并冲突待解决 |
| `timeout` | 执行超时 |
| `manual` | 用户手动暂停 |

### 默认状态转换

| 命令 | 开始时 | 成功结束 | 失败结束 |
|------|--------|----------|----------|
| run | Active + developing | Idle + developing | Idle + error |
| review | Active + reviewing | Idle + reviewing | Idle + error |
| resume | Active + (保持) | Idle + (保持) | Idle + error |
| complete | Active + merging | Completed | Idle + conflict/error |
| delete | - | (从 status.json 移除) | Idle + error |
| reset | - | Pending | Idle + error |

### 异常场景处理

| 场景 | 状态 | phase | idle_reason |
|------|------|-------|-------------|
| run 中途崩溃（worktree 已建） | Idle | developing | error |
| review pipeline 失败 | Idle | reviewing | error |
| complete 合并冲突 | Idle | merging | conflict |
| complete 后清理失败 | Idle | merging | error |
| 任务删除 | (移除记录) | - | - |
| 任务重置 | Pending | (none) | - |

## Step 类型

### 1. script

执行 shell 脚本，支持 `on_error` 定义失败时的备选步骤。

### 2. agent

运行 Claude agent，支持：
- `print`: 非交互模式 (-p)
- `model`: haiku/sonnet/opus
- `prompt`: 内联或 @file 引用
- `tools`: 可用工具列表
- `allowed_tools`: 自动批准的工具
- `skip_permissions`: 跳过权限提示
- `output_format`: text/json/stream-json
- 完整 Claude CLI 参数支持

### 3. internal

调用 wt 原子操作：

| 类别 | 操作 |
|------|------|
| worktree | create, destroy |
| branch | create, delete, merge, checkout |
| window | create, close, focus |
| files | copy, backup, clean |
| status | set, get |
| phase | set, get |
| notify | notify, confirm, abort |

### 4. condition

条件判断，支持 `if`/`then`/`else` 结构。

## Pipeline 模式

多个 agent 通过 stream-json 自动串联：

```bash
claude -p --output-format stream-json "prompt1" | \
claude -p --input-format stream-json --output-format stream-json "prompt2" | \
claude -p --input-format stream-json "prompt3"
```

前一个 agent 的输出自动成为后一个的上下文。

## 变量

所有 step 中可使用：

| 变量 | 说明 | 环境变量 |
|------|------|----------|
| `${task}` | 任务名 | `WT_TASK` |
| `${branch}` | 分支名 | `WT_BRANCH` |
| `${worktree}` | worktree 路径 | `WT_WORKTREE` |
| `${repo_root}` | 仓库根目录 | `WT_REPO_ROOT` |
| `${session}` | multiplexer session | `WT_SESSION` |
| `${window}` | multiplexer window | `WT_WINDOW` |
| `${phase}` | 当前阶段 | `WT_PHASE` |

## 错误处理

1. **默认行为**：step 失败 → 任务进入 Idle 状态，`idle_reason: "error"`
2. **on_error**：定义备选步骤
3. **on_conflict**：定义冲突处理策略

---

## 设计背景

### 为什么选择 JSONC？

- YAML 的问题：复杂结构表达不够精确
- JSONC 优点：支持注释、IDE 支持好、结构清晰
- 决策：使用 `.wt/config.jsonc`

### Pipeline 机制的发现

**来源**：通过 WebSearch 搜索 "Claude Code CLI print mode piping stdin stream-json"

**关键发现**（来自 [ruvnet/claude-flow Wiki](https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining)）：

Claude Code 支持 stream-json chaining，两个关键 flags：
- `--output-format stream-json` - 输出 NDJSON 格式
- `--input-format stream-json` - 接收 NDJSON 输入

### 状态模型演变

| 版本 | 状态 | 问题 |
|------|------|------|
| v1 (原始) | Pending/Running/Review/Completed | Review 含义模糊 |
| v2 | Active/Paused/Completed | 去掉 Pending 导致依赖任务无处归类 |
| v3 | Pending/Active/Idle/Completed | Codex 指出 Paused 和 Active 定义矛盾 |
| v4 (当前) | Pending/Active/Idle/Completed + phase + active_since | 解决所有问题 |

### Codex 辩论的关键洞察

**第一轮辩论**（v2 → v3）：
1. 去掉 Pending 后，依赖未完成的任务无处归类
2. review 阶段跑 pipeline 时状态定义矛盾
3. 未启动状态缺失

**第二轮辩论**（v3 → v4）：
1. 状态由谁设置不清晰 → 用户通过 hooks 设置，wt 提供默认
2. 中间崩溃场景 → Idle + error
3. complete 失败 → Idle + conflict/error
4. delete/reset 误用 Completed → delete 移除记录
5. pipeline 阻塞不可见 → active_since 监控

### 用户的开发工作流

1. 写任务文档 (human)
2. Agent 执行开发 (agent)
3. 另一个 Agent 核对任务 (agent) ← 多 agent 验证
4. Agent 处理合并冲突 (agent) ← 通过管道补充上下文
5. 可选：Agent 做合并后审核 (agent)
6. 完成 → 解锁下一层并行任务

关键特点：**多 agent 协作**，不是单 agent 执行。

---

## 参考资料

- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [ruvnet/claude-flow Wiki - Stream Chaining](https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining)
