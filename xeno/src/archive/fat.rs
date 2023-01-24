use std::io::{Read, Seek, Write};
use std::path::Path;
use std::path::PathBuf;

use fatfs::{Date, DateTime, FileAttributes};

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

/// FAT12, FAT16, FAT32 compatibility
pub struct FatArchive<R: Read + Write + Seek> {
    reader: R,
}

#[derive(Debug, Clone)]
pub struct FatEntry {
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: String,
    modified: DateTime,
    accessed: Date,
    created: DateTime,
    attr: FileAttributes,
}

impl Entry for FatEntry {
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
        Ok(PathBuf::from(&self.path))
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

impl FatEntry {
    pub fn fat_readonly(&self) -> bool {
        (self.attr & FileAttributes::READ_ONLY).bits() == 1
    }

    pub fn fat_hidden(&self) -> bool {
        (self.attr & FileAttributes::HIDDEN).bits() == 1
    }

    pub fn fat_system(&self) -> bool {
        (self.attr & FileAttributes::SYSTEM).bits() == 1
    }

    pub fn fat_volume_id(&self) -> bool {
        (self.attr & FileAttributes::VOLUME_ID).bits() == 1
    }

    pub fn fat_directory(&self) -> bool {
        (self.attr & FileAttributes::DIRECTORY).bits() == 1
    }

    pub fn fat_archive(&self) -> bool {
        (self.attr & FileAttributes::ARCHIVE).bits() == 1
    }

    pub fn fat_lfn(&self) -> bool {
        (self.attr & FileAttributes::LFN).bits() == 1
    }
}

pub struct FatEntries {
    inner: Vec<FatEntry>,
    current: usize,
}

impl Iterator for FatEntries {
    type Item = Result<FatEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<FatEntry, ArchiveError>> {
        if self.current >= self.inner.len() {
            return None;
        }

        let entry = &self.inner[self.current];

        self.current += 1;
        Some(Ok(entry.to_owned()))
    }
}

impl<R> FatArchive<R>
where
    R: Read + Write + Seek,
{
    pub fn entries(&mut self) -> Result<FatEntries, ArchiveError> {
        let fs = fatfs::FileSystem::new(&mut self.reader, fatfs::FsOptions::new())
            .map_err(ArchiveError::Io)?;
        let root_dir = fs.root_dir();
        let mut fat_entries = vec![];
        for entry in root_dir.iter().flatten() {
            let wrapper_entry = FatEntry {
                is_dir: entry.is_dir(),
                is_file: entry.is_file(),
                size: entry.len(),
                path: entry.file_name(),
                modified: entry.modified(),
                accessed: entry.accessed(),
                created: entry.created(),
                attr: entry.attributes(),
            };
            fat_entries.push(wrapper_entry);
        }

        Ok(FatEntries {
            inner: fat_entries,
            current: 0,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to)?;
        }

        let fs = fatfs::FileSystem::new(&mut self.reader, fatfs::FsOptions::new())
            .map_err(ArchiveError::Io)?;
        let root_dir = fs.root_dir();

        let mut failures = vec![];
        for entry in root_dir.iter().flatten() {
            if entry.is_file() {
                let path = entry.file_name();
                let mut entry = entry.to_file();
                let dest = to.join(path);
                let mut writer = match std::fs::File::create(dest) {
                    Ok(writer) => writer,
                    Err(e) => {
                        let err = ArchiveError::Io(e);
                        failures.push(err);
                        continue;
                    }
                };
                let mut buf = [0u8; 4096];
                loop {
                    let bytes_read = match entry.read(&mut buf) {
                        Ok(size) => size,
                        Err(e) => {
                            let err = ArchiveError::Io(e);
                            failures.push(err);
                            break;
                        }
                    };

                    if bytes_read == 0 {
                        break;
                    }

                    writer.write(&buf[..bytes_read])?;
                }
            } else if entry.is_dir() {
                let path = entry.file_name();
                let dest = to.join(path);
                if let Err(e) = std::fs::create_dir_all(dest) {
                    let err = ArchiveError::Io(e);
                    failures.push(err);
                }
            } else {
                log::debug!("Unrecognized entry: {:?}", &entry);
            }
        }

        if !failures.is_empty() {
            let err = ArchiveError::ExtractFailed { sources: failures };
            return Err(err);
        }

        Ok(())
    }

    pub fn unpack_file(
        &mut self,
        entry: &FatEntry,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let fs = fatfs::FileSystem::new(&mut self.reader, fatfs::FsOptions::new())
            .map_err(ArchiveError::Io)?;
        let root_dir = fs.root_dir();
        if entry.is_dir {
            return Ok(());
        }

        let mut entry = root_dir.open_file(&entry.path).map_err(ArchiveError::Io)?;

        let mut writer = std::fs::File::create(to)?;
        let mut buf = [0u8; 4096];
        loop {
            let bytes_read = match entry.read(&mut buf) {
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

    pub fn create_with_path(
        path: impl AsRef<Path>,
        password: Option<Vec<u8>>,
    ) -> Result<FatArchive<impl Read + Write + Seek>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader, password)
    }

    pub fn create_with_reader(
        rdr: impl Read + Write + Seek,
        password: Option<Vec<u8>>,
    ) -> Result<FatArchive<impl Read + Write + Seek>, ArchiveError> {
        let archive = FatArchive { reader: rdr };
        Ok(archive)
    }
}
