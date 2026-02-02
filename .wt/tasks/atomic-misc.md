---
name: atomic-misc
depends:
  - config-hooks
---

# 任务：实现文件/状态/通知原子操作

## 目标

实现文件操作、状态操作、通知等原子操作。

## 具体工作

### 1. 文件操作 `src/services/files.rs`

```rust
/// 复制 copy_files 到 worktree
pub fn copy_files(config: &WtConfig, worktree_path: &str) -> Result<()>;

/// 备份 worktree 到 .wt/backups/
pub fn backup(task: &str, worktree_path: &str, backup_dir: &str) -> Result<String>;

/// 清理目录（删除 target/, node_modules/ 等）
pub fn clean(worktree_path: &str, patterns: &[&str]) -> Result<()>;
```

### 2. 状态操作 `src/services/status_ops.rs`

```rust
/// 设置任务状态
pub fn set_status(task: &str, status: TaskStatus) -> Result<()>;

/// 获取任务状态（返回字符串供脚本使用）
pub fn get_status(task: &str) -> Result<String>;

/// 检查任务是否存在
pub fn task_exists(task: &str) -> Result<bool>;

/// 检查依赖是否全部完成
pub fn deps_ready(task: &str) -> Result<bool>;

/// 获取被当前任务阻塞的任务列表
pub fn list_blocked_by(task: &str) -> Result<Vec<String>>;
```

### 3. 通知操作 `src/services/notify.rs`

```rust
/// 发送系统通知
pub fn notify(title: &str, message: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("osascript")
            .args(["-e", &format!(
                "display notification \"{}\" with title \"{}\"",
                message, title
            )])
            .status()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("notify-send")
            .args([title, message])
            .status()?;
    }

    Ok(())
}

/// 交互式确认
pub fn confirm(message: &str) -> Result<bool> {
    print!("{} [y/N] ", message);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase() == "y")
}

/// 中止并报错
pub fn abort(message: &str) -> ! {
    eprintln!("Aborted: {}", message);
    std::process::exit(1);
}

/// 记录日志
pub fn log(task: &str, message: &str) -> Result<()> {
    let log_dir = Path::new(".wt/logs");
    fs::create_dir_all(log_dir)?;
    let log_file = log_dir.join(format!("{}.log", task));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message)?;
    Ok(())
}
```

### 4. 配置操作 `src/services/config_ops.rs`

```rust
/// 获取配置值
pub fn get_config(key: &str) -> Result<String> {
    let config = WtConfig::load()?;
    match key {
        "claude_command" => Ok(config.claude_command.clone()),
        "session_name" => Ok(config.session_name.clone()),
        "multiplexer" => Ok(config.multiplexer_type().to_string()),
        _ => Err(WtError::InvalidInput(format!("Unknown config key: {}", key))),
    }
}
```

### 5. 内部子命令

```rust
// wt internal files:backup <task>
// wt internal files:clean <worktree>
// wt internal status:set <task> <status>
// wt internal status:get <task>
// wt internal task:exists <task>
// wt internal task:deps-ready <task>
// wt internal notify <title> <message>
// wt internal confirm <message>
// wt internal abort <message>
// wt internal log <task> <message>
// wt internal config:get <key>
```

### 6. 用户调用方式

```bash
wt internal files:backup ${task}
wt internal status:set ${task} completed
wt internal task:deps-ready ${task} || wt internal abort "Dependencies not ready"
wt internal notify "wt" "Task ${task} completed!"
wt internal confirm "Delete worktree?" || exit 0
wt internal log ${task} "Starting merge process"
```

## 测试

- 测试文件备份/清理
- 测试状态读写
- 测试通知（macOS/Linux）
- 测试 confirm 交互

## 完成标准

- [ ] 文件操作实现
- [ ] 状态操作实现
- [ ] 通知操作实现
- [ ] 配置操作实现
- [ ] 内部子命令注册
- [ ] 测试通过
