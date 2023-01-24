use std::error::Error;
use std::io::{Read, Seek};
use std::path::Path;
use std::path::PathBuf;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

use delharc::LhaDecodeReader;

pub struct LhaArchive<R: Read + Seek> {
    inner: LhaDecodeReader<R>,
}

#[derive(Debug, Clone)]
pub struct LhaEntry {
    size: u64,
    path: PathBuf,
}

impl Entry for LhaEntry {
    fn file_type(&self) -> FileType {
        FileType::RegularFile
    }

    fn hand_link(&self) -> Option<PathBuf> {
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        Ok(self.path.clone())
    }

    fn gid(&self) -> std::io::Result<Option<u64>> {
        Ok(None)
    }

    fn uid(&self) -> std::io::Result<Option<u64>> {
        Ok(None)
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn sym_link(&self) -> Option<PathBuf> {
        None
    }
}

pub struct LhaEntries {
    inner: Vec<LhaEntry>,
    current: usize,
}

impl Iterator for LhaEntries {
    type Item = Result<LhaEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<LhaEntry, ArchiveError>> {
        if self.current >= self.inner.len() {
            return None;
        }

        let entry = &self.inner[self.current];
        self.current += 1;
        Some(Ok(entry.to_owned()))
    }
}

impl<R> LhaArchive<R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<LhaEntries, ArchiveError> {
        let mut lha_entries = vec![];
        loop {
            let header = self.inner.header();
            let entry = LhaEntry {
                path: header.parse_pathname(),
                size: header.original_size,
            };
            lha_entries.push(entry);

            if !self
                .inner
                .next_file()
                .map_err(|e| ArchiveError::GenericsError2(format!("{:?}", e)))?
            {
                break;
            }
        }

        Ok(LhaEntries {
            inner: lha_entries,
            current: 0,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to)?;
        }

        let mut failures = vec![];
        loop {
            let header = self.inner.header();
            let filename = header.parse_pathname();
            let dest = to.join(filename);

            if self.inner.is_decoder_supported() {
                let extraction_status = std::fs::File::create(&dest).and_then(|mut writer| {
                    std::io::copy(&mut self.inner, &mut writer).and_then(|_| self.inner.crc_check())
                });

                if let Err(e) = extraction_status {
                    failures.push(ArchiveError::Io(e));
                }
            } else if header.is_directory() {
                log::debug!("skipping: an empty directory");
            } else {
                eprintln!("skipping: has unsupported compression method");
            }

            if !self
                .inner
                .next_file()
                .map_err(|e| ArchiveError::GenericsError2(format!("{:?}", e)))?
            {
                break;
            }
        }

        if !failures.is_empty() {
            return Err(ArchiveError::ExtractFailed { sources: failures });
        }

        Ok(())
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<LhaArchive<impl Read + Seek>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }

    pub fn create_with_reader(
        rdr: impl Read + Seek,
    ) -> Result<LhaArchive<impl Read + Seek>, ArchiveError> {
        let inner = LhaDecodeReader::new(rdr)
            .map_err(|e| ArchiveError::GenericsError2(format!("{:?}", &e)))?;
        let archive = LhaArchive { inner };
        Ok(archive)
    }
}
