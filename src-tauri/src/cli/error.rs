use std::{
    error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
};

/// Custom result type for PolyLauncher operations
pub type PolyResult<T> = Result<T, PolyError>;

/// Error types for PolyLauncher operations
#[derive(Debug)]
pub enum PolyError {
    Io(io::Error),
    Reqwest(reqwest::Error),
    Json(serde_json::Error),
    PathError(String),
    DownloadError(String),
    HarNotFound(String),
    NonEmptyDir(PathBuf),
}

impl Display for PolyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PolyError::Io(e) => write!(f, "IO error: {}", e),
            PolyError::Reqwest(e) => write!(f, "Network error: {}", e),
            PolyError::Json(e) => write!(f, "JSON parsing error: {}", e),
            PolyError::PathError(msg) => write!(f, "Path error: {}", msg),
            PolyError::DownloadError(msg) => write!(f, "Download error: {}", msg),
            PolyError::HarNotFound(version) => {
                write!(f, "HAR file for version {} not found", version)
            }
            PolyError::NonEmptyDir(path) => {
                write!(f, "The directory '{}' is not empty", path.display())
            }
        }
    }
}

impl error::Error for PolyError {}

impl From<io::Error> for PolyError {
    fn from(err: io::Error) -> Self {
        PolyError::Io(err)
    }
}

impl From<reqwest::Error> for PolyError {
    fn from(err: reqwest::Error) -> Self {
        PolyError::Reqwest(err)
    }
}

impl From<serde_json::Error> for PolyError {
    fn from(err: serde_json::Error) -> Self {
        PolyError::Json(err)
    }
}
