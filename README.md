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
- 自动调用工具：read / write / edit / grep / glob / fetch / bash
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
- Windows: `%APPDATA%\onemini\config.toml`

也可通过环境变量或命令行参数临时覆盖配置（如 `ONEMINI_API_KEY`、`--api-key`），无需 `.env` 文件。

**UI 主题**（复古 Game Boy / NES 风格）：

```toml
# config.toml
[ui]
theme = "gameboy"   # modern | gameboy | nes
```

或环境变量 `ONEMINI_THEME=nes`（优先于配置文件）。交互模式中可用 `/theme` 切换并保存。

输出层会自动清洗模型泄露的内部指令、思考标签与流式未闭合 Markdown，技能列表查询（如「有哪些技能」）会短路为程序化 ASCII 表格，避免 LLM 手写 Markdown 断裂。

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
- `/model` 选择模型（交互列表，也可 `/model deepseek-chat` 直接指定）
- `/reasoning` 选择是否显示思考过程
- `/theme` 从列表选择 UI 主题（modern / gameboy / nes）（也可 `/reasoning on|off`）
- `/mode` 选择权限模式（交互列表，也可 `/mode plan` 直接指定）
- `/permissions` 查看权限规则摘要
- `/skills` 从列表选择并激活技能（`/skills list` 仅列出）
- `/help` 查看帮助
- `/exit` 退出

## Agent Skills

内置 [Agent Skills](https://github.com/anthropics/skills) 格式技能。**docx / pdf / pptx / xlsx 已随 CLI 内置**（Apache-2.0，含 `skills/` 脚本）；设计类与 Anthropic 增强版可额外安装：

```bash
onemini skills catalog                    # 查看可安装列表
onemini skills install design             # 页面设计类（frontend-design 等）
onemini skills install docs               # 可选：Anthropic 官方文档技能（覆盖内置）
onemini skills list
/pdf 合并这两个文件                       # 自动匹配或手动激活
/frontend-design 做一个 landing page
```

| 类型 | 代表技能 | 说明 |
|------|---------|------|
| 文档 | docx, pdf, pptx, xlsx | **内置**，含 scripts/ + shared/office/ |
| 设计 | frontend-design, canvas-design | 首次配置预装或 install design |
| 开发 | mcp-builder, webapp-testing | install 安装 |

内置编程向技能：`/commit-message` `/debug` `/code-review` 等（见 `/skills`）。

**首次配置**（`onemini init` 或首次运行 `onemini`）会自动预装 5 个设计技能。匹配任务时会**自动启用**对应技能（如「做一个 landing page」→ `frontend-design`），无需输入 `/技能名`。关闭自动匹配：`ONEMINI_NO_AUTO_SKILLS=1`。跳过预装：`ONEMINI_SKIP_SKILL_BOOTSTRAP=1`。

自定义：`.onemini/skills/` 或 `~/.config/onemini/skills/`。详见 [`skills/README.md`](skills/README.md)。

## 安全与权限

### 权限模式

| 模式 | CLI 参数 | 说明 |
|------|----------|------|
| default | （默认） | 变更类工具需确认 |
| plan | `--permission-mode plan` | 只读：仅 read / grep / glob / fetch |
| accept-edits | `--permission-mode accept-edits` | 工作区内 write/edit 与安全 bash（mkdir/mv/cp 等）自动放行 |
| auto | `--permission-mode auto` | 启发式分类器自动判断 |
| dont-ask | `--permission-mode dont-ask` | 未匹配 allow 规则则拒绝 |
| bypass | `--dangerously-skip-permissions` | 跳过确认（仅隔离环境） |

交互模式中用 `/mode` 从列表选择 default / plan / accept-edits / auto；用 `/model`、`/reasoning` 切换模型与思考过程显示；用 `/permissions` 查看当前规则。工具执行前的权限确认同样为列表选择（允许 / 拒绝 / 始终允许）。

非交互一次性任务：`-y` / `--yes` 仅允许已匹配 **allow** 规则的操作，不会全局 bypass。

### 配置文件

| 文件 | 路径 |
|------|------|
| 应用配置 | `~/.config/onemini/config.toml`（macOS 为 `~/Library/Application Support/onemini/`） |
| 用户权限规则 | `permissions.toml` |
| 用户 Hooks | `hooks.toml` |
| 托管策略 | macOS `/Library/Application Support/onemini/managed.toml`；Linux `/etc/onemini/managed.toml` |
| 加密会话 | `latest.json.enc` |

示例见 [`release/permissions.toml.example`](release/permissions.toml.example)、[`release/managed.toml.example`](release/managed.toml.example)。

### API 密钥

- 保存配置时优先写入系统钥匙串（`keychain` feature，默认开启），`config.toml` 中为占位符
- `base_url` 必须为 **HTTPS**
- 敏感配置文件权限为 **0600**

### OS 沙箱（bash）

默认 `sandbox.enabled = true`。bash 在沙箱内执行：

- **Linux**：需安装 `bubblewrap`（`bwrap`）
- **macOS**：使用 `sandbox-exec`
- 无可用沙箱后端时 **拒绝执行 bash**（可在 `config.toml` 中设 `sandbox.enabled = false` 降级，不推荐）

沙箱内 bash 在默认配置下可免人工确认（沙箱边界替代确认）。注意：当前 bwrap 为便捷模式（只读挂载根文件系统），非最小权限沙箱。

### 访问网页（fetch）

内置 `fetch` 工具可通过 HTTPS 获取公网网页（如 Google），**不经过 bash 沙箱**。首次使用需权限确认；`plan` 模式下也可用。限制：仅 `https://`、禁止内网/本地地址、响应体上限 2MB。

若需 bash 内 `curl` 联网，在 `config.toml` 中设置：

```toml
[sandbox]
allow_network = true
```

macOS 上 `allow_network = true` 时，沙箱会放行出站网络。

### 子 Agent 隔离

- 委派任务默认只读工具；`--worktree-delegate` 或 `delegate_use_worktree = true` 时使用 git worktree 隔离目录

### 托管策略（本地文件）

通过 `managed.toml` 可禁用 bypass / auto、下发 deny 规则与企业 Hooks（**非远程轮询**）。环境变量 `ONEMINI_MANAGED_SETTINGS` 可指定路径。

## 常用 CLI 参数

```text
onemini [OPTIONS] [TASK] [COMMAND]

Options:
  -C, --directory <DIR>                   工作目录
      --resume                            恢复上次会话
  -p, --print <PROMPT>                    一次性任务（执行后退出）
      --max-rounds <N>                    最大工具调用轮次
      --permission-mode <MODE>              default|plan|accept-edits|auto|dont-ask
  -y, --yes                               非交互：仅 allow 规则可过
      --worktree-delegate                 子 Agent 使用 git worktree
      --dangerously-skip-permissions      跳过权限确认（仅隔离环境）

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

**PowerShell**（推荐）：

```powershell
irm https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.ps1 | iex
```

**CMD（命令提示符）** — 若提示 `irm` 找不到，请用 CMD 执行：

```cmd
curl -fsSL https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.cmd -o %TEMP%\install.cmd && %TEMP%\install.cmd
```

脚本会尝试通过 **winget** 自动安装缺失的 Python 3 与 Git（含 OpenSSL）；若 winget 不可用，会输出手动安装指引。安装完成后将 `onemini.exe` 放到 `%USERPROFILE%\.local\bin\`，并**自动写入用户 PATH**。若不想自动改 PATH，可设 `$env:ONEMINI_SKIP_PATH = "1"`。

安装完成后请**新开一个终端**，然后运行：

```powershell
onemini --help
```

**手动下载 zip（离线 / 网络受限）**：

1. 从 [GitHub Releases](https://github.com/AJI1026/OneMini-CLI/releases) 下载 `onemini-win-x64.zip` 并解压
2. 在解压目录任选其一：
   - 双击 `install-local.bat`
   - 或 PowerShell：`.\install-local.ps1`
   - 或：`.\onemini.exe install`
3. 新开终端后运行 `onemini config setup`

> 本地安装脚本不做 Ed25519 验签；需要完整验签时请使用上方在线 `install.ps1`。

| 环境变量 | 说明 |
|----------|------|
| `ONEMINI_INSTALL_DIR` | 自定义安装目录（默认 `%USERPROFILE%\.local\bin`） |
| `ONEMINI_VERSION` | 指定版本，如 `0.1.0` |
| `ONEMINI_IGNORE_DEPRECATED` | 设为 `1` 允许安装已弃用版本 |
| `ONEMINI_SKIP_PATH` | 设为 `1` 跳过自动 PATH 配置 |
| `ONEMINI_SKIP_DEPS` | 设为 `1` 跳过 winget 自动安装 Python/Git |

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
| Ed25519 签名 | 每个 Release 产物附带 `.sig`；CLI 与 install 脚本**内置公钥**，下载后自动验签，失败则拒绝安装 |
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
