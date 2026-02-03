# wt 后续开发路线图

## 当前状态

Phases v2 重构进度：
- 新模型层 (step/workflow/phase/project/state) ✅
- 执行引擎 (executor/observer) ✅
- 命令层更新 (next/prev/stop/reset/step) ✅
- 旧代码清理 (hooks/pipelines) ✅
- **next 命令连接执行引擎** ✅ (Session 34)
- **Observer 集成 + Phase 7 命令完善** ✅ (Session 35)

---

## Phase 6: 连接执行引擎 ✅

### 6.1 配置格式定义 ✅

已完成：
- PhasesConfig 在 project.rs 中定义
- Phase/Workflow/Step 配置模型完整
- config.phase_sequence() 和 get_phase() 方法

### 6.2 next/prev 命令重写 ✅

已完成：
- [x] 使用 config.phase_sequence() 而非硬编码枚举
- [x] allocate_resources() 创建 worktree/branch/window
- [x] start_agent_in_window() 启动交互式 agent
- [x] AgentStep::default_develop() 默认开发 agent
- [x] prev.rs 同步更新使用配置

### 6.3 Observer 集成 ✅

已完成：
- [x] WorkflowExecutor 集成 TerminalObserver + LogObserver
- [x] 执行 workflow 时输出进度到 stderr
- [x] 日志写入 .wt/logs/<task>/<phase>/

---

## Phase 7: 完善命令 ✅

### 7.1 prev 命令 ✅

已完成：
- [x] 使用配置中的 phase sequence
- [x] 实现 on_exit workflow 执行
- [x] 处理资源清理（回退到 pending 时）

### 7.2 stop 命令 ✅

已完成：
- [x] 记录停止事件到日志
- [x] 更新提示信息显示当前 phase
- [x] 清理遗留 wt run 引用

### 7.3 step 命令 ✅

已完成：
- [x] 记录 step 操作到 step-actions.log
- [x] 保存消息到 last-step-result.txt 供步骤间通信
- [x] 支持 WT_PHASE、WT_STEP 环境变量

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
| P1 | 6.3 | 有输出反馈 | ✅ |
| P1 | 7.1-7.3 | 完整的阶段控制 | ✅ |
| P2 | 8.x | 更好的用户体验 | 待做 |
| P3 | 9.x | 高级功能 | 待做 |

---

## Session 规划

| Session | 目标 | 状态 |
|---------|------|------|
| 34 | Phase 6.2a/b - next 连接执行引擎 + agent 启动 | ✅ |
| 35 | Phase 6.2c + 6.3 + Phase 7 - Observer 集成 + 命令完善 | ✅ |
| 36 | Phase 8.x - TUI 更新 | 下一步 |
| 37 | Phase 9.x - 高级功能 | 待做 |
