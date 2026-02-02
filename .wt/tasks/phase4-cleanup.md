---
name: phase4-cleanup
depends:
  - phase3-commands
---

# Phase 4: 清理

删除旧代码，更新文档。

## 目标

1. 删除所有旧代码
2. 重命名 v2 文件
3. 更新文档

## 任务清单

### 1. 删除旧代码

- [ ] 删除 `src/models/config.rs` (旧 YAML 配置)
- [ ] 删除 `src/models/status.rs` (旧状态模型)
- [ ] 删除 `src/models/hook_context.rs` (旧变量展开)
- [ ] 删除 `src/services/hooks.rs` (旧 hooks 引擎)
- [ ] 删除 `docs/claude-code-hooks.md` (旧文档)

### 2. 重命名文件

- [ ] `src/models/config_v2.rs` → `src/models/config.rs`
- [ ] `src/models/status_v2.rs` → `src/models/status.rs`
- [ ] 更新所有 `mod.rs` 导出
- [ ] 更新所有 `use` 语句

### 3. 更新文档

- [ ] **README.md**
  - 更新配置格式（YAML → JSONC）
  - 更新状态说明（新状态模型）
  - 更新命令说明
  - 添加 Pipeline 说明

- [ ] **.claude/CLAUDE.md**
  - 更新目录结构
  - 更新配置格式
  - 更新核心概念

- [ ] **.claude/HANDOFF.md**
  - 记录重构完成

### 4. 最终验证

- [ ] `cargo build --release` 成功
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无警告
- [ ] 手动测试核心流程：
  - `wt init` 生成新格式配置
  - `wt run` 执行 hooks
  - `wt review` 执行 hooks
  - `wt complete` 执行 hooks
  - `wt status` 显示正确

## 验收标准

- [ ] 无旧代码残留
- [ ] 文档与代码一致
- [ ] 所有测试通过
- [ ] 手动测试通过

## 参考

- 规格文档：`.claude/specs/agent-hooks.md`
- 可复用代码列表见规格文档 "实施计划" 部分
