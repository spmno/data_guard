use anyhow::Result;
use parquet::file::reader::{FileReader, SerializedFileReader};
use std::fs::File;
use std::path::PathBuf;

pub struct ParquetReader {
    file_path: PathBuf,
    reader: SerializedFileReader<File>,
}

impl ParquetReader {
    pub fn new(file_path: PathBuf) -> Result<Self> {
        let file = File::open(&file_path)?;
        let reader = SerializedFileReader::new(file)?;
        Ok(Self {
            file_path,
            reader,
        })
    }

    pub fn get_file_info(&self) -> Result<(i64, usize, u64)> {
        let metadata = self.reader.metadata();
        let row_count = metadata.file_metadata().num_rows();
        let column_count = metadata.file_metadata().schema().get_fields().len();
        let file_size = std::fs::metadata(&self.file_path)?.len();
        Ok((row_count, column_count, file_size))
    }

    pub fn get_schema(&self) -> String {
        let metadata = self.reader.metadata();
        format!("{:?}", metadata.file_metadata().schema())
    }

    pub fn read_batches(&mut self, max_rows: usize) -> Result<Vec<arrow::record_batch::RecordBatch>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let file = std::fs::File::open(&self.file_path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        let mut batches = Vec::new();
        let mut total_rows = 0;

        while let Some(batch) = reader.next() {
            let batch = batch?;
            let batch_rows = batch.num_rows();
            
            if total_rows + batch_rows <= max_rows {
                batches.push(batch);
                total_rows += batch_rows;
            } else {
                let remaining = max_rows - total_rows;
                let sliced_batch = batch.slice(0, remaining);
                batches.push(sliced_batch);
                break;
            }
        }

        Ok(batches)
    }
}
