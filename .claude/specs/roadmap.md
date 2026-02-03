# wt 后续开发路线图

## 当前状态

**已完成**:
- Phases v2 重构 (Phase 1-7) ✅
- TUI v2 重构 (Phase 8) ✅
- Dead Code Cleanup ✅ (Session 39 完成，0 warnings)
- 文档更新 ✅

---

## 下一步工作

### 1. 代码质量改进

**Spec**: `.claude/specs/code-quality-improvements.md`

**高优先级**:
- 修复 TUI 中的 unsafe unwrap
- 完善 executor 模块 TODO

**中优先级**:
- 拆分大模块 (store.rs, status.rs)
- 增加 services 层测试

### 2. Phase 9: 高级功能 (待做)

| 子阶段 | 目标 | 状态 |
|--------|------|------|
| 9.1 | 并发执行 - DAG 并行、多任务并行 | 待做 |
| 9.2 | 条件分支 - condition step | 待做 |
| 9.3 | 错误恢复 - on_error、重试、断点续执行 | 待做 |

---

## 已完成阶段

| Phase | 内容 | Session |
|-------|------|---------|
| 1-5 | Phases v2 核心模型 + 执行引擎 + 命令 | 31-33 |
| 6 | 配置格式 + next/prev 连接执行引擎 | 34 |
| 7 | prev/stop/step 命令完善 | 35 |
| 8 | TUI v2 重构 | 36-37 |
| - | Hooks 清理 + Dead Code Cleanup | 38-39 |
