# 用 pptxgenjs 从零创建

```bash
npm install pptxgenjs
```

## 最小示例

```javascript
const pptxgen = require("pptxgenjs");
const pres = new pptxgen();

pres.defineSlideMaster({
  title: "MASTER",
  background: { color: "1E2761" },
});

let slide = pres.addSlide({ masterName: "MASTER" });
slide.addText("标题", { x: 0.5, y: 0.4, w: 9, h: 1, fontSize: 36, color: "FFFFFF", bold: true });
slide.addText("副标题", { x: 0.5, y: 1.5, w: 9, fontSize: 18, color: "CADCFC" });

slide = pres.addSlide();
slide.background = { color: "F5F5F5" };
slide.addText("内容页", { x: 0.5, y: 0.3, w: 9, fontSize: 28, color: "1E2761" });
slide.addShape(pres.shapes.RECTANGLE, { x: 5, y: 1.2, w: 4, h: 3.5, fill: { color: "CADCFC" } });

pres.writeFile({ fileName: "deck.pptx" });
```

## 规则

- 单位：英寸  
- `margin: 0` 对齐形状与文本框边缘  
- 每页至少一个 `addImage` / `addShape` / chart  
- 导出后 **必须** QA：`markitdown` + `thumbnail.py` + `validate.py`

## 图表

```javascript
slide.addChart(pres.charts.BAR, [{ name: "Q1", labels: ["A","B"], values: [10, 20] }], {
  x: 0.5, y: 1.5, w: 5, h: 4,
});
```

## 常见坑

- 文本框默认 padding 导致「对不齐」→ 设 `margin: 0` 或手动 offset  
- 低对比度文字 — 深底深字 / 浅底浅字  
- 全文件同一 layout — 至少交替 2–3 种版式
