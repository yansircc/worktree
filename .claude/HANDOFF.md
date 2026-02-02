# Handoff 文档 - wt 开发进度

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
