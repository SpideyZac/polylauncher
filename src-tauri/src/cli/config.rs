use std::{
    env::{current_exe, home_dir},
    path::PathBuf,
};

use crate::error::{PolyError, PolyResult};

/// Latest stable version of PolyTrack
pub const LATEST_VERSION: &str = "0.5.2";

/// Base URL prefix for PolyTrack downloads
pub const URL_PREFIX: &str = "https://app-polytrack.kodub.com/";

/// Maximum number of download retry attempts
pub const MAX_DOWNLOAD_RETRIES: u32 = 5;

/// Delay between retry attempts in seconds
pub const RETRY_DELAY_SECS: u64 = 5;

/// Get the PolyLauncher home directory
pub fn get_polylauncher_dir() -> PolyResult<PathBuf> {
    let home = home_dir()
        .ok_or_else(|| PolyError::PathError("Failed to get home directory".to_string()))?;

    Ok(home.join(".polylauncher"))
}

/// Get the directory for a specific PolyTrack version
pub fn get_version_dir(version: &str) -> PolyResult<PathBuf> {
    Ok(get_polylauncher_dir()?
        .join("polytrack_versions")
        .join(version))
}

/// Get the directory for the template project
pub fn get_template_project_dir() -> PolyResult<PathBuf> {
    let exe = current_exe()
        .map_err(|e| PolyError::PathError(format!("Failed to get executable path: {}", e)))?;

    let exe_parent = exe.parent().ok_or_else(|| {
        PolyError::PathError("Failed to get executable parent directory".to_string())
    })?;

    Ok(exe_parent.join("resources").join("template_project"))
}

/// Get the HAR file path for a specific version
pub fn get_har_file_path(version: &str) -> PolyResult<PathBuf> {
    let exe = current_exe()
        .map_err(|e| PolyError::PathError(format!("Failed to get executable path: {}", e)))?;

    let exe_parent = exe.parent().ok_or_else(|| {
        PolyError::PathError("Failed to get executable parent directory".to_string())
    })?;

    Ok(exe_parent
        .join("resources")
        .join("hars")
        .join(format!("{}.har", version)))
}

/// Resolve version string (converts "latest" to actual version number)
pub fn resolve_version(version: &str) -> String {
    if version == "latest" {
        LATEST_VERSION.to_string()
    } else {
        version.to_string()
    }
}
