---
name: atomic-mux
depends:
  - config-hooks
---

# 任务：实现 Multiplexer 原子操作

## 目标

将 Multiplexer 操作封装为可在 hooks 中调用的原子操作。

## 具体工作

### 1. 扩展 `src/services/multiplexer/mod.rs` 的 Trait

```rust
pub trait Multiplexer {
    // 现有方法
    fn create_session(&self, session: &str) -> Result<()>;
    fn session_exists(&self, session: &str) -> bool;
    fn create_window(&self, session: &str, window: &str, cwd: &str, command: &str) -> Result<()>;
    fn kill_window_if_exists(&self, session: &str, window: &str) -> Result<()>;

    // 新增方法
    fn window_exists(&self, session: &str, window: &str) -> Result<bool>;
    fn focus_window(&self, session: &str, window: &str) -> Result<()>;
    fn send_keys(&self, session: &str, window: &str, keys: &str) -> Result<()>;
    fn list_windows(&self, session: &str) -> Result<Vec<String>>;
}
```

### 2. 实现 tmux 和 zellij 的新方法

**tmux.rs:**
```rust
fn window_exists(&self, session: &str, window: &str) -> Result<bool> {
    let output = Command::new("tmux")
        .args(["list-windows", "-t", session, "-F", "#{window_name}"])
        .output()?;
    let windows = String::from_utf8_lossy(&output.stdout);
    Ok(windows.lines().any(|w| w == window))
}

fn focus_window(&self, session: &str, window: &str) -> Result<()> {
    Command::new("tmux")
        .args(["select-window", "-t", &format!("{}:{}", session, window)])
        .status()?;
    Ok(())
}

fn send_keys(&self, session: &str, window: &str, keys: &str) -> Result<()> {
    Command::new("tmux")
        .args(["send-keys", "-t", &format!("{}:{}", session, window), keys])
        .status()?;
    Ok(())
}
```

**zellij.rs:**
```rust
// zellij 的实现方式不同，需要研究 zellij CLI
// 可能需要使用 zellij action 命令
```

### 3. 创建内部子命令 `src/commands/internal/mux.rs`

```rust
pub fn execute(operation: &str, args: Vec<String>) -> Result<()> {
    let config = WtConfig::load()?;
    let mux = config.create_multiplexer();

    match operation {
        "create-window" => mux.create_window(&args[0], &args[1], &args[2], &args[3]),
        "close-window" => mux.kill_window_if_exists(&args[0], &args[1]),
        "focus-window" => mux.focus_window(&args[0], &args[1]),
        "window-exists" => {
            if mux.window_exists(&args[0], &args[1])? {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        },
        "send-keys" => mux.send_keys(&args[0], &args[1], &args[2]),
        _ => Err(WtError::InvalidInput(format!("Unknown mux operation: {}", operation))),
    }
}
```

### 4. 用户调用方式

```bash
wt internal mux:close-window ${session} ${window}
wt internal mux:focus-window ${session} ${window}
wt internal mux:window-exists ${session} ${window} && echo "exists"
wt internal mux:send-keys ${session} ${window} "exit"
```

## 测试

- 测试 tmux 各操作
- 测试 zellij 各操作（如果可行）
- 测试 window-exists 返回正确的 exit code

## 完成标准

- [ ] Multiplexer trait 扩展
- [ ] TmuxBackend 实现新方法
- [ ] ZellijBackend 实现新方法
- [ ] 内部子命令注册
- [ ] 测试通过
