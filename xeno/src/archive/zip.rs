use std::io::{Read, Seek};
use std::path::Path;
use std::path::PathBuf;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct ZipArchive<R: Read> {
    inner: zip::ZipArchive<R>,
    password: Option<Vec<u8>>,
}

pub struct ZipEntry {
    index: usize,
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: PathBuf,
    mode: Option<u32>,
}

impl Entry for ZipEntry {
    fn file_type(&self) -> FileType {
        if self.is_dir {
            return FileType::Directory;
        }
        if self.is_file {
            return FileType::RegularFile;
        }

        FileType::Other
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

impl ZipEntry {
    pub fn unix_mode(&self) -> Option<u32> {
        self.mode
    }
}

pub struct ZipEntries<'a, R> {
    inner: &'a mut zip::ZipArchive<R>,
    password: Option<Vec<u8>>,
    current: usize,
    total: usize,
}

impl<'a, R> Iterator for ZipEntries<'a, R>
where
    R: Seek + Read,
{
    type Item = Result<ZipEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<ZipEntry, ArchiveError>> {
        if self.current >= self.total {
            return None;
        }

        let entry = match &self.password {
            Some(password) => match self.inner.by_index_decrypt(self.current, password) {
                Ok(result) => {
                    result.ok()
                },
                _ => None,
            },
            _ => self.inner.by_index(self.current).ok(),
        };

        let entry = if let Some(entry) = entry {
            ZipEntry {
                index: self.current,
                is_dir: entry.is_dir(),
                is_file: entry.is_file(),
                size: entry.size(),
                path: PathBuf::from(entry.name()),
                mode: entry.unix_mode(),
            }
        } else {
            return None;
        };

        self.current += 1;
        Some(Ok(entry))
    }
}


impl<R> ZipArchive<R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<ZipEntries<R>, ArchiveError> {
        let total = self.inner.len();

        Ok(ZipEntries {
            inner: &mut self.inner,
            password: self.password.clone(),
            current: 0,
            total,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.extract(to).map_err(ArchiveError::ZipError)
    }

    pub fn unpack_file(&mut self, entry: &ZipEntry, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut reader = self.inner.by_index(entry.index)
            .map_err(ArchiveError::ZipError)?;
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut reader, &mut writer)
            .map_err(ArchiveError::Io)?;
        Ok(())
    }

    pub fn create_with_path(path: impl AsRef<Path>, password: Option<Vec<u8>>) -> Result<ZipArchive<impl Read + Seek>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader, password)
    }

    pub fn create_with_reader(rdr: impl Read + Seek, password: Option<Vec<u8>>) -> Result<ZipArchive<impl Read + Seek>, ArchiveError> {
        let inner = zip::ZipArchive::new(rdr)
            .map_err(ArchiveError::ZipError)?;
        let archive = ZipArchive {
            inner,
            password
        };
        Ok(archive)
    }
}
