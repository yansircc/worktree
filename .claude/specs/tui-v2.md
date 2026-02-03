# TUI v2 设计规格

## 概述

重构 `wt status` TUI，从简单的任务列表升级为**左右分栏的监控仪表盘**。

## 设计目标

1. **一览全局**：左侧显示所有任务的关键指标
2. **深入细节**：右侧显示选中任务的详情
3. **实时反馈**：通过「最新消息」直观展示 Agent 活动
4. **快速干预**：提供必要的控制操作（next/stop/log）

## 布局

```
┌─ wt ────────────────────┬────────────────────────────────────────────┐
│                         │ auth: developing                           │
│  1 ● auth      dev  45% │ ─────────────────────────────────────────  │
│  2 ● database  rev  23% │ on_enter workflow                          │
│  3 ◐ ui        dev  80% │ ├─ [1] setup      ✓     2s                 │
│  4 ○ config    pnd    - │ ├─ [2] develop    ●    12m                 │
│                         │ └─ [3] verify     ○                        │
│                         │                                            │
│                         │ Context   [████████░░░░]  45%              │
│                         │ Duration  12m                              │
│                         │ Git       3 commits  +120/-30              │
│                         │ Tool      Edit                             │
│                         │                                            │
│                         │ ─────────────────────────────────────────  │
│                         │ > Editing src/commands/status.rs...        │
│                         │                                            │
├─────────────────────────┴────────────────────────────────────────────┤
│ [j/k] select  [Enter] attach  [n] next  [s] stop  [l] log  [q] quit  │
└──────────────────────────────────────────────────────────────────────┘
```

### 左侧：Task 列表

**固定宽度**：25 列

**格式**：
```
  1 ● auth      dev  45%
  │ │ │         │    │
  │ │ │         │    └─ context% (核心指标)
  │ │ │         └─ phase 缩写 (dev/rev/pnd)
  │ │ └─ 任务名 (固定 10 字符，截断)
  │ └─ 状态图标
  └─ 索引 (1-based)
```

**状态图标**：
- `●` Active（有进程在跑）
- `◐` Idle（有资源，无进程）
- `○` Pending（无资源）
- `✓` Completed（已完成，仅 `--all` 时显示）

**Phase 缩写**：
- `dev` = developing
- `rev` = reviewing
- `pnd` = pending
- 其他阶段取前 3 字符

**Context 颜色**：
- < 80%：青色（正常）
- >= 80%：黄色（警告）
- >= 95%：红色（危险）

**特殊显示**：
- 有冲突：`1 ⚠ auth dev 45%`（红色）
- Idle 带原因：`3 ◐ ui dev 80% blk`（黄色）

**排序**：按创建时间（即任务文件顺序）

**过滤**：默认不显示 Completed，`--all` 显示全部

### 右侧：详情面板

根据任务状态显示不同内容。

#### Active 任务

```
auth: developing
─────────────────────────────────
on_enter workflow
├─ [1] setup      ✓     2s
├─ [2] develop    ●    12m
└─ [3] verify     ○

Context   [████████░░░░]  45%
Duration  12m
Git       3 commits  +120/-30
Tool      Edit

─────────────────────────────────
> Editing src/commands/status.rs...
```

#### Idle 任务

```
ui: developing (idle)
─────────────────────────────────
on_enter workflow
├─ [1] setup      ✓     2s
├─ [2] develop    ◐    24m
└─ [3] verify     ○

Reason    blocked: "需要确认 API 设计"
Context   80%
Duration  24m
Git       5 commits  +200/-50

─────────────────────────────────
Press [n] to resume, [Enter] to inspect
```

#### Pending 任务

```
config: pending
─────────────────────────────────
No resources allocated

Dependencies:
  ✓ database (completed)
  ● auth (developing...)

─────────────────────────────────
Press [n] to start
```

#### 无任务 / 帮助

```
wt - Worktree Task Manager
─────────────────────────────────
No tasks selected

Keyboard shortcuts:
  j/k     Navigate tasks
  Enter   Attach to agent window
  n       Next phase (wt next)
  s       Stop task (wt stop)
  l       View logs
  q       Quit

─────────────────────────────────
```

### 底部：快捷键栏

```
[j/k] select  [Enter] attach  [n] next  [s] stop  [l] log  [q] quit
```

快捷键根据选中任务状态变化（灰色表示不可用）。

## 快捷键

| 按键 | 作用 | Active | Idle | Pending |
|------|------|--------|------|---------|
| `j` / `↓` | 选择下一个 | ✓ | ✓ | ✓ |
| `k` / `↑` | 选择上一个 | ✓ | ✓ | ✓ |
| `Enter` | 进入 Agent 窗口 | ✓ | Shell | - |
| `n` | 执行 wt next | ✓ | ✓ | ✓ |
| `s` | 执行 wt stop | ✓ | - | - |
| `l` | 打开 log（新 tmux 窗口） | ✓ | ✓ | - |
| `t` | Tail transcript | ✓ | ✓ | - |
| `q` / `Esc` | 退出 | ✓ | ✓ | ✓ |

**Enter 行为细节**：
- Active + tmux 窗口存在：切换到该窗口
- Active + tmux 窗口不存在：显示恢复命令
- Idle：打开 worktree 目录的 shell
- Pending：无操作

**l (log) 行为**：
- 在新 tmux 窗口打开当前 Step 的日志
- 命令：`tmux new-window -n "log:auth" "less +F .wt/logs/auth/developing/step-2.log"`
- 不退出 TUI

## 数据来源

| 字段 | 来源 |
|------|------|
| 任务列表 | `TaskStore::load()` |
| 状态 | `StatusStore` |
| Phase | `StatusStore.phase` |
| Context % | `transcript::parse_transcript()` |
| Duration | `transcript::parse_transcript()` |
| Current Tool | `transcript::parse_transcript()` |
| Git 统计 | `git::get_worktree_metrics()` |
| 冲突检测 | `git::get_worktree_metrics()` |
| 最新消息 | **新增** `transcript::get_latest_message()` |
| Workflow/Step | `StatusStore` 或 config |

## 新增函数

### `transcript::get_latest_message()`

```rust
/// 从 transcript 获取最新的 assistant 消息摘要
pub fn get_latest_message(transcript_path: &Path) -> Option<String> {
    // 1. 读取文件最后 N 行（避免读取整个文件）
    // 2. 解析 JSONL，找最后一条 assistant 类型消息
    // 3. 截断到合适长度（~50 字符）
    // 4. 返回 "> {message}..."
}
```

## 边界情况

### 终端太小

最小尺寸：60 列 x 10 行

小于此尺寸时显示：
```
Terminal too small
Need at least 60x10
Current: 50x8
```

### 无 Transcript

Agent Step 刚启动，尚无 transcript：
```
> (waiting for output...)
```

### 列表为空

所有任务都是 Completed（且未使用 `--all`）：
```
┌─ wt ────────────────────┬────────────────────────────────────────────┐
│                         │ wt - Worktree Task Manager                 │
│  (no active tasks)      │ ─────────────────────────────────────────  │
│                         │ All tasks completed!                       │
│  Use --all to show      │                                            │
│  completed tasks        │ Use 'wt create' to add new tasks           │
│                         │                                            │
...
```

## 配置

### 刷新频率

固定 2 秒，暂不支持配置。

### 左侧宽度

固定 25 列，暂不支持配置。

## 实现计划

### Phase 1：重写 TUI 结构 ✅

1. **`tui/ui.rs`** - 重写布局为左右分栏 ✅
2. **`tui/app.rs`** - 更新 `TaskDisplay` 结构 ✅
3. **`services/transcript.rs`** - 新增 `get_latest_message()` ✅
4. **`cli.rs`** - 添加 `--all` 参数 ✅

### Phase 2：完善交互 ✅

**新增快捷键**：
1. `n` - 执行 `wt next`（推进阶段） ✅
2. `s` - 执行 `wt stop`（停止进程） ✅
3. `l` - 在新 tmux 窗口打开 log ✅

**清理旧代码** ✅：
- 移除 `r`/`u`/`c` 快捷键
- 移除 `can_mark_idle()`/`mark_idle()`/`can_resume()`/`can_complete()`/`mark_completed()` 方法
- 重写 `actions.rs` 使用 `wt stop`/`wt next` 命令

### Phase 3：可选增强（部分完成）

1. `p` - 执行 `wt prev`（回退阶段） ✅
2. Enter 对 Idle 任务打开 worktree shell ✅
3. 显示真实 workflow/step 进度 - ⏸️ 待多步骤 workflow 支持后实现

## 测试

### 手动测试场景

1. **基本显示**：启动多个任务，验证列表和详情显示正确
2. **状态切换**：stop/next，验证状态更新
3. **Context 警告**：验证颜色变化
4. **冲突检测**：制造冲突，验证警告显示
5. **最新消息**：验证消息实时更新
6. **快捷键**：验证所有快捷键功能
7. **边界情况**：小终端、无任务、无 transcript

### 自动化测试

TUI 测试较难自动化，以手动测试为主。可考虑：
- 单独测试 `get_latest_message()` 函数
- 测试数据渲染逻辑（不含实际终端）
