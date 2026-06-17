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

首次运行 `onemini` 会自动进入交互式配置向导（选择服务商 → 模型 ID → Base URL → API Key）。

也可随时手动配置：

```bash
onemini config setup
```

或手动指定：

```bash
onemini config set --api-key "sk-..." --base-url "https://api.deepseek.com" --model "deepseek-chat"
```

交互模式中可用 `/config` 查看配置，`/config setup` 重新配置。

配置文件位置：

- macOS: `~/Library/Application Support/onemini/config.toml`
- Linux: `~/.config/onemini/config.toml`

也可通过环境变量或命令行参数临时覆盖配置（如 `ONEMINI_API_KEY`、`--api-key`），无需 `.env` 文件。

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
- `/config setup` 重新配置 API / 模型
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
  update    检查并更新 CLI 版本
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
curl -fL --progress-bar https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
```

该脚本会自动检测平台，下载对应预编译二进制并安装到 `~/.local/bin/`（可通过 `ONEMINI_INSTALL_DIR` 自定义），并**自动写入 shell 配置文件**（macOS 默认 `~/.zshrc`，Linux 默认 `~/.bashrc`）添加 PATH。若不想自动改配置，可设 `ONEMINI_SKIP_PATH=1`。

下载过程中会显示进度条。若 GitHub 访问较慢或超时，可尝试：

```bash
# 使用 GitHub 镜像加速（国内网络常见）
ONEMINI_MIRROR=https://ghproxy.com \
  curl -fL --progress-bar https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash

# 或先下载脚本再本地执行（便于排查）
curl -fL --progress-bar -o /tmp/onemini-install.sh \
  https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh
bash /tmp/onemini-install.sh
```

| 环境变量 | 说明 |
|----------|------|
| `ONEMINI_MIRROR` | GitHub 镜像前缀，如 `https://ghproxy.com` |
| `ONEMINI_RAW_BASE` | 自定义 `versions.json` 等 raw 文件根 URL |
| `ONEMINI_CONNECT_TIMEOUT` | 连接超时（秒，默认 15） |
| `ONEMINI_DOWNLOAD_TIMEOUT` | 单次下载超时（秒，默认 600） |
| `ONEMINI_QUIET=1` | 静默下载（不显示进度条） |

> **注意**：一键安装依赖 GitHub Release 已发布且 `release/versions.json` 已写入真实 SHA256 与签名。若尚未打 tag 发布，请使用下方源码编译方式。

#### Windows 一键安装

在 **PowerShell** 中执行（需已安装 [Python 3](https://python.org) 与 [Git for Windows](https://git-scm.com/download/win)（含 OpenSSL））：

```powershell
irm https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.ps1 | iex
```

脚本会下载 `onemini.exe` 到 `%USERPROFILE%\.local\bin\`，并**自动写入用户 PATH**（无需手动打开系统设置）。若不想自动改 PATH，可设 `$env:ONEMINI_SKIP_PATH = "1"`。

安装完成后请**新开一个终端**，然后运行：

```powershell
onemini --help
```

| 环境变量 | 说明 |
|----------|------|
| `ONEMINI_INSTALL_DIR` | 自定义安装目录（默认 `%USERPROFILE%\.local\bin`） |
| `ONEMINI_VERSION` | 指定版本，如 `0.1.0` |
| `ONEMINI_IGNORE_DEPRECATED` | 设为 `1` 允许安装已弃用版本 |
| `ONEMINI_SKIP_PATH` | 设为 `1` 跳过自动 PATH 配置 |

### 更新 CLI

已安装用户可直接使用内置更新命令（从 GitHub Releases 拉取，**配置不会丢失**）：

```bash
# 检查是否有新版本
onemini update --check

# 更新到 latest release
onemini update

# 更新到指定小版本（0.1.x 补丁/小版本）
onemini update --version 0.1.1

# 强制重装当前 latest（修复损坏安装）
onemini update --force

# 安装已标记为 deprecated 的旧版本（默认拒绝）
onemini update --version 0.1.0 --ignore-deprecated
```

更新流程会先校验 `release/versions.json` 的 Ed25519 签名，再下载对应 `.tar.gz` 及其 `.sig`；**仅 SHA256 不足以保证安全**（见下方安全设计）。

也可重新运行安装脚本（等价于更新到 latest）：

```bash
curl -fL --progress-bar https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.sh | bash
```

#### 维护者：发布 0.1.x 小版本

在 **不改动主版本** 的前提下发布补丁（如 `0.1.0` → `0.1.1`）：

```bash
# 1. 修改 Cargo.toml 中的 version = "0.1.1"
# 2. 提交并推送
git commit -am "chore: bump version to 0.1.1"
git push origin main

# 3. 打 tag 并推送（CI 会自动构建 Release 产物）
git tag -a v0.1.1 -m "v0.1.1: 修复 xxx"
git push origin v0.1.1
```

用户侧执行 `onemini update` 即可自动从 `v0.1.0` 升到 `v0.1.1`（semver 比较：`0.1.1 > 0.1.0`）。

#### Release 安全设计

| 措施 | 说明 |
|------|------|
| HTTPS only | 所有下载 URL 必须为 `https://`；客户端强制 TLS 1.2+，禁止 HTTP 回退 |
| Ed25519 签名 | 每个 Release 产物附带 `.sig`；CLI 内置公钥（`release/signing_public_key.b64`），下载后自动验签，失败则拒绝安装 |
| 索引防篡改 | `release/versions.json` 由维护者签名（`versions.json.sig`）；CLI 先验索引再使用其中的 URL |
| 弃用策略 | 在 `versions.json` 中标记 `"deprecated": true`；CLI 默认拦截，需 `--ignore-deprecated` 才允许安装 |

维护者首次发布前，请阅读 [`release/README.md`](release/README.md) 配置 `ONEMINI_SIGNING_KEY` GitHub Secret。

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
