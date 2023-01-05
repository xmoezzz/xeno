use std::{borrow::Cow, fmt::Display};
#[derive(Debug)]
pub enum Error {
    BadSignature([u8; 6]),
    UnsupportedVersion { major: u8, minor: u8 },
    ChecksumVerificationFailed,
    NextHeaderCrcMismatch,
    Io(std::io::Error),
    FileOpen(std::io::Error, String),
    Other(Cow<'static, str>),
    BadTerminatedStreamsInfo(u8),
    BadTerminatedUnpackInfo,
    BadTerminatedPackInfo(u8),
    BadTerminatedSubStreamsInfo,
    BadTerminatedheader(u8),

    ExternalUnsupported,
    UnsupportedCompressionMethod(String),
    MaxMemLimited { max_kb: usize, actaul_kb: usize },
    PasswordRequired,
}

impl Error {
    #[inline]
    pub fn other<S: Into<Cow<'static, str>>>(s: S) -> Self {
        Self::Other(s.into())
    }

    #[inline]
    pub fn io(e: std::io::Error) -> Self {
        Self::Io(e)
    }

    pub(crate) fn file_open(e: std::io::Error, filename: String) -> Self {
        Self::FileOpen(e, filename)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl std::error::Error for Error {}
