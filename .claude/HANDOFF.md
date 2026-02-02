# Handoff 文档 - wt 开发进度

## Session 16 完成的工作 (2026-02-02)

### Zellij 后端修复

**问题**：zellij 的 `go-to-tab-name` 和 `go-to-tab` 命令在没有 tty 时会阻塞。

**解决方案**：使用 KDL layout 文件一次性创建 session + tab + 命令。

1. **create_window 重写**
   - 创建临时 layout 文件定义 tab 和命令
   - 用 `zellij --session <name> --new-session-with-layout <layout>` 创建
   - 避免使用会阻塞的 `go-to-tab-name`、`write-chars` 等命令

2. **kill_window 简化**
   - 直接用 `zellij delete-session --force` 删除整个 session
   - wt 通常每个任务用独立 session，所以这是合理的

3. **CommandRunner 改进**
   - zellij 命令自动重定向 stdin 为 null（防止阻塞）

**涉及文件**：
- `src/services/multiplexer/zellij.rs` - layout 方案重写
- `src/services/command.rs` - null_stdin 支持

### 代码清理 - 统一 multiplexer 命名

1. **JSON API 字段重命名**
   - `tmux_alive` → `mux_alive`（TaskMetrics, TaskInfo）
   - 用户提示 `"(tmux closed)"` → `"(window closed)"`

2. **函数参数重命名**
   - `display::running_icon(tmux_alive, ...)` → `running_icon(mux_alive, ...)`

3. **配置文件清理**
   - `.wt/config.yaml`: `tmux_session` → `session_name`
   - 测试注释更新

**涉及文件**：
- `src/commands/status/types.rs`
- `src/commands/status/actions.rs`
- `src/commands/status/display.rs`
- `src/display.rs`
- `tests/cli/init.rs`

---

## Session 15 完成的工作 (2026-02-02)

### Multiplexer 抽象层

添加 `Multiplexer` trait 支持 tmux 和 zellij：

1. **新文件**
   - `src/services/multiplexer/mod.rs` - trait + enum + factory
   - `src/services/multiplexer/tmux.rs` - TmuxBackend
   - `src/services/multiplexer/zellij.rs` - ZellijBackend

2. **配置变更**
   ```yaml
   multiplexer: tmux  # 或 zellij
   session_name: wt
   ```

3. **Instance 字段**
   - `session_name`, `window_name`, `multiplexer`

---

## Session 14 完成的工作 (2026-02-02)

### Shell Function 修复

修复 `wt archive/reset` 后无法自动 cd 回主仓库的问题：

1. **使用 git-common-dir 代替 show-toplevel**
   - `git rev-parse --show-toplevel` 在 worktree 中返回 worktree 路径
   - `git rev-parse --path-format=absolute --git-common-dir` 返回主仓库的 `.git` 路径
   - Shell function 去掉 `/.git` 后缀得到主仓库路径

2. **archive 顺序修复**
   - 在删除 worktree 前保存 status.json（scratch 环境有 symlink）
   - 从主仓库目录执行 `git branch -D`（worktree 删除后 cwd 不存在）

3. **新增 git 辅助函数**
   - `git::get_repo_root()` - 获取主仓库路径
   - `git::delete_branch_in(branch, cwd)` - 在指定目录执行删除

**涉及文件**：
- `src/commands/completions.rs` - shell function 更新
- `src/commands/archive.rs` - 保存顺序 + 使用 delete_branch_in
- `src/commands/reset.rs` - 使用 delete_branch_in
- `src/services/git.rs` - 新增 get_repo_root, delete_branch_in

### PR #2 Review & 关闭

- Review 发现 PR 的 shell function 会覆盖我们的修复
- 确认 PR 内容已通过 PR #1 合并
- 关闭 PR #2 并说明原因

### 清理工作

- 清空 `.wt/status.json`
- 删除 7 个遗留的 wt/* 分支
- Squash 4 个修复提交为 1 个

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 16 | Zellij 后端修复（layout 方案）、命名统一 |
| 15 | Multiplexer 抽象层（tmux + zellij 支持）|
| 14 | Shell function 修复（cd 回主仓库）、PR #2 关闭 |
| 13 | PR Review：task index 支持、shell completions |
| 12 | 测试模块优化：新增 62 个测试 |
| 11 | 实现 `wt new` 命令（scratch 环境） |
| 10 | TUI 花屏修复、空格截断修复、默认交互模式 |
| 9 | 代码重构、`wt start --all`、init_script 并行化 |
| 1-8 | 初始实现、TUI、tail/logs、archive 等 |

---

## 待实现功能

暂无。

---

## 已知问题

1. **旧任务无法 tail**：无 session_id（已通过 find_latest_transcript 缓解）
2. **context_percent**：使用固定 200k

---

## 相关文件

### Multiplexer 抽象层
| 文件 | 说明 |
|------|------|
| `src/services/multiplexer/mod.rs` | Multiplexer trait + factory |
| `src/services/multiplexer/tmux.rs` | TmuxBackend 实现 |
| `src/services/multiplexer/zellij.rs` | ZellijBackend 实现 |

### Shell 集成
| 文件 | 说明 |
|------|------|
| `src/commands/completions.rs` | shell function (wt new/archive/reset 的 cd 行为) |

### 核心命令
| 文件 | 说明 |
|------|------|
| `src/commands/archive.rs` | 归档，使用 get_repo_root + delete_branch_in |
| `src/commands/reset.rs` | 重置，使用 get_repo_root + delete_branch_in |
| `src/commands/new.rs` | 创建 scratch 环境，支持 --print-path |

### Git 服务
| 文件 | 说明 |
|------|------|
| `src/services/git.rs` | get_repo_root(), delete_branch_in() |
