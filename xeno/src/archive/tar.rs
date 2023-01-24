use core::arch;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use lzma_rs::lzma_decompress;
use tar::Archive as TarArchiveInner;
use zstd::Decoder as ZstdDecoder;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct TarArchive<R: Read> {
    inner: TarArchiveInner<R>,
}

pub struct TarGzArchive<R: Read> {
    inner: TarArchiveInner<R>,
}

pub struct TarBz2Archive<R: Read> {
    inner: TarArchiveInner<R>,
}

pub struct TarZstdArchive<R: Read> {
    inner: TarArchiveInner<R>,
}

pub struct TarEntry<'a, R: Read> {
    inner: tar::Entry<'a, R>,
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
            _ => FileType::Other,
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

impl<'a, R: Read> Read for TarEntry<'a, R> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(into)
    }
}

pub struct TarEntries<'a, R: Read> {
    inner: tar::Entries<'a, R>,
}

impl<'a, R: Read> Iterator for TarEntries<'a, R> {
    type Item = Result<TarEntry<'a, R>, ArchiveError>;

    fn next(&mut self) -> Option<Result<TarEntry<'a, R>, ArchiveError>> {
        self.inner.next().map(|result| {
            result
                .map(|e| TarEntry { inner: e })
                .map_err(ArchiveError::Io)
        })
    }
}

impl<R> TarArchive<R>
where
    R: Read,
{
    pub fn entries(&mut self) -> std::io::Result<TarEntries<R>> {
        let inner = self.inner.entries()?;
        Ok(TarEntries { inner })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.unpack(to).map_err(ArchiveError::Io)
    }

    pub fn unpack_file(
        &mut self,
        entry: &mut TarEntry<R>,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut entry.inner, &mut writer)?;
        Ok(())
    }

    pub fn create_with_reader(reader: impl Read) -> Result<TarArchive<impl Read>, ArchiveError> {
        let archive = tar::Archive::new(reader);
        Ok(TarArchive { inner: archive })
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<TarArchive<impl Read>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}

impl<R> TarGzArchive<R>
where
    R: Read,
{
    pub fn entries(&mut self) -> std::io::Result<TarEntries<R>> {
        let inner = self.inner.entries()?;
        Ok(TarEntries { inner })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.unpack(to).map_err(ArchiveError::Io)
    }

    pub fn unpack_file(
        &mut self,
        entry: &mut TarEntry<R>,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut entry.inner, &mut writer)?;
        Ok(())
    }

    pub fn create_with_reader(reader: impl Read) -> Result<TarGzArchive<impl Read>, ArchiveError> {
        let reader = flate2::read::GzDecoder::new(reader);
        let archive = tar::Archive::new(reader);
        Ok(TarGzArchive { inner: archive })
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<TarGzArchive<impl Read>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}

impl<R> TarBz2Archive<R>
where
    R: Read,
{
    pub fn entries(&mut self) -> std::io::Result<TarEntries<R>> {
        let inner = self.inner.entries()?;
        Ok(TarEntries { inner })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.unpack(to).map_err(ArchiveError::Io)
    }

    pub fn unpack_file(
        &mut self,
        entry: &mut TarEntry<R>,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut entry.inner, &mut writer)?;
        Ok(())
    }

    pub fn create_with_reader(reader: impl Read) -> Result<TarBz2Archive<impl Read>, ArchiveError> {
        let reader = BzDecoder::new(reader);
        let archive = tar::Archive::new(reader);
        Ok(TarBz2Archive { inner: archive })
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<TarBz2Archive<impl Read>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}

impl<R> TarZstdArchive<R>
where
    R: Read,
{
    pub fn entries(&mut self) -> std::io::Result<TarEntries<R>> {
        let inner = self.inner.entries()?;
        Ok(TarEntries { inner })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        self.inner.unpack(to).map_err(ArchiveError::Io)
    }

    pub fn unpack_file(
        &mut self,
        entry: &mut TarEntry<R>,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let mut writer = std::fs::File::create(to)?;
        let _ = std::io::copy(&mut entry.inner, &mut writer)?;
        Ok(())
    }

    pub fn create_with_reader(
        reader: impl Read,
    ) -> Result<TarZstdArchive<impl Read>, ArchiveError> {
        let reader = ZstdDecoder::new(reader)?;
        let archive = tar::Archive::new(reader);
        Ok(TarZstdArchive { inner: archive })
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<TarZstdArchive<impl Read>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}
