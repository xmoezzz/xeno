use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ntfs::{
    indexes::NtfsFileNameIndex, structured_values::NtfsFileAttributeFlags, Ntfs, NtfsReadSeek,
};
use time::PrimitiveDateTime;

use crate::archive::{Entry, FileType};
use crate::utils::error::ArchiveError;

pub struct NtfsArchive<R>
where
    R: Read + Seek,
{
    reader: Arc<Mutex<R>>,
}

#[derive(Clone, Debug)]
pub struct NtfsEntry {
    is_dir: bool,
    is_file: bool,
    size: u64,
    path: String,
    nt_creation_time: u64,
    nt_access_time: u64,
    nt_modification_time: u64,
    attr: NtfsFileAttributeFlags,
}

impl Entry for NtfsEntry {
    fn file_type(&self) -> FileType {
        if self.is_dir {
            return FileType::Directory;
        }
        if self.is_file {
            return FileType::RegularFile;
        }

        FileType::Other
    }

    fn hand_link(&self) -> Option<PathBuf> {
        None
    }

    fn path_name(&self) -> std::io::Result<PathBuf> {
        Ok(PathBuf::from(&self.path))
    }

    fn gid(&self) -> std::io::Result<Option<u64>> {
        Ok(None)
    }

    fn uid(&self) -> std::io::Result<Option<u64>> {
        Ok(None)
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn sym_link(&self) -> Option<PathBuf> {
        None
    }
}

impl NtfsEntry {
    fn nt_timestamp_to_unix_timestamp(nt_timestamp: u64) -> u64 {
        let win_sec = nt_timestamp / 10000000;
        win_sec - 11644473600
    }

    pub fn creation_time(&self) -> Option<PrimitiveDateTime> {
        let timestamp = Self::nt_timestamp_to_unix_timestamp(self.nt_creation_time);
        let dt = time::OffsetDateTime::from_unix_timestamp(timestamp as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }

    pub fn access_time(&self) -> Option<PrimitiveDateTime> {
        let timestmap = Self::nt_timestamp_to_unix_timestamp(self.nt_access_time);
        let dt = time::OffsetDateTime::from_unix_timestamp(timestmap as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }

    pub fn modification_time(&self) -> Option<PrimitiveDateTime> {
        let timestmap = Self::nt_timestamp_to_unix_timestamp(self.nt_modification_time);
        let dt = time::OffsetDateTime::from_unix_timestamp(timestmap as i64).ok();
        dt.map(|dt| PrimitiveDateTime::new(dt.date(), dt.time()))
    }

    /// File is marked read-only.
    pub fn ntfs_readonly(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::READ_ONLY).bits() == 1
    }

    /// File is hidden (in file browsers that care).
    pub fn ntfs_hidden(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::HIDDEN).bits() == 1
    }

    /// File is marked as a system file.
    pub fn ntfs_system(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::SYSTEM).bits() == 1
    }

    /// File is marked for archival (cf. <https://en.wikipedia.org/wiki/Archive_bit>).
    pub fn ntfs_archive(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::ARCHIVE).bits() == 1
    }

    /// File denotes a device.
    pub fn ntfs_device(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::DEVICE).bits() == 1
    }

    /// Set when no other attributes are set.
    pub fn ntfs_normal(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::NORMAL).bits() == 1
    }

    /// File is a temporary file that is likely to be deleted.
    pub fn ntfs_temporary(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::TEMPORARY).bits() == 1
    }

    /// File is stored sparsely.
    pub fn ntfs_sparse_file(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::SPARSE_FILE).bits() == 1
    }

    /// File is a reparse point.
    pub fn ntfs_reparse_point(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::REPARSE_POINT).bits() == 1
    }

    /// File is transparently compressed by the filesystem (using LZNT1 algorithm).
    /// For directories, this attribute denotes that compression is enabled by default for new files inside that directory.
    pub fn ntfs_compressed(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::COMPRESSED).bits() == 1
    }

    pub fn ntfs_offline(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::OFFLINE).bits() == 1
    }

    /// File has not (yet) been indexed by the Windows Indexing Service.
    pub fn ntfs_content_indexed(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::NOT_CONTENT_INDEXED).bits() == 1
    }

    /// File is encrypted via EFS.
    /// For directories, this attribute denotes that encryption is enabled by default for new files inside that directory.
    pub fn ntfs_encrypted(&self) -> bool {
        (self.attr & NtfsFileAttributeFlags::ENCRYPTED).bits() == 1
    }
}

#[derive(Clone, Debug)]
pub struct NtfsEntries {
    inner: Vec<NtfsEntry>,
    current: usize,
}

impl Iterator for NtfsEntries {
    type Item = Result<NtfsEntry, ArchiveError>;

    fn next(&mut self) -> Option<Result<NtfsEntry, ArchiveError>> {
        if self.current > self.inner.len() {
            return None;
        }

        let entry = &self.inner[self.current];
        self.current += 1;
        Some(Ok(entry.to_owned()))
    }
}

impl<R> NtfsArchive<R>
where
    R: Read + Seek,
{
    pub fn entries(&mut self) -> Result<NtfsEntries, ArchiveError> {
        let guard_reader = self.reader.clone();
        let mut gr = guard_reader.lock().unwrap();
        let mut reader = gr.by_ref();
        reader.seek(SeekFrom::Start(0)).map_err(ArchiveError::Io)?;
        let ntfs = Ntfs::new(&mut reader).map_err(ArchiveError::NtfsError)?;

        let root_dir = ntfs
            .root_directory(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let index = root_dir
            .directory_index(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let mut iter = index.entries();

        // TODO
        let mut ntfs_entries = vec![];
        while let Some(entry) = iter.next(&mut reader) {
            match entry {
                Ok(entry) => {
                    if let Some(Ok(file_name)) = entry.key() {
                        let wrapper_entry = NtfsEntry {
                            path: file_name.name().to_string(),
                            is_dir: file_name.is_directory(),
                            is_file: !file_name.is_directory(),
                            size: file_name.data_size(),
                            nt_creation_time: file_name.creation_time().nt_timestamp(),
                            nt_access_time: file_name.access_time().nt_timestamp(),
                            nt_modification_time: file_name.modification_time().nt_timestamp(),
                            attr: file_name.file_attributes(),
                        };
                        ntfs_entries.push(wrapper_entry);
                    }
                }
                _ => {}
            }
        }

        Ok(NtfsEntries {
            inner: ntfs_entries,
            current: 0,
        })
    }

    pub fn unpack_all(&mut self, to: impl AsRef<Path>) -> Result<(), ArchiveError> {
        let to = to.as_ref();
        if !to.exists() {
            std::fs::create_dir_all(to)?;
        }

        let guard_reader = self.reader.clone();
        let mut gr = guard_reader.lock().unwrap();
        let mut reader = gr.by_ref();

        let mut failures = vec![];
        reader.seek(SeekFrom::Start(0)).map_err(ArchiveError::Io)?;
        let ntfs = Ntfs::new(&mut reader).map_err(ArchiveError::NtfsError)?;

        let root_dir = ntfs
            .root_directory(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let index = root_dir
            .directory_index(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let mut iter = index.entries();

        while let Some(entry) = iter.next(&mut reader) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    let err = ArchiveError::NtfsError(e);
                    failures.push(err);
                    continue;
                }
            };

            if let Some(Ok(file_name)) = entry.key() {
                let entry = entry.to_file(&ntfs, &mut reader).unwrap();
                let filename = file_name.name().to_string();
                let stream_name = format!("{}:$DATA", &filename);
                let data = match entry.data(&mut reader, &stream_name) {
                    Some(Ok(data)) => data,
                    Some(Err(e)) => {
                        let err = ArchiveError::NtfsError(e);
                        failures.push(err);
                        continue;
                    }
                    _ => {
                        log::info!(
                            "The file does not have a \"{}\" $DATA attribute.",
                            &filename
                        );
                        continue;
                    }
                };

                let data_attribute = data.to_attribute();
                let mut data_value = match data_attribute.value(&mut reader) {
                    Ok(data_value) => data_value,
                    Err(e) => {
                        let err = ArchiveError::NtfsError(e);
                        failures.push(err);
                        continue;
                    }
                };

                let dest = to.join(filename);
                let mut writer = match std::fs::File::create(dest) {
                    Ok(writer) => writer,
                    Err(e) => {
                        let err = ArchiveError::Io(e);
                        failures.push(err);
                        continue;
                    }
                };
                let mut buf = [0u8; 4096];
                loop {
                    let bytes_read = match data_value.read(&mut reader, &mut buf) {
                        Ok(size) => size,
                        Err(e) => {
                            let err = ArchiveError::NtfsError(e);
                            failures.push(err);
                            break;
                        }
                    };

                    if bytes_read == 0 {
                        break;
                    }

                    writer.write(&buf[..bytes_read])?;
                }
            }
        }

        if !failures.is_empty() {
            let err = ArchiveError::ExtractFailed { sources: failures };
            return Err(err);
        }
        Ok(())
    }

    pub fn unpack_file(
        &mut self,
        entry: &NtfsEntry,
        to: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        let guard_reader = self.reader.clone();
        let mut gr = guard_reader.lock().unwrap();
        let mut reader = gr.by_ref();

        reader.seek(SeekFrom::Start(0)).map_err(ArchiveError::Io)?;
        let ntfs = Ntfs::new(&mut reader).map_err(ArchiveError::NtfsError)?;
        let root_dir = ntfs
            .root_directory(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let index = root_dir
            .directory_index(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let mut finder = index.finder();

        let file = match NtfsFileNameIndex::find(&mut finder, &ntfs, &mut reader, &entry.path) {
            Some(Ok(entry)) => entry
                .to_file(&ntfs, &mut reader)
                .map_err(ArchiveError::NtfsError)?,
            Some(Err(e)) => {
                let err = ArchiveError::NtfsError(e);
                return Err(err);
            }
            None => {
                let err_string = format!("No such file or directory \"{}\".", &entry.path);
                let err = ArchiveError::GenericsError2(err_string);
                return Err(err);
            }
        };

        let data_stream_name = format!("{}:$DATA", &entry.path);
        let data_item = match file.data(&mut reader, &data_stream_name) {
            Some(data_item) => data_item,
            None => {
                format!(
                    "The file does not have a \"{}\" $DATA attribute.",
                    data_stream_name
                );
                return Ok(());
            }
        };
        let data_item = match data_item {
            Ok(item) => item,
            Err(e) => {
                let err = ArchiveError::NtfsError(e);
                return Err(err);
            }
        };
        let data_attribute = data_item.to_attribute();
        let mut data_value = match data_attribute.value(&mut reader) {
            Ok(data_value) => data_value,
            Err(e) => {
                let err = ArchiveError::NtfsError(e);
                return Err(err);
            }
        };

        let mut buf = [0u8; 4096];
        let mut writer = match std::fs::File::create(to) {
            Ok(writer) => writer,
            Err(e) => {
                let err = ArchiveError::Io(e);
                return Err(err);
            }
        };

        loop {
            let bytes_read = match data_value.read(&mut reader, &mut buf) {
                Ok(size) => size,
                Err(e) => {
                    let err = ArchiveError::NtfsError(e);
                    return Err(err);
                }
            };
            if bytes_read == 0 {
                break;
            }

            writer.write(&buf[..bytes_read])?;
        }

        Ok(())
    }

    pub fn create_with_reader(
        rdr: impl Read + Seek,
    ) -> Result<NtfsArchive<impl Read + Seek>, ArchiveError> {
        let shared_reader = Arc::new(Mutex::new(Box::new(rdr)));
        let guard_reader = shared_reader.clone();
        let mut gr = guard_reader.lock().unwrap();
        let mut reader = gr.by_ref();
        let ntfs = Ntfs::new(&mut reader).map_err(ArchiveError::NtfsError)?;
        let root_dir = ntfs
            .root_directory(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let index = root_dir
            .directory_index(&mut reader)
            .map_err(ArchiveError::NtfsError)?;
        let archive = NtfsArchive {
            reader: shared_reader,
        };

        Ok(archive)
    }

    pub fn create_with_path(
        path: impl AsRef<Path>,
    ) -> Result<NtfsArchive<impl Read + Seek>, ArchiveError> {
        let reader = std::fs::File::open(path)?;
        Self::create_with_reader(reader)
    }
}
