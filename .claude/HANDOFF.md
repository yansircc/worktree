# Handoff 文档 - wt 开发进度

## Session 22 完成的工作 (2026-02-03)

### Agent Hooks 系统重设计

通过深度访谈 (`/interview`) 和 Codex 辩论 (`fuckme`) 完成了 Agent Hooks 系统的设计：

1. **需求访谈** - 确定核心设计决策
   - 配置格式：JSONC (`.wt/config.jsonc`)
   - 设计原则：命令 = Hook，全部 Hooks 化
   - Step 类型：script, agent, internal, condition
   - 多 Agent：Pipeline 模式（stream-json 串联）

2. **状态模型演变**
   - v2: Active/Paused/Completed（去掉 Pending）
   - v3: 恢复 Pending，状态+阶段分离
   - v4: 完善异常处理，增加 active_since 监控

3. **Codex 辩论验证**
   - 第一轮：发现 Pending 不能去掉，Paused 定义矛盾
   - 第二轮：明确状态管理、异常处理、delete 行为

4. **实施计划制定**
   - 策略：模块级 Greenfield
   - 4 个 Phase：基础设施 → Hooks 引擎 → 命令重写 → 清理

### 产出文件

- `.claude/specs/agent-hooks.md` (v4) - 完整规格和实施计划

---

## 项目状态

### 任务完成情况

```
Phase 1-4 (旧 hooks):     ✅ 完成（将被重构）
cleanup-legacy:           ✅ 完成
Agent Hooks 设计:         ✅ 完成
Agent Hooks 实现:         ⏳ 待开始
```

### 下一步工作

**Agent Hooks 重构** - 按照 `.claude/specs/agent-hooks.md` 实施：

1. Phase 1: 基础设施
   - JSONC 配置解析
   - 新状态模型

2. Phase 2: Hooks 引擎
   - Step 执行器
   - Pipeline 执行器

3. Phase 3: 命令重写

4. Phase 4: 清理旧代码

---

## 文件索引

### 新系统设计（待实现）

| 文件 | 说明 |
|------|------|
| `.claude/specs/agent-hooks.md` | **核心规格文档**，包含完整设计和实施计划 |

### 可复用的现有代码

| 文件 | 说明 |
|------|------|
| `src/services/git.rs` | Git 原子操作 |
| `src/services/multiplexer/` | tmux/zellij 后端 |
| `src/services/command.rs` | 命令执行辅助 |
| `src/commands/internal/` | 内部原子操作 CLI |
| `src/error.rs` | 错误类型 |

### 将被删除的旧代码

| 文件 | 说明 |
|------|------|
| `src/models/config.rs` | 旧 YAML 配置解析 |
| `src/models/status.rs` | 旧状态模型 |
| `src/models/hook_context.rs` | 旧变量展开 |
| `src/services/hooks.rs` | 旧 hooks 引擎 |

---

## 关键上下文

### Pipeline 机制

来源：WebSearch "Claude Code CLI print mode piping stdin stream-json"

Claude Code 支持 stream-json chaining：
```bash
claude -p --output-format stream-json "prompt1" | \
claude -p --input-format stream-json --output-format stream-json "prompt2" | \
claude -p --input-format stream-json "prompt3"
```

参考：https://github.com/ruvnet/claude-flow/wiki/Stream-Chaining

### 状态模型最终设计

```
Pending → Active ⇄ Idle → Completed
             │       │
             └───────┴── phase (developing/reviewing/merging)
```

- **Pending** = 未创建资源
- **Active** = 有进程在运行（任何进程）
- **Idle** = 资源存在但无进程
- **Completed** = 完成

### 用户工作流

多 agent 协作模式：
1. Agent 开发 → 2. Agent 核对 → 3. Agent 合并 → 4. Agent 审核（可选）

---

## 历史 Session 摘要

| Session | 主要工作 |
|---------|----------|
| 22 | Agent Hooks 系统设计（访谈 + Codex 辩论） |
| 21 | cleanup-legacy + Phase 4 docs 完成 |
| 20 | Phase 3 完成、修复 zellij 合并问题 |
| 19 | 补全 atomic-misc CLI 子命令、创建 hooks.rs |
| 18 | Hooks 系统设计、任务规划、Phase 1-2 服务层 |
| 17 | 代码清理、Zellij 后端改进 |
| 16 | 任务状态重设计、Zellij layout 方案 |
| 15 | Multiplexer 抽象层 |
| 1-14 | 初始实现、TUI、tail/logs、task index、completions 等 |
