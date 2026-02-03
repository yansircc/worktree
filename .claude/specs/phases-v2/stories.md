# 用户故事

## Epic 0: Step 状态管理

### Story 0.1: Script Step 状态自动转换

**作为** wt 系统
**我想要** 根据 script 执行结果自动设置 step 状态
**以便** 状态与实际一致

**验收标准：**
- [ ] script 开始执行 → step 状态变为 running
- [ ] script exit 0 → step 状态变为 success
- [ ] script exit != 0 → step 状态变为 failed
- [ ] script 超时 → step 状态变为 timeout
- [ ] condition 不满足 → step 状态变为 skipped

---

### Story 0.2: Agent Step 状态标记

**作为** Agent
**我想要** 通过命令标记当前 step 的状态
**以便** wt 系统知道我的执行结果

**验收标准：**
- [ ] `wt step done` → step 状态变为 success
- [ ] `wt step block "原因"` → step 状态变为 blocked
- [ ] `wt step fail "原因"` → step 状态变为 failed
- [ ] 消息保存到 StepResult.message

**测试场景：**
```bash
# Agent 在执行过程中
wt step done  # 标记成功

# 需要 human 确认
wt step block "需要确认 API 设计方案"

# 无法完成
wt step fail "依赖的服务不可用"
```

---

### Story 0.3: Phase 状态从 Steps 派生

**作为** wt 系统
**我想要** 从所有 steps 的状态派生 phase 状态
**以便** 知道整个 workflow 的执行情况

**验收标准：**
- [ ] 有 step running → phase running
- [ ] 所有 steps success/skipped → phase success
- [ ] 任一 step failed → phase failed
- [ ] 任一 step blocked → phase blocked

---

### Story 0.4: Phase Success 触发自动推进

**作为** wt 用户
**我想要** phase 成功后自动进入下一阶段
**以便** 减少手动操作

**验收标准：**
- [ ] Phase success → 执行 on_exit workflow
- [ ] on_exit 完成 → 资源转换
- [ ] 资源转换完成 → 进入下一 phase
- [ ] 执行下一 phase 的 on_enter workflow

---

### Story 0.5: Phase Blocked/Failed 等待介入

**作为** wt 用户
**我想要** phase blocked/failed 时任务暂停等待我处理
**以便** 我可以介入解决问题

**验收标准：**
- [ ] Phase blocked → task 状态变为 idle
- [ ] Phase failed → task 状态变为 idle
- [ ] 显示 blocked/failed 的原因
- [ ] Human 可用 `wt next` 强制推进

---

## Epic 1: 阶段驱动的任务生命周期

### Story 1.1: 自动阶段推进

**作为** wt 用户
**我想要** 当 workflow 完成后自动进入下一阶段
**以便** 减少手动操作

**验收标准：**
- [ ] developing 阶段的 on_enter workflow 完成后，自动执行 on_exit
- [ ] on_exit 完成后，自动进入 reviewing 阶段
- [ ] reviewing 阶段的 workflow 完成后，自动进入 completed
- [ ] completed 阶段自动销毁 worktree 资源

**测试场景：**
```bash
# 创建任务
wt create auth

# 推进到 developing（从 pending）
wt next auth

# 等待 agent 完成开发...
# 预期：自动进入 reviewing

# 等待 agent 完成审查...
# 预期：自动进入 completed，worktree 已清理
```

---

### Story 1.2: 强制推进 (next)

**作为** wt 用户
**我想要** 在手动修复后强制推进阶段
**以便** 跳过未完成的 workflow

**验收标准：**
- [ ] `wt next <task>` 停止当前进程
- [ ] 执行 on_exit workflow
- [ ] 资源转换（按阶段定义）
- [ ] 进入下一阶段，执行 on_enter workflow
- [ ] 在 completed 阶段执行 next 报错

**测试场景：**
```bash
# 任务在 developing 阶段，agent 卡住了
wt stop auth

# 用户手动修复代码
vim .wt/worktrees/auth/src/main.rs

# 强制推进到 reviewing
wt next auth
# 预期：进入 reviewing，执行 review workflow
```

---

### Story 1.3: 强制回退 (prev)

**作为** wt 用户
**我想要** 回退到上一阶段
**以便** 重新处理问题

**验收标准：**
- [ ] `wt prev <task>` 停止当前进程
- [ ] 资源转换（按阶段定义）
- [ ] 进入上一阶段
- [ ] 不执行 on_enter（回退不触发 workflow）
- [ ] 在 pending 阶段执行 prev 报错

**测试场景：**
```bash
# 任务在 reviewing 阶段，发现需要继续开发
wt prev auth
# 预期：回到 developing 阶段，保留 worktree
```

---

### Story 1.4: 停止进程 (stop)

**作为** wt 用户
**我想要** 停止当前运行的 agent
**以便** 手动干预或查看状态

**验收标准：**
- [ ] `wt stop <task>` 停止 tmux/zellij 窗口中的进程
- [ ] 任务状态变为 idle
- [ ] phase 保持不变
- [ ] idle_reason 设为 manual

**测试场景：**
```bash
# 任务在 developing 阶段，agent 正在运行
wt status auth
# 预期：status = active

wt stop auth
wt status auth
# 预期：status = idle, phase = developing
```

---

## Epic 2: Workflow 执行引擎

### Story 2.1: Step 执行和日志捕获

**作为** wt 用户
**我想要** 每个 step 的输出自动保存到日志
**以便** 后续分析和调试

**验收标准：**
- [ ] 每个 step 的 stdout/stderr 保存到 `.wt/logs/{task}/{phase}/step-{n}.log`
- [ ] 生成 StepResult（status, exit_code, duration, output_file）
- [ ] context.json 累积记录所有 StepResults

**测试场景：**
```bash
wt next auth  # 进入 developing

# 检查日志
ls .wt/logs/auth/developing/
# 预期：step-1.log, step-2.log, context.json
```

---

### Story 2.2: 管道上下文传递

**作为** workflow 中的 agent
**我想要** 获取之前 step 的执行结果
**以便** 决定如何处理（如分析失败原因）

**验收标准：**
- [ ] Agent 通过环境变量或 stdin 收到 steps_completed
- [ ] 包含每个 step 的 id, status, exit_code, log 路径
- [ ] Agent 可用 Read/Grep 工具分析日志

**测试场景：**
```jsonc
// on_enter workflow
[
  { "run": "npm run lint" },  // 假设失败
  { "agent": { "prompt": "分析 lint 错误并修复", "model": "sonnet" } }
]
// Agent 收到 steps_completed，包含 lint 失败信息
```

---

### Story 2.3: 管道默认不中断

**作为** wt 用户
**我想要** step 失败时管道继续执行
**以便** agent 可以处理失败情况

**验收标准：**
- [ ] 默认 on_failure: continue
- [ ] step 失败后继续执行下一个 step
- [ ] 可配置 on_failure: abort 强制中断

**测试场景：**
```jsonc
// lint 失败，但 agent 继续执行
[
  { "run": "npm run lint" },  // exit 1
  { "agent": { "prompt": "修复 lint 错误" } }  // 继续执行
]
```

---

### Story 2.4: Agent Chain 自动串联

**作为** wt 用户
**我想要** 连续的 agent steps 自动通过 stream-json 串联
**以便** agent 间共享上下文

**验收标准：**
- [ ] 连续 agent steps 默认 chain: auto
- [ ] 自动添加 --output-format stream-json 和 --input-format stream-json
- [ ] 可配置 chain: false 禁用串联

**测试场景：**
```jsonc
[
  { "agent": { "prompt": "快速检查", "model": "haiku" } },
  { "agent": { "prompt": "深度审查", "model": "sonnet" } }
]
// 展开为：claude ... | claude ...
```

---

## Epic 3: 状态管理

### Story 3.1: 可观测状态

**作为** wt 开发者
**我想要** 状态由资源/进程实际情况派生
**以便** 状态始终与现实一致

**验收标准：**
- [ ] 检测 worktree 是否存在（git worktree list）
- [ ] 检测 branch 是否存在（git branch）
- [ ] 检测 window 是否存在（tmux/zellij）
- [ ] 检测进程是否运行

**状态派生规则：**
```
pending:   phase == null && resources.all == not_exists
active:    phase ∈ {developing, reviewing} && process.running
idle:      phase ∈ {developing, reviewing} && !process.running
completed: phase == completed
```

---

### Story 3.2: 资源自动管理

**作为** wt 用户
**我想要** 阶段转换时自动创建/销毁资源
**以便** 不需要手动管理 worktree

**验收标准：**
- [ ] pending → developing: 创建 worktree + branch + window
- [ ] developing → reviewing: 保持资源
- [ ] reviewing → completed: 销毁 worktree，保留 branch（已合并）
- [ ] reset: 备份代码到 .wt/backups/，销毁资源

---

## Epic 4: 配置系统

### Story 4.1: 内置默认配置

**作为** wt 新用户
**我想要** 零配置即可使用
**以便** 快速开始

**验收标准：**
- [ ] 无 .wt/config.jsonc 时使用内置默认
- [ ] 默认阶段序列：pending → developing → reviewing → completed
- [ ] 默认 developing on_enter：运行 agent 完成任务
- [ ] 默认 reviewing on_enter：运行 agent 审查代码

---

### Story 4.2: 用户覆盖配置

**作为** wt 高级用户
**我想要** 自定义阶段序列和 workflow
**以便** 适配我的开发流程

**验收标准：**
- [ ] 可覆盖 phase_sequence（如添加 testing 阶段）
- [ ] 可覆盖/扩展 phases 定义
- [ ] 可定义 workflows 片段并引用
- [ ] 配置合并：内置 → 项目 → 命令行

**测试场景：**
```jsonc
// .wt/config.jsonc
{
  "phase_sequence": ["pending", "developing", "testing", "reviewing", "completed"],
  "phases": {
    "testing": {
      "goal": "运行测试",
      "resources": "full",
      "on_enter": [
        { "run": "npm test" }
      ]
    }
  }
}
```

---

## Epic 5: 迁移

### Story 5.1: 清理旧命令

**作为** wt 开发者
**我想要** 删除旧的 run/review/complete/pause/resume 命令
**以便** 保持代码简洁

**验收标准：**
- [ ] 删除 src/commands/run.rs
- [ ] 删除 src/commands/review.rs
- [ ] 删除 src/commands/pause.rs
- [ ] 删除 src/commands/resume.rs
- [ ] 删除 src/commands/complete.rs
- [ ] 更新 CLI 定义
- [ ] 更新帮助文档

---

### Story 5.2: 清理旧 hooks 系统

**作为** wt 开发者
**我想要** 删除旧的 hooks 相关代码
**以便** 保持代码简洁

**验收标准：**
- [ ] 删除 src/services/hooks/ 目录
- [ ] 删除 config.rs 中的 HooksConfig
- [ ] 删除 builtin_pipelines.rs
- [ ] 更新 WtConfig 使用 phases 字段
