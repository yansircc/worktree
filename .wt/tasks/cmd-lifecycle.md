---
name: cmd-lifecycle
depends:
  - hooks-engine
  - atomic-mux
  - atomic-misc
---

# 任务：重构 review/resume/reset 命令

## 目标

为 review、resume、reset 命令集成 hooks 系统。

## 具体工作

### 1. 重构 `src/commands/review.rs`

```rust
pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    let name = store.resolve_task_ref(&task_ref)?;
    let context = build_context(&config, &store, &name)?;

    // 执行 before_review hook（检查、lint 等）
    hooks.run_hook("before_review", &context)?;

    // 关闭 multiplexer 窗口
    close_window(&config, &store, &name)?;

    // 更新状态
    store.set_status(&name, TaskStatus::Review)?;

    // 执行 after_review hook
    hooks.run_hook("after_review", &context)?;

    println!("Task '{}' marked for review.", name);
    Ok(())
}
```

### 2. 重构 `src/commands/resume.rs`

```rust
pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    let name = store.resolve_task_ref(&task_ref)?;

    // 检查状态是 Review
    if store.get_status(&name) != TaskStatus::Review {
        return Err(WtError::InvalidInput(...));
    }

    let context = build_context(&config, &store, &name)?;

    // 执行 before_resume hook
    hooks.run_hook("before_resume", &context)?;

    // 创建 multiplexer 窗口，启动 claude
    start_claude_in_window(...)?;

    // 更新状态
    store.set_status(&name, TaskStatus::Running)?;

    println!("Task '{}' resumed.", name);
    Ok(())
}
```

### 3. 重构 `src/commands/reset.rs`

```rust
pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    let name = store.resolve_task_ref(&task_ref)?;
    let context = build_context(&config, &store, &name)?;

    // 执行 before_reset hook（清理、备份等）
    hooks.run_hook("before_reset", &context)?;

    // 关闭 multiplexer 窗口（如果存在）
    close_window_if_exists(&config, &store, &name)?;

    // 备份代码（如果配置了）
    if config.should_backup_on_reset() {
        backup_worktree(&name, &context.worktree)?;
    }

    // 删除 worktree 和分支
    cleanup_resources(&name)?;

    // 更新状态
    store.set_status(&name, TaskStatus::Pending)?;
    store.clear_instance(&name)?;

    println!("Task '{}' reset to pending.", name);
    Ok(())
}
```

### 4. 状态转换图

```
Running → Review:   before_review, after_review
Review → Running:   before_resume
Any → Pending:      before_reset
```

## 测试

- 测试 review 触发 before_review hook
- 测试 review hook 失败时不改变状态
- 测试 resume 触发 before_resume hook
- 测试 reset 触发 before_reset hook
- 测试 reset 备份功能

## 完成标准

- [ ] review 集成 before_review / after_review hooks
- [ ] resume 集成 before_resume hook
- [ ] reset 集成 before_reset hook
- [ ] 备份功能可通过 hook 或配置控制
- [ ] 测试通过
