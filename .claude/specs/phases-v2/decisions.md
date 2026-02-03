# 设计决策记录 (ADR)

## D0: 完整概念层级

**状态：** 已决定

**背景：**
wt 需要管理多 agent 并行开发，如何组织概念？

**决策：** 5 层概念层级

```
Project → Task → Phase → Workflow → Step
```

**理由：**
1. 每层职责明确，不重叠
2. 状态可以从下往上派生
3. 配置可以从上往下覆盖

---

## D1: 状态从 Step 派生

**状态：** 已决定

**背景：**
如何设计任务状态？谁来决定"阶段目标是否达成"？

**问题分析：**
1. Script 的成功/失败可以用 exit code 判断（机器判断）
2. Agent 的成功/失败需要语义判断（主观）
3. "代码写好了吗"这类问题无法用 bool 判断

**决策：** 状态从 Step 层逐级派生

```
StepState → WorkflowState → PhaseState → TaskStatus → ProjectStatus
```

**Step 状态：**
- pending, running, success, failed, blocked, timeout, skipped

**派生规则：**
- 有 step running → workflow running
- 所有 steps success → workflow success → 自动推进
- 任一 step blocked → workflow blocked → idle
- 任一 step failed → workflow failed → idle

**理由：**
1. 状态粒度在最小单元（step），逻辑清晰
2. Script 和 Agent 统一模型（都有状态）
3. Task 状态是派生的，不会不一致

---

## D2: Step 三维度：执行/观测/验证

**状态：** 已决定

**背景：**
Step 应该包含什么？

**决策：** 三个正交维度

| 维度 | 问题 | 工具 |
|------|------|------|
| Execute | 做什么 | run / agent |
| Observe | 如何观测 | terminal / log |
| Verify | 如何验证 | self / script / agent / human / schema |

**理由：**
1. 三个维度正交，可独立配置
2. tmux/zellij 是观测工具，不是执行工具
3. 验证可以是机器、agent 或 human

---

## D3: Agent 通过 settings.hooks.stop 自验证

**状态：** 已决定

**背景：**
Agent 如何标记自己的执行结果？

**决策：** Agent 可配置 stop hook，在停止时自己评估并标记状态

```jsonc
{
  "agent": {
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
```

**理由：**
1. Agent 有完整上下文，能做语义判断
2. 利用 Claude CLI 的 hooks 机制
3. 不需要外部验证器介入

---

## D4: 命令是强制标记，正常流程自动推进

**状态：** 已决定

**背景：**
用户何时需要手动执行命令？

**决策：**
- 正常流程：workflow success → 自动进入下一阶段
- 命令只是强制标记，用于手动干预场景

**命令：**
- `wt step done/block/fail` - Agent 标记 step 状态
- `wt next/prev/stop` - Human 强制操作

**理由：**
1. 大多数情况下用户不需要手动操作
2. 命令只在例外情况使用（手动修复、跳过失败）

---

## D5: 命令名用 next/prev 而非 push/pull

**状态：** 已决定

**背景：**
阶段转换命令叫什么名字？

**决策：** next/prev/stop

**理由：**
1. push/pull 容易与 git 混淆
2. next/prev 直观、简短、不与 git 冲突

---

## D6: Phase Timing 只有 on_enter/on_exit

**状态：** 已决定

**背景：**
Phase 需要多少 timing？on_error、on_blocked 等需要吗？

**决策：** 只保留 on_enter/on_exit，通过 exit_reason 区分

```jsonc
{
  "on_exit": {
    "steps": [
      {
        "condition": "${phase.exit_reason} == 'success'",
        "run": "..."
      },
      {
        "condition": "${phase.exit_reason} == 'forced'",
        "run": "..."
      }
    ]
  }
}
```

**理由：**
1. 减少 timing 种类，保持简单
2. exit_reason 可以区分不同退出场景
3. 错误处理可以在 workflow 层做

---

## D7: 配置分层覆盖

**状态：** 已决定

**背景：**
如何平衡易用性和可定制性？

**决策：** 三层配置，逐层覆盖

```
内置默认 (wt binary)
    ↓ 合并
Project 配置 (.wt/config.jsonc)
    ↓ 合并
Task 配置 (.wt/tasks/xxx.md frontmatter)
    ↓
最终配置
```

**理由：**
1. 零配置即可使用
2. 需要时可覆盖任何配置
3. Task 可以有自己的特殊配置

---

## D8: 保持 JSONC 格式

**状态：** 已决定

**背景：**
配置文件用 YAML 还是 JSONC？

**决策：** 保持 JSONC

**理由：**
1. 迁移成本为零，已有解析代码
2. 结构更严格，不容易出 indent 错误
3. YAML 的优势在我们场景下用处不大

---

## D9: Workflow 支持 DAG 执行模式

**状态：** 已决定

**背景：**
Steps 之间可能有依赖关系，如何处理？

**决策：** 支持三种执行模式

- sequential: 顺序执行
- parallel: 并行执行
- dag: 依赖图执行

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

**理由：**
1. 覆盖常见编排需求
2. DAG 可以优化执行时间
3. 保持向后兼容（默认 sequential）

---

## D10: Project 支持并行策略

**状态：** 已决定

**背景：**
多任务并行时如何控制资源？

**决策：** Project 配置并行策略

```jsonc
{
  "concurrency": {
    "max_active_tasks": 5,
    "max_agents": 3
  }
}
```

**理由：**
1. 避免资源过载
2. 控制 agent 数量
3. 可按项目配置

---

## D11: Step 显式 input/output

**状态：** 已决定

**背景：**
Steps 之间如何传递数据？

**决策：** 显式定义 input/output

```jsonc
{
  "input": {
    "env": { "DEBUG": "true" },
    "files": ["@.wt/tasks/${task}.md"],
    "context": ["${steps.prev.output}"]
  },
  "output": {
    "artifacts": ["dist/**"],
    "export": { "version": "..." }
  }
}
```

**理由：**
1. 数据流清晰可见
2. 便于调试和理解
3. 支持复杂的数据传递场景

---

## D12: observe 是可观测性配置，不是观察者模式

**状态：** 已决定

**背景：**
规格中的 `observe` 出现在 Step/Workflow/Phase/Project 四个层级，包含输出位置、multiplexer 窗口、通知等配置。这与软件设计中的"观察者模式"容易混淆。

**问题分析：**
1. 规格中的 `observe` 回答的问题是「我想怎样感知执行状态」
2. 它包含：输出位置 (terminal/file)、交互模式、multiplexer 窗口、通知 (slack)、进度显示
3. 从用户视角看，「在哪个窗口看执行」和「日志写到哪」都是「观测」的一部分
4. 但从实现角度，这混合了被动接收（记录）和主动控制（创建窗口）

**决策：** 保持规格设计，`observe` 是**配置视角**的概念

用户配置 `observe` 描述「想怎样观测」，实现时拆分为不同组件：

```
observe 配置 (用户写的)
    │
    ├── output: terminal/file/both
    │       → TerminalReporter / FileLogger
    │
    ├── multiplexer: { window, focus }
    │       → 交给 Executor 处理窗口创建/切换
    │
    └── notifications: { on_blocked: slack }
        → NotificationSender (未来实现)
```

**实现映射：**

| 配置项 | 实现组件 | 职责 |
|--------|----------|------|
| observe.output = terminal | TerminalReporter | 输出进度到 stderr |
| observe.output = file | FileLogger | 写入日志文件 |
| observe.multiplexer | Executor | 创建/切换 multiplexer 窗口 |
| observe.notifications | NotificationSender | 发送通知 (未来) |

**理由：**
1. 保持用户配置的直观性——「observe」比「output + execution_environment + notifications」更易理解
2. 实现时按职责拆分，避免单一组件承担过多责任
3. multiplexer 是「执行环境」，但从用户视角属于「观测配置」，两者不矛盾
