# oneMini-cli 文档索引

与 `oneMini-web/docs/` 相同，按用途分类；各子项目 `README.md` 为入口，此处仅作导航。

| 路径 | 说明 |
|------|------|
| [assets/](./assets/) | README 配图（SVG 源文件 + PNG 导出） |
| [../assets/README.md](../assets/README.md) | 品牌图标（logo / ico / icns） |
| [../release/README.md](../release/README.md) | 发布签名、`versions.json` 与 CI 流程 |
| [../ONEMINI.md](../ONEMINI.md) | 项目上下文（CLI 自动加载） |
| [../skills/](../skills/) | Agent 技能包（`SKILL.md` 等） |

## 配图维护

```bash
./scripts/generate-readme-images.sh   # SVG → PNG
./scripts/generate-brand-icons.sh     # 同步 web 端 logo
```
