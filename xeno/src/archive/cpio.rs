use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::vec;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

use cpio_reader::Mode;
use time::PrimitiveDateTime;

pub struct CpioArchive<'a, R: Read + Seek> {
    buffer: memmap::Mmap,
    _mark: std::marker::PhantomData<&'a R>,
}

#[derive(Clone, Debug)]
pub struct CpioEntry {
    filename: String,
    size: u64,
    dev: Option<u32>,
    devmajor: Option<u32>,
    devminor: Option<u32>,
    ino: u32,
    mode: Mode,
    uid: u32,
    gid: u32,
    nlink: u32,
    rdev: Option<u32>,
    rdevmajor: Option<u32>,
    rdevminor: Option<u32>,
    mtime: u64,
}

impl Entry for CpioEntry {
    fn file_type(&self) -> FileType {
        if self.mode.bits() & Mode::NAMED_PIPE_FIFO.bits() == 1 {
            return FileType::NamedPipe;
        } else if self.mode.bits() & Mode::CHARACTER_SPECIAL_DEVICE.bits() == 1 {
            return FileType::CharacterDevice;
        } else if self.mode.bits() & Mode::DIRECTORY.bits() == 1 {
            return FileType::Directory;
        } else if self.mode.bits() & Mode::BLOCK_SPECIAL_DEVICE.bits() == 1 {
            return FileType::BlockDevice;
        } else if self.mode.bits() & Mode::REGULAR_FILE.bits() == 1 {
            return FileType::RegularFile;
        } else if self.mode.bits() & Mode::SYMBOLIK_LINK.bits() == 1 {
            return FileType::SymbolicLink;
        } else if self.mode.bits() & Mode::SOCKET.bits() == 1 {
            return FileType::Socket;
        }

        FileType::Other
    }

    fn hand_link(&self) -> Option<PathBuf> {
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        Ok(PathBuf::from(&self.filename))
    }

    fn gid(&self) -> std::io::Result<Option<u64>> {
        Ok(Some(self.gid as u64))
    }

    fn uid(&self) -> std::io::Result<Option<u64>> {
        Ok(Some(self.uid as u64))
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn sym_link(&self) -> Option<PathBuf> {
        None
    }
}

impl CpioEntry {
    pub fn dev(&self) -> Option<u32> {
        self.dev
    }

    pub fn devmajor(&self) -> Option<u32> {
        self.devmajor
    }

    pub fn devminor(&self) -> Option<u32> {
        self.devminor
    }

    pub fn inode(&self) -> u32 {
        self.ino
    }

    pub fn unix_mode(&self) -> u32 {
        self.mode.bits() & 0o777
    }

    pub fn nlink(&self) -> u32 {
        self.nlink
    }

    pub fn rdev(&self) -> Option<u32> {
        self.rdev
    }

    pub fn rdevmajor(&self) -> Option<u32> {
        self.rdevmajor
    }

    pub fn rdevminor(&self) -> Option<u32> {
        self.rdevminor
    }

    pub fn mtime(&self) -> Option<PrimitiveDateTime> {
        let dt = time::OffsetDateTime::from_unix_timestamp(self.mtime as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }
}

pub struct CpioEntries {
    entries: Vec<CpioEntry>,
}

impl<'a, R> CpioArchive<'a, R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<CpioEntries, ArchiveError> {
        let mut cpio_entries = vec![];
        for entry in cpio_reader::iter_files(&self.buffer) {
            let entry_wrapper = CpioEntry {
                filename: entry.name().to_owned(),
                size: entry.file().len() as u64,
                dev: entry.dev(),
                devmajor: entry.devmajor(),
                devminor: entry.devminor(),
                ino: entry.ino(),
                mode: entry.mode(),
                uid: entry.uid(),
                gid: entry.gid(),
                nlink: entry.nlink(),
                rdev: entry.rdev(),
                rdevmajor: entry.rdevmajor(),
                rdevminor: entry.rdevminor(),
                mtime: entry.mtime(),
            };
            cpio_entries.push(entry_wrapper);
        }

        Ok(CpioEntries {
            entries: cpio_entries,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to)?;
        }

        let mut failures = vec![];
        for entry in cpio_reader::iter_files(&self.buffer) {
            let dest = to.join(entry.name());
            match std::fs::File::create(dest).and_then(|mut writer| writer.write_all(entry.file()))
            {
                Err(e) => {
                    let err = ArchiveError::Io(e);
                    failures.push(err);
                    continue;
                }
                _ => {}
            };
        }

        Ok(())
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<CpioArchive<'a, R>, ArchiveError> {
        let file = std::fs::File::open(path)?;
        let buffer = unsafe { memmap::MmapOptions::new().map(&file).unwrap() };

        let archive = CpioArchive {
            buffer,
            _mark: std::marker::PhantomData::<&'a R>,
        };

        Ok(archive)
    }
}
