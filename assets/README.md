# 品牌图标（与 oneMini-web 同源）

| 文件 | 用途 |
|------|------|
| `logo.png` | 源图，来自 `oneMini-web/public/logo/logo.png` |
| `onemini.ico` | Windows `onemini.exe` 图标（`build.rs` 编译时嵌入） |
| `AppIcon.icns` | macOS `OneMini.app` 图标 |

更新 logo 后重新生成：

```bash
./scripts/generate-brand-icons.sh
```

若 monorepo 内 `oneMini-web` 与 CLI 同级，脚本会自动从 web 复制最新 `logo.png`。
