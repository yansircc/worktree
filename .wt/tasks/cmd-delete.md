---
name: cmd-delete
depends:
  - hooks-engine
  - atomic-git
  - atomic-misc
---

# 任务：扩展 delete 命令

## 目标

扩展 `wt delete` 命令，支持删除已完成的任务（不仅是 scratch），并集成 hooks。

## 具体工作

### 1. 扩展 CLI `src/cli.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Delete a task's resources (worktree, branch)
    Delete {
        /// Task name or index
        task: String,

        /// Force delete (skip confirmation for non-completed tasks)
        #[arg(long, short)]
        force: bool,
    },

    // archive 作为别名（向后兼容）
    #[command(hide = true)]
    Archive {
        task: String,
    },
}
```

### 2. 重构 `src/commands/delete.rs`

```rust
pub fn execute(task_ref: String, force: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let mut store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    let name = store.resolve_task_ref(&task_ref)?;
    let status = store.get_status(&name);
    let is_scratch = store.is_scratch(&name);

    // 检查是否可以删除
    match status {
        TaskStatus::Completed => {
            // 已完成的任务可以直接删除
        }
        TaskStatus::Pending => {
            // Pending 没有资源需要清理
            return Err(WtError::InvalidInput(format!(
                "Task '{}' is pending, nothing to delete.",
                name
            )));
        }
        _ if is_scratch => {
            // Scratch 可以直接删除
        }
        _ if force => {
            // 强制删除
            println!("Warning: Force deleting task '{}' in {} status.", name, status.display_name());
        }
        _ => {
            return Err(WtError::InvalidInput(format!(
                "Task '{}' is {}. Use --force to delete, or complete it first.",
                name, status.display_name()
            )));
        }
    }

    let context = build_context(&config, &store, &name)?;

    // 执行 before_delete hook（备份、清理等）
    hooks.run_hook("before_delete", &context)?;

    // 关闭 multiplexer 窗口
    close_window_if_exists(&config, &store, &name)?;

    // 删除 worktree
    if let Some(instance) = store.get_instance(&name) {
        if Path::new(&instance.worktree_path).exists() {
            git::delete_worktree(&context.repo_root, &instance.worktree_path)?;
        }

        // 删除分支
        git::delete_branch(&context.repo_root, &instance.branch)?;
    }

    // 清理状态
    if is_scratch {
        store.remove_scratch(&name)?;
    } else {
        store.clear_instance(&name)?;
        // 对于已完成的任务，保留 Completed 状态记录
        // 对于强制删除的任务，改为 Pending
        if status != TaskStatus::Completed {
            store.set_status(&name, TaskStatus::Pending)?;
        }
    }

    println!("Deleted resources for '{}'.", name);
    Ok(())
}
```

### 3. 向后兼容

```rust
// src/commands/archive.rs
pub fn execute(task_ref: String) -> Result<()> {
    eprintln!("Warning: 'wt archive' is deprecated, use 'wt delete' instead.");
    super::delete::execute(task_ref, false)
}
```

### 4. 删除逻辑

| 状态 | 行为 |
|------|------|
| Completed | 直接删除资源，保留完成记录 |
| Review/Running | 需要 --force，删除后回到 Pending |
| Pending | 报错（没有资源） |
| Scratch | 直接删除，移除所有记录 |

### 5. Hooks

```yaml
hooks:
  before_delete: |
    # 备份代码
    wt internal files:backup ${task}
    # 清理大文件
    rm -rf ${worktree}/target/
```

## 测试

- 测试删除 Completed 任务
- 测试删除 Scratch
- 测试删除 Running 任务（需要 --force）
- 测试 before_delete hook
- 测试 `wt archive` 向后兼容

## 完成标准

- [ ] 支持删除 Completed 任务
- [ ] 支持 --force 强制删除
- [ ] 集成 before_delete hook
- [ ] 保留 archive 作为别名
- [ ] 测试通过
