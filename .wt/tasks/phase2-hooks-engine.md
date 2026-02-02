---
name: phase2-hooks-engine
depends:
  - phase1-foundation
---

# Phase 2: Hooks 引擎

实现 Step 执行器、Pipeline 执行器和 Hooks 引擎。

## 目标

1. **Step 执行器** (`src/services/hooks/step.rs`)
2. **Pipeline 执行器** (`src/services/hooks/pipeline.rs`)
3. **Hooks 引擎** (`src/services/hooks/engine.rs`)

## 任务清单

### 1. Step 执行器

实现四种 step 类型的执行：

- [ ] **script step**: 执行 shell 脚本
  ```rust
  fn execute_script(run: &str, on_error: Option<&Step>, ctx: &HookContext) -> Result<()>
  ```

- [ ] **agent step**: 构建并执行 claude 命令
  ```rust
  fn execute_agent(agent: &AgentStep, ctx: &HookContext) -> Result<()>
  // 需要处理 interactive vs non-interactive (-p) 模式
  // interactive: 在 multiplexer window 中运行
  // non-interactive: 直接执行，输出到日志
  ```

- [ ] **internal step**: 调用 wt internal 命令
  ```rust
  fn execute_internal(run: &str, on_conflict: Option<&Step>, ctx: &HookContext) -> Result<()>
  ```

- [ ] **condition step**: 条件判断
  ```rust
  fn execute_condition(if_: &str, then: &Step, else_: Option<&Step>, ctx: &HookContext) -> Result<()>
  // if_ 命令 exit 0 = true, 否则 false
  ```

### 2. Pipeline 执行器

实现多 agent stream-json 串联：

- [ ] 构建管道命令：
  ```bash
  claude -p --output-format stream-json "prompt1" | \
  claude -p --input-format stream-json --output-format stream-json "prompt2" | \
  claude -p --input-format stream-json "prompt3"
  ```

- [ ] 进程管理（spawn pipeline）
- [ ] 错误传播（任一 agent 失败则整个 pipeline 失败）
- [ ] 输出日志收集

### 3. Hooks 引擎

- [ ] 加载 hook 配置（从 ConfigV2）
- [ ] 变量展开：
  - `${task}` - 任务名
  - `${branch}` - 分支名
  - `${worktree}` - worktree 路径
  - `${session}` - multiplexer session
  - `${window}` - multiplexer window
  - `${repo_root}` - 仓库根目录
  - `${phase}` - 当前阶段

- [ ] 执行 steps 数组（顺序执行）
- [ ] 执行 pipeline（多 agent 串联）
- [ ] 状态自动管理：
  - hook 开始前设置 Active
  - hook 成功后设置 Idle + done
  - hook 失败后设置 Idle + error/conflict

### 4. 目录结构

```
src/services/hooks/
├── mod.rs         # 模块导出
├── step.rs        # Step 执行器
├── pipeline.rs    # Pipeline 执行器
├── engine.rs      # Hooks 引擎
└── context.rs     # HookContext (变量展开)
```

## 验收标准

- [ ] script step 能执行 shell 命令
- [ ] agent step 能启动 claude（交互/非交互）
- [ ] internal step 能调用 wt internal
- [ ] condition step 能正确判断分支
- [ ] pipeline 能串联多个 agent
- [ ] 变量展开正确
- [ ] 状态自动管理正确
- [ ] `cargo test` 通过

## 参考

- 规格文档：`.claude/specs/agent-hooks.md`
- Pipeline 机制：https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining
- Claude Code flags: `--output-format stream-json`, `--input-format stream-json`
