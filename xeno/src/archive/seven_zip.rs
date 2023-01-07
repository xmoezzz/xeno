use std::io::Seek;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::{Mutex, Arc};


use crate::archive::{Archive, Entry, FileType};
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
}

pub struct ZipEntries<'a, R: Read + Seek> {
    inner: &'a mut sevenz_rust::SevenZReader<R>,
    current: usize,
    _mark: std::marker::PhantomData<&'a R>
}


impl<'a, R: Read + Seek> Iterator for ZipEntries<'a, R> {
    type Item = Result<SevenZipEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<SevenZipEntry, ArchiveError>> {
        let total = self.inner.archive.files.len();
        if self.current >= total {
            return None;
        }

        let entry = &self.inner.archive.files[self.current];
        
        let entry = SevenZipEntry {
                index: self.current, 
                is_dir: entry.is_directory(), 
                is_file: !entry.is_directory(), 
                size: entry.size(),
                path: PathBuf::from(entry.name())
            };
        
        self.current += 1;
        Some(Ok(entry))
    }
}


impl<R> SevenZipArchive<R>
where
    R: Read + Seek
{
    pub fn entries(&mut self) -> std::io::Result<ZipEntries<R>> {
        Ok(ZipEntries {
            inner: &mut self.inner,
            current: 0,
            _mark: std::marker::PhantomData::<&R>
        })
    }
}

impl<R> Archive<R> for SevenZipArchive<R>
where
    R: Read + Seek
{

}

impl<R> SevenZipArchive<R>
where
    for<'a> R: Read + Seek,
{
    fn open(rdr: R, size: u64, password: Option<String>) -> Result<impl Archive<R>, ArchiveError> {
        let password = password.unwrap_or_default();
        let p = password.as_str();
        let inner = sevenz_rust::SevenZReader::<R>::new(rdr, size, p.into())
            .map_err(|e| { ArchiveError::SevenZipError(e) })?;
        let arc = SevenZipArchive { inner };
        Ok(arc)
    }
}
