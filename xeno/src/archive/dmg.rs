// use std::io::{BufReader, Read, Seek};
// use std::path::PathBuf;
// use std::sync::Arc;
// use apple_dmg::DmgReader;

// pub struct DmgArchive<R: Read + Seek> {
//     inner: DmgReader<R>,
//     password: Option<String>
// }


// pub struct DmgEntries<'a, R: Read + Seek> {
//     dmg_inner: Arc<&'a mut DmgReader<R>>,
//     current_partition: usize,
// }

// impl<'a, R: Read + Seek> Iterator for DmgEntries<'a, R> {
//     type Item = zip::result::ZipResult<ZipEntry<'a, R>>;

//     fn next(&mut self) -> Option<zip::result::ZipResult<ZipEntry<'a, R>>> {
//         if self.current >= self.total {
//             return None;
//         }

//         self.dmg_inner.plist().partitions().len();
//         let partition = self.dmg_inner.plist().partitions().get(self.current_partition);
//         if let Some(partition) = partition {
            
//         }

//         let entry = self.inner.by_index(self.current)
//             .map(|result|  ZipEntry {
//                 index: self.current, 
//                 is_dir: result.is_dir(), 
//                 is_file: result.is_file(), 
//                 size: result.size(),
//                 path: PathBuf::from(result.name()),
//                 zip_inner: self.inner.clone(),
//                 _mark: std::marker::PhantomData::<&'a R> } );
        
//         self.current += 1;
//         Some(entry)
//     }
// }


// impl<R> DmgArchive<R>
// where
//     R: Read + Seek
// {
//     pub fn entries(&mut self) -> std::io::Result<ZipEntries<R>> {
//         let total = self.inner.len();
//         Ok(ZipEntries {
//             inner: Arc::new(Mutex::new(&mut self.inner)),
//             current: 0,
//             total,
//             password: self.password.clone(),
//             _mark: std::marker::PhantomData::<&R>
//         })
//     }
// }


