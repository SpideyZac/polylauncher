use std::{
    fs::{create_dir_all, write},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use colored::Colorize;
use reqwest::blocking::get;

use crate::{
    config::{MAX_DOWNLOAD_RETRIES, RETRY_DELAY_SECS},
    error::{PolyError, PolyResult},
};

/// Represents a file to be downloaded
#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest_path: PathBuf,
    pub display_name: String,
}

/// Download statistics for tracking progress
#[derive(Debug)]
pub struct DownloadStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
}

impl DownloadStats {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            failed: 0,
        }
    }
}

/// Downloads a single file with retry logic
fn download_file_with_retry(task: &DownloadTask) -> PolyResult<()> {
    // Ensure parent directory exists
    if let Some(parent) = task.dest_path.parent() {
        create_dir_all(parent)?;
    }

    let mut last_error = None;

    // Retry loop
    for attempt in 1..=MAX_DOWNLOAD_RETRIES {
        println!(
            "{}",
            format!(
                "Downloading {} (attempt {}/{})...",
                task.display_name, attempt, MAX_DOWNLOAD_RETRIES
            )
            .blue()
        );

        match get(&task.url) {
            Ok(response) => {
                // Check if the response is successful
                if response.status().is_success() {
                    // Read response bytes
                    match response.bytes() {
                        Ok(bytes) => {
                            // Write to file
                            write(&task.dest_path, &bytes)?;
                            println!(
                                "{}",
                                format!("✓ Successfully downloaded {}", task.display_name).green()
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            last_error = Some(format!("Failed to read response bytes: {}", e));
                        }
                    }
                } else {
                    last_error = Some(format!("HTTP status: {}", response.status()));
                }
            }
            Err(e) => {
                last_error = Some(format!("Network error: {}", e));
            }
        }

        // Log the error and wait before retrying (except on last attempt)
        if let Some(ref err) = last_error {
            eprintln!(
                "{}",
                format!(
                    "✗ Failed to download {}: {} (attempt {}/{})",
                    task.display_name, err, attempt, MAX_DOWNLOAD_RETRIES
                )
                .yellow()
            );

            if attempt < MAX_DOWNLOAD_RETRIES {
                sleep(Duration::from_secs(RETRY_DELAY_SECS));
            }
        }
    }

    // All retries failed
    Err(PolyError::DownloadError(format!(
        "Failed to download {} after {} attempts: {}",
        task.display_name,
        MAX_DOWNLOAD_RETRIES,
        last_error.unwrap_or_else(|| "Unknown error".to_string())
    )))
}

/// Downloads multiple files in parallel
pub fn download_files_parallel(tasks: Vec<DownloadTask>) -> PolyResult<DownloadStats> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let total = tasks.len();
    let stats = Arc::new(Mutex::new(DownloadStats::new(total)));

    println!(
        "{}",
        format!("Starting download of {} files...", total).cyan()
    );

    // Download files in parallel
    tasks
        .par_iter()
        .for_each(|task| match download_file_with_retry(task) {
            Ok(_) => {
                let mut stats = stats.lock().unwrap();
                stats.completed += 1;
            }
            Err(e) => {
                eprintln!("{}", format!("✗ {}", e).red());
                let mut stats = stats.lock().unwrap();
                stats.failed += 1;
            }
        });

    let final_stats = Arc::try_unwrap(stats)
        .expect("Failed to unwrap stats")
        .into_inner()
        .unwrap();

    // Print summary
    println!("\n{}", "Download Summary:".cyan().bold());
    println!("  Total files: {}", final_stats.total);
    println!(
        "  {}",
        format!("✓ Successful: {}", final_stats.completed).green()
    );

    if final_stats.failed > 0 {
        println!("  {}", format!("✗ Failed: {}", final_stats.failed).red());
    }

    // Return error if any downloads failed
    if final_stats.failed > 0 {
        return Err(PolyError::DownloadError(format!(
            "{} out of {} files failed to download",
            final_stats.failed, final_stats.total
        )));
    }

    Ok(final_stats)
}
