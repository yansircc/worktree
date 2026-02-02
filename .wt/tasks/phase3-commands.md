---
name: phase3-commands
depends:
  - phase2-hooks-engine
---

# Phase 3: 命令重写

将所有命令重写为 hook 触发器。

## 目标

命令层变得非常薄：
1. 解析参数
2. 检查前置条件（状态、依赖）
3. 触发对应的 hook
4. 处理结果

## 任务清单

### 1. 重写现有命令

- [ ] **run.rs** - 触发 hooks.run
  ```rust
  // 前置条件：状态为 Pending，依赖都 Completed
  // 触发 hook 后：Active + developing
  ```

- [ ] **review.rs** - 触发 hooks.review
  ```rust
  // 前置条件：状态为 Active 或 Idle，phase 为 developing
  // 触发 hook 后：Idle + reviewing (或 Active 如果 pipeline 在跑)
  ```

- [ ] **resume.rs** - 触发 hooks.resume
  ```rust
  // 前置条件：状态为 Idle
  // 触发 hook 后：Active + (保持 phase)
  ```

- [ ] **complete.rs** - 触发 hooks.complete
  ```rust
  // 前置条件：状态为 Idle，phase 为 reviewing
  // 触发 hook 后：Completed (或 Idle + conflict/error)
  ```

- [ ] **delete.rs** - 触发 hooks.delete
  ```rust
  // 触发 hook 后：从 status.json 移除记录
  ```

- [ ] **reset.rs** - 触发 hooks.reset
  ```rust
  // 触发 hook 后：Pending + phase None
  ```

### 2. 新增命令

- [ ] **hooks.rs** - 手动触发 hook（调试用）
  ```bash
  wt hooks run <hook_name> [--task <task>]
  ```

- [ ] **pause.rs** - 暂停任务
  ```bash
  wt pause <task> [--reason <reason>]
  # Active → Idle + manual
  ```

### 3. 更新 CLI 定义

- [ ] 在 `src/cli.rs` 中添加新命令
- [ ] 更新 `src/main.rs` 路由

### 4. 更新状态显示

- [ ] `wt status` 显示新的状态模型
- [ ] `wt status --verbose` 显示 status + phase + idle_reason + active_since
- [ ] `wt list` 适配新状态

## 验收标准

- [ ] 所有命令通过 hooks 执行
- [ ] 状态转换正确
- [ ] `wt hooks run` 能手动触发 hook
- [ ] `wt pause` 能暂停任务
- [ ] `wt status` 正确显示新状态
- [ ] 错误处理正确（Idle + error/conflict）
- [ ] `cargo test` 通过

## 参考

- 规格文档：`.claude/specs/agent-hooks.md`
- 默认状态转换表见规格文档 "状态管理" 部分
