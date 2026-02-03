# wt 后续开发路线图

## 当前状态

**已完成**:
- Phases v2 重构 (Phase 1-7) ✅
- TUI v2 重构 (Phase 8) ✅
- Dead Code Cleanup ✅
- Phase 9.1 并发执行 ✅
- Phase 9.2 条件分支 ✅
- 代码质量改进 ✅ (评分: A)

---

## 下一步工作

### Phase 9.3: 错误恢复 (待做)

| 功能 | 说明 | 优先级 |
|------|------|--------|
| on_error 配置 | step 失败时的处理策略 | 高 |
| 重试机制 | 自动重试失败的 step | 中 |
| 断点续执行 | 从失败点恢复执行 | 低 |

### 可选改进

| 项目 | 说明 | 状态 |
|------|------|------|
| status.rs 拆分 | 592行，可考虑拆分 | 可选 |
| artifacts 收集 | step 输出文件收集 | Phase 9 |
| agent verification | 使用 agent 验证 step 结果 | Phase 9 |

---

## 已完成阶段

| Phase | 内容 | Session |
|-------|------|---------|
| 1-5 | Phases v2 核心模型 + 执行引擎 + 命令 | 31-33 |
| 6 | 配置格式 + next/prev 连接执行引擎 | 34 |
| 7 | prev/stop/step 命令完善 | 35 |
| 8 | TUI v2 重构 | 36-37 |
| - | Hooks 清理 + Dead Code Cleanup | 38-39 |
| - | 代码质量改进 (TUI unwrap, store 拆分) | 40-41 |
| 9.1 | 并发执行 (rayon 线程池) | 41 |
| 9.2 | 条件分支 (ConditionEvaluator 模块) | 41 |
| - | condition 模块重构 (拆分为 5 文件) | 41 |
