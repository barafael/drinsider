//! Error type shared by the whole application.

/// Custom error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error that may occur during I/O operations.
    #[error("IO error: `{0}`")]
    Io(#[from] std::io::Error),
    /// Error that may occur while parsing ELF files.
    #[error("ELF parse error: `{0}`")]
    Elf(#[from] elf::ParseError),
    /// Error that may occur while analyzing library dependencies.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Dependency analysis error: `{0}`")]
    Ldd(#[from] lddtree::Error),
}

/// Type alias for the standard [`Result`] type.
pub type Result<T> = std::result::Result<T, Error>;
