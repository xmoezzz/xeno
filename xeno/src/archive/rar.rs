use std::io::Seek;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use unrar::archive::OpenArchive;

use crate::archive::{Archive, Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct RarArchive<'a, R: Read + Seek> {
    password: Option<String>,
    filepath: String,
    _mark: std::marker::PhantomData<&'a R>,
}

pub struct RarEntry<'a, R: Read + Seek> {
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: PathBuf,
    _mark: std::marker::PhantomData<&'a R>,
}

pub struct RarEntries<'a, R: Read> {
    inner: OpenArchive,
    _mark: std::marker::PhantomData<&'a R>,
}

impl<'a, R: Read + Seek> Iterator for RarEntries<'a, R> {
    type Item = zip::result::ZipResult<RarEntry<'a, R>>;

    fn next(&mut self) -> Option<zip::result::ZipResult<RarEntry<'a, R>>> {
        let entry = self.inner.next();
        if let Some(Ok(entry)) = entry {
            let rar_entry = RarEntry {
                is_dir: entry.is_directory(),
                is_file: entry.is_file(),
                size: entry.unpacked_size as u64,
                path: PathBuf::from(entry.filename),
                _mark: std::marker::PhantomData::<&'a R>,
            };
            return Some(Ok(rar_entry));
        }

        None
    }
}

impl<'a, R> Archive<R> for RarArchive<'a, R> where R: Read + Seek {}

impl<'a, R> RarArchive<'a, R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<RarEntries<R>, ArchiveError> {
        let archive = self.open_archive();
        let lister = archive.list().map_err(ArchiveError::RarError);

        let lister = lister?;
        Ok(RarEntries {
            inner: lister,
            _mark: std::marker::PhantomData::<&R>,
        })
    }

    fn open_archive(&self) -> unrar::Archive {
        let archive = match self.password.clone() {
            Some(password) => unrar::Archive::with_password(self.filepath.to_owned(), password),
            None => unrar::Archive::new(self.filepath.to_owned()),
        };

        archive
    }
}
