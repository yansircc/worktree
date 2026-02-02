# Session Handoff

## Session 15 (Part 2): Multiplexer 抽象层

### 完成的工作

**新增 multiplexer 抽象层**，支持 tmux 和 zellij：

1. **新文件**
   - `src/services/multiplexer/mod.rs` - Multiplexer trait + MultiplexerType enum + 工厂函数
   - `src/services/multiplexer/tmux.rs` - TmuxBackend 实现
   - `src/services/multiplexer/zellij.rs` - ZellijBackend 实现

2. **修改的文件**
   - `src/error.rs` - 新增 `Zellij`、`MultiplexerNotInstalled` 错误
   - `src/services/command.rs` - 新增 `zellij()` 工厂方法
   - `src/constants.rs` - `DEFAULT_TMUX_SESSION` → `DEFAULT_SESSION_NAME`
   - `src/models/config.rs` - 新增 `multiplexer`、`session_name` 字段
   - `src/models/task.rs` - Instance 字段重命名 (`tmux_session` → `session_name`, `tmux_window` → `window_name`)，新增 `multiplexer` 字段
   - 所有命令文件 - 从 `tmux::` 调用改为 multiplexer trait

3. **删除的文件**
   - `src/services/tmux.rs` - 已迁移到 multiplexer/tmux.rs

4. **配置变更** (不向后兼容)
   ```yaml
   # 旧配置
   tmux_session: wt

   # 新配置
   multiplexer: tmux  # 或 zellij
   session_name: wt
   ```

---

## Session 15 (Part 1): wt merge 命令实现

### 完成的工作

**核心功能：将 `wt merged` 改造为 `wt merge`**

从"标记状态"变为"执行实际 git merge"：
- 使用 Claude 自动完成 rebase + squash merge + commit
- 支持交互模式（默认，在 tmux 中启动 TUI）和 agent 模式（`--agent`，静默执行）
- merge 成功后自动执行 `wt archive` 清理

**文件变更：**

| 操作 | 文件 |
|------|------|
| 新增 | `src/commands/merge.rs` - 新命令实现 |
| 新增 | `.wt/prompts/merge.md` - Claude merge 提示词 |
| 新增 | `tests/cli/merge.rs` - 新命令测试 |
| 删除 | `src/commands/merged.rs` - 旧命令 |
| 删除 | `tests/cli/merged.rs` - 旧测试 |
| 修改 | `src/cli.rs` - Merged → Merge，添加 --agent 标志 |
| 修改 | `src/main.rs` - 更新路由 |
| 修改 | `src/commands/mod.rs` - merged → merge |
| 修改 | `src/commands/done.rs` - 更新提示信息 |
| 修改 | `src/tui/app.rs` - 内联 mark_merged 逻辑 |
| 修改 | `tests/cli/scratch.rs` - merged → merge |

### 使用方式

```bash
# 交互模式（在 tmux 窗口中启动 Claude TUI）
wt merge <task>

# Agent 模式（静默执行，用于自动化）
wt merge <task> --agent
```

**前置条件：**
- 任务状态必须是 Done
- 必须有 worktree（instance 存在）
- 需要 `.wt/prompts/merge.md` 提示词文件

### 兼容性说明

- `wt merged` 命令已移除
- 仅标记状态功能通过 `wt status --action merged --task <name>` 或 TUI 中的 `m` 键保留

---

## 之前 Sessions 摘要

- Session 1-10: 基础功能实现（init, create, start, done, merged, archive, reset, list, next, validate）
- Session 11-12: TUI 状态面板、tail 命令、logs 命令
- Session 13: shell 补全（completions generate/install）
- Session 14: wt new（scratch 环境）

## 相关文件索引

| 功能 | 文件 |
|------|------|
| CLI 定义 | `src/cli.rs` |
| 命令实现 | `src/commands/*.rs` |
| 数据模型 | `src/models/` |
| 外部服务 | `src/services/` (含 multiplexer/) |
| TUI | `src/tui/` |
| 测试 | `tests/cli/`, `tests/integration/` |
| 文档 | `README.md`, `.claude/CLAUDE.md` |
| Merge 提示词 | `.wt/prompts/merge.md` |
