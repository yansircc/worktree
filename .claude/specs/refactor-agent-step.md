# Refactor Spec: Agent Step 结构重构

## 背景

Session 26 快速迭代后，`Step::Agent` 变体膨胀到 35+ 个字段，导致：
- 代码冗长难维护
- `builtin_pipeline()` 有 200+ 行重复代码
- 测试辅助函数 `make_agent_step()` 冗长
- `step.rs` 和 `pipeline.rs` 命令构建逻辑重复

## 目标

1. 简化 `Step::Agent` 的构造和使用
2. 统一 Claude CLI 命令构建逻辑
3. 减少重复代码
4. 保持 serde 兼容性（配置文件格式不变）

## 方案

### Phase 1: 拆分 AgentStep 结构体

#### 1.1 创建 `AgentStep` 结构体

```rust
// src/models/agent_step.rs (新文件)

use serde::{Deserialize, Serialize};

/// Agent step configuration - all fields have sensible defaults
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStep {
    // === Required ===
    pub prompt: String,

    // === Basic ===
    #[serde(default)]
    pub print: bool,
    #[serde(default = "default_model")]
    pub model: String,

    // === System Prompt ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_system_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_system_prompt_file: Option<String>,

    // === Tools ===
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,

    // === Permissions ===
    #[serde(default)]
    pub skip_permissions: bool,
    #[serde(default)]
    pub allow_skip_permissions: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_prompt_tool: Option<String>,

    // === Limits ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_budget_usd: Option<f64>,

    // === Session ===
    #[serde(default, rename = "continue")]
    pub continue_session: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub fork_session: bool,
    #[serde(default)]
    pub no_session_persistence: bool,

    // === I/O ===
    #[serde(default = "default_output_format")]
    pub output_format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_format: Option<String>,
    #[serde(default)]
    pub include_partial_messages: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<String>,

    // === Model ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<String>,

    // === Subagents ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    // === Other ===
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_dir: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_config: Option<String>,
    #[serde(default)]
    pub strict_mcp_config: bool,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_dir: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub betas: Vec<String>,

    // === Browser/IDE ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome: Option<bool>,
    #[serde(default)]
    pub ide: bool,
    #[serde(default)]
    pub disable_slash_commands: bool,

    // === Window (for REPL mode) ===
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
}

fn default_model() -> String {
    "sonnet".to_string()
}

fn default_output_format() -> String {
    "text".to_string()
}

impl Default for AgentStep {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            print: false,
            model: default_model(),
            // 其他所有字段都是 Option/Vec/bool 的默认值
            system_prompt: None,
            system_prompt_file: None,
            // ... 省略
        }
    }
}

impl AgentStep {
    /// Create a new agent step with just a prompt
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Builder-style setter for model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Builder-style setter for print mode
    pub fn with_print(mut self) -> Self {
        self.print = true;
        self
    }

    /// Builder-style setter for max_turns
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// Builder-style setter for tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    // ... 其他常用 builder 方法
}
```

#### 1.2 修改 Step 枚举

```rust
// src/models/config.rs

pub enum Step {
    Script {
        run: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_error: Option<Box<Step>>,
    },

    Agent(AgentStep),  // 改为包装结构体

    Internal {
        run: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_conflict: Option<Box<Step>>,
    },

    Condition {
        #[serde(rename = "if")]
        if_: String,
        then: Box<Step>,
        #[serde(rename = "else", default, skip_serializing_if = "Option::is_none")]
        else_: Option<Box<Step>>,
    },
}
```

**注意**：serde `#[serde(untagged)]` 可能需要调整，需要测试 JSON 解析是否仍然正确。

### Phase 2: ClaudeCommandBuilder

```rust
// src/services/claude.rs (新文件)

use crate::models::AgentStep;
use crate::services::hooks::ExecutionContext;

/// Builder for Claude CLI command arguments
pub struct ClaudeCommandBuilder {
    args: Vec<String>,
}

impl ClaudeCommandBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// Build from AgentStep with context for variable expansion
    pub fn from_agent_step(step: &AgentStep, ctx: &ExecutionContext) -> Self {
        let mut builder = Self::new();

        // Print mode
        if step.print {
            builder = builder.print();
        }

        // Model
        builder = builder.model(&step.model);

        // System prompt
        if let Some(sp) = &step.system_prompt {
            builder = builder.system_prompt(&ctx.expand(sp));
        }
        if let Some(spf) = &step.system_prompt_file {
            builder = builder.system_prompt_file(&ctx.expand(spf));
        }
        // ... 其他字段

        builder
    }

    pub fn print(mut self) -> Self {
        self.args.push("-p".to_string());
        self
    }

    pub fn model(mut self, m: &str) -> Self {
        let model = match m {
            "haiku" => "claude-haiku-4-20250514",
            "sonnet" => "claude-sonnet-4-20250514",
            "opus" => "claude-opus-4-20250514",
            other => other,
        };
        self.args.extend(["--model".into(), model.into()]);
        self
    }

    pub fn system_prompt(mut self, sp: &str) -> Self {
        self.args.extend(["--system-prompt".into(), sp.into()]);
        self
    }

    pub fn system_prompt_file(mut self, path: &str) -> Self {
        self.args.extend(["--system-prompt-file".into(), path.into()]);
        self
    }

    pub fn max_turns(mut self, turns: u32) -> Self {
        self.args.extend(["--max-turns".into(), turns.to_string()]);
        self
    }

    pub fn output_format(mut self, fmt: &str) -> Self {
        self.args.extend(["--output-format".into(), fmt.into()]);
        self
    }

    pub fn input_format(mut self, fmt: &str) -> Self {
        self.args.extend(["--input-format".into(), fmt.into()]);
        self
    }

    pub fn include_partial_messages(mut self) -> Self {
        self.args.push("--include-partial-messages".into());
        self
    }

    pub fn skip_permissions(mut self) -> Self {
        self.args.push("--dangerously-skip-permissions".into());
        self
    }

    pub fn verbose(mut self) -> Self {
        self.args.push("--verbose".into());
        self
    }

    pub fn prompt(mut self, p: &str) -> Self {
        self.args.push(shell_escape(p));
        self
    }

    pub fn build(self) -> Vec<String> {
        self.args
    }

    pub fn build_command_string(self, claude_cmd: &str) -> String {
        format!("{} {}", claude_cmd, self.args.join(" "))
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
```

### Phase 3: 简化 builtin_pipeline()

```rust
// src/models/config.rs

fn builtin_pipeline(name: &str) -> Option<Vec<Step>> {
    match name {
        "code-review" => Some(vec![
            Step::Agent(
                AgentStep::new("Quick lint check for task ${task}. Report any obvious issues.")
                    .with_model("haiku")
                    .with_print()
                    .with_max_turns(5)
                    .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                    .with_no_session_persistence()
            ),
            Step::Agent(
                AgentStep::new("Deep code review for task ${task}. Check for bugs, security issues, and suggest improvements.")
                    .with_model("sonnet")
                    .with_print()
                    .with_max_turns(10)
                    .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                    .with_no_session_persistence()
            ),
        ]),
        "merge" => Some(vec![
            Step::Agent(
                AgentStep::new("Merge task ${task}. Rebase ${branch} onto main, resolve conflicts if any, then squash merge.")
                    .with_model("sonnet")
                    .with_print()
                    .with_max_turns(20)
                    .with_tools(vec!["Bash".into(), "Read".into(), "Edit".into()])
                    .with_allowed_tools(vec!["Bash(git *)".into()])
                    .with_append_system_prompt("You are a git expert. Steps: 1) git fetch origin, 2) git rebase origin/main, 3) resolve conflicts if any, 4) git checkout main, 5) git merge --squash ${branch}, 6) git commit. Report any issues.")
                    .with_no_session_persistence()
            ),
        ]),
        "refactor" => Some(vec![
            Step::Agent(
                AgentStep::new("Analyze code structure for refactoring task ${task}. Identify patterns and issues.")
                    .with_model("haiku")
                    .with_print()
                    .with_max_turns(5)
                    .with_tools(vec!["Read".into(), "Grep".into(), "Glob".into()])
                    .with_no_session_persistence()
            ),
            Step::Agent(
                AgentStep::new("Apply refactoring based on the analysis. Make changes incrementally and verify each step.")
                    .with_model("sonnet")
                    .with_print()
                    .with_max_turns(20)
                    .with_tools(vec!["Read".into(), "Edit".into(), "Bash".into()])
                    .with_no_session_persistence()
            ),
        ]),
        _ => None,
    }
}
```

**对比**：从 ~200 行 → ~50 行

### Phase 4: 简化 step.rs 和 pipeline.rs

```rust
// src/services/hooks/step.rs

fn execute_agent(&self, step: &AgentStep) -> Result<StepResult> {
    let builder = ClaudeCommandBuilder::from_agent_step(step, self.context);

    if step.print {
        // Non-interactive: add prompt and execute
        let args = builder.prompt(&self.context.expand(&step.prompt)).build();
        self.execute_command(&args)
    } else {
        // REPL mode: send to multiplexer window
        let args = builder.build();
        self.execute_agent_in_window(&args, &step.prompt, step.window.as_deref())
    }
}
```

```rust
// src/services/hooks/pipeline.rs

fn build_pipeline_command(&self, agents: &[&Step]) -> String {
    let mut parts = Vec::new();
    let total = agents.len();

    for (i, step) in agents.iter().enumerate() {
        if let Step::Agent(agent_step) = step {
            let mut builder = ClaudeCommandBuilder::from_agent_step(agent_step, self.context);

            // Pipeline-specific: auto stream-json
            if i > 0 {
                builder = builder.input_format("stream-json");
            }
            if i < total - 1 {
                builder = builder.output_format("stream-json");
                builder = builder.include_partial_messages();
            }

            let cmd = builder
                .prompt(&self.context.expand(&agent_step.prompt))
                .build_command_string(&self.config.claude_command);

            parts.push(cmd);
        }
    }

    parts.join(" | ")
}
```

### Phase 5: PipelineStore（可选）

```rust
// src/services/hooks/pipeline_store.rs

pub struct PipelineStore {
    dir: PathBuf,
}

impl PipelineStore {
    pub fn new(repo_root: &str) -> Self {
        Self {
            dir: PathBuf::from(repo_root).join(".wt/pipelines"),
        }
    }

    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir).map_err(|e| WtError::Io {
            operation: "create_dir".into(),
            path: self.dir.display().to_string(),
            message: e.to_string(),
        })
    }

    pub fn list(&self) -> Result<Vec<PipelineStatus>> { ... }
    pub fn get(&self, id: &str) -> Result<Option<PipelineStatus>> { ... }
    pub fn save(&self, status: &PipelineStatus) -> Result<()> { ... }
    pub fn delete(&self, id: &str) -> Result<()> { ... }
    pub fn cleanup(&self, max_age: Duration) -> Result<usize> { ... }
}
```

## 实施步骤

### Step 1: 创建 AgentStep 结构体
- [ ] 创建 `src/models/agent_step.rs`
- [ ] 实现 Default 和 builder 方法
- [ ] 添加单元测试

### Step 2: 修改 Step 枚举
- [ ] 将 `Step::Agent { ... }` 改为 `Step::Agent(AgentStep)`
- [ ] **关键**：验证 serde 序列化/反序列化不变
- [ ] 更新 config.rs 测试

### Step 3: 创建 ClaudeCommandBuilder
- [ ] 创建 `src/services/claude.rs`
- [ ] 实现 `from_agent_step()` 方法
- [ ] 添加单元测试

### Step 4: 重构 step.rs
- [ ] 使用 ClaudeCommandBuilder
- [ ] 简化 `execute_agent()`
- [ ] 验证测试通过

### Step 5: 重构 pipeline.rs
- [ ] 使用 ClaudeCommandBuilder
- [ ] 简化 `build_pipeline_command()`
- [ ] 验证测试通过

### Step 6: 简化 builtin_pipeline()
- [ ] 使用 AgentStep builder
- [ ] 从 ~200 行减少到 ~50 行

### Step 7: 清理测试辅助
- [ ] 删除 `make_agent_step()`
- [ ] 直接使用 `AgentStep::new().with_*()`

### Step 8: 可选 - PipelineStore
- [ ] 提取 pipeline 状态管理逻辑
- [ ] 简化 `list_pipelines`, `kill_pipeline`, `cleanup_pipelines`

## 验收标准

1. **所有测试通过** - `cargo test` 无失败
2. **配置兼容** - 现有 `.wt/config.jsonc` 文件无需修改
3. **代码行数减少** - `builtin_pipeline()` 从 ~200 行降到 ~50 行
4. **无 dead code warnings** - `cargo build` 无警告
5. **step.rs 和 pipeline.rs 共享命令构建** - 通过 ClaudeCommandBuilder

## 风险

1. **serde 兼容性** - `Step::Agent(AgentStep)` vs `Step::Agent { ... }` 可能影响 JSON 解析
   - 缓解：先写测试验证

2. **重构范围蔓延** - 可能发现更多需要改的地方
   - 缓解：严格按 Phase 执行，每个 Phase 完成后提交

## 预估工作量

- Phase 1-2 (AgentStep + Step): 1 小时
- Phase 3 (ClaudeCommandBuilder): 30 分钟
- Phase 4-5 (step.rs + pipeline.rs): 30 分钟
- Phase 6 (builtin_pipeline): 15 分钟
- Phase 7 (测试清理): 15 分钟
- Phase 8 (PipelineStore): 30 分钟（可选）

**总计**: ~3 小时（不含 Phase 8）
