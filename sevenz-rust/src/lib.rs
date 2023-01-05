
#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(feature = "aes256")]
mod aes256sha256;

pub(crate) mod archive;
mod bcj;
pub(crate) mod decoders;
mod delta;
mod error;
pub(crate) mod folder;
mod lzma;
mod password;
mod reader;
#[cfg(not(target_arch = "wasm32"))]
mod de_funcs;
#[cfg(not(target_arch = "wasm32"))]
pub use de_funcs::*;
pub use password::Password;
pub use archive::SevenZArchiveEntry;
pub use error::Error;
pub use reader::SevenZReader;