# Handoff 文档 - wt 开发进度

## Session 38 完成的工作 (2026-02-03)

### 1. 清理遗留 Hooks 代码 ✅

从 `config.rs` 删除:
- `Step`, `HookDef`, `HooksConfig`, `PipelinesConfig` 类型
- `WtConfig.hooks` 和 `WtConfig.pipelines` 字段
- 相关方法和测试

更新 `wt init` 生成 phases 配置。

### 2. Dead Code Cleanup (部分完成)

**已删除**:
- `error.rs` - 4 个未使用的错误变体
- `models/project.rs` - `Project`, `ProjectStatus`, `ResourceConfig` 等 (446 → 114 行)
- `models/state.rs` - `DerivedTaskStatus` 和派生函数 (330 → 97 行)
- `models/status.rs`, `store.rs` - v2 桥接方法
- `services/dependency.rs` - `check_dependencies_completed()` 函数

**代码精简**: 49716 → 46723 行 (减少约 3000 行)

**剩余警告**: 27 个 (phase.rs, step.rs, workflow.rs, observer/, task_context.rs)

### 测试结果

```
lib: 191 passed ✅
cli: 106 passed ✅
integration: 46 passed ✅
```

---

## 下一步工作

### 继续 Dead Code Cleanup

**Spec**: `.claude/specs/dead-code-cleanup.md`

剩余清理项:
- `models/phase.rs` - 未使用的方法 (display_name, needs_resources 等)
- `models/step.rs` - `VerifyType`, `StepExecute` 枚举
- `models/workflow.rs` - 未使用的方法
- `services/task_context.rs` - 未使用的方法
- `services/observer/` - 未使用的类型和方法

**目标**: `cargo build` 无 warning

---

## 项目状态

### 可用命令

```bash
# 任务管理
wt init / create / validate / list / delete

# 阶段控制
wt next <task>       # 推进到下一阶段
wt prev <task>       # 回退到上一阶段
wt stop <task>       # 停止任务进程
wt reset <task>      # 重置任务
wt step done/block/fail  # Agent 标记 step 状态

# 状态和日志
wt status [--all] [--json]  # TUI 或 JSON 输出
wt tail <task>       # 查看 transcript
wt logs              # 生成日志
```

### TUI 快捷键

| 键 | 功能 |
|----|------|
| `j/k` | 上下选择 |
| `Enter` | 切换到任务窗口 |
| `n` | 执行 `wt next` |
| `p` | 执行 `wt prev` |
| `s` | 执行 `wt stop` |
| `l` | 打开日志 |
| `t` | 打开 transcript |
| `?` | 帮助 |
| `q` | 退出 |

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 38 | Hooks 清理 + Dead Code Cleanup (部分) |
| 37 | TUI v2 增强 - `p` 快捷键, Idle 任务 Enter 行为 |
| 36 | TUI v2 完成 - 左右分栏布局, 新快捷键 |
| 35 | Phase 7 完成 - Observer 集成 + 命令完善 |
| 34 | Phase 6 完成 - next 连接执行引擎 |
| 31-33 | Phases v2 核心实现 |
