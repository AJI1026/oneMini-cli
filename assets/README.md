# 品牌图标（与 oneMini-web 同源）

| 文件 | 用途 |
|------|------|
| `logo.png` | 源图；优先从 `oneMini-web/public/logo/logo.png` 复制，或由 `om-app-icon.svg` 栅格化生成 |
| `onemini.ico` | Windows `onemini.exe` 图标（`build.rs` 编译时嵌入） |
| `AppIcon.icns` | macOS `OneMini.app` 图标 |

Web 端静态图标：

| 文件 | 用途 |
|------|------|
| `oneMini-web/public/logo/favicon.svg` | 浏览器 favicon（源：`src/branding/om-favicon.svg`，裁剪 viewBox 放大显示） |
| `oneMini-web/public/logo/om-app-icon.svg` | CLI / PWA / 桌面应用图标（圆角方底 + OM monogram） |

更新 logo 后重新生成：

```bash
./scripts/generate-brand-icons.sh
```

脚本会按以下顺序解析 `logo.png`：

1. 已有 `assets/logo.png`
2. 复制 `oneMini-web/public/logo/logo.png`
3. 从 `oneMini-web/public/logo/om-app-icon.svg` 栅格化为 512×512 PNG（需 `rsvg-convert`、ImageMagick 或 macOS `qlmanage`）

若 monorepo 内 `oneMini-web` 与 CLI 同级，脚本会自动读取 web 端最新图标资源。
