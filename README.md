# OneMini-CLI

OneMini 系产品的终端 AI 编程助手，使用 Rust 实现，体验类似 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)。

在终端中与 AI 协作编写代码：读取/编辑文件、搜索代码库、执行 shell 命令。

## 功能

| 能力 | 说明 |
|------|------|
| **交互式 REPL** | 持续对话，流式输出 |
| **一次性模式** | `-p "任务描述"` 执行后退出 |
| **Agent 工具循环** | 自动调用工具直至完成任务 |
| **内置工具** | `read` `write` `edit` `grep` `glob` `bash` |
| **项目上下文** | 自动加载 `ONEMINI.md` / `AGENTS.md` / `CLAUDE.md` |
| **权限确认** | 写入文件与执行命令前需用户批准 |

## 快速开始

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 配置 API

```bash
# 方式一：终端交互式配置（推荐）
cargo run -- config setup
# 按提示输入 API Key、Base URL、模型名称，自动保存到配置文件

# 方式二：命令行直接设置
cargo run -- config set --api-key "sk-..." --base-url "https://api.deepseek.com" --model "deepseek-chat"

# 方式三：环境变量（在 shell 中执行）
export ONEMINI_API_KEY="sk-..."
export ONEMINI_BASE_URL="https://api.deepseek.com"
export ONEMINI_MODEL="deepseek-chat"

# 配置文件路径
# macOS: ~/Library/Application Support/onemini/config.toml
# Linux: ~/.config/onemini/config.toml
```

### 3. 运行

```bash
# 交互模式（默认）
cargo run --release

# 指定工作目录
cargo run --release -- -C /path/to/project

# 一次性任务
cargo run --release -- -p "列出 src 目录下的所有 .rs 文件"

# 查看帮助
cargo run --release -- --help
```

安装到系统 PATH：

```bash
cargo install --path .
onemini
```

## 配置

配置文件路径：`~/.config/onemini/config.toml`

```toml
api_key = "sk-..."
base_url = "https://api.deepseek.com"
model = "deepseek-chat"
temperature = 0.2
max_tokens = 8192
```

### 支持的 API

任何 **OpenAI 兼容** 的 Chat Completions 接口均可使用，例如：

- DeepSeek (`https://api.deepseek.com`)
- OpenAI (`https://api.openai.com/v1`)
- 硅基流动、Moonshot 等

## 交互命令

在 REPL 中可使用：

| 命令 | 说明 |
|------|------|
| `/help` | 显示帮助 |
| `/clear` | 清空对话历史 |
| `/config` | 显示当前配置 |
| `/exit` | 退出 |

## 项目上下文

在项目根目录放置以下文件，CLI 会自动注入系统提示词：

- `ONEMINI.md`（推荐）
- `AGENTS.md`
- `CLAUDE.md`

## CLI 参数

```
onemini [OPTIONS] [COMMAND]

Options:
  -p, --print <PROMPT>       一次性执行任务后退出
  -C, --directory <DIR>      工作目录
  -m, --model <MODEL>        模型名称
      --base-url <URL>       API Base URL
      --api-key <KEY>        API Key
      --max-rounds <N>       最大工具调用轮次 [default: 25]
      --dangerously-skip-permissions  跳过权限确认（仅用于脚本）

Commands:
  chat    交互式对话（默认）
  config  配置管理
    show   显示当前配置（默认）
    setup  交互式配置 API Key、Base URL、模型
    set    通过命令行设置并保存配置项
  init    初始化配置（等同于 config setup）
```

## 架构

```
src/
├── main.rs          # 入口
├── cli.rs           # 命令行参数
├── config.rs        # 配置管理
├── repl.rs          # 交互式 REPL
├── agent/           # Agent 循环与系统提示词
├── llm/             # OpenAI 兼容 API 客户端
├── tools/           # read/write/edit/grep/glob/bash
└── ui.rs            # 终端输出样式
```

## 与 OneMini 平台的关系

本 CLI 为**独立终端工具**，直接调用 OpenAI 兼容 API，无需启动 OneMini 后端。

若需对接 OneMini Platform（`/api/platform/agent/*`），可后续扩展 `platform` 模式，复用服务端密钥与 MCP 工具。

## 许可

Apache-2.0（见 [LICENSE](LICENSE)）
