---
name: cmd-complete
depends:
  - hooks-engine
  - atomic-git
  - atomic-misc
---

# 任务：新建 complete 命令替代 merge

## 目标

创建新的 `wt complete` 命令，替代复杂的 `wt merge`。新命令不再调用 Claude，而是通过 hooks 让用户自定义 merge 流程。

## 具体工作

### 1. CLI 定义 `src/cli.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Complete a task (merge to main)
    Complete {
        /// Task name or index
        task: String,
    },

    // 保留 merge 作为别名
    #[command(hide = true)]
    Merge {
        task: String,
        #[arg(long)]
        agent: bool,  // 忽略，仅为兼容
    },
}
```

### 2. 创建 `src/commands/complete.rs`

```rust
pub fn execute(task_ref: String) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    let name = store.resolve_task_ref(&task_ref)?;

    // 检查状态是 Review
    if store.get_status(&name) != TaskStatus::Review {
        return Err(WtError::InvalidInput(format!(
            "Task '{}' must be in review status. Run 'wt review {}' first.",
            name, name
        )));
    }

    let context = build_context(&config, &store, &name)?;

    // 执行 before_complete hook（测试、lint 等）
    println!("Running before_complete hook...");
    hooks.run_hook("before_complete", &context)?;

    // 执行默认的 merge 流程（如果用户没有在 hook 中自定义）
    // 或者完全依赖 hook，不提供默认行为
    if !config.hooks.has_custom_complete() {
        default_merge_flow(&config, &context)?;
    }

    // 更新状态
    store.set_status(&name, TaskStatus::Completed)?;

    // 执行 after_complete hook
    hooks.run_hook("after_complete", &context)?;

    println!("Task '{}' completed.", name);
    Ok(())
}

/// 默认的 merge 流程
fn default_merge_flow(config: &WtConfig, context: &HookContext) -> Result<()> {
    // 1. rebase onto main
    println!("Rebasing onto main...");
    git::rebase(&context.worktree, "origin/main")?;

    // 2. squash merge
    println!("Squash merging...");
    git::squash_merge(&context.repo_root, &context.branch)?;

    // 3. commit
    let message = format!("feat({}): complete task", context.task);
    git::commit(&context.repo_root, &message)?;

    Ok(())
}
```

### 3. 删除旧的 merge.rs

或保留为 complete.rs 的 wrapper（向后兼容）：

```rust
// src/commands/merge.rs
pub fn execute(task_ref: String, _agent_mode: bool) -> Result<()> {
    eprintln!("Warning: 'wt merge' is deprecated, use 'wt complete' instead.");
    super::complete::execute(task_ref)
}
```

### 4. 用户自定义 merge 流程示例

```yaml
hooks:
  before_complete: |
    cargo test

    # 自定义 merge 流程
    cd ${worktree}
    git fetch origin main
    git rebase origin/main

    cd ${repo_root}
    git merge --squash ${branch}
    git commit -m "feat(${task}): completed

    Co-Authored-By: Claude <noreply@anthropic.com>"
```

### 5. 状态转换

```
Review → Completed:  before_complete, [default_merge], after_complete
```

## 测试

- 测试 `wt complete` 正常流程
- 测试 before_complete hook 失败时中止
- 测试默认 merge 流程
- 测试用户自定义 merge 流程
- 测试 `wt merge` 向后兼容

## 完成标准

- [ ] 新建 complete 命令
- [ ] 集成 before_complete / after_complete hooks
- [ ] 提供默认 merge 流程（可选）
- [ ] 保留 merge 命令作为别名
- [ ] 删除 Claude agent 调用
- [ ] 测试通过
