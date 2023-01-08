use std::io::Seek;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::vec;

use cab::{Cabinet, FolderEntries, FileEntries, FolderEntry, FileEntry};

use crate::archive::{Archive, Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct CabArchive<'a, R: Read + Seek> {
    inner: Cabinet<R>,
    _mark: std::marker::PhantomData<&'a R>,
}

#[derive(Clone, Debug)]
pub struct CabEntry {
    filename: String,
    size: u64,
}

pub struct CabEntries<'a, R: Read + Seek> {
    current: usize,
    total: usize,
    inner: Vec<CabEntry>,
    _mark: std::marker::PhantomData<&'a R>
}

impl<'a, R: Read + Seek> Iterator for CabEntries<'a, R> {
    type Item = Result<CabEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<CabEntry, ArchiveError>> {
        if self.current >= self.total {
            return None;
        }

        let entry = &self.inner[self.current];
        self.current += 1;
        Some(Ok(entry.clone()))
    }
}

impl<'a, R> Archive<R> for CabArchive<'a, R> where R: Read + Seek {}

impl<'a, R> CabArchive<'a, R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<CabEntries<R>, ArchiveError> {
        let mut entries = vec![];
        for folder in self.inner.folder_entries() {
            for entry in folder.file_entries() {
                let entry = CabEntry {
                    filename: entry.name().to_string(),
                    size: entry.uncompressed_size() as u64
                };
                entries.push(entry);
            }
        }

        Ok(CabEntries {
            current: 0,
            total: entries.len(),
            inner: entries,
            _mark: std::marker::PhantomData::<&R>,
        })
    }

    fn open(&self, rdr: R) -> Result<CabArchive<'a, R>, ArchiveError> {
        let reader = cab::Cabinet::new(rdr)
            .map_err(ArchiveError::Io)?;
        
        let archive = CabArchive {
            inner: reader,
            _mark: std::marker::PhantomData::<&'a R>,
        };

        Ok(archive)
    }
}


