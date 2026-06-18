---
name: explore-codebase
description: 快速理解陌生代码库；在用户进入新项目、询问架构/入口/模块职责时使用。
---

# Explore Codebase

## 流程

1. **目录概览**：glob 顶层 + 关键子目录（src、app、lib）
2. **入口点**：main、index、router、CLI 命令注册
3. **配置**：构建文件、env 示例、config 模块
4. **数据流**：请求/事件从入口到核心逻辑的 1 条主路径
5. **依赖**：外部服务、数据库、主要第三方库

## 工具策略

- `glob` 找文件模式
- `grep` 找 `@router`、`fn main`、`export`、`class.*Controller`
- `read` 只读关键文件片段，避免一次读整库

## 输出格式

```markdown
## 项目类型
（CLI / Web API / 前端 / 单体 / monorepo）

## 目录结构
（精简树 + 各目录职责）

## 入口与主流程
（1 段话 + 关键文件路径）

## 配置与运行
（如何本地跑起来）

## 值得深入的文件
（3-5 个路径 + 一句话说明）
```

## 原则

- 先广后深，不要第一轮就读所有文件
- 路径用代码引用格式：`path/to/file.rs`
- 不确定处标注「待确认」
