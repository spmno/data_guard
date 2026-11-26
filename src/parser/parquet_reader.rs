//! Parquet 文件读取：文件元信息与流式批次迭代。

use anyhow::{Context, Result};
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::app::FileInfo;

/// 每个批次读取的行数
const BATCH_SIZE: usize = 8192;

/// Parquet 文件读取器。
///
/// `open` 时读取 footer 元数据（行数、schema、文件大小），
/// 之后可多次通过 [`batch_iter`] 流式读取数据。
pub struct ParquetReader {
    path: PathBuf,
    row_count: i64,
    schema: SchemaRef,
    file_size: u64,
}

impl ParquetReader {
    /// 打开 Parquet 文件并读取元数据。
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).with_context(|| format!("无法打开文件: {}", path.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .with_context(|| format!("不是有效的 Parquet 文件: {}", path.display()))?;
        let row_count = builder.metadata().file_metadata().num_rows();
        let schema = builder.schema().clone();
        let file_size = std::fs::metadata(path)
            .with_context(|| format!("无法读取文件元信息: {}", path.display()))?
            .len();

        Ok(Self {
            path: path.to_path_buf(),
            row_count,
            schema,
            file_size,
        })
    }

    /// 文件基础信息。
    pub fn file_info(&self) -> FileInfo {
        FileInfo {
            path: self.path.clone(),
            row_count: self.row_count,
            column_count: self.schema.fields().len(),
            file_size: self.file_size,
        }
    }

    /// 文件的 Arrow schema。
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    /// 文件总行数（来自 Parquet 元数据，不需扫描数据）。
    pub fn row_count(&self) -> i64 {
        self.row_count
    }

    /// 返回流式批次迭代器。
    ///
    /// `limit` 为 `Some(n)` 时最多产出 n 行（最后一个批次自动截断）；
    /// 为 `None` 时扫描全文件。
    pub fn batch_iter(&self, limit: Option<usize>) -> Result<BatchIter> {
        let file = File::open(&self.path)
            .with_context(|| format!("无法打开文件: {}", self.path.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .with_context(|| format!("不是有效的 Parquet 文件: {}", self.path.display()))?;
        let inner = builder.with_batch_size(BATCH_SIZE).build()?;
        Ok(BatchIter {
            inner,
            remaining: limit,
        })
    }
}

/// 逐批产出 `RecordBatch` 的迭代器，按行数上限自动截断最后一个批次。
pub struct BatchIter {
    inner: ParquetRecordBatchReader,
    remaining: Option<usize>,
}

impl Iterator for BatchIter {
    type Item = Result<RecordBatch>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == Some(0) {
            return None;
        }
        match self.inner.next() {
            None => None,
            Some(Err(err)) => Some(Err(anyhow::Error::new(err).context("读取数据批次失败"))),
            Some(Ok(batch)) => {
                if let Some(remaining) = self.remaining {
                    let rows = batch.num_rows();
                    if rows > remaining {
                        self.remaining = Some(0);
                        return Some(Ok(batch.slice(0, remaining)));
                    }
                    self.remaining = Some(remaining - rows);
                }
                Some(Ok(batch))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Option<PathBuf> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        path.exists().then(|| path)
    }

    #[test]
    fn reads_file_info() {
        let Some(path) = fixture("test_healthy.parquet") else {
            return;
        };
        let reader = ParquetReader::open(path).expect("打开文件失败");
        let info = reader.file_info();
        assert_eq!(info.row_count, 1000);
        assert_eq!(info.column_count, 3);
        assert!(info.file_size > 0);
        assert_eq!(reader.schema().fields().len(), 3);
    }

    #[test]
    fn limit_truncates_rows() {
        let Some(path) = fixture("test_healthy.parquet") else {
            return;
        };
        let reader = ParquetReader::open(path).expect("打开文件失败");

        let total: usize = reader
            .batch_iter(None)
            .expect("创建迭代器失败")
            .map(|b| b.expect("读取批次失败").num_rows())
            .sum();
        assert_eq!(total, 1000);

        let limited: usize = reader
            .batch_iter(Some(500))
            .expect("创建迭代器失败")
            .map(|b| b.expect("读取批次失败").num_rows())
            .sum();
        assert_eq!(limited, 500);
    }

    #[test]
    fn zero_limit_yields_no_batches() {
        let Some(path) = fixture("test_healthy.parquet") else {
            return;
        };
        let reader = ParquetReader::open(path).expect("打开文件失败");
        let count = reader.batch_iter(Some(0)).expect("创建迭代器失败").count();
        assert_eq!(count, 0);
    }

    #[test]
    fn invalid_file_is_rejected() {
        let err = ParquetReader::open("Cargo.toml");
        assert!(err.is_err());
    }
}
