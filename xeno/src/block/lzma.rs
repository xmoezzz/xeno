use std::io::{Read, BufRead, BufReader};
use std::path::Path;

use crate::utils::error::ArchiveError;

pub struct LzmaBlock<R: Read + BufRead> {
    inner: R,
}

impl<R> LzmaBlock<R> 
where
    R: Read + BufRead,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        lzma_rs::xz_decompress(&mut self.inner, &mut writer)
            .map_err(ArchiveError::LzmaError)?;

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read + BufRead) -> Result<LzmaBlock<impl Read + BufRead>, ArchiveError> {
        let block = LzmaBlock {
            inner: rdr,
        };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<LzmaBlock<impl Read + BufRead>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
