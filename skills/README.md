# OneMini CLI — Agent Skills

内置技能遵循 [Agent Skills](https://github.com/anthropics/skills) 格式：每个技能一个目录，包含 `SKILL.md`（YAML frontmatter + Markdown 指令）。

## 内置技能

### 编程向（编译进二进制）

| 技能 | 说明 |
|------|------|
| `commit-message` | 根据 git diff 生成 commit message |
| `code-review` | 结构化代码审查 |
| `debug` | 复现 → 定位 → 修复 → 验证 |
| `refactor` | 安全小步重构 |
| `readme` | 编写 README |
| `explore-codebase` | 快速理解陌生代码库 |

### 文档四件套（随包 `skills/` 目录，Apache-2.0）

| 技能 | 说明 |
|------|------|
| `pdf` | 合并/拆分/表单/表格/OCR；`scripts/` + `reference.md` + `forms.md` |
| `docx` | Word 读写/修订/docx-js；`shared/office/` unpack/pack/validate |
| `pptx` | 幻灯片编辑/创建；`editing.md` + `pptxgenjs.md` |
| `xlsx` | Excel 公式/格式；`scripts/recalc.py`（LibreOffice） |

发布包内 `onemini` 与 `skills/` 同级；开发时 `cargo run` 自动使用 crate 内 `skills/`。

**首次启动**若本地无技能脚本，会从 GitHub 自动下载（含进度提示），并尝试 `pip install` Python 依赖。跳过：`ONEMINI_SKIP_SKILL_BOOTSTRAP=1` / `ONEMINI_SKIP_PYTHON_DEPS=1`。

## 使用

```bash
onemini
/skills                    # 列出技能
/pdf 合并 a.pdf b.pdf      # 或自然语言「合并这两个 pdf」
onemini skills show pdf    # 查看 SKILL.md
```

## 依赖（文档类）

| 技能 | Python / 系统 |
|------|----------------|
| pdf | pypdf, pdfplumber, reportlab；可选 poppler, tesseract |
| docx | python-docx, defusedxml；可选 pandoc, Node docx |
| pptx | python-pptx, markitdown；可选 pptxgenjs, LibreOffice |
| xlsx | openpyxl, pandas；LibreOffice（公式重算） |

## 首次配置预装（设计类）

`onemini init` 后会从 anthropics/skills 预装 frontend-design 等 5 个设计技能。跳过：`ONEMINI_SKIP_SKILL_BOOTSTRAP=1`。

## 可选：Anthropic 官方增强版

```bash
onemini skills install docs    # 安装 Anthropic 版 docx/pdf/pptx/xlsx（覆盖内置）
onemini skills install design
```

安装到用户配置目录；与内置同名时**用户版优先**。

## 自定义技能

| 位置 | 作用域 |
|------|--------|
| `~/.config/onemini/skills/` | 用户全局 |
| `.onemini/skills/` | 当前项目 |

模板见各内置 `SKILL.md` frontmatter。
