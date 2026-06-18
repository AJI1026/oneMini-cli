---
name: pptx
description: 创建/读取/编辑 .pptx：幻灯片、deck、演示文稿、模板、备注；用户提到 ppt/pptx/slides/deck 时使用。
---

# PPTX 技能

## 依赖

```bash
pip install python-pptx markitdown
npm install -g pptxgenjs   # 从零创建
# 系统: LibreOffice, poppler (pdftoppm)
```

## 快速参考

| 任务 | 指南 |
|------|------|
| 读文本 | `python -m markitdown file.pptx` |
| 缩略图 | `scripts/thumbnail.py` |
| 编辑模板 | 读 `{PPTX_ROOT}/editing.md` |
| 从零创建 | 读 `{PPTX_ROOT}/pptxgenjs.md` |

## 读取

```bash
python -m markitdown presentation.pptx
python "{PPTX_ROOT}/scripts/thumbnail.py" deck.pptx ./preview/
python "{SKILLS_ROOT}/shared/office/unpack.py" deck.pptx unpacked/
```

## 脚本

```bash
python "{PPTX_ROOT}/scripts/add_slide.py" in.pptx out.pptx --count 2
python "{PPTX_ROOT}/scripts/add_slide.py" in.pptx out.pptx --duplicate 3
python "{PPTX_ROOT}/scripts/clean.py" draft.pptx clean.pptx
```

## 设计原则（勿做 boring deck）

- 选定**内容相关**配色：一主色 (60–70%) + 辅色 + 强调色  
- 深色封面/封底 + 浅色内容（或全程深色 premium）  
- **每页必有视觉元素**（图、图表、图标、形状）  
- 标题 36–44pt，正文 14–16pt，左对齐正文  
- 避免标题下装饰线（AI 幻灯片典型特征）  
- 字体配对：Georgia+Calibri、Arial Black+Arial 等，勿全 Arial

配色示例：

| 主题 | 主色 | 辅色 | 强调 |
|------|------|------|------|
| Midnight | `#1E2761` | `#CADCFC` | `#FFFFFF` |
| Forest | `#2C5F2D` | `#97BC62` | `#F5F5F5` |
| Coral | `#F96167` | `#F9E795` | `#2F3C7E` |

## QA（必做）

**假设第一版有问题。** 必须：

1. `python -m markitdown output.pptx` — 漏字、乱序  
2. `thumbnail.py` — 重叠、对比度、空白过多  
3. `shared/office/validate.py output.pptx`  
4. 修复后重复 1–3

## python-pptx 片段

```python
from pptx import Presentation
from pptx.util import Inches, Pt

prs = Presentation()
slide = prs.slides.add_slide(prs.slide_layouts[1])
slide.shapes.title.text = "标题"
body = slide.placeholders[1].text_frame
body.text = "要点一"
p = body.add_paragraph()
p.text = "要点二"
prs.save("out.pptx")
```

详细编辑与 pptxgenjs 见 `editing.md`、`pptxgenjs.md`。
