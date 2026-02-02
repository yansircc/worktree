# Handoff 文档 - wt 开发进度

## Session 21 完成的工作 (2026-02-03)

### cleanup-legacy 任务全部完成

按照 `.claude/specs/cleanup-legacy.md` 执行的清理工作：

1. **删除废弃命令**
   - 删除 `src/commands/archive.rs`, `src/commands/merge.rs`
   - 从 CLI 中删除 `Start`, `Merge`, `Archive` 枚举变体

2. **删除旧配置字段**
   - 删除 `init_script`, `archive_script`, `review_script`, `merge_script`
   - 简化 `get_hook()` 方法，移除 legacy fallback 逻辑

3. **更新用户提示信息**
   - `wt start` → `wt run` (5 处)
   - `wt merge` → `wt complete` (2 处)

4. **更新配置模板**
   - `wt init` 生成新的 `hooks:` 格式配置

5. **更新文档和测试**

### Phase 4 docs 任务完成

1. **README.md 大幅更新**
   - Hooks 系统详细文档
   - 变量列表
   - 内部操作 (wt internal) 完整参考

2. **.claude/CLAUDE.md 更新**
   - 新的目录结构
   - 新的配置格式

---

## 项目状态

### 任务完成情况

```
Phase 1-2 (基础设施):  5/5 完成
Phase 3 (命令集成):    4/4 完成
Phase 4 (文档):        1/1 完成
cleanup-legacy:        ✅ 完成
```

**所有计划任务已完成！**

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
| `src/commands/run.rs` | run 命令 |
| `src/commands/complete.rs` | complete 命令 |
| `src/commands/delete.rs` | delete 命令 |
| `src/commands/review.rs` | review 命令 |
| `src/commands/resume.rs` | resume 命令 |
| `src/commands/reset.rs` | reset 命令 |
| `src/commands/internal/` | 内部原子操作 |

### 内部操作
| 模块 | 说明 |
|------|------|
| `internal/git.rs` | Git 原子操作 |
| `internal/mux.rs` | Multiplexer 原子操作 |
| `internal/misc.rs` | 文件、状态、任务、配置、通知操作 |

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 21 | cleanup-legacy + Phase 4 docs 全部完成 |
| 20 | Phase 3 完成、修复 zellij 合并问题、清理资源 |
| 19 | 补全 atomic-misc CLI 子命令、创建 hooks.rs |
| 18 | Hooks 系统设计、任务规划、Phase 1-2 服务层 |
| 17 | 代码清理、Zellij 后端改进 |
| 16 | 任务状态重设计、Zellij layout 方案 |
| 15 | Multiplexer 抽象层 |
| 1-14 | 初始实现、TUI、tail/logs、task index、completions 等 |
