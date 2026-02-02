---
name: cmd-run
depends:
  - hooks-engine
  - atomic-git
  - atomic-mux
---

# 任务：重构 start 命令为 run

## 目标

将 `wt start` 改名为 `wt run`，并集成 hooks 系统。

## 具体工作

### 1. CLI 变更 `src/cli.rs`

```rust
#[derive(Subcommand)]
pub enum Commands {
    // 移除 Start，添加 Run
    /// Run a task (create worktree if needed, start Claude)
    Run {
        /// Task name or index
        task: String,

        /// Start all ready tasks
        #[arg(long)]
        all: bool,
    },

    // 保留 Start 作为别名（向后兼容）
    #[command(hide = true)]
    Start {
        task: String,
        #[arg(long)]
        all: bool,
    },
}
```

### 2. 重构 `src/commands/start.rs` → `src/commands/run.rs`

```rust
pub fn execute(task_ref: String, all: bool) -> Result<()> {
    let config = WtConfig::load()?;
    let store = TaskStore::load()?;
    let hooks = HooksEngine::new(config.clone());

    if all {
        return execute_all(&config, &store, &hooks);
    }

    let name = store.resolve_task_ref(&task_ref)?;
    execute_single(&config, &store, &hooks, &name)
}

fn execute_single(
    config: &WtConfig,
    store: &TaskStore,
    hooks: &HooksEngine,
    name: &str,
) -> Result<()> {
    let is_first_run = !store.has_instance(name);

    // 构建 HookContext
    let context = build_context(config, store, name)?;

    if is_first_run {
        // 首次运行
        // 1. 检查依赖
        store.ensure_deps_completed(name)?;

        // 2. 创建 worktree
        create_worktree(...)?;

        // 3. 执行 on_create hook
        hooks.run_hook("on_create", &context)?;
    }

    // 执行 before_run hook
    hooks.run_hook("before_run", &context)?;

    // 创建 multiplexer 窗口，启动 claude
    start_claude_in_window(...)?;

    // 更新状态
    store.set_status(name, TaskStatus::Running)?;

    // 执行 after_run hook
    hooks.run_hook("after_run", &context)?;

    Ok(())
}
```

### 3. 状态转换

```
Pending → Running:  on_create (首次), before_run, after_run
Review → Running:   before_resume (由 resume 命令处理，或合并到 run)
```

### 4. 向后兼容

- `wt start` 作为 `wt run` 的别名
- 旧配置 `init_script` 映射到 `hooks.on_create`

### 5. 更新 main.rs

```rust
Commands::Run { task, all } | Commands::Start { task, all } => {
    commands::run::execute(task, all)
}
```

## 测试

- 测试 `wt run` 首次运行（触发 on_create + before_run）
- 测试 `wt run` 已存在 worktree（只触发 before_run）
- 测试 `wt start` 别名仍然工作
- 测试 hooks 失败时中止

## 完成标准

- [ ] 命令改名 start → run
- [ ] 向后兼容 start 别名
- [ ] 集成 on_create hook
- [ ] 集成 before_run / after_run hooks
- [ ] 测试通过
