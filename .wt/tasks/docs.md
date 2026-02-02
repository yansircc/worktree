---
name: docs
depends:
  - cmd-run
  - cmd-lifecycle
  - cmd-complete
  - cmd-delete
---

# 任务：更新文档

## 目标

更新所有文档，反映 hooks 系统的变化。

## 具体工作

### 1. 更新 README.md

- 新的命令名称（run, complete, delete）
- Hooks 配置说明
- 原子操作列表
- 变量列表
- 迁移指南

### 2. 更新 .claude/CLAUDE.md

- 新的目录结构（新增 services/hooks.rs, services/files.rs 等）
- 新的配置格式
- 命令变化说明

### 3. 创建 .wt/prompts/README.md

说明 prompts 目录的用途（如果保留 merge.md）

### 4. 更新 init 模板

修改 `src/commands/init.rs` 中的默认配置模板：

```rust
fn generate_config(project_name: &str) -> String {
    format!(r#"
# Hooks 配置
hooks:
  # 创建 worktree 后执行
  # on_create: |
  #   npm install

  # 进入 Review 前执行
  # before_review: |
  #   npm run lint
  #   npm run test

  # 完成前执行
  # before_complete: |
  #   npm run build

  # 删除前执行
  # before_delete: |
  #   rm -rf node_modules/
"#)
}
```

### 5. 迁移指南

在 README 或单独文档中说明：

```markdown
## 从旧版本迁移

### 配置迁移

旧配置字段会自动映射到新的 hooks：
- `init_script` → `hooks.on_create`
- `review_script` → `hooks.before_review`
- `merge_script` → `hooks.before_complete`
- `archive_script` → `hooks.before_delete` 和 `hooks.before_reset`

### 命令变化

| 旧命令 | 新命令 | 说明 |
|--------|--------|------|
| `wt start` | `wt run` | 别名保留 |
| `wt merge` | `wt complete` | 别名保留 |
| `wt archive` | `wt delete` | 别名保留 |
```

### 6. 原子操作文档

创建完整的原子操作参考文档：

```markdown
## 原子操作参考

### Git 操作

| 命令 | 说明 | 示例 |
|------|------|------|
| `wt internal git:rebase` | Rebase 到目标分支 | `wt internal git:rebase ${worktree} origin/main` |
| ... | ... | ... |
```

## 完成标准

- [ ] README.md 更新
- [ ] CLAUDE.md 更新
- [ ] init 模板更新
- [ ] 迁移指南完成
- [ ] 原子操作参考文档
