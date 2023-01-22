use std::io::{Seek, Read};
use std::path::{PathBuf, Path};

use backhand::filesystem::{
    Filesystem,
    InnerNode, SquashfsBlockDevice, SquashfsCharacterDevice, SquashfsFile, SquashfsPath,
    SquashfsSymlink,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;


use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct SquashFSArchive {
    inner: Filesystem,
}

#[derive(Debug, Clone)]
pub struct SquashFSEntry<'a> {
    inner: &'a backhand::filesystem::Node,
}

pub struct SquashFSEntries<'a> {
    current: usize,
    inner: Vec<SquashFSEntry<'a>>,
}

impl<'a> Iterator for SquashFSEntries<'a> {
    type Item = Result<SquashFSEntry<'a>, ArchiveError>;

    fn next(&mut self) -> Option<Result<SquashFSEntry<'a>, ArchiveError>> {
        if self.current >= self.inner.len() {
            return None;
        }

        let entry = &self.inner[self.current];
        self.current += 1;
        Some(Ok(entry.to_owned()))
    }
}

impl SquashFSArchive {
    pub fn entries(&mut self) -> Result<SquashFSEntries, ArchiveError> {
        let mut entries = self.inner.nodes
            .iter()
            .map(|n| SquashFSEntry {inner: n})
            .collect::<Vec<_>>();

        Ok(SquashFSEntries {
            current: 0,
            inner: entries,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to)?;
        }

        let mut failures = vec![];
        for node in &self.inner.nodes {
            let path = &node.path;
            match &node.inner {
                InnerNode::File(SquashfsFile { bytes, .. }) => {
                    let path: PathBuf = path.iter().skip(1).collect();
                    log::debug!("file {}", path.display());
                    let filepath = to.join(path);
                    if let Err(e) = std::fs::write(&filepath, bytes) {
                        let err = ArchiveError::Io(e);
                        failures.push(err);
                    }
                },
                InnerNode::Symlink(SquashfsSymlink { link, .. }) => {
                    let path: PathBuf = path.iter().skip(1).collect();
                    log::debug!("symlink {} {}", path.display(), link);
                    let filepath = to.join(path);
                    let link = to.join(&link);

                    cfg_if::cfg_if! {
                        if #[cfg(unix)] {
                            if let Err(e) = std::os::unix::fs::symlink(&link, &filepath) {
                                let err = ArchiveError::Io(e);
                                failures.push(err);
                            }
                        } else {
                            let result = if filepath.is_dir() {
                                std::os::windows::fs::symlink_dir(&link, &filepath)
                            } else {
                                std::os::windows::fs::symlink_file(&link, &filepath)
                            };

                            if let Err(e) = result {
                                let err = ArchiveError::Io(e);
                                failures.push(err);
                            }
                        }
                    }
                },
                InnerNode::Path(SquashfsPath { header, .. }) => {
                    let path: PathBuf = path.iter().skip(1).collect();
                    let path = to.join(&path);
                    log::debug!("path {}", path.display());
                    let _ = std::fs::create_dir_all(&path);
                    cfg_if::cfg_if! {
                        if #[cfg(unix)] {
                            let perms = std::fs::Permissions::from_mode(u32::from(header.permissions));
                            if let Err(e) = std::fs::set_permissions(&path, perms) {
                                let err = ArchiveError::Io(e);
                                failures.push(err);
                            }
                        }
                    }
                },
                InnerNode::CharacterDevice(SquashfsCharacterDevice {
                    header: _,
                    device_number: _,
                }) => {
                    log::info!("[-] character device not supported");
                },
                InnerNode::BlockDevice(SquashfsBlockDevice {
                    header: _,
                    device_number: _,
                }) => {
                    log::info!("[-] block device not supported");
                },
            }
        }
        
        if !failures.is_empty() {
            return Err(ArchiveError::ExtractFailed { sources: failures });
        }
        Ok(())
    }

    pub fn unpack_file(&mut self, entry: &SquashFSEntry, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        let node = entry.inner;
        let path = &node.path;
        match &node.inner {
            InnerNode::File(SquashfsFile { bytes, .. }) => {
                let path: PathBuf = path.iter().skip(1).collect();
                log::debug!("file {}", path.display());
                let filepath = to.join(path);
                if let Err(e) = std::fs::write(&filepath, bytes) {
                    let err = ArchiveError::Io(e);
                    return Err(err);
                }
            },
            InnerNode::Symlink(SquashfsSymlink { link, .. }) => {
                let path: PathBuf = path.iter().skip(1).collect();
                log::debug!("symlink {} {}", path.display(), link);
                let filepath = to.join(path);
                let link = to.join(&link);

                cfg_if::cfg_if! {
                    if #[cfg(unix)] {
                        if let Err(e) = std::os::unix::fs::symlink(&link, &filepath) {
                            let err = ArchiveError::Io(e);
                            return Err(err);
                        }
                    } else {
                        let result = if filepath.is_dir() {
                            std::os::windows::fs::symlink_dir(&link, &filepath)
                        } else {
                            std::os::windows::fs::symlink_file(&link, &filepath)
                        };

                        if let Err(e) = result {
                            let err = ArchiveError::Io(e);
                            return Err(err);
                        }
                    }
                }
            },
            InnerNode::Path(SquashfsPath { header, .. }) => {
                let path: PathBuf = path.iter().skip(1).collect();
                let path = to.join(&path);
                log::debug!("path {}", path.display());
                let _ = std::fs::create_dir_all(&path);
                cfg_if::cfg_if! {
                    if #[cfg(unix)] {
                        let perms = std::fs::Permissions::from_mode(u32::from(header.permissions));
                        if let Err(e) = std::fs::set_permissions(&path, perms) {
                            let err = ArchiveError::Io(e);
                            return Err(err);
                        }
                    }
                }
            },
            InnerNode::CharacterDevice(SquashfsCharacterDevice {
                header: _,
                device_number: _,
            }) => {
                log::info!("[-] character device not supported");
            },
            InnerNode::BlockDevice(SquashfsBlockDevice {
                header: _,
                device_number: _,
            }) => {
                log::info!("[-] block device not supported");
            },
        }
        Ok(())
    }

    pub fn create_with_reader(rdr: impl Read + Seek + 'static) -> Result<SquashFSArchive, ArchiveError> {
        let inner = Filesystem::from_reader(rdr)
            .map_err(ArchiveError::SquashfsError)?;
        
        let archive = SquashFSArchive {
            inner,
        };

        Ok(archive)
    }

    pub fn create_with_path(path: impl AsRef<Path>) -> Result<SquashFSArchive, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}

