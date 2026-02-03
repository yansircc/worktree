# wt 后续开发路线图

## 当前状态

Phases v2 重构进度：
- 新模型层 (step/workflow/phase/project/state) ✅
- 执行引擎 (executor/observer) ✅
- 命令层更新 (next/prev/stop/reset/step) ✅
- 旧代码清理 (hooks/pipelines) ✅
- **next 命令连接执行引擎** ✅ (Session 34)

---

## Phase 6: 连接执行引擎

### 6.1 配置格式定义 ✅

已完成：
- PhasesConfig 在 project.rs 中定义
- Phase/Workflow/Step 配置模型完整
- config.phase_sequence() 和 get_phase() 方法

### 6.2 next 命令重写 ✅

已完成：
- [x] 使用 config.phase_sequence() 而非硬编码枚举
- [x] allocate_resources() 创建 worktree/branch/window
- [x] start_agent_in_window() 启动交互式 agent
- [x] AgentStep::default_develop() 默认开发 agent

待做：
- [ ] prev.rs 同步更新使用配置

### 6.3 Observer 集成

待做：
- [ ] 在 step 执行时触发 observer
- [ ] 实现 terminal observer 输出
- [ ] 实现 log observer 日志记录

---

## Phase 7: 完善命令

### 7.1 prev 命令

**任务**：
- [ ] 使用配置中的 phase sequence
- [ ] 实现 on_exit workflow 执行
- [ ] 处理资源清理（从 reviewing 退回 developing）

### 7.2 stop 命令

**任务**：
- [ ] 停止当前运行的 workflow
- [ ] 保存中间状态
- [ ] 关闭 multiplexer 窗口（可选）

### 7.3 step 命令

**任务**：
- [ ] 让 agent 可以通过 `wt step done/block/fail` 影响 workflow 执行
- [ ] 实现步骤间通信

---

## Phase 8: TUI 更新

### 8.1 显示更新

**任务**：
- [ ] 显示当前 workflow 执行进度
- [ ] 显示 step 状态
- [ ] 显示 observer 输出

### 8.2 交互更新

**任务**：
- [ ] 更新快捷键
- [ ] 添加 workflow 控制（暂停/继续/取消）

---

## Phase 9: 高级功能

### 9.1 并发执行

**任务**：
- [ ] 实现 DAG workflow 的并行执行
- [ ] 支持多任务并行推进

### 9.2 条件分支

**任务**：
- [ ] 实现 condition step
- [ ] 支持基于 step 结果的分支

### 9.3 错误恢复

**任务**：
- [ ] 实现 on_error 处理
- [ ] 支持重试策略
- [ ] 支持断点续执行

---

## 优先级建议

| 优先级 | Phase | 说明 | 状态 |
|--------|-------|------|------|
| P0 | 6.1-6.2 | next 命令可用 | ✅ |
| P1 | 6.3 | 有输出反馈 | 待做 |
| P1 | 7.1-7.3 | 完整的阶段控制 | 待做 |
| P2 | 8.x | 更好的用户体验 | 待做 |
| P3 | 9.x | 高级功能 | 待做 |

---

## Session 规划

| Session | 目标 | 状态 |
|---------|------|------|
| 34 | Phase 6.2a/b - next 连接执行引擎 + agent 启动 | ✅ |
| 35 | Phase 6.2c + 6.3 - prev 更新 + Observer 集成 | 下一步 |
| 36 | Phase 7.x - 完善命令 | 待做 |
| 37 | Phase 8.x - TUI 更新 | 待做 |
