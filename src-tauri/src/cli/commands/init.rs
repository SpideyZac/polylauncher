use std::{
    env::current_dir, fs::{copy, create_dir_all, read_dir, read_to_string, write}, io, path::Path, process::{Command, Stdio}
};

use colored::Colorize;
use serde_json::from_str;
use which::which;

use crate::{
    config::{
        get_har_file_path, get_template_project_dir, get_version_dir, resolve_version, URL_PREFIX,
    },
    downloader::{download_files_parallel, DownloadTask},
    error::{PolyError, PolyResult},
};

/// Handle the init command - downloads and sets up a PolyTrack version
pub fn handle_init(polytrack_version: String) -> PolyResult<()> {
    // Check if current directory is empty
    let cur_working_dir = current_dir().expect("Failed to get current working directory");
    if cur_working_dir
        .read_dir()
        .expect("Failed to read current directory")
        .next()
        .is_some()
    {
        return Err(PolyError::NonEmptyDir(cur_working_dir));
    }

    // Resolve version (converts "latest" to actual version number)
    let version = resolve_version(&polytrack_version);
    println!(
        "{}",
        format!("Initializing PolyTrack version {}...", version)
            .cyan()
            .bold()
    );

    // Get the installation directory
    let install_dir = get_version_dir(&version)?;

    // Check if already installed
    if install_dir.exists() {
        println!(
            "{}",
            format!("PolyTrack version {} is already installed.", version)
                .green()
                .bold()
        );
    } else {
        // Load the HAR file containing URLs to download
        let har_file = get_har_file_path(&version)?;
        if !har_file.exists() {
            return Err(PolyError::HarNotFound(version));
        }

        println!(
            "{}",
            format!("Reading HAR file: {}", har_file.display()).blue()
        );
        let har_contents = read_to_string(&har_file)?;
        let urls: Vec<String> = from_str(&har_contents)?;

        println!(
            "{}",
            format!("Found {} files to download", urls.len()).blue()
        );

        // Create download tasks
        let prefix = format!("{}{}/", URL_PREFIX, version);
        let tasks = create_download_tasks(&urls, &prefix, &install_dir)?;

        // Download all files
        download_files_parallel(tasks)?;

        println!(
            "\n{}",
            format!("✓ Successfully initialized PolyTrack version {}", version)
                .green()
                .bold()
        );
        println!("Installation directory: {}", install_dir.display());
    }

    // Try to initialize a git repository if git is available
    if which("git").is_ok() {
        match Command::new("git")
            .stdout(Stdio::null())
            .arg("init")
            .status()
        {
            Ok(status) if status.success() => {
                println!("{}", "Initialized empty Git repository.".green().bold());
            }
            _ => {
                eprintln!(
                    "{}",
                    "Warning: Failed to initialize Git repository.".yellow()
                );
            }
        }
    }

    // Copy template project files
    let template_dir = get_template_project_dir()?;
    if template_dir.exists() {
        println!("{}", "Copying template project files...".blue());
        copy_dir_recursive(&template_dir, &cur_working_dir)?;
        
        // Update polylauncher.json with the correct version
        let polylauncher_json_path = cur_working_dir.join("polylauncher.json");
        if polylauncher_json_path.exists() {
            let mut polylauncher_json = read_to_string(&polylauncher_json_path)?;
            polylauncher_json = polylauncher_json.replace("<placeholder>", &version);
            write(&polylauncher_json_path, polylauncher_json)?;
        }

        println!("{}", "Template project files copied.".green().bold());
    }

    // Copy version's files to patched/ directory in current directory
    let patched_dir = cur_working_dir.join("patched");
    create_dir_all(&patched_dir)?;
    println!("{}", "Copying version files to patched/ directory...".blue());
    copy_dir_recursive(&install_dir, &patched_dir)?;

    // TODO: Additional setup steps can be added here

    Ok(())
}

/// Copies files from source to destination directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    create_dir_all(dst)?;

    for entry in read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Create download tasks from URLs
fn create_download_tasks(
    urls: &[String],
    prefix: &str,
    install_dir: &Path,
) -> PolyResult<Vec<DownloadTask>> {
    let mut tasks = Vec::new();

    for url in urls {
        // Strip prefix to get relative file path
        let file_path = url.strip_prefix(prefix).ok_or_else(|| {
            PolyError::PathError(format!("URL doesn't start with expected prefix: {}", url))
        })?;

        // Use index.html as default for empty path
        let file_path = if file_path.is_empty() {
            "index.html"
        } else {
            file_path
        };

        let dest_path = install_dir.join(file_path);

        tasks.push(DownloadTask {
            url: url.clone(),
            dest_path,
            display_name: file_path.to_string(),
        });
    }

    Ok(tasks)
}
