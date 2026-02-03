# 代码质量改进计划

## 评估时间: 2026-02-03 (Session 39)

## 总体评分: B+

| 方面 | 评分 |
|------|------|
| 代码结构 | A- |
| 错误处理 | A |
| 测试覆盖 | B+ |
| 代码复杂度 | B |
| 潜在问题 | B |

---

## 高优先级问题

### 1. TUI 中的 unsafe unwrap

**位置**: `src/tui/app.rs`

**问题**: `store.get(task_name).unwrap()` 假设任务存在，但无验证。如果任务引用无效，会导致 TUI 崩溃，用户丢失整个会话。

**修复**: 使用 `if let Some(task) = store.get(task_name)` 或返回错误。

### 2. Executor 模块 TODO 未完成

**位置**: `src/services/executor/`

| 文件 | 行号 | 未实现内容 |
|------|------|-----------|
| `step.rs` | 93-94 | Artifacts 和 exports 收集 |
| `workflow.rs` | 221 | 从文件读取实际输出 |
| `workflow.rs` | 246 | 并行执行的线程池 (目前是串行) |
| `phase.rs` | 183, 196 | 资源分配/释放 |

**建议**: 实现或明确记录为不需要。

---

## 中优先级问题

### 3. 大模块需要拆分

| 文件 | 行数 | 问题 |
|------|------|------|
| `models/store.rs` | 716 | 混合了加载、解析、状态管理、依赖检查 |
| `models/status.rs` | 592 | 混合了 TaskStatus、TaskPhase、IdleReason、TaskState |
| `services/executor/workflow.rs` | 577 | 复杂的工作流执行逻辑 |
| `tui/ui.rs` | 525 | 密集的渲染逻辑 |
| `services/claude.rs` | 497 | 复杂的命令构建器 |

**建议**:
- `store.rs` 拆分为 `task_loader.rs`, `task_resolver.rs`, `dependency_checker.rs`
- `status.rs` 拆分为 `task_status.rs`, `task_phase.rs`, `idle_reason.rs`

### 4. 重复代码

**位置**: `src/commands/status/actions.rs`

4 个相似的 response builder 函数:
- `success_response()`
- `error_response()`
- `error_response_no_task()`
- `task_not_found_response()`

**建议**: 抽取为通用的 `ActionResponse::success()`, `ActionResponse::error()` 方法。

### 5. Services 层缺少测试

| 文件 | 测试数 | 建议 |
|------|--------|------|
| `git.rs` | 5 | 添加 worktree 创建/删除测试 |
| `multiplexer/*.rs` | 0 | 添加窗口操作测试 |
| `claude.rs` | 0 | 添加命令构建测试 |
| `transcript.rs` | 0 | 添加解析测试 |

### 6. TUI 与 Status 命令耦合

**位置**: `src/commands/status/actions.rs`

**问题**: 从 `tui::App` 和 `TuiAction` 导入，命令层不应依赖 TUI 层。

**建议**: 提取共享接口到独立模块。

---

## 低优先级问题

### 7. 直接调用 process::exit()

**位置**: 10 处分散在各命令中

**问题**: 绕过错误处理框架，难以测试和包装。

**建议**: 统一在 `main.rs` 处理退出码。

### 8. 配置加载静默失败

**位置**: `src/services/task_context.rs:40`

```rust
WtConfig::load().unwrap_or_default()
```

**问题**: 如果配置文件有语法错误，静默使用默认值，用户无感知。

**建议**: 在 debug 模式下输出警告。

---

## 测试覆盖情况

### 充分测试 ✅
- TaskStore (26 tests)
- TaskParser (18 tests)
- StatusStore (17 tests)
- CLI 命令 (E2E 完整)

### 缺少测试 ❌
- `commands/stop.rs` (0 tests)
- `commands/prev.rs` (1 test)
- `services/git.rs` (仅 worktree list)
- `services/multiplexer/` (无单元测试)
- `services/claude.rs` (无测试)
- `services/transcript.rs` (无测试)
- `tui/` (需要真实终端，可接受)

---

## 建议的改进顺序

1. **修复 TUI unwrap** - 防止用户遇到崩溃
2. **完善 executor TODO** - 或明确标记为延期
3. **拆分 store.rs** - 提高可维护性
4. **添加 services 测试** - 提高代码可靠性
5. **解耦 TUI-Status** - 改善架构
