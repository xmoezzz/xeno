use std::io::{BufReader, Read};
use std::path::PathBuf;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use lzma_rs::lzma_decompress;
use tar::Archive as TarArchiveInner;
use zstd::Decoder as ZstdDecoder;

use crate::utils::error::ArchiveError;
use crate::archive::{Archive,Entry, FileType};


pub struct TarArchive<R: Read> {
    inner: TarArchiveInner<R>
}

pub struct TarEntry<'a, R: Read> {
    inner: tar::Entry<'a, R>
}

impl<'a, R: Read> Entry for TarEntry<'a, R> {
    fn file_type(&self) -> FileType {
        match self.inner.header().entry_type() {
            tar::EntryType::Regular => FileType::RegularFile,
            tar::EntryType::Link => FileType::HardLink,
            tar::EntryType::Symlink => FileType::SymbolicLink,
            tar::EntryType::Block => FileType::BlockDevice,
            tar::EntryType::Char => FileType::CharacterDevice,
            tar::EntryType::Directory => FileType::Directory,
            tar::EntryType::Fifo => FileType::NamedPipe,
            _ => FileType::Other
        }
    }

    fn hand_link(&self) -> Option<PathBuf> {
        if self.file_type() == FileType::HardLink {
            if let Ok(Some(path)) = self.inner.link_name() {
                return Some(path.as_ref().to_path_buf());
            }
        }
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        let path = self.inner.path()?;
        Ok(path.as_ref().to_path_buf())
    }

    fn size(&self) -> u64 {
        self.inner.size()
    }

    fn gid(&self) -> std::io::Result<Option<u64>> {
        let gid = self.inner.header().gid()?;
        Ok(Some(gid))
    }

    fn uid(&self) -> std::io::Result<Option<u64>> {
        let uid = self.inner.header().uid()?;
        Ok(Some(uid))
    }

    fn sym_link(&self) -> Option<PathBuf> {
        if self.file_type() == FileType::SymbolicLink {
            if let Ok(Some(path)) = self.inner.link_name() {
                return Some(path.as_ref().to_path_buf());
            }
        }
        None
    }
}

pub struct TarEntries<'a, R: Read> {
    inner: tar::Entries<'a, R>
}


impl<'a, R: Read> Iterator for TarEntries<'a, R> {
    type Item = std::io::Result<TarEntry<'a, R>>;

    fn next(&mut self) -> Option<std::io::Result<TarEntry<'a, R>>> {
        self.inner
            .next()
            .map(|result| result.map(|e| TarEntry { inner: e }))
    }
}

impl<R> Archive<R> for TarArchive<R>
where
    R: Read
{

}


impl<R> TarArchive<R>
where
    R: Read
{
    pub fn entries(&mut self) -> std::io::Result<TarEntries<R>> {
        let inner = self.inner.entries()?;
        Ok(TarEntries { inner })
    }
}

impl<R> TarArchive<R>
where for <'a>
    R: Read
{
    fn open(rdr: R) -> Result<impl Archive<R>, ArchiveError>
    {
        let inner = tar::Archive::new(rdr);
        let arc = TarArchive{inner};
        Ok(arc)
    }
}


