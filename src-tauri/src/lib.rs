// TODO: Don't load entire files into memory at once for large files.

use std::{
    collections::HashSet,
    fs::{create_dir_all, read, remove_dir, remove_file, symlink_metadata, write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context};
use files_diff::{apply, diff, hash, CompressAlgorithm, DiffAlgorithm, Patch};
use rkyv::{access, deserialize, rancor::Error, to_bytes, Archive, Deserialize, Serialize};
use walkdir::WalkDir;

const PATCH_PACKAGE_VERSION: u32 = 1;

/// Enum representing the type of operation a patch entry represents.
#[derive(Archive, Serialize, Deserialize)]
enum PatchOperation {
    Add(Vec<u8>),  // File is added
    Remove,        // File is removed
    Modify(Patch), // File is modified
}

/// A single entry in a patch, may contain the diff and relative path info.
#[derive(Archive, Serialize, Deserialize)]
struct PatchEntry {
    pub operation: PatchOperation, // Operation type
    pub rel_path: String,          // Relative file path
}

impl PatchEntry {
    pub fn new(operation: PatchOperation, rel_path: String) -> Self {
        let rel_path = rel_path.replace("\\", "/"); // Normalize to forward slashes
        Self {
            operation,
            rel_path,
        }
    }
}

/// A package containing multiple patch entries.
#[derive(Archive, Serialize, Deserialize)]
struct PatchPackage {
    pub version: u32, // Version for future compatibility
    pub entries: Vec<PatchEntry>,
}

impl PatchPackage {
    pub fn new(version: u32, entries: Vec<PatchEntry>) -> Self {
        Self { version, entries }
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
    let mut unique_paths: Vec<_> = unique_paths.into_iter().collect();
    unique_paths.sort();

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

                if data1 == data2 {
                    // No changes, skip
                    continue;
                }

                let patch = diff(
                    &data1,
                    &data2,
                    DiffAlgorithm::Rsync020,
                    CompressAlgorithm::Zstd,
                )
                .map_err(|e| {
                    anyhow!(
                        "Failed to compute diff for file: {}: {:?}",
                        rel_path.display(),
                        e
                    )
                })?;

                entries.push(PatchEntry::new(
                    PatchOperation::Modify(patch),
                    rel_path.to_string_lossy().to_string(),
                ));
            }
            (true, false) => {
                // File removed in second directory
                entries.push(PatchEntry::new(
                    PatchOperation::Remove,
                    rel_path.to_string_lossy().to_string(),
                ));
            }
            (false, true) => {
                // File added in second directory
                let data2 = read(&file2)
                    .with_context(|| format!("Failed to read file: {}", file2.display()))?;

                entries.push(PatchEntry::new(
                    PatchOperation::Add(data2),
                    rel_path.to_string_lossy().to_string(),
                ));
            }
            (false, false) => {
                // Should never happen, but safe to ignore
            }
        }
    }

    // Serialize and write the patch package to file
    let patch_package = PatchPackage::new(PATCH_PACKAGE_VERSION, entries);
    let serialized = to_bytes::<Error>(&patch_package)
        .map_err(|e| anyhow!("Failed to serialize patch package: {:?}", e))?;

    write(patch_loc, serialized)
        .with_context(|| format!("Failed to write patch file: {}", patch_loc.display()))?;

    Ok(())
}

/// Verifies that a path and all its components are not symlinks.
fn verify_no_symlinks_in_path(path: &Path) -> anyhow::Result<()> {
    let mut cur = PathBuf::new();
    for comp in path.components() {
        cur.push(comp);
        if let Ok(meta) = symlink_metadata(&cur) {
            ensure!(
                !meta.file_type().is_symlink(),
                "Refusing to traverse symlink component: {}",
                cur.display()
            );
        }
    }
    Ok(())
}

/// Safely removes empty parent directories up to (but not including) the target path.
fn remove_empty_parents(file_path: &Path, target_path: &Path) {
    let mut current_path = file_path.parent();
    while let Some(dir) = current_path {
        // Stop if we reach the target path
        if dir == target_path {
            break;
        }

        match dir.read_dir() {
            Ok(entries) => {
                let mut entries = entries;
                if entries.next().is_none() {
                    // Directory is empty, try to remove it (ignore errors)
                    let _ = remove_dir(dir);
                    current_path = dir.parent();
                } else {
                    break; // Directory has entries
                }
            }
            _ => break, // Can't read directory
        }
    }
}

/// Applies a patch package to a target directory.
/// NOTE: It is recommended to back up data before applying patches. This operation may corrupt data.
pub fn apply_patch(patch_loc: &Path, target_path: &Path) -> anyhow::Result<()> {
    // Verify target path is not a symlink
    let meta = symlink_metadata(target_path)?;
    ensure!(
        !meta.file_type().is_symlink(),
        "Target path must not be a symlink"
    );

    // Read the serialized patch package
    let patch_data = read(patch_loc)
        .with_context(|| format!("Failed to read patch file: {}", patch_loc.display()))?;

    // Access archived patch package (without full deserialization - calling methods on it is unsafe)
    let patch_package_archive = access::<ArchivedPatchPackage, Error>(&patch_data)
        .map_err(|e| anyhow!("Failed to access archived patch package: {:?}", e))?;

    // Deserialize patch package
    let patch_package = deserialize::<PatchPackage, Error>(patch_package_archive)
        .map_err(|e| anyhow!("Failed to deserialize patch package: {:?}", e))?;

    // Verify version compatibility
    ensure!(
        patch_package.version == PATCH_PACKAGE_VERSION,
        "Unsupported patch version: {} (expected {})",
        patch_package.version,
        PATCH_PACKAGE_VERSION
    );

    for entry in patch_package.entries {
        let joined = target_path.join(&entry.rel_path);

        // Normalize path without touching filesystem
        let normalized = joined.components().fold(PathBuf::new(), |mut acc, c| {
            match c {
                std::path::Component::ParentDir => {
                    acc.pop();
                }
                std::path::Component::CurDir => {
                    // Skip "." components
                }
                _ => {
                    acc.push(c);
                }
            }
            acc
        });

        // Ensure it stays within target_path
        ensure!(
            normalized.starts_with(target_path),
            "Patch entry path {} escapes target directory {}",
            entry.rel_path,
            target_path.display()
        );

        let file_path = normalized;

        // Defense in depth: verify no symlinks in the entire path
        verify_no_symlinks_in_path(&file_path)?;

        match entry.operation {
            PatchOperation::Add(data) => {
                // Ensure parent directories exist
                if let Some(parent) = file_path.parent() {
                    create_dir_all(parent).with_context(|| {
                        format!(
                            "Failed to create parent directories for file: {}",
                            file_path.display()
                        )
                    })?;
                }

                // Final check: ensure we're not overwriting a symlink
                if file_path.exists() {
                    ensure!(
                        !symlink_metadata(&file_path)?.file_type().is_symlink(),
                        "Refusing to overwrite symlink: {}",
                        file_path.display()
                    );
                }

                write(&file_path, data).with_context(|| {
                    format!("Failed to write added file: {}", file_path.display())
                })?;
            }
            PatchOperation::Remove => {
                if file_path.exists() {
                    // Final check: ensure we're not removing a symlink
                    ensure!(
                        !symlink_metadata(&file_path)?.file_type().is_symlink(),
                        "Refusing to remove symlink: {}",
                        file_path.display()
                    );

                    remove_file(&file_path).with_context(|| {
                        format!("Failed to remove file: {}", file_path.display())
                    })?;

                    // Safely remove empty parent directories
                    remove_empty_parents(&file_path, target_path);
                }
            }
            PatchOperation::Modify(patch) => {
                // Final check: ensure we're not modifying a symlink
                ensure!(
                    !symlink_metadata(&file_path)?.file_type().is_symlink(),
                    "Refusing to modify symlink: {}",
                    file_path.display()
                );

                // Read current file and apply patch
                let original_data = read(&file_path).with_context(|| {
                    format!(
                        "Failed to read file for modification: {}",
                        file_path.display()
                    )
                })?;

                // Verify the hash before applying patch
                if hash(&original_data) != patch.before_hash {
                    return Err(anyhow!(
                        "Hash mismatch before applying patch to file: {}. File may have been modified.",
                        file_path.display()
                    ));
                }

                let modified_data = apply(&original_data, &patch).map_err(|e| {
                    anyhow!(
                        "Failed to apply patch to file {}: {:?}",
                        file_path.display(),
                        e
                    )
                })?;

                // Verify the hash after applying patch
                if hash(&modified_data) != patch.after_hash {
                    return Err(anyhow!(
                        "Hash mismatch after applying patch to file: {}. Patch may be corrupted.",
                        file_path.display()
                    ));
                }

                write(&file_path, modified_data).with_context(|| {
                    format!("Failed to write modified file: {}", file_path.display())
                })?;
            }
        }
    }

    Ok(())
}
