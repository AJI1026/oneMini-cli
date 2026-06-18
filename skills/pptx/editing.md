# PPTX 编辑工作流（模板 / 现有文件）

## 步骤

1. **视觉摸底**  
   ```bash
   python scripts/thumbnail.py template.pptx ./preview/
   ```
2. **解包**  
   ```bash
   python ../shared/office/unpack.py template.pptx unpacked/
   ```
3. **分析** — 查看 `ppt/slides/slide*.xml`、`ppt/slideLayouts/`  
4. **改内容** — 优先 python-pptx；精细 XML 改动在 unpacked 中进行  
5. **清理**  
   ```bash
   python scripts/clean.py draft.pptx cleaned.pptx
   ```
6. **打包**（若走了 XML 路径）  
   ```bash
   python ../shared/office/pack.py unpacked/ output.pptx
   ```
7. **校验 + markitdown + thumbnail QA**

## 复制版式页

```bash
python scripts/add_slide.py base.pptx out.pptx --duplicate 2
```

## 注意

- 不要破坏 `ppt/_rels/*.rels` 引用  
- 改 `slideLayout` 影响所有使用该版式的页  
- 图片在 `ppt/media/`，新增图片需更新 `[Content_Types].xml` 与 rels — **用 python-pptx 更 safe**  
- speaker notes 在 `ppt/notesSlides/`

## 合并多个 PPTX

用 python-pptx 逐页复制 shape element（见 `add_slide.py --duplicate`），或 LibreOffice 宏；避免手动拼 ZIP。
