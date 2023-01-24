use std::io::{BufReader, Read, Write};
use std::path::Path;

use lz4::Decoder;

use crate::utils::error::ArchiveError;

pub struct Lz4Block<R: Read> {
    inner: Decoder<R>,
}

impl<R> Lz4Block<R>
where
    R: Read,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        std::io::copy(&mut self.inner, &mut writer)?;

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read) -> Result<Lz4Block<impl Read>, ArchiveError> {
        let reader = Decoder::new(rdr)?;

        let block = Lz4Block { inner: reader };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<Lz4Block<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
