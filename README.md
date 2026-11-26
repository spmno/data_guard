# data_guard

一个用 Rust 编写的 Parquet 数据质量诊断工具。读取 Parquet 文件，对每一列做"体检"，输出空值率、重复率、健康评级和修复建议，帮助你在大模型训练 / 数据清洗之前快速发现数据问题。

## 功能

- **全文件流式扫描**：逐批读取、跨批次聚合统计，不受文件大小限制
- **文件信息**：行数、列数、文件大小、列类型
- **逐列诊断**（基于真实列值，支持所有 Arrow 数据类型）：
  - 空值率 = 空值数 / 总行数
  - 重复率 = 非空值中重复占比（`1 - 唯一率`）
  - 唯一值数量、非空值数量
  - 文本列统计：平均长度、是否含 HTML 标签
- **健康评级**（Good / Warning / Danger）：

  | 指标 | Good | Warning | Danger |
  |------|------|---------|--------|
  | 空值率 | < 5% | 5% ~ 30% | > 30% |
  | 重复率 | < 10% | 10% ~ 30% | > 30% |

- **修复建议**：空值过高建议删列或填充、重复过多建议去重、常量列建议删除、检测到 HTML 标签建议清洗
- **机器可读输出**：`--json` 输出完整报告，可直接接入数据流水线
- **CI 友好退出码**：`0` 健康（允许警告）、`1` 存在危险列、`2` 运行错误

## 快速开始

### 环境要求

- Rust 1.85+（edition 2024）
- Python 3 + pandas + pyarrow（仅用于生成测试数据，可选）

### 运行

```bash
# 先生成测试数据（可选，仓库里已经带了两份）
python generate_test_data.py

# 诊断一个 parquet 文件（默认全文件扫描）
cargo run -- test_healthy.parquet   # 健康数据
cargo run -- test_issues.parquet    # 含空值 + 重复的问题数据

# 常用选项
cargo run -- test_issues.parquet --limit 5000   # 只扫描前 5000 行
cargo run -- test_issues.parquet --json         # JSON 报告
```

输出示例：

```
data_guard 诊断报告
文件: test_issues.parquet (15.2 KB)
行数: 1000（扫描 1000） | 列数: 3 | 总体评级: 危险

[1/3] id (Int64)
    空值率      0.00%  [良好]
    重复率     29.90%  [警告]
    唯一值  701（非空 1000 / 总行 1000）
    建议:
      - 重复率 29.9%，建议关注重复数据

[2/3] title (LargeUtf8)
    空值率      0.00%  [良好]
    重复率      0.00%  [良好]
    唯一值  1000（非空 1000 / 总行 1000）
    文本统计  平均长度 8.89，无 HTML 标签

[3/3] content (LargeUtf8)
    空值率     42.00%  [危险]
    重复率      0.00%  [良好]
    唯一值  580（非空 580 / 总行 1000）
    文本统计  平均长度 4.00，无 HTML 标签
    建议:
      - 空值率 42.0%，建议删除此列或填充缺失值

汇总: 3 列中 1 列危险、1 列警告
```

## 图形界面（GUI）

`gui/` 是基于 [NSH](https://github.com/longbridge/nsh)（Native Shell）的纯 Rust 桌面应用：

- 系统文件对话框选择 Parquet 文件，诊断在后台线程池执行，界面不卡顿
- 逐列报告用虚拟列表展示（数百列也流畅滚动），健康徽章 / 比率条全取主题 token
- 与 CLI 共享同一套流式诊断引擎（path 依赖核心库，无重复实现）

```bash
# Linux 需要 GTK 4.14+ 开发包（sudo apt install libgtk-4-dev）
cargo run --manifest-path gui/Cargo.toml
```

GUI 是独立的 crate（自带 `[workspace]`，不在根 workspace 内）：默认构建、`cargo test`
与核心 CI 都不会拉取 GUI 的依赖树，保持轻量。

## 项目结构

```
src/
├── main.rs                  # 二进制入口：解析参数、退出码处理
├── lib.rs                   # 库入口（bin + lib 双 target）
├── cli.rs                   # CLI：参数、扫描执行、人类可读 / JSON 报告输出
├── app.rs                   # 数据模型（报告结构体，支持 serde 序列化）
├── parser/
│   └── parquet_reader.rs    # Parquet 读取：元信息 + 流式批次迭代器
└── diagnosis/
    └── engine.rs            # 流式诊断引擎：跨批次聚合、真实值去重、建议生成
tests/
└── parquet_fixtures.rs      # 基于 fixture 文件的端到端集成测试
gui/                        # NSH 图形界面（独立 crate，path 依赖核心库）
├── nsh.json / theme.json   # NSH 应用配置与健康色主题 token
├── DESIGN.md               # 设计体系与动效清单
├── src/lib.rs              # App 根组件：状态接线、文件选择、后台诊断
├── src/ui.rs               # 样式组件层（按钮 / 徽章 / 比率条）
├── src/report_view.rs      # 报告展示：摘要 + 逐列虚拟列表
├── src/main.rs             # 桌面启动器（GTK / AppKit / Win32）
└── tests/app.rs            # 无头端到端测试（真实引擎驱动完整流程）
```

## 测试与 CI

```bash
cargo test                                 # 核心单元测试 + 集成测试
cargo test --manifest-path gui/Cargo.toml  # GUI 无头端到端测试
cargo clippy                               # 静态检查
cargo fmt                                  # 格式化
```

- 单元测试覆盖引擎核心逻辑（空值/重复/常量列/HTML/跨批次聚合/边界情况）
- 集成测试基于仓库自带的 fixture 文件做端到端验证
- GUI 测试用 NSH 无头引擎注入文件对话框应答，驱动按钮 → 诊断 → 报告渲染的完整流程
- GitHub Actions：push / PR 自动跑 fmt + clippy + test（含 GUI job）

## 技术栈

- [parquet-rs](https://github.com/apache/arrow-rs) / Arrow — Parquet 文件读取与通用值格式化
- clap — 命令行参数
- serde / serde_json — 报告序列化
- anyhow — 错误处理
- [NSH](https://github.com/longbridge/nsh)（`gui/`，独立 crate）— 图形界面

> 默认构建不含 GUI：核心库只拉 parquet/arrow 依赖，CI 和日常构建保持轻量快速。

## 路线图 / TODO

- [ ] **GUI 打磨**：报告淡入过渡、扫描中进度条（indeterminate `Progress`）、空态图标
- [ ] **大文件内存优化**：高基数列的去重目前用 `HashSet<String>`，改为固定内存的哈希草图（HyperLogLog）
- [ ] **数值列统计**：min / max / 均值 / 方差 / 异常值
- [ ] **更多规则**：类型混用、分布漂移（两份数据对比）、空字符串与 null 的区分
- [ ] **增量导出**：清洗建议生成可执行脚本（去重 / 填充）
- [ ] **性能**：数值列直接用物理值去重（跳过字符串格式化）

## 已知限制

- HTML 检测是简易启发式（同时出现 `<` 和 `>`），可能误报
- 超大高基数列的去重集合会随唯一值数量线性占用内存
