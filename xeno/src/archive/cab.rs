use std::io::{Seek, Read, BufReader};
use std::path::{Path, PathBuf};

use cab::Cabinet;
use time::PrimitiveDateTime;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct CabArchive<R: Read + Seek> {
    inner: Cabinet<R>,
}

#[derive(Debug, Clone)]
pub struct CabEntry {
    filename: String,
    size: u64,
    name_utf: bool,
    read_only: bool,
    hidden: bool,
    system: bool,
    archive: bool,
    exec: bool,
    time: Option<PrimitiveDateTime>,
}

impl CabEntry {
    /// Returns true if this file has the "name is UTF" attribute set.
    pub fn is_name_utf(&self) -> bool {
        self.name_utf
    }

    /// Returns true if this file has the "read-only" attribute set.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Returns true if this file has the "hidden" attribute set.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Returns true if this file has the "system file" attribute set.
    pub fn is_system(&self) -> bool {
        self.system
    }

    /// Returns true if this file has the "archive" (modified since last
    /// backup) attribute set.
    pub fn is_archive(&self) -> bool {
        self.archive
    }

    /// Returns true if this file has the "execute after extraction" attribute
    /// set.
    pub fn is_exec(&self) -> bool {
        self.exec
    }

    pub fn datatime(&self) -> Option<PrimitiveDateTime> {
        self.time.clone()
    }
}

impl Entry for CabEntry {
    fn file_type(&self) -> FileType {
        FileType::RegularFile
    }

    fn hand_link(&self) -> Option<PathBuf> {
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        Ok(PathBuf::from(&self.filename))
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

pub struct CabEntries {
    current: usize,
    total: usize,
    inner: Vec<CabEntry>,
}

impl Iterator for CabEntries {
    type Item = Result<CabEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<CabEntry, ArchiveError>> {
        if self.current >= self.total {
            return None;
        }

        let entry = &self.inner[self.current];
        self.current += 1;
        Some(Ok(entry.to_owned()))
    }
}


impl<R> CabArchive<R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<CabEntries, ArchiveError> {
        let mut entries = vec![];
        for folder in self.inner.folder_entries() {
            for entry in folder.file_entries() {
                let entry = CabEntry {
                    filename: entry.name().to_string(),
                    size: entry.uncompressed_size() as u64,
                    name_utf: entry.is_name_utf(),
                    read_only: entry.is_read_only(),
                    hidden: entry.is_hidden(),
                    system: entry.is_system(),
                    archive: entry.is_archive(),
                    exec: entry.is_exec(),
                    time: entry.datetime(),
                };
                entries.push(entry);
            }
        }

        Ok(CabEntries {
            current: 0,
            total: entries.len(),
            inner: entries,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let entries = self.entries()?;
        let mut failures = vec![];
        for entry in entries {
            match entry {
                Ok(entry) => {
                    let to_path = to.as_ref().to_path_buf();
                    let path = to_path.join(&entry.filename);
                    if let Err(e) = self.unpack_file(&entry, path) {
                        failures.push(e);
                    }
                },
                Err(e) => {
                    failures.push(e);
                }
            }
        }

        if !failures.is_empty() {
            return Err(ArchiveError::ExtractFailed { sources: failures });
        }

        Ok(())
    }

    pub fn unpack_file(&mut self, entry: &CabEntry, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let mut reader = self.inner.read_file(&entry.filename)?;
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut reader, &mut writer)?;
        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read + Seek) -> Result<CabArchive<impl Read + Seek>, ArchiveError> {
        let reader = cab::Cabinet::new(rdr)
            .map_err(ArchiveError::Io)?;
        
        let archive = CabArchive {
            inner: reader,
        };

        Ok(archive)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<CabArchive<impl Read + Seek>, ArchiveError> {
        let rdr = BufReader::new(std::fs::File::open(path)?);
        Self::create_with_reader(rdr)
    }
}


