---
name: docx
description: 创建/读取/编辑 Word (.docx)：报告、备忘录、信函、目录、修订、批注、表格、图片；用户提到 Word/docx 时使用。
---

# DOCX 创建、编辑与分析

`.docx` 是 ZIP + XML。共享工具位于 `{SKILLS_ROOT}/shared/office/`。

## 依赖

```bash
pip install python-docx defusedxml
npm install -g docx   # docx-js，用于从零生成复杂版式
# 系统: LibreOffice (soffice), pandoc（可选）
```

## 快速参考

| 任务 | 方法 |
|------|------|
| 读正文 | pandoc 或 python-docx |
| 读 XML/修订 | `shared/office/unpack.py` |
| 写回 | `shared/office/pack.py` |
| 校验 | `shared/office/validate.py` |
| 接受修订 | `scripts/accept_changes.py` |
| 新建复杂文档 | docx-js（见下） |
| 简单读写 | python-docx |

## 共享 Office 脚本

```bash
OFFICE="{SKILLS_ROOT}/shared/office"
python "$OFFICE/unpack.py" doc.docx unpacked/
# 编辑 unpacked/word/document.xml 等
python "$OFFICE/pack.py" unpacked/ doc-edited.docx
python "$OFFICE/validate.py" doc-edited.docx
```

## 读取

```bash
pandoc --track-changes=all doc.docx -o out.md
python -c "from docx import Document; d=Document('doc.docx'); print('\n'.join(p.text for p in d.paragraphs))"
```

## 接受全部修订

```bash
python "{DOCX_ROOT}/scripts/accept_changes.py" tracked.docx clean.docx
```

## 转 PDF / 图片预览

```bash
python "{SKILLS_ROOT}/shared/office/soffice.py" doc.docx --to pdf
pdftoppm -jpeg -r 150 doc.pdf page
```

## 用 python-docx 创建（简单文档）

```python
from docx import Document
from docx.shared import Inches, Pt

doc = Document()
doc.add_heading("报告标题", 0)
doc.add_paragraph("正文段落。")
table = doc.add_table(rows=2, cols=2)
table.cell(0, 0).text = "A"
doc.save("out.docx")
```

## 用 docx-js 创建（专业版式）

安装：`npm install docx`

**关键规则**

1. **页面尺寸** — docx-js 默认 A4；美 Letter 需显式 DXA（1440 DXA = 1 inch）  
2. **列表** — 禁止手写 `•`；用 `LevelFormat.BULLET` 编号配置  
3. **标题** — 用 `Heading1`/`Heading2` 样式 ID 以支持目录  
4. **字体** — 默认 Arial  
5. 生成后 **必须** `validate.py`

```javascript
const { Document, Packer, Paragraph, TextRun, HeadingLevel } = require("docx");
const fs = require("fs");

const doc = new Document({
  sections: [{
    properties: {
      page: {
        size: { width: 12240, height: 15840 },
        margin: { top: 1440, right: 1440, bottom: 1440, left: 1440 },
      },
    },
    children: [
      new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("标题")] }),
    ],
  }],
});
Packer.toBuffer(doc).then(b => fs.writeFileSync("out.docx", b));
```

### 常见纸张（DXA）

| 纸张 | 宽 | 高 |
|------|-----|-----|
| US Letter | 12240 | 15840 |
| A4 | 11906 | 16838 |

## 编辑已有文档（XML 工作流）

1. `unpack.py` → 编辑 `word/document.xml`、`word/styles.xml`  
2. `pack.py` → `validate.py`  
3. 失败则对比原始 XML 结构，最小 diff 修复

## 查找替换

优先 python-docx：

```python
from docx import Document
doc = Document("in.docx")
for p in doc.paragraphs:
    if "OLD" in p.text:
        for run in p.runs:
            run.text = run.text.replace("OLD", "NEW")
doc.save("out.docx")
```

## 图片

```python
doc.add_picture("logo.png", width=Inches(2))
```

## 禁止

- 未 validate 就交付复杂 docx-js 产物  
- 在 Word XML 中插入未转义的特殊字符  
- 将 `.doc` 当 `.docx` 编辑 — 先 `soffice.py --to docx`

## 交付前 QA

- [ ] validate.py 通过  
- [ ] pandoc 或 python-docx 抽查段落  
- [ ] 若有目录，在 Word/LibreOffice 中更新域（或说明用户需 F9 更新）
