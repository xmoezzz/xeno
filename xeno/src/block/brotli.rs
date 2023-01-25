use std::io::{BufReader, Read, Write};
use std::path::Path;

use crate::utils::error::ArchiveError;

pub struct BrotliBlock<R: Read> {
    inner: R,
}

impl<R> BrotliBlock<R>
where
    R: Read,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        brotli::BrotliDecompress(&mut self.inner, &mut writer)?;

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read) -> Result<BrotliBlock<impl Read>, ArchiveError> {
        let block = BrotliBlock { inner: rdr };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<BrotliBlock<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
