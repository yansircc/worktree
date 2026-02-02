# 任务规划工作流

## 前置条件

确保项目已初始化 wt：

```bash
wt init  # 创建 .wt/config.yaml 配置和 .wt/tasks/ 目录
```

## 流程

```
理解需求 → 分析代码库 → 拆分任务 → 确定依赖 → 创建任务 → 验证
```

## 第一步：理解需求

用 AskUserQuestion 确认：

```
1. 要实现什么功能？
2. 有技术约束吗？（框架、库等）
3. 预期拆分粒度？（粗/细）
```

## 第二步：分析代码库

使用 Glob、Grep、Read 分析：

- 项目结构
- 现有模式
- 相关文件

## 第三步：拆分任务

原则：

| 原则 | 说明 |
|------|------|
| 单一职责 | 每个任务只做一件事 |
| 可独立验证 | 任务完成后可单独测试 |
| 最小依赖 | 尽量减少任务间依赖 |
| 并行优先 | 能并行的不要串行 |

## 第四步：确定依赖

依赖图示例：

```
auth ──────┬──→ user-api ──┐
           │               ├──→ dashboard
database ──┴──→ post-api ──┘
```

规则：
- 依赖必须是已存在的任务
- 避免循环依赖
- 层级不宜超过 3 层

## 第五步：创建任务

**批量创建示例：**

```bash
# 先创建无依赖的任务
wt create --json '{"name": "auth", "depends": [], "description": "实现用户认证\n\n## 要求\n- NextAuth.js\n- GitHub OAuth"}'

wt create --json '{"name": "database", "depends": [], "description": "设置数据库\n\n## 要求\n- Prisma\n- PostgreSQL"}'

# 再创建有依赖的任务
wt create --json '{"name": "user-api", "depends": ["auth", "database"], "description": "用户 API\n\n## 端点\n- GET /api/users\n- POST /api/users"}'

wt create --json '{"name": "post-api", "depends": ["auth", "database"], "description": "文章 API"}'

wt create --json '{"name": "dashboard", "depends": ["user-api", "post-api"], "description": "用户仪表盘"}'
```

## 第六步：验证

```bash
# 验证格式
wt validate

# 查看任务树
wt list
```

预期输出：

```
auth [○]
├── user-api [○]
│   └── dashboard [○]
database [○]
├── user-api [○]
└── post-api [○]
    └── dashboard [○]
```

## 展示给用户

创建完成后，展示：

1. 任务列表（`wt list` 输出）
2. 依赖关系图
3. 建议执行顺序

让用户确认后，可以开始 `wt run`。

## 启动后

```bash
# 查看可启动的任务
wt next

# 启动任务
wt run auth

# 查看状态
wt status
```
