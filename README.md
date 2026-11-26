# data_guard

一个用 Rust 编写的 Parquet 数据质量诊断工具。读取 Parquet 文件，对每一列做"体检"，输出空值率、重复率、健康评级和修复建议，帮助你在大模型训练 / 数据清洗之前快速发现数据问题。

## 功能

- **文件信息**：行数、列数、文件大小、Schema
- **逐列诊断**：
  - 空值率（null rate）
  - 重复率（duplicate rate）
  - 唯一值数量（unique count）
  - 文本列统计：平均长度、是否含 HTML 标签
- **健康评级**（Good / Warning / Danger）：

  | 指标 | Good | Warning | Danger |
  |------|------|---------|--------|
  | 空值率 | < 5% | 5% ~ 30% | > 30% |
  | 重复率 | < 10% | 10% ~ 30% | > 30% |

- **修复建议**：例如空值过高建议删列或填充、重复过多建议去重、检测到 HTML 标签建议清洗

## 快速开始

### 环境要求

- Rust（edition 2024，建议较新的 stable 工具链）
- Python 3 + pandas + pyarrow（仅用于生成测试数据，可选）

### 运行

```bash
# 先生成测试数据（可选，仓库里已经带了两份）
python generate_test_data.py

# 诊断一个 parquet 文件
cargo run -- test_healthy.parquet   # 健康数据
cargo run -- test_issues.parquet    # 含空值 + 重复的问题数据
```

输出示例：

```
File Info:
Row count: 1000
Column count: 3
...

Diagnosis Report:

Column: content
  Null rate: 42.00%
  Duplicate rate: 0.00%
  Health status: Null=Danger, Duplicate=Good
  Suggestions:
    - 空值率 42.00%，建议删除此列或均值填充
```

> 当前 CLI 默认只读取前 1000 行做诊断。

## 项目结构

```
src/
├── main.rs                  # 入口：解析命令行参数，走 CLI 流程
├── cli.rs                   # CLI：打印文件信息与诊断报告
├── app.rs                   # 应用数据模型（AppState / ColumnReport / HealthScore 等）
├── parser/
│   └── parquet_reader.rs    # Parquet 读取封装（基于 parquet + arrow）
├── diagnosis/
│   └── engine.rs            # 诊断引擎：空值率、重复率、文本统计、建议生成
└── ui/                      # GPUI 图形界面（开发中，尚未接入 main）
    ├── main_window.rs
    └── table_view.rs
```

## 技术栈

- [parquet-rs](https://github.com/apache/arrow-rs) / Arrow — Parquet 文件读取
- [gpui](https://github.com/zed-industries/zed)（Zed 的 UI 框架）— 图形界面
- anyhow — 错误处理

## 路线图 / TODO

- [ ] 接入 GPUI 图形界面（窗口 + 表格视图）
- [ ] 诊断覆盖全量数据，而不是前 1000 行
- [ ] 数值列统计（min / max / 均值 / 异常值）
- [ ] 分布漂移检测、类型混用检测
- [ ] 修复建议一键导出（如清洗脚本）

## 已知问题

- 重复率统计目前基于占位值（`value_{i}`），实际重复检测尚未使用真实列值，结果不可信
- `main.rs` 暂未引用 `src/ui`，GUI 模块还未编译进二进制
