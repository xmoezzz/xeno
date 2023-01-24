use std::{
    path::PathBuf,
    ffi::OsString,
};

use thiserror::Error;
use unrar::archive::OpenArchive;


#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Failed to spawn external process: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("osstring")]
    OsString(OsString),
    #[error("The archive is encrypted")]
    Encrypted,
    #[error("7zip error: {0}")]
    SevenZipError(#[source] sevenz_rust::Error),
    #[error("rar error: {0}")]
    RarError(unrar::error::UnrarError<OpenArchive>),
    #[error("rar error: {0}")]
    RarError2(unrar::error::UnrarError<Vec<unrar::archive::Entry>>),
    #[error("xar error: {0}")]
    XarError(#[source] apple_xar::Error),
    #[error("zip error: {0}")]
    ZipError(#[source] zip::result::ZipError),
    #[error("squashfs error: {0}")]
    SquashfsError(#[source] backhand::error::SquashfsError),
    #[error("ntfs error: {0}")]
    NtfsError(#[source] ntfs::NtfsError),
    #[error("lzma error: {0}")]
    LzmaError(#[source] lzma_rs::error::Error),

    #[error("{0}")]
    GenericsError(&'static str),

    #[error("{0}")]
    GenericsError2(String),

    #[error("{0}")]
    GenericsError3(#[from] anyhow::Error),

    #[error("{0}")]
    GenericsError4(#[from] &'static dyn std::error::Error),

    #[error("Some or all extractions failed: {sources:?}")]
    ExtractFailed { sources: Vec<ArchiveError> },
}
