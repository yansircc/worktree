# 测试指南

## 运行测试

```bash
cargo test                    # 全部
cargo test --lib              # 单元测试
cargo test --test cli         # CLI E2E
cargo test --test integration # 集成测试
cargo test --test cli init    # 单个命令
```

## 测试分类

| 类型 | 位置 | 说明 |
|------|------|------|
| 单元 | `src/**/*.rs` 内 `#[cfg(test)]` | 快速，测单个函数 |
| 集成 | `tests/integration/` | 模块协作 |
| CLI | `tests/cli/` | 真实二进制 + 临时 git 仓库 |

## 编写测试

```rust
// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_功能_场景_预期() {
        let result = function(input);
        assert_eq!(result, expected);
    }
}

// CLI 测试
#[test]
fn test_command_scenario() {
    let dir = setup_test_repo();
    let (ok, stdout, _) = run_wt(dir.path(), &["command", "args"]);
    assert!(ok);
}
```

## 手动测试

需要真实 tmux/worktree 环境的功能，使用 mock 项目测试：

```bash
cd /Users/yansir/code/nextjs-project/try-wt
```

### 阶段推进测试

```bash
wt create ui                   # 创建任务
wt next ui                     # pending → developing (创建 worktree + 启动 agent)
wt status --json               # 预期: status=active, phase=developing
wt stop ui                     # 停止进程
wt status --json               # 预期: status=idle, phase=developing
wt next ui                     # developing → reviewing
wt prev ui                     # reviewing → developing
wt reset ui                    # 重置到 pending
```

### step 命令测试

```bash
# 在 worktree 中执行 (模拟 agent)
cd .wt/worktrees/ui
wt step done                   # 标记成功
wt step block "需要确认设计"    # 标记阻塞
wt step fail "无法完成"        # 标记失败
```

### --action 操作

```bash
wt next ui                               # 启动任务
wt status --action list --task ui        # 预期: available_actions 含 next/prev/stop
wt status --action enter --task ui       # 预期: command.type = tmux_switch
wt status --action next --task ui        # 预期: success, 推进阶段
wt status --action stop --task ui        # 预期: success, 停止进程
wt reset ui                              # 重置
```

### 错误场景

```bash
wt status --action list                  # 缺 --task → JSON error
wt status --action list --task xxx       # 不存在 → JSON error
wt status --action unknown --task ui     # 未知操作 → JSON error
wt prev ui                               # pending 状态 → 错误
wt next ui && wt next ui && wt next ui   # completed 状态再 next → 错误
```

### 冲突检测

```bash
wt next ui

# 主仓库
echo "main" >> README.md && git add . && git commit -m "main"

# worktree
cd .wt/worktrees/ui
echo "wt" >> README.md && git add . && git commit -m "wt"
git merge main  # 冲突

# 验证
cd /Users/yansir/code/nextjs-project/try-wt
wt status --json | jq '.tasks[0].has_conflict'  # true

# 清理
cd .wt/worktrees/ui && git merge --abort
cd /Users/yansir/code/nextjs-project/try-wt
git reset --hard HEAD~1
wt reset ui
```
