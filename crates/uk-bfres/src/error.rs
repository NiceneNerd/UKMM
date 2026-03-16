#[derive(Debug, thiserror::Error)]
pub enum BfresError {
    #[error("Invalid BFRES magic: expected 'FRES', got {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Not a Wii U BFRES file (wrong BOM or version)")]
    NotWiiU,

    #[error("Unexpected end of data at offset {offset}, need {needed} bytes")]
    UnexpectedEof { offset: usize, needed: usize },

    #[error("Invalid string table offset: {0:#x}")]
    InvalidStringOffset(u64),

    #[error("Invalid dictionary at offset {0:#x}")]
    InvalidDict(u64),

    #[error("Unsupported GX2 texture format: {0:#x}")]
    UnsupportedTextureFormat(u32),

    #[error("Texture conversion error: {0}")]
    TextureConversion(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, BfresError>;
