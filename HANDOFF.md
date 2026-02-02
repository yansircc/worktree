# Session Handoff

## Session 16: 任务状态重设计

### 完成的工作

**核心变更：将 5 状态简化为 4 状态**

```
旧: Pending → Running → Done → Merged → Archived
新: Pending → Running → Review → Completed
```

**命令变更：**

| 旧命令 | 新命令 | 说明 |
|--------|--------|------|
| `wt done` | `wt review` | 标记任务待审核 |
| - | `wt resume` | 从 Review 恢复到 Running |
| `wt archive` | `wt delete` | 仅用于删除 scratch 环境 |
| `wt merge` | `wt merge` | 保持，merge 成功后自动清理 |

**文件变更：**

| 操作 | 文件 |
|------|------|
| 新增 | `src/commands/review.rs` - 替代 done.rs |
| 新增 | `src/commands/resume.rs` - Review → Running |
| 新增 | `src/commands/delete.rs` - scratch 专用 |
| 删除 | `src/commands/done.rs` |
| 删除 | `src/commands/archive.rs` |
| 修改 | `src/models/task.rs` - TaskStatus enum 简化 |
| 修改 | `src/models/config.rs` - 新增 review_script, merge_script |
| 修改 | `src/services/dependency.rs` - 依赖检查改为 Completed |
| 修改 | TUI 和 Actions API - 适配新状态 |
| 修改 | 所有测试文件 - 适配新状态名 |

**向后兼容：**
- 旧 status.json 中的 `done` 自动映射到 `Review`
- 旧 status.json 中的 `merged`/`archived` 自动映射到 `Completed`

### TUI 快捷键

| 按键 | 功能 |
|------|------|
| `r` | 标记 review (原 `d`) |
| `u` | resume (新增) |
| `c` | complete (原 `m` + `a`) |

---

## 之前 Sessions 摘要

- Session 1-10: 基础功能（init, create, start, done, merged, archive, reset, list, next, validate）
- Session 11-12: TUI 状态面板、tail 命令、logs 命令
- Session 13: shell 补全（completions generate/install）
- Session 14: wt new（scratch 环境）
- Session 15: wt merge 命令实现（rebase + squash merge + auto archive）

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
