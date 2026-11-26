//! data_guard —— Parquet 数据质量诊断核心库。
//!
//! 结构：
//! - [`parser`]：Parquet 文件读取（文件元信息 + 流式批次迭代）
//! - [`diagnosis`]：流式诊断引擎（空值率 / 重复率 / 文本统计 / 建议）
//! - [`app`]：数据模型（报告结构体，支持 serde 序列化）
//! - [`cli`]：命令行入口

pub mod app;
pub mod cli;
pub mod diagnosis;
pub mod parser;
