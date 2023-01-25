use std::io::{BufReader, Read, Write};
use std::path::Path;

use snap::read::FrameDecoder;

use crate::utils::error::ArchiveError;

pub struct SnappyBlock<R: Read> {
    inner: FrameDecoder<R>,
}

impl<R> SnappyBlock<R>
where
    R: Read,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        std::io::copy(&mut self.inner, &mut writer)?;

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read) -> Result<SnappyBlock<impl Read>, ArchiveError> {
        let inner = snap::read::FrameDecoder::new(rdr);
        let block = SnappyBlock { inner };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<SnappyBlock<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
