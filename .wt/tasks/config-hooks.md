---
name: config-hooks
depends: []
---

# 任务：扩展配置支持 Hooks

## 目标

扩展 `WtConfig` 支持新的 hooks 配置格式，同时保持向后兼容。

## 具体工作

### 1. 扩展 `src/models/config.rs`

添加新的配置结构：

```rust
#[derive(Debug, Deserialize, Default)]
pub struct HooksConfig {
    pub on_create: Option<String>,
    pub before_run: Option<String>,
    pub after_run: Option<String>,
    pub before_review: Option<String>,
    pub after_review: Option<String>,
    pub before_resume: Option<String>,
    pub before_complete: Option<String>,
    pub after_complete: Option<String>,
    pub before_delete: Option<String>,
    pub before_reset: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WtConfig {
    // ... 现有字段 ...

    #[serde(default)]
    pub hooks: HooksConfig,
}
```

### 2. 向后兼容

保留旧字段，映射到新的 hooks：
- `init_script` → `hooks.on_create`
- `review_script` → `hooks.before_review`
- `merge_script` → `hooks.before_complete`
- `archive_script` → `hooks.before_delete` 和 `hooks.before_reset`

实现 `WtConfig::get_hook(hook_name)` 方法，优先使用新格式，fallback 到旧格式。

### 3. 变量定义

创建 `src/models/hook_context.rs`：

```rust
pub struct HookContext {
    pub task: String,
    pub branch: String,
    pub worktree: String,
    pub repo_root: String,
    pub session: String,
    pub window: String,
    pub status: String,
    pub prev_status: Option<String>,
}

impl HookContext {
    pub fn expand_variables(&self, script: &str) -> String {
        script
            .replace("${task}", &self.task)
            .replace("${branch}", &self.branch)
            // ... 其他变量
    }
}
```

## 测试

- 测试新格式解析
- 测试旧格式向后兼容
- 测试变量展开

## 完成标准

- [ ] `HooksConfig` 结构定义完成
- [ ] `WtConfig` 支持解析 hooks 配置
- [ ] 向后兼容旧配置字段
- [ ] `HookContext` 变量展开功能
- [ ] 单元测试通过
