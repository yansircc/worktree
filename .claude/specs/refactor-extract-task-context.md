# 重构规格：提取 TaskContext + 拆分大文件

## 背景

Session 28 完成 dead code 清理后，代码库整体健康。但存在以下架构问题：

1. **commands 层重复模式** - 每个命令都重复 load → resolve → validate → execute → save
2. **store.rs 职责过多** - 970 行，混合了解析、验证、排序、状态同步
3. **config.rs 混合职责** - 777 行，包含结构定义和内置 pipeline

## 目标

1. 减少 commands 层 ~200 行重复代码
2. 提高 store.rs 可读性和可测试性
3. 建立更清晰的模块边界

---

## Phase 1: 提取 TaskContext (P1 - 低风险高收益)

### 问题

8+ 个 command 重复以下模式：

```rust
// run.rs, review.rs, resume.rs, pause.rs, complete.rs, delete.rs, reset.rs, tail.rs
let store = TaskStore::load()?;
let name = store.resolve_task_ref(&task_ref)?;
let task = store.get(&name).ok_or_else(|| WtError::TaskNotFound(name.clone()))?;
let state = store.status.get(&name);
// 检查状态...
// 执行操作...
store.status.save()?;
```

### 方案

创建 `services/task_context.rs`：

```rust
/// 任务执行上下文 - 封装 load/resolve/save 流程
pub struct TaskContext {
    store: TaskStore,
    config: WtConfig,
    task_name: String,
}

impl TaskContext {
    /// 加载任务上下文
    pub fn load(task_ref: &str) -> Result<Self> {
        let store = TaskStore::load()?;
        let config = WtConfig::load().unwrap_or_default();
        let task_name = store.resolve_task_ref(task_ref)?;

        if !store.tasks.contains_key(&task_name) {
            return Err(WtError::TaskNotFound(task_name));
        }

        Ok(Self { store, config, task_name })
    }

    /// 获取任务
    pub fn task(&self) -> &Task {
        self.store.tasks.get(&self.task_name).unwrap()
    }

    /// 获取状态
    pub fn state(&self) -> &TaskState {
        self.store.status.get(&self.task_name)
    }

    /// 获取可变状态
    pub fn state_mut(&mut self) -> &mut TaskState {
        self.store.status.get_mut(&self.task_name)
    }

    /// 获取配置
    pub fn config(&self) -> &WtConfig {
        &self.config
    }

    /// 保存状态变更
    pub fn save_status(self) -> Result<()> {
        self.store.status.save()
    }
}
```

### 重构后的 command

```rust
// Before (review.rs)
pub fn execute(task_ref: String) -> Result<()> {
    let store = TaskStore::load()?;
    let name = store.resolve_task_ref(&task_ref)?;
    let task = store.get(&name).ok_or_else(|| WtError::TaskNotFound(name.clone()))?;
    // ... 20 行状态检查和操作
    store.status.save()?;
    Ok(())
}

// After
pub fn execute(task_ref: String) -> Result<()> {
    let mut ctx = TaskContext::load(&task_ref)?;
    // ... 直接操作
    ctx.save_status()
}
```

### 验证

```bash
cargo test --lib
cargo test --test cli
```

---

## Phase 2: 拆分 store.rs (P2 - 中等成本)

### 当前结构 (970 行)

```
TaskStore
├── load() / save()           # 存储
├── parse_task_file()         # 解析
├── parse_frontmatter()       # 解析
├── validate_task_name()      # 验证
├── resolve_task_ref()        # 解析引用
├── list() / sorted_by_dependency() # 排序
├── sync_window_status()      # 状态同步
└── ... 测试 (~400 行)
```

### 目标结构

```
models/
├── store.rs           # TaskStore - 纯存储 (~150 行)
│   ├── load() / save()
│   ├── get() / get_mut()
│   └── list()
│
├── task_parser.rs     # 文件解析 (~200 行)
│   ├── parse_file()
│   ├── parse_frontmatter()
│   └── validate_name()
│
└── task_resolver.rs   # 引用解析 (~100 行)
    ├── resolve_ref()      # "auth" | "1" → "auth"
    └── sorted_by_dependency()
```

### 迁移步骤

1. 创建 `task_parser.rs`，移动解析函数
2. 创建 `task_resolver.rs`，移动解析函数
3. 更新 `store.rs` 调用新模块
4. 更新 `mod.rs` 导出
5. 运行测试

---

## Phase 3: 拆分 config.rs (P3 - 可选)

### 当前结构 (777 行)

- `Step` 枚举 + 序列化
- `HookDef` 枚举
- `WtConfig` 结构体
- `builtin_pipeline()` 函数 (~150 行)
- 默认值函数 (7 个)
- 测试 (~300 行)

### 目标结构

```
models/
├── config.rs              # WtConfig (~200 行)
├── step.rs                # Step, HookDef (~150 行)
└── builtin_pipelines.rs   # 内置 pipeline 定义 (~100 行)
```

---

## 风险评估

| Phase | 风险 | 回滚方式 |
|-------|------|----------|
| 1 | 低 - 纯增量 | 删除 task_context.rs |
| 2 | 中 - 需更新 import | git revert |
| 3 | 中 - 需更新 import | git revert |

## 预期效果

| 指标 | 前 | 后 |
|------|-----|-----|
| commands 层重复代码 | ~200 行 | ~50 行 |
| store.rs 行数 | 970 | ~400 |
| config.rs 行数 | 777 | ~400 |
| 模块数量 | 不变 | +3 |

---

## 执行顺序

1. **Phase 1** - 可独立执行，立即见效
2. **Phase 2** - 依赖 Phase 1 完成后评估
3. **Phase 3** - 可选，根据需要执行
