---
name: blender-modeling
title: Blender 建模
description: 引导通过 OneMini CLI 本机 MCP 连接 Blender；Web 端不开放。用户要求打开 Blender、建模或导出模型时使用。
---

# Blender 建模

## 重要：渠道限制

- **OneMini Web 不支持**通过对话连接本机 Blender MCP（本机权限过高，不适合多用户 Web）。
- **正式入口是 OneMini CLI**：在用户本机配置 MCP，与 Blender 同机运行。
- 若用户在 Web 提问：说明须改用 CLI；可给手操步骤，**不要假装已调用 MCP / 已改场景**。

## CLI 前置条件

1. 本机已安装 Blender，并启用 Blender MCP 插件（侧栏连接）
2. 在 CLI 侧配置 MCP（如 `uvx blender-mcp`，server id 建议 `blender`）
3. 确认 CLI 已连上后再执行建模类工具调用

## 流程

1. 确认目标：道具 / 角色粗模 / 场景块面；目标面数与用途（预览 / 导出 glTF）
2. **仅在 CLI 且 MCP 可用时**：先查场景信息，再小步创建/修改物体
3. 命名规范：`prop_` / `char_` / `env_` 前缀；材质与物体名一致
4. 导出建议：优先 glTF/GLB；说明原点、单位与轴向，便于导入 OneMini 3D 预览

## 输出格式

```markdown
## 目标
…

## 渠道
（请使用 CLI / 仅手操指引）

## 建模步骤
1. …
2. …

## 导出
- 格式：
- 检查项：缩放、原点、多余灯光/相机
```

## 原则

- Web 会话：只给指引，不声称已连接 Blender
- CLI 会话：优先调用已暴露的 MCP 工具；不要编造未出现的工具名
- 复杂造型拆成多次小改动，便于回退
