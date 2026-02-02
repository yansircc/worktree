# Agent Hooks 规格 v4

## 概述

重新设计 wt 的 hooks 系统和状态模型，支持多 agent 协作的现代开发工作流。

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

### 状态文件结构

```json
{
  "tasks": {
    "auth": {
      "status": "active",
      "phase": "developing",
      "idle_reason": null,
      "active_since": "2026-02-03T10:30:00Z",
      "instance": {
        "branch": "wt/auth-abc123",
        "worktree_path": ".wt/worktrees/auth",
        "session_name": "wt",
        "window_name": "auth"
      }
    }
  }
}
```

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

### 状态管理

**状态由 hooks 显式设置**，wt 提供默认配置：

```jsonc
// 用户可在 step 中显式设置状态
{ "type": "internal", "run": "status:set active" }
{ "type": "internal", "run": "phase:set reviewing" }

// 或使用默认行为（wt 在 hook 前后自动设置）
```

**默认状态转换**（用户不配置时）：

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

### 监控字段

```json
{
  "tasks": {
    "auth": {
      "status": "active",
      "phase": "developing",
      "idle_reason": null,
      "active_since": "2026-02-03T10:30:00Z",  // 进入 Active 的时间
      "instance": { ... }
    }
  }
}
```

`active_since` 用于监控：
- 检测长时间 Active 但无进展的任务
- human 或 agent 可据此判断是否需要介入

## 配置格式

### 文件

- 路径: `.wt/config.jsonc`
- 格式: JSONC (JSON with Comments)

### 完整示例

```jsonc
{
  // ============================================
  // 基础配置
  // ============================================
  "multiplexer": "tmux",
  "session_name": "my-project",
  "claude_command": "claude",
  "worktree_dir": ".wt/worktrees",

  // ============================================
  // Hooks - 每个命令的行为定义
  // ============================================
  "hooks": {

    // wt run: 启动开发
    // 状态: Pending → Active, 阶段: → developing
    "run": [
      // 资源创建（wt 内置操作）
      { "type": "internal", "run": "worktree:create" },
      { "type": "internal", "run": "branch:create" },

      // 环境准备（替代 copy_files + on_create）
      { "type": "script", "run": "cp .env.example .env" },
      { "type": "script", "run": "npm install" },

      // 启动 agent（替代 start_args）
      { "type": "internal", "run": "window:create" },
      {
        "type": "agent",
        "interactive": true,
        "model": "sonnet",
        "prompt": "@.wt/tasks/${task}.md 请完成这个任务"
      }
    ],

    // wt review: 进入审核阶段
    // 状态: * → Active (执行中) → Idle (完成后), 阶段: → reviewing
    "review": [
      // 自动检查
      { "type": "script", "run": "npm run lint && npm run test" },

      // 关闭开发窗口
      { "type": "internal", "run": "window:close" },

      // 启动审核 agent（可选）
      {
        "type": "agent",
        "interactive": false,
        "model": "sonnet",
        "prompt": "Review the code changes for task ${task}. Check against @.wt/tasks/${task}.md"
      }
    ],

    // wt resume: 恢复开发
    // 状态: Idle → Active, 阶段: 保持不变
    "resume": [
      { "type": "internal", "run": "window:create" },
      {
        "type": "agent",
        "interactive": true,
        "model": "sonnet",
        "prompt": "Continue working on @.wt/tasks/${task}.md"
      }
    ],

    // wt complete: 完成任务
    // 状态: * → Completed, 阶段: reviewing → merging → (done)
    "complete": [
      // 最终验证
      { "type": "script", "run": "npm run build && npm run test" },

      // 合并（可能有冲突）
      {
        "type": "internal",
        "run": "branch:merge",
        "on_conflict": {
          "type": "agent",
          "interactive": false,
          "model": "sonnet",
          "prompt": "Resolve merge conflicts for ${task}. Context: @.wt/tasks/${task}.md"
        }
      },

      // 合并后验证（可选）
      {
        "type": "agent",
        "interactive": false,
        "model": "haiku",
        "prompt": "Verify the merge was successful. Run tests and check for regressions."
      },

      // 清理资源
      { "type": "internal", "run": "worktree:destroy" },
      { "type": "internal", "run": "branch:delete" }
    ],

    // wt delete: 删除任务
    "delete": [
      { "type": "script", "run": "rm -rf node_modules/ .next/" },
      { "type": "internal", "run": "worktree:destroy" },
      { "type": "internal", "run": "branch:delete" }
    ],

    // wt reset: 重置任务
    "reset": [
      { "type": "internal", "run": "files:backup" },
      { "type": "script", "run": "rm -rf node_modules/ .next/" },
      { "type": "internal", "run": "worktree:destroy" },
      { "type": "internal", "run": "branch:delete" }
    ]
  }
}
```

## Step 类型

### 1. script

执行 shell 脚本：

```jsonc
{
  "type": "script",
  "run": "npm run lint",
  "on_error": { ... }  // 可选：失败时执行的步骤
}
```

### 2. agent

运行 Claude agent：

```jsonc
{
  "type": "agent",
  "interactive": false,           // false = -p 模式, true = REPL 模式
  "model": "haiku",               // haiku | sonnet | opus | 完整模型名
  "prompt": "...",                // 内联 prompt 或 @file 引用
  "tools": ["Read", "Edit"],      // 可用工具列表
  "allowed_tools": ["Bash(npm *)"], // 自动批准的工具
  "skip_permissions": false,      // 是否跳过权限提示
  "output_format": "text",        // text | json | stream-json
  "window": "new"                 // 交互模式: main | new
}
```

### 3. internal

调用 wt 原子操作：

```jsonc
{
  "type": "internal",
  "run": "worktree:create",
  "on_conflict": { ... }  // 特定操作的错误处理
}
```

可用操作：

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

条件判断：

```jsonc
{
  "type": "condition",
  "if": "wt internal git:has-changes ${worktree}",
  "then": { ... },
  "else": { ... }
}
```

## Pipeline 模式

多个 agent 通过 stream-json 自动串联：

```jsonc
{
  "hooks": {
    "review": {
      "pipeline": [
        {
          "type": "agent",
          "model": "haiku",
          "prompt": "List all changed files and summarize changes"
        },
        {
          "type": "agent",
          "model": "sonnet",
          "prompt": "Based on the above, perform detailed code review"
        },
        {
          "type": "agent",
          "model": "haiku",
          "prompt": "Summarize findings and suggest improvements"
        }
      ]
    }
  }
}
```

wt 自动转换为：

```bash
claude -p --output-format stream-json "prompt1" | \
claude -p --input-format stream-json --output-format stream-json "prompt2" | \
claude -p --input-format stream-json "prompt3"
```

## 变量

所有 step 中可使用：

| 变量 | 说明 |
|------|------|
| `${task}` | 任务名 |
| `${branch}` | 分支名 |
| `${worktree}` | worktree 路径 |
| `${session}` | multiplexer session |
| `${window}` | multiplexer window |
| `${repo_root}` | 仓库根目录 |
| `${phase}` | 当前阶段 |

## 新增 CLI 命令

```bash
# 手动触发 hook（调试用）
wt hooks run <hook_name> [--task <task>]

# 暂停任务（Active → Idle）
wt pause <task> [--reason <reason>]

# 查看任务状态和阶段
wt status --verbose  # 显示 status + phase + idle_reason
```

## 错误处理

1. **默认行为**：step 失败 → 任务进入 Idle 状态，`idle_reason: "error"`
2. **on_error**：定义备选步骤
3. **on_conflict**：定义冲突处理策略

## 日志

agent 输出保存到：

```
.wt/logs/
├── auth/
│   ├── run.log
│   ├── review.log
│   └── complete.log
└── ...
```

## 与现有系统对比

| 旧配置 | 新配置 |
|--------|--------|
| `copy_files: [".env"]` | `hooks.run` 中的 script step |
| `start_args: "..."` | `hooks.run` 中的 agent step |
| `on_create: "npm install"` | `hooks.run` 中的 script step |
| `before_review: "..."` | `hooks.review` 的第一个 step |

| 旧状态 | 新状态 |
|--------|--------|
| Pending | Pending（保留，表示未创建资源） |
| Running | Active + phase: developing |
| Review | Active/Idle + phase: reviewing |
| Completed | Completed |

### 状态语义变化

| 维度 | 旧设计 | 新设计 |
|------|--------|--------|
| Pending | 等待执行 | 未创建资源（依赖未完成等） |
| Running/Active | agent 在跑 | **任何进程**在跑（agent/script/pipeline） |
| Review/Idle | 等待人工 | 资源存在但**无进程**运行 |
| 状态含义 | 混合资源+业务 | 严格反映进程状态 |
| 业务进度 | 隐含在状态中 | 独立的 phase 字段 |

## 验收标准

1. **配置解析**
   - [ ] 解析 `.wt/config.jsonc`
   - [ ] 支持 JSONC 注释
   - [ ] 配置验证

2. **状态模型**
   - [ ] Pending / Active / Idle / Completed 四状态
   - [ ] phase 字段记录阶段（developing/reviewing/merging）
   - [ ] idle_reason 记录空闲原因
   - [ ] active_since 记录进入 Active 的时间
   - [ ] 状态严格反映进程是否在运行
   - [ ] delete 后从 status.json 移除记录
   - [ ] 提供默认状态转换，用户可通过 hooks 覆盖

3. **Step 执行**
   - [ ] script: 执行 shell
   - [ ] agent: 交互/非交互模式
   - [ ] internal: 调用原子操作
   - [ ] condition: 条件判断

4. **Pipeline**
   - [ ] 多 agent stream-json 串联
   - [ ] 错误传播

5. **CLI 命令**
   - [ ] `wt hooks run <hook>`
   - [ ] `wt pause <task>`
   - [ ] 状态显示更新

## 实施计划

### 策略：模块级 Greenfield

保留复用的模块：
- `services/git.rs` - Git 原子操作
- `services/multiplexer/` - tmux/zellij 后端
- `services/command.rs` - 命令执行辅助
- `commands/internal/` - 内部原子操作 CLI

彻底重写的模块：
- `models/config.rs` → JSONC 解析
- `models/status.rs` → 新状态模型 (Pending/Active/Idle/Completed + phase)
- `services/hooks.rs` → 新 hooks 引擎 (steps/pipeline/condition)
- `commands/*.rs` → 精简为 hook 触发器

### Phase 1: 基础设施

1. **JSONC 配置解析** (`models/config_v2.rs`)
   - [ ] 添加 `jsonc-parser` 或类似 crate
   - [ ] 定义新的配置结构体
   - [ ] 解析 `.wt/config.jsonc`
   - [ ] 配置验证和错误提示

2. **新状态模型** (`models/status_v2.rs`)
   - [ ] TaskStatus: Pending/Active/Idle/Completed
   - [ ] TaskPhase: developing/reviewing/merging
   - [ ] idle_reason 枚举
   - [ ] active_since 时间戳
   - [ ] 状态文件读写

### Phase 2: Hooks 引擎

3. **Step 执行器** (`services/hooks/step.rs`)
   - [ ] script step: 执行 shell
   - [ ] agent step: 构建 claude 命令
   - [ ] internal step: 调用 wt internal
   - [ ] condition step: 条件判断

4. **Pipeline 执行器** (`services/hooks/pipeline.rs`)
   - [ ] 构建 stream-json 管道命令
   - [ ] 进程管理和错误传播

5. **Hooks 引擎** (`services/hooks/engine.rs`)
   - [ ] 加载 hook 配置
   - [ ] 变量展开 (${task}, ${branch}, etc.)
   - [ ] 执行 steps 或 pipeline
   - [ ] 状态自动管理（默认行为）

### Phase 3: 命令重写

6. **命令层重写**
   - [ ] `run.rs` - 触发 hooks.run
   - [ ] `review.rs` - 触发 hooks.review
   - [ ] `resume.rs` - 触发 hooks.resume
   - [ ] `complete.rs` - 触发 hooks.complete
   - [ ] `delete.rs` - 触发 hooks.delete + 移除状态
   - [ ] `reset.rs` - 触发 hooks.reset

7. **新增命令**
   - [ ] `wt hooks run <hook>` - 手动触发 hook
   - [ ] `wt pause <task>` - 暂停任务

### Phase 4: 清理

8. **删除旧代码**
   - [ ] 删除 `models/config.rs` (旧 YAML 解析)
   - [ ] 删除 `models/status.rs` (旧状态模型)
   - [ ] 删除 `services/hooks.rs` (旧 hooks)
   - [ ] 删除 `models/hook_context.rs` (旧变量展开)
   - [ ] 更新 `mod.rs` 导出

9. **文档更新**
   - [ ] 更新 README.md
   - [ ] 更新 .claude/CLAUDE.md
   - [ ] 删除旧的 docs/claude-code-hooks.md

---

## 关键资源和上下文

### 为什么选择 JSONC？

访谈中讨论了配置格式：
- YAML 的问题：复杂结构表达不够精确
- JSONC 优点：支持注释、IDE 支持好、结构清晰
- 决策：使用 `.wt/config.jsonc`

### Pipeline 机制是如何发现的？

**来源**：通过 WebSearch 搜索 "Claude Code CLI print mode piping stdin stream-json"

**关键发现**（来自 [ruvnet/claude-flow Wiki](https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining)）：

Claude Code 支持 stream-json chaining：
```bash
claude -p --output-format stream-json "prompt1" | \
claude -p --input-format stream-json --output-format stream-json "prompt2" | \
claude -p --input-format stream-json "prompt3"
```

两个关键 flags：
- `--output-format stream-json` - 输出 NDJSON 格式
- `--input-format stream-json` - 接收 NDJSON 输入

这使得多个 agent 可以通过管道串联，前一个的输出自动成为后一个的上下文。

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

理解用户工作流对设计很重要：
1. 写任务文档 (human)
2. Agent 执行开发 (agent)
3. 另一个 Agent 核对任务 (agent) ← 多 agent 验证
4. Agent 处理合并冲突 (agent) ← 通过管道补充上下文
5. 可选：Agent 做合并后审核 (agent)
6. 完成 → 解锁下一层并行任务

关键特点：**多 agent 协作**，不是单 agent 执行。

### 现有代码中可复用的部分

```
src/
├── services/
│   ├── git.rs              ✅ 复用 - Git 原子操作已完善
│   ├── multiplexer/        ✅ 复用 - tmux/zellij 后端
│   │   ├── mod.rs
│   │   ├── tmux.rs
│   │   └── zellij.rs
│   └── command.rs          ✅ 复用 - 命令执行辅助
├── commands/
│   └── internal/           ✅ 复用 - 原子操作 CLI
│       ├── mod.rs
│       ├── git.rs
│       ├── mux.rs
│       └── misc.rs
└── error.rs                ✅ 复用 - 错误类型
```

### 推荐的 Rust crates

| 功能 | 推荐 crate | 说明 |
|------|------------|------|
| JSONC 解析 | `json_comments` 或 `jsonc-parser` | 支持注释的 JSON |
| 时间处理 | `chrono` | active_since 时间戳 |
| 进程管道 | `std::process` | Pipeline 实现 |

---

## 参考资料

### Claude Code 官方文档
- [CLI Reference](https://code.claude.com/docs/en/cli-reference) - 完整 CLI 参数
- [SDK Documentation](https://docs.claude.com/en/docs/agent-sdk) - 程序化使用

### Stream-JSON Chaining
- [ruvnet/claude-flow Wiki](https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining) - Pipeline 机制详解
- 关键 flags: `--output-format stream-json`, `--input-format stream-json`

### 本次访谈记录
- 设计决策通过 `/interview` skill 进行
- 使用 `fuckme` 工具与 Codex 进行两轮辩论验证设计
