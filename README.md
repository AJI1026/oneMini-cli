# OneMini-CLI

OneMini-CLI 是一个终端 AI 编程助手，适合持续协作的复杂开发任务。
你可以在同一轮会话里完成代码编写、调试、重构，并保持上下文。

## 推荐使用方式

默认进入交互会话（持续协作）：

```bash
onemini
```

恢复上次会话（包含历史与任务状态）：

```bash
onemini --resume
# 或
onemini resume
```

一次性任务（执行后退出，适合脚本）：

```bash
onemini "列出 src 目录下的所有 .rs 文件"
onemini -p "运行 cargo test 并解释失败原因"
```

## 它能做什么

- 持续交互会话（多轮上下文）
- 任务流：计划 -> 执行 -> 验证 -> 总结
- 自动调用工具：read / write / edit / grep / glob / bash
- 调试与重构流程支持（失败重试、验证建议）
- 写文件和执行命令前权限确认
- 自动读取项目上下文（`ONEMINI.md`、`AGENTS.md`、`CLAUDE.md`）

## 快速开始

### 1. 安装 Rust（如已安装可跳过）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 编译并安装

```bash
git clone https://github.com/AJI1026/OneMini-CLI.git
cd OneMini-CLI
cargo install --path .
```

> 也可直接运行编译产物，详见下方 [编译与打包](#编译与打包) 和 [运行方式](#运行方式)。

### 3. 配置 API

```bash
onemini config setup
```

或手动指定：

```bash
onemini config set --api-key "sk-..." --base-url "https://api.deepseek.com" --model "deepseek-chat"
```

配置文件位置：

- macOS: `~/Library/Application Support/onemini/config.toml`
- Linux: `~/.config/onemini/config.toml`

### 4. 开始协作

进入项目目录后启动：

```bash
onemini -C /path/to/project
```

示例任务：

```bash
# 在会话中直接输入
帮我修复 cargo test 的失败用例，并给出验证结果

# 或一次性执行
onemini "重构 config 模块，保持现有行为并通过 cargo test"
```

## 会话命令

在交互模式中可用：

- `/plan` 查看当前任务计划
- `/status` 查看步骤、验证状态、最近错误
- `/retry` 重试最近失败步骤
- `/clear` 清空会话与任务状态
- `/config` 查看配置
- `/help` 查看帮助
- `/exit` 退出

## 常用 CLI 参数

```text
onemini [OPTIONS] [TASK] [COMMAND]

Options:
  -C, --directory <DIR>                   工作目录
      --resume                            恢复上次会话
  -p, --print <PROMPT>                    一次性任务（执行后退出）
      --max-rounds <N>                    最大工具调用轮次
      --dangerously-skip-permissions      跳过权限确认（仅脚本）

Commands:
  chat      交互式会话（默认）
  resume    恢复上次会话
  config    配置管理
  init      初始化配置
```

## 编译与打包

### 环境要求

- **Rust** 工具链（推荐 stable，MSRV 1.70+）
- 如需 HTTPS 联网调用 API，确保系统安装了 CA 证书

安装 Rust：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 源码编译

在项目根目录执行：

```bash
# Debug 编译（快速，带调试符号，产物在 target/debug/）
cargo build

# Release 编译（推荐，开启优化，产物在 target/release/）
cargo build --release --locked
```

| 选项 | 说明 |
|------|------|
| `--release` | 开启优化（LTO、瘦身），运行更快 |
| `--locked` | 使用 `Cargo.lock` 锁定的依赖版本，保证可复现 |

### 打包为系统命令

编译后安装到 `~/.cargo/bin/`，之后可在任意目录直接调用 `onemini`：

```bash
# 从本地源码安装（等价于 build --release + 拷贝到 ~/.cargo/bin/）
cargo install --path .

# 确认安装成功
onemini --help
```

### 一键安装（从 Release 下载预编译二进制）

```bash
curl -fsSL https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
```

该脚本会自动检测平台，下载对应预编译二进制并安装到 `~/.cargo/bin/`。

---

## 运行方式

### 方式一：直接运行编译产物（无需安装）

```bash
# Release 产物
./target/release/onemini

# 或 Debug 产物
./target/debug/onemini
```

### 方式二：cargo run（开发调试用）

```bash
# 自动编译并运行（Release）
cargo run --release --locked

# 传递参数给 onemini：在 -- 之后
cargo run --release --locked -- -C /path/to/project
cargo run --release --locked -- -p "列出 src 目录下的所有 .rs 文件"
```

### 方式三：安装后直接调用

```bash
onemini                  # 进入交互会话
onemini --resume          # 恢复上次会话
onemini -C /path/to/project  # 指定工作目录
onemini -p "运行 cargo test 并解释失败原因"  # 一次性任务
```

## 许可

Apache-2.0（见 `LICENSE`）
