---
name: xlsx
description: 打开/编辑/创建 Excel (.xlsx/.xlsm/.csv)：公式、格式、图表、清洗数据；用户提到 xlsx/excel/表格 且交付物为电子表格时使用。
---

# XLSX 创建、编辑与分析

## 依赖

```bash
pip install openpyxl pandas
# 公式重算: LibreOffice (soffice)
```

## 核心原则

### 必须用 Excel 公式，禁止 Python 算完硬编码

```python
# ❌ sheet["B10"] = df["Sales"].sum()
# ✅ sheet["B10"] = "=SUM(B2:B9)"
```

### 含公式时 MUST 重算

```bash
python "{XLSX_ROOT}/scripts/recalc.py" output.xlsx
```

返回 JSON：`status: ok` 或 `errors_found` + `error_summary`。修复后重复 recalc。

## 输出质量

- 专业字体（Arial / 等线）  
- **零公式错误**（#REF! #DIV/0! 等）  
- 改模板时 **完全保留** 原格式与 conventions  

### 财务模型配色（无模板时）

| 含义 | 颜色 |
|------|------|
| 硬编码输入 | 蓝色字体 |
| 公式 | 黑色 |
| 跨表链接 | 绿色 |
| 外链 | 红色 |
| 关键假设 | 黄色底 |

### 数字格式

- 年份：文本 `"2024"`  
- 货币：`$#,##0`，单位写在表头  
- 零显示为 `-`  
- 百分比：0.0%  
- 负数：括号 `(123)`

## pandas — 分析与清洗

```python
import pandas as pd
df = pd.read_excel("messy.xlsx", sheet_name=0)
df = df.dropna(how="all")
df.to_excel("clean.xlsx", index=False)
```

## openpyxl — 公式与格式

```python
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill

wb = Workbook()
ws = wb.active
ws["A1"] = "Revenue ($mm)"
ws["A2"] = 100
ws["A3"] = 120
ws["A4"] = "=SUM(A2:A3)"
ws["A4"].font = Font(color="000000")
wb.save("model.xlsx")
```

```bash
python "{XLSX_ROOT}/scripts/recalc.py" model.xlsx
```

## 读取现有文件

```python
from openpyxl import load_workbook
wb = load_workbook("in.xlsx", data_only=False)  # 看公式
wb_val = load_workbook("in.xlsx", data_only=True)  # 看缓存值
```

## 图表

用 openpyxl.chart 或导出数据到 sheet 后插入 `BarChart` / `LineChart`；大数据用 pandas 聚合后再写表。

## 工作流

1. 明确：新建 / 改模板 / 清洗 CSV  
2. 选工具：pandas 清洗 → openpyxl 公式与格式  
3. 保存  
4. **recalc.py**  
5. 若有错误，按 `error_summary` 定位单元格修复  
6. 抽样验证关键单元格

## 硬编码文档

关键假设旁注释：`Source: 10-K FY2024, p.45, …`

## 禁止

- Python 计算合计后写入静态值（除非用户明确要求静态快照）  
- 破坏已有模板的颜色/格式约定  
- 跳过 recalc 交付含公式的文件
