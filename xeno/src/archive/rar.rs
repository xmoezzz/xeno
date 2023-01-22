use std::path::{PathBuf, Path};

use unrar::archive::OpenArchive;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct RarArchive {
    password: Option<String>,
    filepath: String,
}

pub struct RarEntry {
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: PathBuf,
}


impl Entry for RarEntry {
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


pub struct RarEntries {
    inner: OpenArchive,
}

impl RarEntries {
    fn next(&mut self) -> Option<zip::result::ZipResult<RarEntry>> {
        let entry = self.inner.next();
        if let Some(Ok(entry)) = entry {
            let rar_entry = RarEntry {
                is_dir: entry.is_directory(),
                is_file: entry.is_file(),
                size: entry.unpacked_size as u64,
                path: PathBuf::from(entry.filename),
            };
            return Some(Ok(rar_entry));
        }

        None
    }
}

impl RarArchive
{
    pub fn entries(&mut self) -> Result<RarEntries, ArchiveError> {
        let archive = self.open_archive();
        let lister = archive.list().map_err(ArchiveError::RarError);

        let lister = lister?;
        Ok(RarEntries {
            inner: lister,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let archive = self.open_archive();
        let to = to.as_ref().to_path_buf().into_os_string().into_string()
            .map_err(ArchiveError::OsString)?;
        let mut unpacker = archive.extract_to(to)
            .map_err(ArchiveError::RarError)?;
        let _ = unpacker.process()
            .map_err(ArchiveError::RarError2)?;
        Ok(())
    }

    fn open_archive(&self) -> unrar::Archive {
        let archive = match self.password.clone() {
            Some(password) => unrar::Archive::with_password(self.filepath.to_owned(), password),
            None => unrar::Archive::new(self.filepath.to_owned()),
        };

        archive
    }

    pub fn create_with_path(path: impl AsRef<Path>, password: Option<String>) -> Result<RarArchive, ArchiveError> {
        let archive = RarArchive {
            filepath: path.as_ref().to_path_buf().into_os_string().into_string()
                .map_err(ArchiveError::OsString)?,
            password: password,
        };

        Ok(archive)
    }
}
