---
name: hooks-engine
depends:
  - config-hooks
---

# 任务：实现 Hooks 执行引擎

## 目标

实现 hooks 脚本的执行引擎，支持变量注入、错误处理、日志记录。

## 具体工作

### 1. 创建 `src/services/hooks.rs`

```rust
use crate::models::{HookContext, WtConfig};
use crate::error::{Result, WtError};

pub struct HooksEngine {
    config: WtConfig,
}

impl HooksEngine {
    pub fn new(config: WtConfig) -> Self {
        Self { config }
    }

    /// 执行指定的 hook
    pub fn run_hook(&self, hook_name: &str, context: &HookContext) -> Result<()> {
        let script = match self.config.get_hook(hook_name) {
            Some(s) => s,
            None => return Ok(()), // 没有定义 hook，跳过
        };

        let expanded = context.expand_variables(&script);
        self.execute_script(&expanded, context)
    }

    /// 执行脚本
    fn execute_script(&self, script: &str, context: &HookContext) -> Result<()> {
        // 1. 设置环境变量
        // 2. 在正确的目录执行
        // 3. 捕获输出
        // 4. 处理错误
    }
}
```

### 2. Hook 执行流程

1. 获取 hook 脚本内容
2. 展开变量 (`${task}`, `${branch}`, etc.)
3. 设置环境变量 (供原子操作使用)
4. 在 `repo_root` 目录执行脚本
5. 实时输出 stdout/stderr
6. 检查退出码，非零则返回错误

### 3. 错误处理

```rust
#[derive(Error, Debug)]
pub enum WtError {
    // ... 现有错误 ...

    #[error("Hook '{hook}' failed: {message}")]
    HookFailed {
        hook: String,
        message: String,
        exit_code: Option<i32>,
    },
}
```

### 4. 集成到命令中

提供便捷方法：

```rust
impl HooksEngine {
    pub fn before_review(&self, context: &HookContext) -> Result<()> {
        self.run_hook("before_review", context)
    }

    pub fn after_complete(&self, context: &HookContext) -> Result<()> {
        self.run_hook("after_complete", context)
    }
    // ...
}
```

## 测试

- 测试变量展开正确
- 测试脚本执行成功/失败
- 测试无 hook 时跳过
- 测试错误信息清晰

## 完成标准

- [ ] `HooksEngine` 结构实现
- [ ] 变量注入正确工作
- [ ] 脚本执行并捕获输出
- [ ] 错误处理完善
- [ ] 单元测试通过
