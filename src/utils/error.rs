use std::{
    path::PathBuf,
    process::{ExitStatus, Output},
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Failed to spawn external process: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("The archive is encrypted")]
    Encrypted
}
