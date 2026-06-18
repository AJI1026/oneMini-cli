# PDF 参考

## 水印

```python
from pypdf import PdfReader, PdfWriter
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter
import io

packet = io.BytesIO()
c = canvas.Canvas(packet, pagesize=letter)
c.setFont("Helvetica", 40)
c.setFillGray(0.5, 0.3)
c.drawCentredString(300, 400, "DRAFT")
c.save()
mark = PdfReader(packet).pages[0]

r, w = PdfReader("doc.pdf"), PdfWriter()
for page in r.pages:
    page.merge_page(mark)
    w.add_page(page)
with open("watermarked.pdf", "wb") as f:
    w.write(f)
```

## 书签 / 大纲

```python
from pypdf import PdfWriter
w = PdfWriter(clone_from=PdfReader("in.pdf"))
w.add_outline_item("Chapter 1", 0)
w.write(open("out.pdf", "wb"))
```

## 提取嵌入图片

```python
from pypdf import PdfReader
r = PdfReader("doc.pdf")
for i, page in enumerate(r.pages):
    for img in page.images:
        open(f"p{i}_{img.name}", "wb").write(img.data)
```

## JavaScript（pdf-lib）— 仅当 pypdf 不足时

```bash
npm install pdf-lib
```

适合精细坐标放置、现有 PDF 上绘制。Node 脚本应写入项目目录后 bash 执行。

## 性能

- 百页以上：按页流式处理，不要 `pages[:]` 一次进内存  
- 表格多：pdfplumber 较慢，可限定页码 `pages=[0,1,2]`  
- OCR：逐页 tesseract，合并为 `.txt` 或重建 PDF

## 故障排查

| 现象 | 处理 |
|------|------|
| extract_text 为空 | 扫描件 → OCR；或 pdftotext -layout |
| 合并后体积暴涨 | 重新压缩/打印为 PDF（soffice --convert-to pdf） |
| 表单填写不显示 | 已启用 appearances；用 Adobe/Preview 打开验证 |
