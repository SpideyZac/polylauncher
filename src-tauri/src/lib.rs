use std::{
    collections::HashSet,
    fs::{read, remove_file, write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use files_diff::{apply, diff, CompressAlgorithm, DiffAlgorithm, Patch};
use rkyv::{access, deserialize, rancor::Error, to_bytes, Archive, Deserialize, Serialize};
use walkdir::WalkDir;

/// Enum representing the type of operation a patch entry represents.
#[derive(Archive, Serialize, Deserialize)]
enum PatchOperation {
    Add,    // File is added
    Remove, // File is removed
    Modify, // File is modified
}

/// A single entry in a patch, may contain the diff and relative path info.
#[derive(Archive, Serialize, Deserialize)]
struct PatchEntry {
    pub patch: Option<Patch>,      // Patch data if applicable
    pub operation: PatchOperation, // Operation type
    pub rel_path: String,          // Relative file path
}

impl PatchEntry {
    pub fn new(patch: Option<Patch>, operation: PatchOperation, rel_path: String) -> Self {
        Self {
            patch,
            operation,
            rel_path,
        }
    }
}

/// A package containing multiple patch entries.
#[derive(Archive, Serialize, Deserialize)]
struct PatchPackage {
    pub entries: Vec<PatchEntry>,
}

impl PatchPackage {
    pub fn new(entries: Vec<PatchEntry>) -> Self {
        Self { entries }
    }
}

/// Recursively collects all file paths under a directory, returning paths relative to `base_path`.
fn collect_file_paths(base_path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in WalkDir::new(base_path) {
        let entry = entry.with_context(|| {
            format!("Failed to read directory entry in: {}", base_path.display())
        })?;

        if entry.file_type().is_file() {
            // Strip the base path to get relative path
            let rel_path = entry
                .path()
                .strip_prefix(base_path)
                .with_context(|| {
                    format!(
                        "Failed to strip prefix from path: {}",
                        entry.path().display()
                    )
                })?
                .to_path_buf();
            paths.push(rel_path);
        }
    }

    Ok(paths)
}

/// Creates a patch file that represents changes between `path1` and `path2`.
pub fn create_patch(patch_loc: &Path, path1: &Path, path2: &Path) -> anyhow::Result<()> {
    let mut entries = Vec::new();

    // Collect relative file paths for both directories
    let paths1 =
        collect_file_paths(path1).context("Failed to collect files from first directory")?;
    let paths2 =
        collect_file_paths(path2).context("Failed to collect files from second directory")?;

    // Unique set of all file paths across both directories
    let unique_paths: HashSet<PathBuf> = paths1.into_iter().chain(paths2).collect();

    for rel_path in unique_paths {
        let file1 = path1.join(&rel_path);
        let file2 = path2.join(&rel_path);

        let exists_in_1 = file1.exists();
        let exists_in_2 = file2.exists();

        match (exists_in_1, exists_in_2) {
            (true, true) => {
                // File exists in both directories; compute modification patch
                let data1 = read(&file1)
                    .with_context(|| format!("Failed to read file: {}", file1.display()))?;
                let data2 = read(&file2)
                    .with_context(|| format!("Failed to read file: {}", file2.display()))?;

                if let Ok(patch) = diff(
                    &data1,
                    &data2,
                    DiffAlgorithm::Rsync020,
                    CompressAlgorithm::Zstd,
                ) {
                    entries.push(PatchEntry::new(
                        Some(patch),
                        PatchOperation::Modify,
                        rel_path.to_string_lossy().to_string(),
                    ));
                }
            }
            (true, false) => {
                // File removed in second directory
                entries.push(PatchEntry::new(
                    None,
                    PatchOperation::Remove,
                    rel_path.to_string_lossy().to_string(),
                ));
            }
            (false, true) => {
                // File added in second directory
                let data2 = read(&file2)
                    .with_context(|| format!("Failed to read file: {}", file2.display()))?;

                let patch = diff(
                    &[], // Empty original file
                    &data2,
                    DiffAlgorithm::Rsync020,
                    CompressAlgorithm::Zstd,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to create addition diff for file {}: {:?}",
                        rel_path.display(),
                        e
                    )
                })?;

                entries.push(PatchEntry::new(
                    Some(patch),
                    PatchOperation::Add,
                    rel_path.to_string_lossy().to_string(),
                ));
            }
            (false, false) => {
                // Should never happen, but safe to ignore
            }
        }
    }

    // Serialize and write the patch package to file
    let patch_package = PatchPackage::new(entries);
    let serialized = to_bytes::<Error>(&patch_package)
        .map_err(|e| anyhow!("Failed to serialize patch package: {:?}", e))?;

    write(patch_loc, serialized)
        .with_context(|| format!("Failed to write patch file: {}", patch_loc.display()))?;

    Ok(())
}

/// Applies a patch package to a target directory.
pub fn apply_patch(patch_loc: &Path, target_path: &Path) -> anyhow::Result<()> {
    // Read the serialized patch package
    let patch_data = read(patch_loc)
        .with_context(|| format!("Failed to read patch file: {}", patch_loc.display()))?;

    // Access archived patch package (without full deserialization)
    let patch_package_archive = access::<ArchivedPatchPackage, Error>(&patch_data)
        .map_err(|e| anyhow!("Failed to access archived patch package: {:?}", e))?;

    // Deserialize patch package
    let patch_package = deserialize::<PatchPackage, Error>(patch_package_archive)
        .map_err(|e| anyhow!("Failed to deserialize patch package: {:?}", e))?;

    for entry in patch_package.entries {
        let file_path = target_path.join(&entry.rel_path);

        match entry.operation {
            PatchOperation::Add => {
                if let Some(patch) = entry.patch {
                    // Apply addition patch (from empty data)
                    let added_data = apply(&[], &patch).map_err(|e| {
                        anyhow!(
                            "Failed to apply addition patch to file {}: {:?}",
                            file_path.display(),
                            e
                        )
                    })?;

                    write(&file_path, added_data).with_context(|| {
                        format!("Failed to write added file: {}", file_path.display())
                    })?;
                }
            }
            PatchOperation::Remove => {
                if file_path.exists() {
                    remove_file(&file_path).with_context(|| {
                        format!("Failed to remove file: {}", file_path.display())
                    })?;
                }
            }
            PatchOperation::Modify => {
                if let Some(patch) = entry.patch {
                    // Read current file and apply patch
                    let original_data = read(&file_path).with_context(|| {
                        format!(
                            "Failed to read file for modification: {}",
                            file_path.display()
                        )
                    })?;

                    let modified_data = apply(&original_data, &patch).map_err(|e| {
                        anyhow!(
                            "Failed to apply patch to file {}: {:?}",
                            file_path.display(),
                            e
                        )
                    })?;

                    write(&file_path, modified_data).with_context(|| {
                        format!("Failed to write modified file: {}", file_path.display())
                    })?;
                }
            }
        }
    }

    Ok(())
}
