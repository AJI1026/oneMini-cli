---
name: commit-message
description: 根据 git diff 生成规范的 commit message；在用户要求写提交说明、review staged changes 或完成一批改动后使用。
---

# Commit Message

## 流程

1. 运行 `git status` 与 `git diff`（若有 staged：`git diff --cached`）
2. 归纳改动：**为何改**（动机）与 **改了什么**（范围）
3. 输出 1 条 commit message，可选 2-3 条备选

## 格式（Conventional Commits）

```
<type>(<scope>): <subject>

<body 可选，说明动机与影响>
```

**type**：feat | fix | docs | style | refactor | test | chore | perf | ci

**规则**

- subject 使用祈使句、≤72 字符、无句号
- 不写 `[Cursor]` 等工具水印
- 不猜测未在 diff 中出现的改动

## 示例

```
fix(auth): reject expired refresh tokens before DB lookup

Validate JWT exp claim first to avoid unnecessary queries on logout flood.
```
