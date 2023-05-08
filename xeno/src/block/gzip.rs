use std::io::{BufReader, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;

use crate::utils::error::ArchiveError;

pub struct GzipBlock<R: Read> {
    inner: GzDecoder<R>,
}

impl<R> GzipBlock<R>
where
    R: Read,
{
    pub fn unpack_to(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        let mut buf = [0u8; 4096];
        loop {
            let bytes_read = match self.inner.read(&mut buf) {
                Ok(size) => size,
                Err(e) => {
                    let err = ArchiveError::Io(e);
                    return Err(err);
                }
            };

            if bytes_read == 0 {
                break;
            }

            writer.write_all(&buf[..bytes_read])?;
        }

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read) -> Result<GzipBlock<impl Read>, ArchiveError> {
        let reader = GzDecoder::new(rdr);

        let block = GzipBlock { inner: reader };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<GzipBlock<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}
