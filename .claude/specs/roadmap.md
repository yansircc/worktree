# wt 后续开发路线图

## 当前状态

Phases v2 重构已完成：
- 新模型层 (step/workflow/phase/project/state) ✅
- 执行引擎 (executor/observer) ✅
- 命令层更新 (next/prev/stop/reset/step) ✅
- 旧代码清理 (hooks/pipelines) ✅

**但是**：新的模型和执行引擎还没有被命令层实际使用。目前的命令只做简单的状态更新。

---

## Phase 6: 连接执行引擎

**目标**：让 `wt next` 命令实际使用新的执行引擎

### 6.1 配置格式定义

定义 phases 配置格式：

```jsonc
{
  "phases": {
    "developing": {
      "on_enter": {
        "workflow": [
          { "type": "script", "run": "npm install" },
          { "type": "agent", "prompt": "@.wt/tasks/${task}.md" }
        ]
      },
      "on_exit": {
        "done": { "workflow": [...] },
        "error": { "workflow": [...] }
      }
    },
    "reviewing": {
      "on_enter": {
        "workflow": [
          { "type": "agent", "prompt": "Review code for ${task}" }
        ]
      }
    }
  }
}
```

**任务**：
- [ ] 在 `config.rs` 中定义 PhasesConfig 解析
- [ ] 定义 WorkflowConfig 和 StepConfig
- [ ] 添加配置验证

### 6.2 next 命令重写

重写 `wt next` 使用执行引擎：

```rust
// 伪代码
fn execute(task_ref: String) -> Result<()> {
    let ctx = TaskContext::load(&task_ref)?;
    let config = ctx.config.phases()?;

    // 获取当前/下一阶段
    let (current_phase, next_phase) = get_phase_transition(&ctx);

    // 执行 on_exit workflow
    if let Some(exit_workflow) = current_phase.on_exit.get(&exit_reason) {
        executor.execute(exit_workflow, &ctx)?;
    }

    // 执行 on_enter workflow
    if let Some(enter_workflow) = next_phase.on_enter {
        executor.execute(enter_workflow, &ctx)?;
    }

    // 更新状态
    ctx.set_phase(next_phase);
    ctx.save()?;
}
```

**任务**：
- [ ] 重写 `next.rs` 使用 PhaseExecutor
- [ ] 处理 worktree/branch 创建（首次进入 developing）
- [ ] 处理 multiplexer 窗口创建/切换
- [ ] 实现 agent 进程启动

### 6.3 Observer 集成

集成观测系统：

**任务**：
- [ ] 在 step 执行时触发 observer
- [ ] 实现 terminal observer 输出
- [ ] 实现 log observer 日志记录

---

## Phase 7: 完善命令

### 7.1 prev 命令

**任务**：
- [ ] 实现 prev 的 workflow 执行
- [ ] 处理资源清理（从 reviewing 退回 developing）

### 7.2 stop 命令

**任务**：
- [ ] 停止当前运行的 workflow
- [ ] 保存中间状态
- [ ] 关闭 multiplexer 窗口（可选）

### 7.3 step 命令

**任务**：
- [ ] 让 agent 可以通过 `wt step done/block/fail` 影响 workflow 执行
- [ ] 实现步骤间通信

---

## Phase 8: TUI 更新

### 8.1 显示更新

**任务**：
- [ ] 显示当前 workflow 执行进度
- [ ] 显示 step 状态
- [ ] 显示 observer 输出

### 8.2 交互更新

**任务**：
- [ ] 更新快捷键（删除旧的 r/c 等）
- [ ] 添加 workflow 控制（暂停/继续/取消）

---

## Phase 9: 高级功能

### 9.1 并发执行

**任务**：
- [ ] 实现 DAG workflow 的并行执行
- [ ] 支持多任务并行推进

### 9.2 条件分支

**任务**：
- [ ] 实现 condition step
- [ ] 支持基于 step 结果的分支

### 9.3 错误恢复

**任务**：
- [ ] 实现 on_error 处理
- [ ] 支持重试策略
- [ ] 支持断点续执行

---

## 优先级建议

| 优先级 | Phase | 说明 |
|--------|-------|------|
| P0 | 6.1-6.2 | 让 next 命令可用（MVP） |
| P1 | 6.3 | 有输出反馈 |
| P1 | 7.1-7.3 | 完整的阶段控制 |
| P2 | 8.x | 更好的用户体验 |
| P3 | 9.x | 高级功能 |

---

## Session 规划

| Session | 目标 |
|---------|------|
| 34 | Phase 6.1 - 配置格式定义 |
| 35 | Phase 6.2 - next 命令重写 (worktree/branch 创建) |
| 36 | Phase 6.2 - next 命令重写 (agent 启动) |
| 37 | Phase 6.3 + 7.x - Observer + 其他命令 |
| 38 | Phase 8.x - TUI 更新 |
