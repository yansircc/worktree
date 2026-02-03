# 技术架构

## 概念层级

```
Project (项目)
  │
  └─ Tasks[] (任务集合)
       │
       └─ Task (任务)
            │
            └─ Phases[] (阶段序列)
                 │
                 └─ Phase (阶段)
                      │
                      ├─ on_enter: Workflow
                      └─ on_exit: Workflow
                           │
                           └─ Workflow (工作流)
                                │
                                └─ Steps[] (步骤列表)
                                     │
                                     └─ Step (步骤)
                                          ├─ execute   # 执行器
                                          ├─ input     # 输入
                                          ├─ output    # 输出
                                          ├─ observe   # 观测器
                                          └─ verify    # 验证器
```

---

## Step（步骤）

Step 是最小的执行单元，包含三个正交维度：执行、观测、验证。

### Step 完整定义

```jsonc
{
  // ========== 标识 ==========
  "id": "develop",
  "name": "开发功能",

  // ========== 执行器 (Execute) ==========
  // 二选一：run 或 agent
  "run": "npm test",
  // 或
  "agent": {
    "prompt": "@.wt/tasks/${task}.md 请完成这个任务",
    "model": "sonnet",
    "max_turns": 50,
    "tools": ["Read", "Edit", "Bash"],
    "settings": {
      "file": ".wt/agent-settings/develop.json",
      "hooks": {
        "stop": {
          "prompt": "评估任务完成情况，执行 wt step done/block/fail",
          "model": "haiku"
        }
      },
      "skills": [".wt/skills/code-review.md"]
    }
  },

  // ========== 输入 (Input) ==========
  "input": {
    "env": {
      "DEBUG": "true",
      "API_KEY": "${secrets.api_key}"
    },
    "files": ["@.wt/tasks/${task}.md"],
    "context": ["${steps.analyze.output}"],
    "stdin": "${prev.stdout}"
  },

  // ========== 输出 (Output) ==========
  "output": {
    "artifacts": ["dist/**", "coverage/"],
    "export": {
      "version": "grep version package.json",
      "changed_files": "git diff --name-only"
    }
  },

  // ========== 观测器 (Observe) ==========
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

  // ========== 验证器 (Verify) ==========
  "verify": {
    // 验证方式：self | script | agent | human | schema
    "type": "self"               // agent 通过 wt step done/block/fail 自标记
    // 或
    "type": "script",
    "run": "npm test",
    "on_pass": "success",
    "on_fail": "failed"
    // 或
    "type": "agent",
    "agent": {
      "prompt": "检查是否完成，回答 PASS 或 FAIL",
      "model": "haiku"
    }
    // 或
    "type": "human",
    "prompt": "请检查代码是否正确",
    "timeout": "24h"
  },

  // ========== 控制 ==========
  "condition": "${prev.state} == 'success'",
  "timeout": "30m",
  "retry": {
    "max_attempts": 2,
    "delay": "30s"
  }
}
```

### StepState

```rust
pub enum StepState {
    Pending,    // 等待执行
    Running,    // 正在执行
    Success,    // 执行成功
    Failed,     // 执行失败
    Blocked,    // 需要介入（Agent 标记）
    Timeout,    // 超时
    Skipped,    // 跳过（condition 不满足）
}
```

**状态转换：**

```
Script Step:
  pending → running（进程启动）
  running → success（verify pass 或 exit 0）
  running → failed（verify fail 或 exit != 0）
  running → timeout（超时）

Agent Step:
  pending → running（agent 开始）
  running → success（wt step done 或 verify pass）
  running → blocked（wt step block）
  running → failed（wt step fail 或 verify fail）
  running → timeout（超时）
```

### StepResult

```rust
pub struct StepResult {
    pub step_id: Option<String>,
    pub state: StepState,
    pub exit_code: Option<i32>,
    pub message: Option<String>,    // blocked/failed 时的原因
    pub output_file: PathBuf,
    pub artifacts: Vec<PathBuf>,
    pub exports: HashMap<String, String>,
    pub duration: Duration,
}
```

---

## Workflow（工作流）

Workflow 是 Steps 的编排，定义如何协调多个 Step。

### Workflow 完整定义

```jsonc
{
  // ========== 标识 ==========
  "id": "develop-workflow",
  "name": "开发工作流",

  // ========== Steps ==========
  "steps": [
    { "id": "setup", "run": "npm install", ... },
    { "id": "develop", "agent": {...}, ... },
    { "id": "test", "run": "npm test", ... }
  ],

  // ========== 执行策略 ==========
  "execution": {
    "mode": "sequential",        // sequential | parallel | dag
    "on_step_failure": "abort",  // abort | continue | skip_dependents
    "on_step_blocked": "pause",  // pause | abort | continue
    "retry": {
      "max_attempts": 2,
      "delay": "30s"
    }
  },

  // ========== 上下文 ==========
  "context": {
    "shared": {
      "project_type": "rust",
      "test_command": "cargo test"
    },
    "passing": "accumulate"      // accumulate | last_only | explicit
  },

  // ========== 观测 ==========
  "observe": {
    "progress": true,
    "summary": true
  }
}
```

### 执行模式

**Sequential（顺序）：**
```
a → b → c
```

**Parallel（并行）：**
```
┌─ lint  ─┐
├─ test  ─┼─▶ 全部完成
└─ build ─┘
```

**DAG（依赖图）：**
```jsonc
{
  "execution": { "mode": "dag" },
  "steps": [
    { "id": "install", ... },
    { "id": "lint", "depends": ["install"], ... },
    { "id": "test", "depends": ["install"], ... },
    { "id": "build", "depends": ["lint", "test"], ... }
  ]
}
```

```
         ┌─ lint ──┐
install ─┤         ├─▶ build
         └─ test ──┘
```

### WorkflowState（派生）

```rust
pub enum WorkflowState {
    Pending,    // 未开始
    Running,    // 有 step 在执行
    Success,    // 所有 steps 成功
    Failed,     // 有 step 失败
    Blocked,    // 有 step 阻塞
}

impl WorkflowState {
    pub fn derive(steps: &[StepResult]) -> Self {
        if steps.iter().any(|s| s.state == StepState::Running) {
            WorkflowState::Running
        } else if steps.iter().any(|s| s.state == StepState::Failed) {
            WorkflowState::Failed
        } else if steps.iter().any(|s| s.state == StepState::Blocked) {
            WorkflowState::Blocked
        } else if steps.iter().all(|s| matches!(s.state, StepState::Success | StepState::Skipped)) {
            WorkflowState::Success
        } else {
            WorkflowState::Pending
        }
    }
}
```

---

## Phase（阶段）

Phase 是任务生命周期的一个阶段，有明确目标和资源要求。

### Phase 完整定义

```jsonc
{
  // ========== 标识 ==========
  "id": "developing",
  "name": "开发阶段",
  "goal": "完成任务描述的功能",

  // ========== 资源 ==========
  "resources": "full",           // none | full

  // ========== 前置条件 ==========
  "prerequisites": {
    "dependencies": "completed", // completed | any
    "phase": ["pending"],        // 允许从哪些阶段进入
    "condition": "${task.priority} != 'blocked'"
  },

  // ========== Timing ==========
  "on_enter": {
    // Workflow 定义
    "steps": [...],
    "execution": { "mode": "sequential" }
  },

  "on_exit": {
    "steps": [
      {
        "condition": "${phase.exit_reason} == 'success'",
        "agent": { "prompt": "生成 commit message" }
      },
      {
        "condition": "${phase.exit_reason} == 'forced'",
        "run": "echo 'Skipped' >> .wt/logs/${task}/skipped.log"
      }
    ]
  },

  // ========== 观测 ==========
  "observe": {
    "progress": true,
    "notifications": {
      "on_blocked": "slack",
      "on_success": "none"
    }
  },

  // ========== 超时 ==========
  "timeout": {
    "duration": "4h",
    "action": "block"            // block | fail | notify
  }
}
```

### PhaseState（派生）

```rust
pub enum PhaseState {
    Pending,    // 未开始
    Running,    // workflow 执行中
    Success,    // workflow 成功
    Failed,     // workflow 失败
    Blocked,    // workflow 阻塞
}
```

### Timing 和 exit_reason

on_exit 可以根据 exit_reason 执行不同逻辑：

| exit_reason | 触发场景 |
|-------------|----------|
| `success` | workflow 正常完成 |
| `forced` | human 执行 wt next 强制推进 |
| `failed` | workflow 失败后强制推进 |

---

## Task（任务）

Task 是可独立完成的工作单元，对应一个 git 分支和 worktree。

### Task 完整定义

```jsonc
{
  // ========== 标识 ==========
  "id": "auth",
  "name": "用户认证模块",
  "description": "实现用户登录、注册、权限验证功能",

  // ========== 依赖 ==========
  "dependencies": ["database", "config"],

  // ========== Phase 配置 ==========
  "phases": {
    "sequence": ["pending", "developing", "testing", "reviewing", "completed"],
    "overrides": {
      "developing": {
        "on_enter": [
          { "run": "cargo build" },
          { "agent": { "prompt": "@.wt/tasks/${task}.md" } }
        ]
      }
    }
  },

  // ========== 资源实例 ==========
  "instance": {
    "branch": "wt/auth-${random}",
    "worktree": ".wt/worktrees/auth",
    "window": "auth"
  },

  // ========== 观测 ==========
  "observe": {
    "notifications": {
      "on_completed": "slack",
      "on_blocked": "email"
    }
  },

  // ========== 元数据 ==========
  "metadata": {
    "priority": "high",
    "assignee": "agent-1",
    "tags": ["backend", "security"]
  }
}
```

### 任务文件格式（.wt/tasks/auth.md）

```markdown
---
name: 用户认证模块
dependencies:
  - database
  - config
phases:
  sequence: [pending, developing, testing, reviewing, completed]
metadata:
  priority: high
  tags: [backend, security]
---

## 任务描述

实现用户认证模块...

## 验收标准

- [ ] 注册接口 POST /api/register
- [ ] 登录接口 POST /api/login
```

### TaskStatus（派生）

```rust
pub enum TaskStatus {
    Pending,    // 未开始（无资源）
    Active,     // 执行中（有进程）
    Idle,       // 暂停/等待（无进程，有资源）
    Completed,  // 已完成
}

impl TaskStatus {
    pub fn derive(phase: &Option<PhaseId>, phase_state: &PhaseState) -> Self {
        match (phase, phase_state) {
            (None, _) => TaskStatus::Pending,
            (Some(p), _) if p == "completed" => TaskStatus::Completed,
            (_, PhaseState::Running) => TaskStatus::Active,
            (_, PhaseState::Success) => TaskStatus::Active,  // 即将自动推进
            (_, PhaseState::Blocked | PhaseState::Failed) => TaskStatus::Idle,
            _ => TaskStatus::Idle,
        }
    }
}
```

---

## Project（项目）

Project 是 wt 管理的顶层概念，包含所有任务和全局配置。

### Project 完整定义

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
        "resources": "full",
        "on_enter": [...]
      },
      "reviewing": {
        "resources": "full",
        "on_enter": [...]
      },
      "completed": { "resources": "none" }
    }
  },

  // ========== Workflow 片段库 ==========
  "workflows": {
    "lint-and-test": [...],
    "deploy-staging": [...]
  },

  // ========== 并行策略 ==========
  "concurrency": {
    "max_active_tasks": 5,
    "max_agents": 3,
    "resource_limits": {
      "cpu": "80%",
      "memory": "8GB"
    }
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

### ProjectStatus（派生）

```rust
pub struct ProjectStatus {
    pub total: usize,
    pub pending: usize,
    pub active: usize,
    pub idle: usize,
    pub completed: usize,
    pub progress: f32,  // completed / total
}
```

---

## 状态派生链

```
StepState (每个 step)
    │
    ▼ 聚合
WorkflowState (当前 workflow)
    │
    ▼ 映射
PhaseState (当前 phase)
    │
    ▼ + resources
TaskStatus (任务状态)
    │
    ▼ 聚合
ProjectStatus (项目状态)
```

### 派生规则

**StepState → WorkflowState：**
- 有 step running → running
- 任一 step failed → failed
- 任一 step blocked → blocked
- 所有 step success/skipped → success

**WorkflowState → PhaseState：**
- 直接映射

**PhaseState → TaskStatus：**
- phase == null → pending
- phase == completed → completed
- phase running/success → active
- phase blocked/failed → idle

**TaskStatus → ProjectStatus：**
- 统计各状态数量

---

## 目录结构

```
project-root/
├── .wt/                          # Project 根目录
│   ├── config.jsonc              # Project 配置
│   ├── state.json                # 运行时状态
│   ├── tasks/                    # Task 定义
│   │   ├── auth.md
│   │   ├── database.md
│   │   └── ui.md
│   ├── worktrees/                # Git worktrees
│   │   ├── auth/
│   │   ├── database/
│   │   └── ui/
│   ├── logs/                     # 执行日志
│   │   └── {task}/{phase}/
│   │       ├── step-1.log
│   │       ├── step-2.log
│   │       └── context.json
│   ├── agent-settings/           # Agent 配置
│   │   ├── develop.json
│   │   └── review.json
│   ├── skills/                   # 自定义 skills
│   │   └── code-review.md
│   └── backups/                  # 重置备份
│       └── {task}-{timestamp}/
├── src/                          # 项目源码
└── ...
```

---

## 配置合并规则

```
内置默认 (wt binary)
    ↓ 合并
Project 配置 (.wt/config.jsonc)
    ↓ 合并
Task 配置 (.wt/tasks/xxx.md frontmatter)
    ↓
最终配置
```

---

## 模块结构

```
src/
├── models/
│   ├── mod.rs
│   ├── project.rs        # Project
│   ├── task.rs           # Task
│   ├── phase.rs          # Phase
│   ├── workflow.rs       # Workflow
│   ├── step.rs           # Step
│   ├── state.rs          # 各级 State 和派生逻辑
│   ├── config.rs         # 配置解析和合并
│   └── store.rs          # 存储
│
├── services/
│   ├── mod.rs
│   ├── git.rs            # Git 操作
│   ├── multiplexer/      # tmux/zellij
│   ├── executor/         # 执行引擎
│   │   ├── mod.rs
│   │   ├── step.rs       # StepExecutor
│   │   ├── workflow.rs   # WorkflowExecutor
│   │   └── phase.rs      # PhaseTransition
│   └── observer/         # 观测
│       ├── mod.rs
│       ├── terminal.rs   # 终端观测
│       └── log.rs        # 日志记录
│
├── commands/
│   ├── mod.rs
│   ├── step.rs           # wt step done/block/fail
│   ├── next.rs           # wt next
│   ├── prev.rs           # wt prev
│   ├── stop.rs           # wt stop
│   ├── reset.rs
│   ├── delete.rs
│   ├── list.rs
│   └── status/
│
└── tui/                  # TUI 仪表盘
```
