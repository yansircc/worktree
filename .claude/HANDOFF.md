# Handoff 文档 - wt 开发进度

## Session 20 完成的工作 (2026-02-02)

### 1. Phase 3 全部完成

4 个任务开发完成并合并到 main：

| 任务 | 说明 | Commit |
|------|------|--------|
| cmd-run | `start` → `run` 改名，集成 hooks | 已合并 |
| cmd-lifecycle | `review`/`resume`/`reset` 集成 hooks | `d280f1b` |
| cmd-complete | 新命令替代 `merge` | `5d87504` |
| cmd-delete | 扩展删除功能 + `--force` | `2bd932b` |

### 2. 修复 zellij 合并导致的文件丢失

合并 zellij 分支时部分文件被覆盖，已修复：
- 恢复 `src/services/hooks.rs`
- 恢复 `src/models/hook_context.rs`
- 恢复 `src/models/config.rs` 中的 HooksConfig
- 添加缺失的 `HookFailed` 错误变体
- 添加 `has_custom_complete_hook()` 方法

Commit: `38611d1`

### 3. 清理资源

- 删除 4 个遗留的 tmux merge 窗口
- 删除 zellij scratch 环境（worktree + 分支）

---

## 待完成工作

### cleanup-legacy (Spec 已创建)

清理遗留代码和更新文档，详见 `.claude/specs/cleanup-legacy.md`：

1. **删除废弃命令**：`archive.rs`, `merge.rs`, CLI 中的 Start/Merge/Archive
2. **删除旧配置字段**：`init_script`, `archive_script`, `review_script`, `merge_script`
3. **更新用户提示**：5 处使用旧命令名的地方
4. **更新配置模板**：`wt init` 生成新的 `hooks:` 格式
5. **更新文档**：README.md, CLAUDE.md, testing.md, skills 等

### docs (Phase 4)

`.wt/tasks/docs.md` - 依赖已就绪，可以和 cleanup-legacy 一起做

---

## 项目状态

### 任务完成情况

```
Phase 1-2 (基础设施):  5/5 完成
Phase 3 (命令集成):    4/4 完成
Phase 4 (文档):        0/1 待开始
cleanup-legacy:        待开始
```

### Hooks 系统集成状态

| 命令 | on_create | before_run | after_run | before_review | after_review | before_resume | before_complete | after_complete | before_delete | before_reset |
|------|-----------|------------|-----------|---------------|--------------|---------------|-----------------|----------------|---------------|--------------|
| run | ✅ | ✅ | ✅ | - | - | - | - | - | - | - |
| review | - | - | - | ✅ | ✅ | - | - | - | - | - |
| resume | - | - | - | - | - | ✅ | - | - | - | - |
| complete | - | - | - | - | - | - | ✅ | ✅ | - | - |
| delete | - | - | - | - | - | - | - | - | ✅ | - |
| reset | - | - | - | - | - | - | - | - | - | ✅ |

---

## 文件索引

### Hooks 系统
| 文件 | 说明 |
|------|------|
| `src/models/config.rs` | HooksConfig, HookName, get_hook() |
| `src/models/hook_context.rs` | HookContext, expand_variables() |
| `src/services/hooks.rs` | HooksEngine |

### 命令
| 文件 | 说明 |
|------|------|
| `src/commands/run.rs` | run 命令（原 start） |
| `src/commands/complete.rs` | complete 命令（原 merge） |
| `src/commands/delete.rs` | delete 命令（扩展版） |
| `src/commands/review.rs` | review 命令 |
| `src/commands/resume.rs` | resume 命令 |
| `src/commands/reset.rs` | reset 命令 |

### 待删除（cleanup-legacy）
| 文件 | 说明 |
|------|------|
| `src/commands/archive.rs` | 废弃别名 |
| `src/commands/merge.rs` | 废弃别名 |
| `.wt/prompts/merge.md` | 过时的 merge prompt |

---

## 已知问题

1. **dead_code 警告**：`ScriptFailed` 错误变体未使用
2. **unused_imports 警告**：`HooksConfig` 未直接使用（通过 WtConfig 间接使用）

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 20 | Phase 3 完成、修复 zellij 合并问题、清理资源 |
| 19 | 补全 atomic-misc CLI 子命令、创建 hooks.rs |
| 18 | Hooks 系统设计、任务规划、Phase 1-2 服务层 |
| 17 | 代码清理、Zellij 后端改进 |
| 16 | 任务状态重设计、Zellij layout 方案 |
| 15 | Multiplexer 抽象层 |
| 1-14 | 初始实现、TUI、tail/logs、task index、completions 等 |
