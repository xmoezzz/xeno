use std::{io::Read, path::PathBuf};

use crate::utils::error::ArchiveError;
mod tar;

pub enum ReadFormat {
    SevenZip,
    All,
    Ar,
    Cab,
    Cpio,
    Empty,
    Gnutar,
    Iso9660,
    Lha,
    Mtree,
    Rar,
    Raw,
    Tar,
    Xar,
    Zip,
    Dmg,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FileType {
    BlockDevice,
    SymbolicLink,
    HardLink,
    Socket,
    CharacterDevice,
    Directory,
    NamedPipe,
    Mount,
    RegularFile,
    Other
}


pub enum ExtractOption {
    // The user and group IDs should be set on the restored file. By default, the user and group
    // IDs are not restored.
    Owner,
    // Full permissions (including SGID, SUID, and sticky bits) should be restored exactly as
    // specified, without obeying the current umask. Note that SUID and SGID bits can only be
    // restored if the user and group ID of the object on disk are correct. If
    // `ExtractOption::Owner` is not specified, then SUID and SGID bits will only be restored if
    // the default user and group IDs of newly-created objects on disk happen to match those
    // specified in the archive entry. By default, only basic permissions are restored, and umask
    // is obeyed.
    Permissions,
    // The timestamps (mtime, ctime, and atime) should be restored. By default, they are ignored.
    // Note that restoring of atime is not currently supported.
    Time,
    // Existing files on disk will not be overwritten. By default, existing regular files are
    // truncated and overwritten; existing directories will have their permissions updated; other
    // pre-existing objects are unlinked and recreated from scratch.
    NoOverwrite,
    // Existing files on disk will be unlinked before any attempt to create them. In some cases,
    // this can prove to be a significant performance improvement. By default, existing files are
    // truncated and rewritten, but the file is not recreated. In particular, the default behavior
    // does not break existing hard links.
    Unlink,
    // Attempt to restore ACLs. By default, extended ACLs are ignored.
    ACL,
    // Attempt to restore extended file flags. By default, file flags are ignored.
    FFlags,
    // Attempt to restore POSIX.1e extended attributes. By default, they are ignored.
    XAttr,
    // Refuse to extract any object whose final location would be altered by a symlink on disk.
    // This is intended to help guard against a variety of mischief caused by archives that
    // (deliberately or otherwise) extract files outside of the current directory. The default is
    // not to perform this check. If ARCHIVE_EXTRACT_UNLINK is specified together with this option,
    // the library will remove any intermediate symlinks it finds and return an error only if such
    // symlink could not be removed.
    SecureSymlinks,
    // Refuse to extract a path that contains a `..` element anywhere within it. The default is to
    // not refuse such paths. Note that paths ending in `..` always cause an error, regardless of
    // this flag.
    SecureNoDotDot,
    // Default: Create parent directories as needed
    NoAutoDir,
    // Default: Overwrite files, even if one on disk is newer
    NoOverwriteNewer,
    // Scan data for blocks of NUL bytes and try to recreate them with holes. This results in
    // sparse files, independent of whether the archive format supports or uses them.
    Sparse,
    // Default: Do not restore Mac extended metadata
    // This has no effect except on Mac OS
    MacMetadata,
    // Default: Use HFS+ compression if it was compressed
    // This has no effect except on Mac OS v10.6 or later
    NoHFSCompression,
    // Default: Do not use HFS+ compression if it was not compressed
    // This has no effect except on Mac OS v10.6 or later
    HFSCompressionForced,
    // Default: Do not reject entries with absolute paths */
    SecureNoAbsolutePaths,
    // Default: Do not clear no-change flags when unlinking object */
    ClearNoChangeFFlags,
}

pub trait Entry {
    fn file_type(&self) -> FileType; 
    fn hand_link(&self) -> Option<PathBuf>;
    fn path_name(&self) -> std::io::Result<PathBuf>;
    fn gid(&self) -> std::io::Result<u64>;
    fn uid(&self) -> std::io::Result<u64>;
    fn size(&self) -> u64;
    fn sym_link(&self) -> Option<PathBuf>;
}

pub trait Archive<R: Read> {
    
}
