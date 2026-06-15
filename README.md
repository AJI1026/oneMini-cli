# OneMini-CLI

OneMini-CLI 是一个终端 AI 助手，使用 Rust 开发。
你可以在命令行里完成代码读取、编辑、搜索和命令执行。

## 它能做什么

- 交互式对话（REPL）
- 一次性任务执行（执行完即退出）
- 自动调用内置工具完成任务
- 写文件和执行命令前进行权限确认
- 自动读取项目上下文文件（`ONEMINI.md`、`AGENTS.md`、`CLAUDE.md`）

## 快速开始

### 第一步：安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 第二步：配置 API

推荐用交互式配置：

```bash
cargo run -- config setup
```

也可以用命令行一次性设置：

```bash
cargo run -- config set --api-key "sk-..." --base-url "https://api.deepseek.com" --model "deepseek-chat"
```

或者使用环境变量：

```bash
export ONEMINI_API_KEY="sk-..."
export ONEMINI_BASE_URL="https://api.deepseek.com"
export ONEMINI_MODEL="deepseek-chat"
```

配置文件默认位置：

- macOS: `~/Library/Application Support/onemini/config.toml`
- Linux: `~/.config/onemini/config.toml`

### 第三步：运行

交互模式（默认）：

```bash
cargo run --release
```

指定工作目录：

```bash
cargo run --release -- -C /path/to/project
```

一次性执行任务：

```bash
cargo run --release -- -p "列出 src 目录下的所有 .rs 文件"
```

查看帮助：

```bash
cargo run --release -- --help
```

## 安装到系统命令

安装后可直接在终端使用 `onemini`：

```bash
cargo install --path .
onemini --help
```

## 常用命令（REPL 内）

- `/help` 显示帮助
- `/clear` 清空历史
- `/config` 查看当前配置
- `/exit` 退出程序

## CLI 参数

```text
onemini [OPTIONS] [COMMAND]

Options:
  -p, --print <PROMPT>                    一次性执行任务后退出
  -C, --directory <DIR>                   工作目录
  -m, --model <MODEL>                     模型名称
      --base-url <URL>                    API Base URL
      --api-key <KEY>                     API Key
      --max-rounds <N>                    最大工具调用轮次 [default: 25]
      --dangerously-skip-permissions      跳过权限确认（仅脚本场景）

Commands:
  chat      交互式对话（默认）
  config    配置管理
  init      初始化配置（等同于 config setup）
```

## 支持的 API

支持 OpenAI 兼容的 Chat Completions 接口，例如：

- DeepSeek: `https://api.deepseek.com`
- OpenAI: `https://api.openai.com/v1`
- 其他兼容服务

## 项目目录

```text
src/
├── main.rs
├── cli.rs
├── config.rs
├── repl.rs
├── agent/
├── llm/
├── tools/
└── ui.rs
```

## 许可

Apache-2.0（见 `LICENSE`）
