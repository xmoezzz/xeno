use std::io::{Read, BufReader, Write};
use std::path::Path;

use flate2::read::ZlibDecoder;

use crate::utils::error::ArchiveError;

pub struct ZlibBlock<R: Read> {
    inner: ZlibDecoder<R>,
}

impl<R> ZlibBlock<R> 
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

    pub fn create_with_reader(rdr: impl Read) -> Result<ZlibBlock<impl Read>, ArchiveError> {
        let reader = ZlibDecoder::new(rdr);
        
        let block = ZlibBlock {
            inner: reader,
        };

        Ok(block)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<ZlibBlock<impl Read>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}