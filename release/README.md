# Release 签名与版本索引

OneMini-CLI 的 `onemini update` 与 `scripts/install.sh` 均依赖本目录下的**签名版本索引**，防止中间人篡改 `latest` 指向恶意版本。

## 文件说明

| 文件 | 说明 |
|------|------|
| `signing_public_key.b64` | Ed25519 公钥（Base64，32 字节），编译进 CLI |
| `versions.json` | 版本索引（latest、各平台 HTTPS 下载 URL、SHA256） |
| `versions.json.sig` | 对 `versions.json` 的 Ed25519 签名 |
| `signing_secret_key.b64` | **私钥，仅本地/CI 使用，勿提交**（已在 `.gitignore`） |

| 平台 | 文件名 | 说明 |
|------|--------|------|
| macOS (Apple Silicon) | `onemini-mac-arm64.tar.gz` | `onemini update` / 脚本安装 |
| macOS (Apple Silicon) | `onemini-mac-arm64-bundle.zip` | 含 `OneMini.app`（OneMini logo 图标），双击首次启动自动安装 |
| Linux x64 | `onemini-linux-x64.tar.gz` | 脚本安装 / 手动解压 |
| Windows x64 | `onemini-win-x64.zip` | 含 `onemini.exe`（OneMini logo 图标）+ `onemini.exe.sig`，双击 exe 自动安装 |

每个**对外发布**的产物附带 `.sig` 签名文件；Windows / macOS 离线包内另含**包内验签**用的 `onemini.exe.sig` 或 `Contents/Resources/onemini.sig`。SHA256 见 `versions.json`。

## 维护者：首次配置

```bash
# 1. 生成密钥对（仅首次）
cargo run --bin onemini-sign -- keygen --out-dir release

# 2. 将 release/signing_public_key.b64 提交到仓库
# 3. 将 release/signing_secret_key.b64 的内容存入 GitHub Secret：
#    ONEMINI_SIGNING_KEY
```

## 发布流程（CI 自动）

推送 `v*` tag 后，`.github/workflows/release.yml` 会：

1. 构建各平台产物（含 macOS `OneMini.app` zip、Windows zip）
2. 运行 `scripts/sign-release.sh`：嵌入包内签名 → 为每个 `.tar.gz` / `.zip` 生成外部 `.sig`
3. 更新并签名 `release/versions.json`
4. 上传产物与 `versions.json` 到 GitHub Release
5. 将签名后的 `versions.json` 推回 `main`

## 手动签名（调试）

```bash
export ONEMINI_SIGNING_KEY="$(cat release/signing_secret_key.b64 | tr -d '\n')"
cargo run --bin onemini-sign -- sign --file path/to/onemini-mac-arm64.tar.gz
cargo run --bin onemini-sign -- sign --file release/versions.json
```

## 标记弃用版本

在 `versions.json` 中为旧版本设置：

```json
"deprecated": true,
"deprecation_reason": "CVE-XXXX：远程代码执行，请立即升级"
```

CLI 默认拒绝安装；用户需添加 `--ignore-deprecated`（或 `ONEMINI_IGNORE_DEPRECATED=1` 用于 install.sh）。

## 安全策略摘要

- **公钥锚定**：`onemini update` 与 `scripts/install.sh` / `install.ps1` **内置** `signing_public_key.b64`，安装时不再从网络下载公钥（镜像无法替换信任根）
- **HTTPS only**：所有 URL 必须为 `https://`，客户端启用 TLS 1.2+，禁止 HTTP 回退
- **签名校验优先**：SHA256 仅作辅助；在线安装前必须验证外部 `.sig`（Ed25519 over SHA256 digest）
- **离线包内验签**：Windows / macOS Release zip 内含对二进制自身的签名，双击安装时自动校验
- **索引防篡改**：先验证 `versions.json.sig`，再信任其中的 URL
