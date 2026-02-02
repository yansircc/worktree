# Spec: 清理遗留代码和更新文档

## 背景

Phase 3 hooks 系统集成已完成，但项目中仍有大量遗留代码和过时文档需要清理。由于是个人玩具项目，不需要考虑向后兼容。

## 目标

1. 删除废弃的命令别名（start, merge, archive）
2. 删除旧的配置字段（init_script, archive_script, review_script, merge_script）
3. 更新所有用户提示信息使用新命令名
4. 更新配置模板使用新的 hooks 格式
5. 更新所有文档

## 具体任务

### 1. 删除废弃命令

**删除文件：**
- `src/commands/archive.rs`
- `src/commands/merge.rs`

**修改 `src/commands/mod.rs`：**
- 删除 `pub mod archive;`
- 删除 `pub mod merge;`

**修改 `src/cli.rs`：**
- 删除 `Start` 枚举变体（第 52-61 行）
- 删除 `Merge` 枚举变体（第 81-90 行）
- 删除 `Archive` 枚举变体（第 102-107 行）

**修改 `src/main.rs`：**
- 删除 `Commands::Start` 的 match 分支
- 删除 `Commands::Merge` 的 match 分支
- 删除 `Commands::Archive` 的 match 分支

### 2. 删除旧配置字段

**修改 `src/models/config.rs`：**
- 删除 `WtConfig` 中的字段：
  - `init_script: Option<String>`
  - `archive_script: Option<String>`
  - `review_script: Option<String>`
  - `merge_script: Option<String>`
- 简化 `get_hook()` 方法，删除 legacy fallback 逻辑
- 删除/更新相关测试

**修改 `src/services/config_ops.rs`：**
- 删除对旧字段的 get 支持
- 更新错误信息中的 valid keys 列表

**修改 `src/services/hooks.rs`：**
- 删除测试 legacy fallback 的测试用例

### 3. 更新用户提示信息

| 文件 | 行号 | 当前 | 改为 |
|------|------|------|------|
| `src/commands/review.rs` | 66 | `wt merge {}` | `wt complete {}` |
| `src/commands/delete.rs` | 62 | `wt merge {}` | `wt complete {}` |
| `src/commands/delete.rs` | 45 | `wt start {}` | `wt run {}` |
| `src/commands/resume.rs` | 18 | `wt start {}` | `wt run {}` |
| `src/commands/init.rs` | 238 | `wt start <task>` | `wt run <task>` |

### 4. 更新配置模板 (init.rs)

将 `generate_config()` 函数中的配置模板从旧格式：
```yaml
init_script: |
  npm install
archive_script: |
  rm -rf node_modules/
review_script: |
  npm run lint
merge_script: |
  npm run build
```

更新为新格式：
```yaml
hooks:
  on_create: |
    npm install
  before_review: |
    npm run lint
    npm run test
  before_complete: |
    npm run build
  after_complete: |
    echo "Task ${task} completed!"
  before_delete: |
    rm -rf node_modules/
  before_reset: |
    rm -rf node_modules/
```

### 5. 更新文档

**删除：**
- `.wt/prompts/merge.md` - 已被 complete 命令替代

**更新 `README.md`：**
- 命令从 `wt start` → `wt run`
- 命令从 `wt merge` → `wt complete`
- 配置示例使用 `hooks:` 格式
- 删除 `--agent` 参数说明

**更新 `.claude/CLAUDE.md`：**
- 命令列表使用新名称
- 配置示例使用 `hooks:` 格式
- 状态流转图更新命令名

**更新 `.claude/rules/testing.md`：**
- 测试命令使用新名称

**更新 `.claude/skills/wt/SKILL.md`：**
- 命令使用新名称

**更新 `.claude/skills/wt/workflows/plan.md`：**
- 命令使用新名称

## 验证

- [ ] `cargo build` 无错误
- [ ] `cargo test` 全部通过
- [ ] `wt run --help` 显示正确
- [ ] `wt complete --help` 显示正确
- [ ] `wt start` 报错（命令不存在）
- [ ] `wt merge` 报错（命令不存在）
- [ ] `wt init` 生成新格式配置

## 完成标准

- 所有废弃命令已删除
- 所有旧配置字段已删除
- 所有文档使用新命令名和新配置格式
- 测试全部通过
