# API 参考

## CLI 命令

### Step 状态命令（Agent 使用）

这些命令由 Agent 在执行过程中调用，标记当前 step 的状态。

#### wt step done

标记当前 step 成功完成。

```bash
wt step done
```

**行为：**
1. 当前 step 状态 → success
2. 如果所有 steps 都 success，workflow 状态 → success
3. 如果 workflow success，phase 状态 → success
4. 如果 phase success，自动触发阶段转换

---

#### wt step block

标记当前 step 阻塞，需要 human 介入。

```bash
wt step block "需要确认 API 设计"
wt step block  # 无消息
```

**行为：**
1. 当前 step 状态 → blocked
2. Workflow 状态 → blocked
3. Phase 状态 → blocked
4. Task 状态 → idle
5. 等待 human 介入

---

#### wt step fail

标记当前 step 失败。

```bash
wt step fail "无法完成：测试始终失败"
wt step fail  # 无消息
```

**行为：**
1. 当前 step 状态 → failed
2. Workflow 状态 → failed
3. Phase 状态 → failed
4. Task 状态 → idle
5. 等待 human 介入

---

### Phase 转换命令（Human 使用）

这些命令由 Human 强制执行阶段转换。

#### wt next

强制推进到下一阶段。

```bash
wt next <task>
wt next <index>   # 支持索引
```

**行为：**
1. 停止当前进程（如果有）
2. 设置 exit_reason = forced 或 success
3. 执行 current_phase.on_exit
4. 资源转换
5. 更新 phase
6. 执行 next_phase.on_enter

---

#### wt prev

强制回退到上一阶段。

```bash
wt prev <task>
wt prev <index>
```

**行为：**
1. 停止当前进程（如果有）
2. 资源转换
3. 更新 phase
4. 不执行 on_enter（回退不触发 workflow）

---

#### wt stop

强制停止当前进程。

```bash
wt stop <task>
wt stop <index>
```

**行为：**
1. 停止 tmux/zellij 窗口中的进程
2. 当前 step 状态保持 running（等待恢复或强制推进）
3. Task 状态 → idle

---

#### wt reset

重置任务到 pending。

```bash
wt reset <task>
wt reset <index>
```

**行为：**
1. 备份代码到 .wt/backups/{task}-{timestamp}/
2. 销毁所有资源（worktree, window）
3. 设置 phase = null
4. 保留 branch（用户可手动删除）

---

#### wt delete

彻底删除任务。

```bash
wt delete <task>
wt delete <index>
```

---

### 查询命令

#### wt list

```bash
wt list
wt list --json
```

#### wt status

```bash
wt status <task>
wt status --json
```

---

## Step 配置

### 完整 Step 配置

```jsonc
{
  // ========== 标识 ==========
  "id": "develop",
  "name": "开发功能",

  // ========== 执行器 ==========
  // Script
  "run": "npm test",
  // 或 Agent
  "agent": {
    "prompt": "完成任务",
    "model": "sonnet",
    "max_turns": 50,
    "tools": ["Read", "Edit", "Bash"],
    "allowed_tools": ["Read"],
    "settings": {
      "file": ".wt/agent-settings/develop.json",
      "hooks": {
        "stop": {
          "prompt": "评估并标记状态",
          "model": "haiku"
        }
      },
      "skills": [".wt/skills/code-review.md"]
    }
  },

  // ========== 输入 ==========
  "input": {
    "env": { "DEBUG": "true" },
    "files": ["@.wt/tasks/${task}.md"],
    "context": ["${steps.prev.output}"],
    "stdin": "${prev.stdout}"
  },

  // ========== 输出 ==========
  "output": {
    "artifacts": ["dist/**"],
    "export": { "version": "grep version package.json" }
  },

  // ========== 观测器 ==========
  "observe": {
    "mode": "interactive",       // interactive | background
    "output": "both",            // terminal | file | both
    "multiplexer": {
      "window": "${task}",
      "focus": true
    },
    "log": {
      "path": ".wt/logs/${task}/${phase}/step-${step_index}.log",
      "stream": true
    }
  },

  // ========== 验证器 ==========
  "verify": {
    "type": "self"               // self | script | agent | human | schema
  },

  // ========== 控制 ==========
  "condition": "${prev.state} == 'success'",
  "timeout": "30m",
  "retry": { "max_attempts": 2, "delay": "30s" }
}
```

### 验证器类型

#### self（默认）

Agent 通过 `wt step done/block/fail` 自标记。

```jsonc
{ "verify": { "type": "self" } }
```

#### script

脚本验证，exit 0 = pass。

```jsonc
{
  "verify": {
    "type": "script",
    "run": "npm test",
    "on_pass": "success",
    "on_fail": "failed"
  }
}
```

#### agent

另一个 Agent 验证。

```jsonc
{
  "verify": {
    "type": "agent",
    "agent": {
      "prompt": "检查是否完成，回答 PASS 或 FAIL",
      "model": "haiku"
    }
  }
}
```

#### human

人工验证。

```jsonc
{
  "verify": {
    "type": "human",
    "prompt": "请检查代码是否正确",
    "timeout": "24h"
  }
}
```

#### schema

Schema 验证（JSON 输出）。

```jsonc
{
  "verify": {
    "type": "schema",
    "schema": { "type": "object", "required": ["name"] }
  }
}
```

---

## Workflow 配置

```jsonc
{
  "id": "develop-workflow",
  "name": "开发工作流",

  "steps": [
    { "id": "setup", "run": "npm install" },
    { "id": "develop", "agent": {...} },
    { "id": "test", "run": "npm test" }
  ],

  "execution": {
    "mode": "sequential",        // sequential | parallel | dag
    "on_step_failure": "abort",  // abort | continue | skip_dependents
    "on_step_blocked": "pause",  // pause | abort | continue
    "retry": { "max_attempts": 2, "delay": "30s" }
  },

  "context": {
    "shared": { "project_type": "rust" },
    "passing": "accumulate"      // accumulate | last_only | explicit
  },

  "observe": {
    "progress": true,
    "summary": true
  }
}
```

### DAG 模式

```jsonc
{
  "execution": { "mode": "dag" },
  "steps": [
    { "id": "install" },
    { "id": "lint", "depends": ["install"] },
    { "id": "test", "depends": ["install"] },
    { "id": "build", "depends": ["lint", "test"] }
  ]
}
```

---

## Phase 配置

```jsonc
{
  "id": "developing",
  "name": "开发阶段",
  "goal": "完成任务描述的功能",

  "resources": "full",           // none | full

  "prerequisites": {
    "dependencies": "completed", // completed | any
    "phase": ["pending"],
    "condition": "${task.priority} != 'blocked'"
  },

  "on_enter": {
    "steps": [...],
    "execution": { "mode": "sequential" }
  },

  "on_exit": {
    "steps": [
      {
        "condition": "${phase.exit_reason} == 'success'",
        "run": "git add -A && git commit -m 'feat: ${task}'"
      }
    ]
  },

  "observe": {
    "progress": true,
    "notifications": { "on_blocked": "slack" }
  },

  "timeout": { "duration": "4h", "action": "block" }
}
```

---

## Task 配置

### 任务文件（.wt/tasks/auth.md）

```markdown
---
name: 用户认证模块
dependencies:
  - database
  - config
phases:
  sequence: [pending, developing, testing, reviewing, completed]
  overrides:
    developing:
      on_enter:
        - run: cargo build
        - agent:
            prompt: "@.wt/tasks/${task}.md"
observe:
  notifications:
    on_completed: slack
metadata:
  priority: high
  tags: [backend, security]
---

## 任务描述

实现用户认证模块...

## 验收标准

- [ ] 注册接口
- [ ] 登录接口
```

---

## Project 配置

### .wt/config.jsonc

```jsonc
{
  // ========== 标识 ==========
  "name": "my-awesome-app",
  "description": "一个很棒的应用",

  // ========== 资源 ==========
  "resources": {
    "multiplexer": "tmux",
    "session": "my-awesome-app",
    "worktree_dir": ".wt/worktrees"
  },

  // ========== Phase 默认配置 ==========
  "phases": {
    "sequence": ["pending", "developing", "reviewing", "completed"],
    "definitions": {
      "pending": { "resources": "none" },
      "developing": {
        "goal": "完成任务描述的功能",
        "resources": "full",
        "prerequisites": { "dependencies": "completed" },
        "on_enter": [
          {
            "agent": {
              "prompt": "@.wt/tasks/${task}.md 请完成这个任务",
              "model": "sonnet",
              "settings": {
                "hooks": {
                  "stop": {
                    "prompt": "评估任务，执行 wt step done/block/fail",
                    "model": "haiku"
                  }
                }
              }
            }
          }
        ]
      },
      "reviewing": {
        "goal": "确保代码质量",
        "resources": "full",
        "on_enter": [
          {
            "agent": {
              "prompt": "审查代码质量和安全性",
              "model": "sonnet"
            }
          }
        ]
      },
      "completed": {
        "resources": "none",
        "on_enter": [
          { "run": "git checkout main" },
          { "run": "git merge --squash ${branch}" },
          { "run": "git commit -m \"feat(${task}): ${task_name}\"" }
        ]
      }
    }
  },

  // ========== Workflow 片段库 ==========
  "workflows": {
    "lint-and-test": [
      { "run": "npm run lint" },
      { "run": "npm test" }
    ]
  },

  // ========== 并行策略 ==========
  "concurrency": {
    "max_active_tasks": 5,
    "max_agents": 3
  },

  // ========== 观测 ==========
  "observe": {
    "dashboard": true,
    "notifications": {
      "backend": "slack",
      "on_all_completed": true,
      "on_any_blocked": true
    }
  }
}
```

---

## 变量

### Step 可用变量

| 变量 | 说明 |
|------|------|
| `${task}` | 任务 ID |
| `${task_name}` | 任务名称 |
| `${branch}` | 分支名 |
| `${worktree}` | worktree 路径 |
| `${repo_root}` | 仓库根目录 |
| `${session}` | multiplexer session |
| `${window}` | multiplexer window |
| `${phase}` | 当前阶段 |
| `${step_index}` | 当前 step 索引 |
| `${prev.state}` | 上一步状态 |
| `${prev.exit_code}` | 上一步退出码 |
| `${prev.output_file}` | 上一步日志 |
| `${steps.<id>.output}` | 指定 step 的输出 |
| `${phase.exit_reason}` | 阶段退出原因 |

### 环境变量

| 环境变量 | 说明 |
|----------|------|
| `WT_TASK` | 任务 ID |
| `WT_BRANCH` | 分支名 |
| `WT_WORKTREE` | worktree 路径 |
| `WT_REPO_ROOT` | 仓库根目录 |
| `WT_SESSION` | multiplexer session |
| `WT_WINDOW` | multiplexer window |
| `WT_PHASE` | 当前阶段 |

---

## 日志结构

```
.wt/logs/{task}/{phase}/
├── step-1.log        # 第一个 step 的 stdout/stderr
├── step-2.log
└── context.json      # 累积的 StepResults
```

### context.json

```json
{
  "task": "auth",
  "phase": "developing",
  "steps": [
    {
      "id": "setup",
      "state": "success",
      "exit_code": 0,
      "output_file": ".wt/logs/auth/developing/step-1.log",
      "duration_ms": 5200
    },
    {
      "id": "develop",
      "state": "blocked",
      "message": "需要确认 API 设计",
      "output_file": ".wt/logs/auth/developing/step-2.log",
      "duration_ms": 120000
    }
  ]
}
```
