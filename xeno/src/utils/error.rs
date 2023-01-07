use std::{
    path::PathBuf,
    process::{ExitStatus, Output},
};

use thiserror::Error;
use unrar::archive::OpenArchive;


#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Failed to spawn external process: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("The archive is encrypted")]
    Encrypted,
    #[error("7zip error: {0}")]
    SevenZipError(#[source] sevenz_rust::Error),
    #[error("rar error: {0}")]
    RarError(unrar::error::UnrarError<OpenArchive>),
    #[error("xar error: {0}")]
    XarError(#[source] apple_xar::Error),

    #[error("{0}")]
    GenericsError(&'static str),

    #[error("{0}")]
    GenericsError2(String),

    #[error("{0}")]
    GenericsError3(#[from] anyhow::Error),
}
