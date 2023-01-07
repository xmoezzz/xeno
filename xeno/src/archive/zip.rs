use std::io::{BufReader, Read, Seek};
use std::path::PathBuf;
use std::sync::Arc;

use crate::utils::error::ArchiveError;
use crate::archive::{Archive,Entry, FileType};

pub struct ZipArchive<R: Read> {
    inner: zip::ZipArchive<R>,
    password: Option<String>
}


pub struct ZipEntry<'a, R: Read + Seek> {
    index: usize,
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: PathBuf,
    _mark: std::marker::PhantomData<&'a R>
}

pub struct ZipEntries<'a, R: Read> {
    inner: &'a mut zip::ZipArchive<R>,
    current: usize,
    total: usize,
    _mark: std::marker::PhantomData<&'a R>
}

impl<'a, R: Read + Seek> Iterator for ZipEntries<'a, R> {
    type Item = zip::result::ZipResult<ZipEntry<'a, R>>;

    fn next(&mut self) -> Option<zip::result::ZipResult<ZipEntry<'a, R>>> {
        if self.current >= self.total {
            return None;
        }

        let entry = self.inner.by_index(self.current)
            .map(|result|  ZipEntry {
                index: self.current, 
                is_dir: result.is_dir(), 
                is_file: result.is_file(), 
                size: result.size(),
                path: PathBuf::from(result.name()),
                _mark: std::marker::PhantomData::<&'a R> } );
        
        self.current += 1;
        Some(entry)
    }
}


impl<'a, R: Read + Seek> Entry for ZipEntry<'a, R> {
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


impl<R> ZipArchive<R>
where
    R: Read + Seek
{
    pub fn entries(&mut self) -> std::io::Result<ZipEntries<R>> {
        let total = self.inner.len();
        Ok(ZipEntries {
            inner: &mut self.inner,
            current: 0,
            total,
            _mark: std::marker::PhantomData::<&R>
        })
    }
}
