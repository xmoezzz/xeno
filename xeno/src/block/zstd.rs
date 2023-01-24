use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::utils::error::ArchiveError;

pub struct ZstdBlock<R: Read + BufRead> {
    inner: R,
}

impl<R> ZstdBlock<R>
where
    R: Read + BufRead,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        zstd::stream::copy_decode(&mut self.inner, &mut writer).map_err(ArchiveError::Io)?;

        Ok(())
    }

    pub fn create_with_reader(
        rdr: impl Read + BufRead,
    ) -> Result<ZstdBlock<impl Read + BufRead>, ArchiveError> {
        let block = ZstdBlock { inner: rdr };

        Ok(block)
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<ZstdBlock<impl Read + BufRead>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
