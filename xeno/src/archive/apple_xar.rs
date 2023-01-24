use std::fmt::Debug;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use apple_xar::reader::XarReader;
use apple_xar::table_of_contents::File;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct XarArchive<R: Read + Seek + Sized + Debug> {
    inner: XarReader<R>,
}

pub struct XarEntry {
    inner: File,
    filename: String,
}

impl Entry for XarEntry {
    fn file_type(&self) -> FileType {
        match self.inner.file_type {
            apple_xar::table_of_contents::FileType::File => FileType::RegularFile,
            apple_xar::table_of_contents::FileType::Directory => FileType::Directory,
            apple_xar::table_of_contents::FileType::HardLink => FileType::HardLink,
            apple_xar::table_of_contents::FileType::Link => FileType::SymbolicLink,
        }
    }

    fn hand_link(&self) -> Option<PathBuf> {
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        Ok(PathBuf::from(&self.filename))
    }

    fn gid(&self) -> std::io::Result<Option<u64>> {
        Ok(self.inner.gid.map(|gid| gid as u64))
    }

    fn uid(&self) -> std::io::Result<Option<u64>> {
        Ok(self.inner.uid.map(|uid| uid as u64))
    }

    fn size(&self) -> u64 {
        self.inner.size.unwrap_or_default()
    }

    fn sym_link(&self) -> Option<PathBuf> {
        None
    }
}

pub struct XarEntries {
    inner: Vec<(String, apple_xar::table_of_contents::File)>,
    current: usize,
    total: usize,
}

impl Iterator for XarEntries {
    type Item = Result<XarEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<XarEntry, ArchiveError>> {
        if self.current >= self.total {
            return None;
        }

        let (filename, entry) = &self.inner[self.current];
        let entry = XarEntry {
            inner: entry.clone(),
            filename: filename.clone(),
        };

        self.current += 1;
        Some(Ok(entry))
    }
}

impl<R> XarArchive<R>
where
    R: Read + Seek + Sized + Debug,
{
    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.unpack(to).map_err(ArchiveError::XarError)
    }

    pub fn unpack_file(
        &mut self,
        entry: &XarEntry,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let dest_dir = to.as_ref();
        let dest_path = dest_dir.join(&entry.filename);
        match entry.inner.file_type {
            apple_xar::table_of_contents::FileType::Directory => {
                std::fs::create_dir(&dest_path)?;
            }
            apple_xar::table_of_contents::FileType::File => {
                let mut fh = std::fs::File::create(&dest_path)?;
                let _ = self
                    .inner
                    .write_file_data_decoded_from_file(&entry.inner, &mut fh)
                    .map_err(ArchiveError::XarError)?;
            }
            apple_xar::table_of_contents::FileType::HardLink => {
                return Err(ArchiveError::XarError(apple_xar::Error::Unsupported(
                    "writing hard links",
                )))
            }
            apple_xar::table_of_contents::FileType::Link => {
                return Err(ArchiveError::XarError(apple_xar::Error::Unsupported(
                    "writing symlinks",
                )))
            }
        };
        Ok(())
    }

    pub fn entries(&mut self) -> Result<XarEntries, ArchiveError> {
        let entries = self.inner.files().map_err(ArchiveError::XarError)?;

        Ok(XarEntries {
            current: 0,
            total: entries.len(),
            inner: entries,
        })
    }

    pub fn create_with_reader(
        reader: impl Read + Seek + Debug + Sized,
    ) -> Result<XarArchive<impl Read + Seek + Debug + Sized>, ArchiveError> {
        let reader = apple_xar::reader::XarReader::new(reader).map_err(ArchiveError::XarError)?;

        let archive = XarArchive { inner: reader };

        Ok(archive)
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<XarArchive<impl Read + Seek + Debug + Sized>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}
