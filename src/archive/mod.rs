

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


pub enum FileType {
    BlockDevice,
    SymbolicLink,
    Socket,
    CharacterDevice,
    Directory,
    NamedPipe,
    Mount,
    RegularFile,
}

pub trait Entry {
    
}
