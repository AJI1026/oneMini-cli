# Release 签名与版本索引

OneMini-CLI 的 `onemini update` 与 `scripts/install.sh` 均依赖本目录下的**签名版本索引**，防止中间人篡改 `latest` 指向恶意版本。

## 文件说明

| 文件 | 说明 |
|------|------|
| `signing_public_key.b64` | Ed25519 公钥（Base64，32 字节），编译进 CLI |
| `versions.json` | 版本索引（latest、各平台 HTTPS 下载 URL、SHA256） |
| `versions.json.sig` | 对 `versions.json` 的 Ed25519 签名 |
| `signing_secret_key.b64` | **私钥，仅本地/CI 使用，勿提交**（已在 `.gitignore`） |

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

1. 构建各平台产物
2. 运行 `scripts/sign-release.sh` 为每个 `.tar.gz` / `.zip` 生成 `.sig`
3. 更新并签名 `release/versions.json`
4. 上传产物与 `versions.json` 到 GitHub Release
5. 将签名后的 `versions.json` 推回 `main`

## 手动签名（调试）

```bash
export ONEMINI_SIGNING_KEY="$(cat release/signing_secret_key.b64 | tr -d '\n')"
cargo run --bin onemini-sign -- sign --file path/to/onemini-x86_64-apple-darwin.tar.gz
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

- **HTTPS only**：所有 URL 必须为 `https://`，客户端启用 TLS 1.2+，禁止 HTTP 回退
- **签名校验优先**：SHA256 仅作辅助；安装前必须验证 `.sig`（Ed25519 over SHA256 digest）
- **索引防篡改**：先验证 `versions.json.sig`，再信任其中的 URL
