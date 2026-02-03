# 命令参考

## Agent 标记命令

Agent 在执行过程中标记当前 Step 状态：

```bash
wt step done              # 当前 step → success，触发自动推进
wt step block "原因"       # 当前 step → blocked，等待 human 介入
wt step fail "原因"        # 当前 step → failed，等待 human 介入
```

### Agent 自验证 (Best Practice)

推荐在 agent step 中启用 Stop hook 自验证机制，确保 agent 退出前完成质量检查：

```jsonc
// phase 配置示例
{
  "on_enter": {
    "steps": [
      {
        "agent": {
          "prompt": "@.wt/tasks/${task}.md",
          "settings": ".wt/templates/verify-settings.json"  // 启用自验证
        },
        "verify": { "type": "self" }
      }
    ]
  }
}
```

**工作流程**：
1. Agent 完成任务尝试退出
2. Stop hook 触发，提示阅读 `.wt/verify.md` 自检清单
3. Agent 根据检查结果调用 `wt step done/block/fail`
4. 状态标记后允许退出

**相关文件**：
- `.wt/hooks/verify-stop.cjs` - Stop hook 脚本
- `.wt/verify.md` - 验证清单模板
- `.wt/templates/verify-settings.json` - Claude settings 模板

## Human 强制命令

Human 强制操作阶段转换：

```bash
wt next <task>            # 推进到下一阶段
wt prev <task>            # 回退到上一阶段
wt stop <task>            # 停止当前进程，保持 phase 不变
wt reset <task>           # 备份代码，重置到 pending
wt delete <task>          # 彻底删除任务
```

### next 行为

1. 停止当前进程（如果有）
2. 执行 on_exit workflow
3. 资源转换
4. 更新 phase
5. 执行 on_enter workflow

### prev 行为

1. 停止当前进程
2. 资源转换
3. 更新 phase
4. **不执行 on_enter**（回退不触发 workflow）

## 任务管理命令

```bash
wt init                   # 初始化 .wt 目录
wt create <name>          # 创建任务
wt validate               # 验证任务配置和循环依赖
wt list                   # 列出任务
wt status [--all] [--json] # TUI 或 JSON 输出
wt tail <task>            # 查看 transcript
wt logs                   # 生成日志
```

## 任务索引

所有命令支持 1-based 索引代替任务名：
- `wt next 1` = `wt next <第一个任务>`
- 优先级：任务名 > 索引号

## TUI 快捷键

| 键 | 功能 |
|----|------|
| `j/k` | 上下选择 |
| `Enter` | 切换到任务窗口 |
| `n` | 执行 `wt next` |
| `p` | 执行 `wt prev` |
| `s` | 执行 `wt stop` |
| `l` | 打开日志 |
| `t` | 打开 transcript |
| `?` | 帮助 |
| `q` | 退出 |

## 配置文件

### .wt/config.jsonc

```jsonc
{
  "multiplexer": "tmux",       // tmux 或 zellij
  "session_name": "project",
  "claude_command": "claude",
  "worktree_dir": ".wt/worktrees",
  "phases": {
    "sequence": ["pending", "developing", "reviewing", "completed"]
  }
}
```

### .wt/tasks/*.md

```yaml
---
name: auth
depends:
  - database
---
任务描述...
```

### .wt/status.json

运行时状态存储，包含每个任务的 status/phase/instance 信息。

## 任务名规则

任务名必须是有效的 git 分支名：
- 不能为空
- 不能含空格、制表符
- 不能含 `~ ^ : ? * [ @ {`
- 不能以 `-` 或 `.` 开头
- 不能以 `.` 或 `.lock` 结尾
- 不能含 `..`
