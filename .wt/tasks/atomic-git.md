---
name: atomic-git
depends:
  - config-hooks
---

# 任务：实现 Git 原子操作

## 目标

将 Git 操作封装为可在 hooks 中调用的原子操作。

## 具体工作

### 1. 扩展 `src/services/git.rs`

现有函数需要重构为更通用的形式，并添加新操作：

```rust
// 现有操作（可能需要调整签名）
pub fn create_branch(repo_root: &str, branch_name: &str) -> Result<()>;
pub fn delete_branch(repo_root: &str, branch_name: &str) -> Result<()>;
pub fn create_worktree(repo_root: &str, path: &str, branch: &str) -> Result<()>;
pub fn delete_worktree(repo_root: &str, path: &str) -> Result<()>;

// 新增操作
pub fn fetch(repo_root: &str, remote: &str) -> Result<()>;
pub fn rebase(worktree_path: &str, target: &str) -> Result<RebaseResult>;
pub fn squash_merge(repo_root: &str, branch: &str) -> Result<()>;
pub fn commit(repo_root: &str, message: &str) -> Result<()>;
pub fn push(repo_root: &str, branch: &str, remote: &str) -> Result<()>;
pub fn has_changes(path: &str) -> Result<bool>;
pub fn has_conflicts(path: &str) -> Result<bool>;
pub fn stash(path: &str) -> Result<()>;
pub fn stash_pop(path: &str) -> Result<()>;
```

### 2. 创建内部子命令 `src/commands/internal/git.rs`

```rust
pub fn execute(operation: &str, args: Vec<String>) -> Result<()> {
    match operation {
        "create-branch" => git::create_branch(&args[0], &args[1]),
        "delete-branch" => git::delete_branch(&args[0], &args[1]),
        "rebase" => git::rebase(&args[0], &args[1]).map(|_| ()),
        "squash-merge" => git::squash_merge(&args[0], &args[1]),
        "commit" => git::commit(&args[0], &args[1]),
        "push" => git::push(&args[0], &args[1], &args.get(2).map(|s| s.as_str()).unwrap_or("origin")),
        "has-changes" => {
            if git::has_changes(&args[0])? {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        },
        "has-conflicts" => {
            if git::has_conflicts(&args[0])? {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        },
        _ => Err(WtError::InvalidInput(format!("Unknown git operation: {}", operation))),
    }
}
```

### 3. CLI 注册

在 `src/cli.rs` 添加：

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ...

    /// Internal commands for hooks (not for direct use)
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        command: InternalCommands,
    },
}

#[derive(Subcommand)]
pub enum InternalCommands {
    #[command(name = "git:create-branch")]
    GitCreateBranch { repo: String, branch: String },
    // ...
}
```

### 4. 用户调用方式

在 hooks 中：
```bash
wt internal git:rebase ${worktree} origin/main
wt internal git:squash-merge ${repo_root} ${branch}
wt internal git:has-changes ${worktree} && echo "has changes"
```

## 测试

- 测试每个 git 操作的成功/失败场景
- 测试内部命令 CLI 解析

## 完成标准

- [ ] 所有 git 原子操作函数实现
- [ ] 内部子命令注册
- [ ] 错误处理完善
- [ ] 测试通过
