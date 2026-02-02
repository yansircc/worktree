---
name: phase1-foundation
depends: []
---

# Phase 1: 基础设施

实现 JSONC 配置解析和新状态模型。

## 目标

1. **JSONC 配置解析** (`src/models/config_v2.rs`)
2. **新状态模型** (`src/models/status_v2.rs`)

## 任务清单

### 1. JSONC 配置解析

- [ ] 添加 `json_comments` 或 `jsonc-parser` crate 到 Cargo.toml
- [ ] 定义新的配置结构体：
  ```rust
  pub struct ConfigV2 {
      pub multiplexer: String,
      pub session_name: String,
      pub claude_command: Option<String>,
      pub worktree_dir: Option<String>,
      pub hooks: HooksConfig,
  }

  pub struct HooksConfig {
      pub run: Option<Vec<Step>>,
      pub review: Option<Vec<Step>>,
      pub resume: Option<Vec<Step>>,
      pub complete: Option<Vec<Step>>,
      pub delete: Option<Vec<Step>>,
      pub reset: Option<Vec<Step>>,
  }

  pub enum Step {
      Script { run: String, on_error: Option<Box<Step>> },
      Agent { interactive: bool, model: String, prompt: String, ... },
      Internal { run: String, on_conflict: Option<Box<Step>> },
      Condition { if_: String, then: Box<Step>, else_: Option<Box<Step>> },
  }
  ```
- [ ] 实现 `.wt/config.jsonc` 解析
- [ ] 配置验证和错误提示
- [ ] 单元测试

### 2. 新状态模型

- [ ] 定义新的状态枚举：
  ```rust
  pub enum TaskStatus {
      Pending,
      Active,
      Idle,
      Completed,
  }

  pub enum TaskPhase {
      None,
      Developing,
      Reviewing,
      Merging,
  }

  pub enum IdleReason {
      Done,
      HumanReview,
      Error,
      Conflict,
      Timeout,
      Manual,
  }
  ```
- [ ] 定义状态文件结构：
  ```rust
  pub struct TaskState {
      pub status: TaskStatus,
      pub phase: TaskPhase,
      pub idle_reason: Option<IdleReason>,
      pub active_since: Option<DateTime<Utc>>,
      pub instance: Option<Instance>,
  }
  ```
- [ ] 实现状态文件读写（兼容新格式）
- [ ] 单元测试

## 验收标准

- [ ] `ConfigV2::load()` 能解析 `.wt/config.jsonc`
- [ ] JSONC 注释被正确忽略
- [ ] 配置验证错误有清晰提示
- [ ] 新状态模型能正确序列化/反序列化
- [ ] `cargo test` 通过

## 参考

- 规格文档：`.claude/specs/agent-hooks.md`
- 配置示例见规格文档 "完整示例" 部分
