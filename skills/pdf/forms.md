# PDF 表单填写

## 流程

1. **发现字段**  
   ```bash
   python scripts/extract_form_field_info.py form.pdf -o fields.json
   ```
2. **编辑 JSON** — 为需要填写的项添加 `"value"`，保留 `field_id` 与 `page`
3. **填写**  
   ```bash
   python scripts/fill_fillable_fields.py form.pdf fields.json filled.pdf
   ```
4. **验证** — 用 `extract_form_field_info.py` 或 pdftotext 抽查

## JSON 示例

```json
[
  { "field_id": "name", "page": 1, "value": "张三" },
  { "field_id": "email", "page": 1, "value": "a@example.com" },
  { "field_id": "agree", "page": 2, "value": "Yes" }
]
```

## 字段类型

| type | 说明 |
|------|------|
| text | 任意字符串 |
| checkbox | 使用 JSON 中给出的 checked/unchecked 值 |
| choice | value 必须在 choice_options 列表内 |

## 非交互式 / 扁平表单

若 `get_fields()` 为空但视觉上有表单：

1. `convert_pdf_to_images.py` 生成预览，确认字段位置  
2. 用 reportlab 叠加文本层，或  
3. 提示用户该 PDF 为纯图像表单，需 OCR + 手动坐标标注

## 常见错误

- 页码与 `extract_form_field_info` 输出不一致 → 重新提取  
- 中文乱码 → 确认 PDF 嵌入字体；填写后 `set_need_appearances_writer(True)` 已在脚本中启用  
- 字段 ID 含空格 → 使用 JSON 中精确 `field_id`
