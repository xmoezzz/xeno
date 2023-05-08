use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use sevenz_rust::SevenZArchiveEntry;
use time::PrimitiveDateTime;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct SevenZipArchive<R: Read + Seek> {
    inner: sevenz_rust::SevenZReader<R>,
}

pub struct SevenZipEntry {
    index: usize,
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: PathBuf,
    inner_creation_date: u64,
    inner_last_modified_date: u64,
}

impl Entry for SevenZipEntry {
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

impl SevenZipEntry {
    pub fn creation_date(&self) -> Option<PrimitiveDateTime> {
        let dt = time::OffsetDateTime::from_unix_timestamp(self.inner_creation_date as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }

    pub fn last_modified_date(&self) -> Option<PrimitiveDateTime> {
        let dt = time::OffsetDateTime::from_unix_timestamp(self.inner_last_modified_date as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }
}

pub struct ZipEntries {
    entries: Vec<SevenZArchiveEntry>,
    current: usize,
}

impl Iterator for ZipEntries {
    type Item = Result<SevenZipEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<SevenZipEntry, ArchiveError>> {
        let total = self.entries.len();
        if self.current >= total {
            return None;
        }

        let entry = &self.entries[self.current];
        let entry = SevenZipEntry {
            index: self.current,
            is_dir: entry.is_directory(),
            is_file: !entry.is_directory(),
            size: entry.size(),
            path: PathBuf::from(entry.name()),
            inner_creation_date: entry.creation_date.into(),
            inner_last_modified_date: entry.last_modified_date.into(),
        };

        self.current += 1;
        Some(Ok(entry))
    }
}

impl<R> SevenZipArchive<R>
where
    R: Read + Seek,
{
    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to).map_err(ArchiveError::Io)?;
        }

        let mut failures = vec![];
        let _result = self.inner.for_each_entries(|entry, reader| {
            if !entry.is_directory() {
                let path = entry.name();
                let dest = to.join(path);
                let writer = match std::fs::File::create(dest) {
                    Ok(writer) => writer,
                    Err(e) => {
                        let err = ArchiveError::Io(e);
                        failures.push(err);
                        return Ok(true);
                    }
                };

                if entry.size() > 0 {
                    let mut writer = Vec::new();
                    match std::io::copy(reader, &mut writer) {
                        Ok(_) => {}
                        Err(e) => {
                            let err = ArchiveError::Io(e);
                            failures.push(err);
                        }
                    }
                }
            }
            Ok(true)
        });

        if !failures.is_empty() {
            return Err(ArchiveError::ExtractFailed { sources: failures });
        }

        Ok(())
    }

    pub fn entries(&mut self) -> std::io::Result<ZipEntries> {
        let mut archive_entries = vec![];
        let _result = self.inner.for_each_entries(|entry, _| {
            archive_entries.push(entry.to_owned());
            Ok(true)
        });

        Ok(ZipEntries {
            entries: archive_entries,
            current: 0,
        })
    }

    pub fn create_with_reader(
        reader: impl Read + Seek,
        size: u64,
        password: Option<String>,
    ) -> Result<SevenZipArchive<impl Read + Seek>, ArchiveError> {
        let password = password.unwrap_or_default();
        let p = password.as_str();
        let inner = sevenz_rust::SevenZReader::new(reader, size, p.into())
            .map_err(|e| ArchiveError::SevenZipError(e))?;
        let archive = SevenZipArchive { inner };
        Ok(archive)
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
        size: u64,
        password: Option<String>,
    ) -> Result<SevenZipArchive<impl Read + Seek>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader, size, password)
    }
}
