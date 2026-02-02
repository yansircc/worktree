# 测试指南

## 运行测试

```bash
cargo test                    # 全部
cargo test --lib              # 单元测试
cargo test --test cli         # CLI E2E
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

### --action 操作

```bash
wt run ui                              # 启动任务
wt status --action list --task ui        # 预期: available_actions 含 review
wt status --action enter --task ui       # 预期: command.type = tmux_switch
wt status --action review --task ui      # 预期: success, tmux 被关闭
wt status --action resume --task ui      # 预期: success, 回到 running
wt status --action complete --task ui    # 预期: success
wt reset ui                              # 重置
```

### 错误场景

```bash
wt status --action list                  # 缺 --task → JSON error
wt status --action list --task xxx       # 不存在 → JSON error
wt status --action unknown --task ui     # 未知操作 → JSON error
```

### 冲突检测

```bash
wt run ui

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
