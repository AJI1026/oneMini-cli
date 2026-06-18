---
name: pdf
description: 处理 PDF：读/写/合并/拆分/旋转/水印/表单填写/表格提取/OCR/加密；用户提到 .pdf 或「PDF」时使用。
---

# PDF 处理

OneMini 内置 PDF 技能（Apache-2.0）。技能目录含 `scripts/`；Office 类操作见 `shared/office/`。

## 依赖

```bash
pip install pypdf pdfplumber reportlab pandas
# 可选: pdf2image poppler-utils(pdftoppm) tesseract(OCR)
```

## 路径约定

激活消息会给出**技能目录** `{PDF_ROOT}`（如 `…/skills/pdf`）。脚本示例：

```bash
python "{PDF_ROOT}/scripts/merge_pdfs.py" a.pdf b.pdf -o merged.pdf
```

## 快速参考

| 任务 | 方法 |
|------|------|
| 读文本 | `extract_text.py` 或 pypdf |
| 提取表格 | `extract_tables.py` + pdfplumber |
| 合并/拆分 | `merge_pdfs.py` / `split_pdf.py` |
| 表单字段 | `extract_form_field_info.py` → 编辑 JSON → `fill_fillable_fields.py` |
| 转图片 | `convert_pdf_to_images.py` |
| 创建 PDF | reportlab（见下） |
| 高级主题 | 读 `{PDF_ROOT}/reference.md` |
| 填表详解 | 读 `{PDF_ROOT}/forms.md` |

## pypdf 常用操作

### 合并

```bash
python "{PDF_ROOT}/scripts/merge_pdfs.py" part1.pdf part2.pdf -o out.pdf
```

### 拆分

```bash
python "{PDF_ROOT}/scripts/split_pdf.py" input.pdf ./pages/
python "{PDF_ROOT}/scripts/split_pdf.py" input.pdf ./pick/ --pages 1,3,5
```

### 提取文本

```bash
python "{PDF_ROOT}/scripts/extract_text.py" doc.pdf -o doc.txt
python "{PDF_ROOT}/scripts/extract_text.py" doc.pdf -o doc.txt --layout
```

### 元数据

```python
from pypdf import PdfReader
r = PdfReader("doc.pdf")
m = r.metadata
print(m.title, m.author, len(r.pages))
```

### 旋转

```python
from pypdf import PdfReader, PdfWriter
reader, writer = PdfReader("in.pdf"), PdfWriter()
page = reader.pages[0]
page.rotate(90)
writer.add_page(page)
with open("rotated.pdf", "wb") as f:
    writer.write(f)
```

### 加密/解密

```python
from pypdf import PdfReader, PdfWriter
r = PdfReader("in.pdf")
if r.is_encrypted:
    r.decrypt("password")
w = PdfWriter(clone_from=r)
w.encrypt("user-pass", "owner-pass")
with open("locked.pdf", "wb") as f:
    w.write(f)
```

## pdfplumber — 表格

```bash
python "{PDF_ROOT}/scripts/extract_tables.py" doc.pdf ./tables/ --format csv
python "{PDF_ROOT}/scripts/extract_tables.py" doc.pdf ./tables/ --format json
```

## reportlab — 创建 PDF

**禁止**在 ReportLab 中使用 Unicode 上下标字符（₈ 等），用 `<sub>` / `<super>` XML 标签：

```python
from reportlab.platypus import SimpleDocTemplate, Paragraph
from reportlab.lib.styles import getSampleStyleSheet
styles = getSampleStyleSheet()
story = [Paragraph("H<sub>2</sub>O", styles["Normal"])]
SimpleDocTemplate("out.pdf").build(story)
```

## 命令行工具（若已安装 poppler）

```bash
pdftotext input.pdf output.txt
pdftotext -layout input.pdf output.txt
pdftoppm -png -r 150 input.pdf page
```

## OCR（扫描件）

1. `convert_pdf_to_images.py` 转图片  
2. `tesseract page-1.png stdout -l chi_sim+eng`  
3. 可选：用 reportlab 或 pypdf 生成可搜索 PDF

## 工作流

1. 确认输入文件与目标（读/写/改/表单）  
2. 优先用 `scripts/` 中已有工具  
3. 表单任务**必须**先 `extract_form_field_info.py`  
4. 大文件分步处理，避免一次加载全部进上下文  
5. 输出后抽查：页数、文本抽样、表单字段

## 输出报告

```
## 操作
## 输入/输出文件
## 验证
```
