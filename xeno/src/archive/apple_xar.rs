use std::io::Seek;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use apple_xar::XarResult;
use apple_xar::reader::XarReader;
use apple_xar::table_of_contents::File;
use unrar::archive::OpenArchive;

use crate::archive::{Archive, Entry, FileType};
use crate::utils::error::ArchiveError;


pub struct XarArchive<'a, R: Read + Seek  + Sized + Debug> {
    inner: XarReader<R>,
    _mark: std::marker::PhantomData<&'a R>,
}

pub struct XarEntry<'a, R: Read + Seek  + Sized + Debug> {
    inner: File,
    filename: String,
    _mark: std::marker::PhantomData<&'a R>,
}



pub struct XarEntries<'a, R: Read> {
    inner: Vec<(String, apple_xar::table_of_contents::File)>,
    current: usize,
    total: usize,
    _mark: std::marker::PhantomData<&'a R>
}

impl<'a, R: Read + Seek + Sized + Debug> Iterator for XarEntries<'a, R> {
    type Item = Result<XarEntry<'a, R>, ArchiveError>;

    fn next(&mut self) -> Option<Result<XarEntry<'a, R>, ArchiveError>> {
        if self.current >= self.total {
            return None;
        }

        let (filename, entry) = &self.inner[self.current];
        let entry = XarEntry {
            inner: entry.clone(),
            filename: filename.clone(),
            _mark: std::marker::PhantomData::<&'a R>
        };
        
        self.current += 1;
        Some(Ok(entry))
    }
}


impl<'a, R> Archive<R> for XarArchive<'a, R> where R: Read + Seek + Sized + Debug {}

impl<'a, R> XarArchive<'a, R>
where
    R: Read + Seek + Sized + Debug,
{
    pub fn entries(&mut self) -> Result<XarEntries<R>, ArchiveError> {
        let entries = self.inner.files()
            .map_err(ArchiveError::XarError)?;

        Ok(XarEntries {
            current: 0,
            total: entries.len(),
            inner: entries,
            _mark: std::marker::PhantomData::<&R>,
        })
    }

    fn open(&self, rdr: R) -> Result<XarArchive<'a, R>, ArchiveError> {
        let reader = apple_xar::reader::XarReader::new(rdr)
            .map_err(ArchiveError::XarError)?;
        
        let archive = XarArchive {
            inner: reader,
            _mark: std::marker::PhantomData::<&'a R>,
        };

        Ok(archive)
    }
}

