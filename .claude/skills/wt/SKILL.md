---
name: wt
description: Worktree 任务管理。帮助用户规划、创建和管理并行开发任务。当用户说"规划任务"、"拆分任务"、"创建 wt 任务"时触发。
---

# wt - Worktree Task Manager

## 入口

| 触发 | 读 |
|------|-----|
| 用户说"规划任务"或"拆分任务" | [workflows/plan.md](workflows/plan.md) |
| 用户说"创建任务" | 直接用 `wt create --json` |
| 用户问"任务格式" | [reference/json-schema.md](reference/json-schema.md) |

## 快速命令

```bash
# 初始化项目（首次使用）
wt init

# 创建任务
wt create --json '{"name": "任务名", "depends": [], "description": "描述"}'

# 验证任务
wt validate

# 查看任务
wt list

# 查看可启动的任务
wt next

# 启动任务
wt run <name>

# 查看状态 (TUI)
wt status

# 查看状态 (JSON，给 agent 用)
wt status --json

# 标记待审核
wt review <name>

# 从 review 恢复
wt resume <name>

# 完成任务 (merge + cleanup)
wt complete <name>

# 重置任务到 pending
wt reset <name>

# 删除任务资源
wt delete <name> [--force]
```

## Status TUI 快捷键

| 按键 | 功能 |
|------|------|
| `↑↓` / `jk` | 导航 |
| `Enter` | 进入 multiplexer 窗口 |
| `t` | tail (查看输出) |
| `r` | 标记 review |
| `u` | resume (继续开发) |
| `c` | complete (完成) |
| `q` | 退出 |

## JSON 格式

```json
{
  "name": "task-name",
  "depends": ["dep1", "dep2"],
  "description": "任务描述（支持 Markdown）"
}
```

## 规则

1. **name**: 小写字母、数字、连字符，不含路径分隔符
2. **depends**: 必须是已存在的任务名
3. **description**: 清晰描述任务目标和验收标准

## 参考

- [reference/json-schema.md](reference/json-schema.md) - 完整 JSON 格式说明
- [workflows/plan.md](workflows/plan.md) - 任务规划工作流
