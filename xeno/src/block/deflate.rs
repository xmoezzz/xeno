use std::io::{Read, BufReader, Write};
use std::path::Path;

use flate2::read::DeflateDecoder;

use crate::utils::error::ArchiveError;

pub struct FlateBlock<R: Read> {
    inner: DeflateDecoder<R>,
}

impl<R> FlateBlock<R> 
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

            writer.write(&buf[..bytes_read])?;
        }

        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read) -> Result<FlateBlock<impl Read>, ArchiveError> {
        let reader = DeflateDecoder::new(rdr);
        
        let block = FlateBlock {
            inner: reader,
        };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<FlateBlock<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}