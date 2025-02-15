#![warn(clippy::all, clippy::pedantic)]

pub mod fs;

pub use anyhow;
pub use walkdir;

// Re-export commonly used types
pub use std::{
    path::{Path, PathBuf},
    io::{self, Result as IoResult},
    sync::Arc,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::Mutex,
};
use walkdir::{DirEntry, WalkDir};

/// Determines if a directory entry is hidden.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check.
///
/// # Returns
///
/// `true` if the entry's file name starts with a dot (except "." and "..").
#[must_use = "Determines if the directory entry is hidden"]
pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != "." && s != "..")
        .unwrap_or(false)
}

/// Determines if a directory entry is a target directory.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check.
///
/// # Returns
///
/// `true` if the entry's file name is "target".
#[must_use = "Determines if the directory entry is a target directory"]
pub fn is_target_dir(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy() == "target"
}

/// Determines if a directory entry is a git repository directory.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check.
///
/// # Returns
///
/// `true` if the entry's file name is ".git".
#[must_use = "Determines if the directory entry is a git repository directory"]
pub fn is_git_dir(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy() == ".git"
}

/// Walks through a directory and applies a callback function to each file with the specified extension.
///
/// # Errors
///
/// Returns an `io::Error` if there's an issue with directory traversal or file operations.
#[must_use = "Walks through a directory and requires handling of the result to ensure proper file processing"]
pub async fn walk_directory<F, Fut>(
    dir: impl AsRef<Path>,
    extension: &str,
    callback: F
) -> anyhow::Result<()>
where
    F: Fn(&Path) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let dir_ref = dir.as_ref();
    #[cfg(debug_assertions)]
    println!("Starting walk of directory: {:?}", dir_ref);
    let walker = WalkDir::new(dir_ref).follow_links(true);

    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            let keep = !is_hidden(e) && !is_git_dir(e) && !is_target_dir(e);
            #[cfg(debug_assertions)]
            println!("Filtering entry: {:?}, keep: {}", e.path(), keep);
            keep
        })
        .filter_map(|r| {
            if let Ok(entry) = r {
                #[cfg(debug_assertions)]
                println!("Found valid entry: {:?}", entry.path());
                Some(entry)
            } else {
                #[cfg(debug_assertions)]
                println!("Invalid entry: {:?}", r.err());
                None
            }
        })
    {
        let path = entry.path().to_owned();
        #[cfg(debug_assertions)]
        println!("Processing path: {:?}", path);
        if let Some(ext) = path.extension() {
            #[cfg(debug_assertions)]
            println!("  Extension: {:?}", ext);
            if ext.to_string_lossy() == extension {
                #[cfg(debug_assertions)]
                println!("  Processing file: {:?}", path);
                callback(&path).await?;
            }
        }
    }

    Ok(())
}

/// Walks through Rust files in a directory and applies a callback function to each file.
/// Skips hidden folders (except "." and ".."), .git folders, and target folders.
///
/// # Errors
///
/// Returns an `io::Error` if a file cannot be opened or read.
pub async fn walk_rust_files<F, Fut>(dir: impl AsRef<Path>, callback: F) -> io::Result<()>
where
    F: Fn(&Path) -> Fut,
    Fut: std::future::Future<Output = io::Result<()>>,
{
    let walker = WalkDir::new(dir).follow_links(true);

    for entry in walker
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_git_dir(e) && !is_target_dir(e))
        .filter_map(Result::ok)
    {
        let path = entry.path().to_owned();
        if entry.file_type().is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            callback(&path).await?;
        }
    }

    Ok(())
}

/// Reads all lines from a file at the given path.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be opened or read.
#[must_use = "Reads all lines from a file and returns them, requiring handling of the result"]
pub async fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let file = File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        lines.push(line.trim().to_string());
        line.clear();
    }
    Ok(lines)
}

/// Reads the content of a file.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be opened or read.
#[must_use = "Reads the content of a file and requires handling of the result to ensure the content is retrieved"]
pub async fn read_file_content(path: &Path) -> io::Result<String> {
    tokio::fs::read_to_string(path).await
}

/// Writes content to a file at the specified path.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be created or written to.
#[must_use = "Writes content to a file and requires handling of the result to ensure data is saved"]
pub async fn write_to_file(path: &Path, content: &str) -> io::Result<()> {
    let mut file = File::create(path).await?;
    file.write_all(content.as_bytes()).await
}

/// Deletes files with a specific extension in a directory and its subdirectories.
///
/// # Errors
///
/// Returns an `io::Error` if there's an issue with file operations.
#[must_use = "Deletes files with a specific extension and requires handling of the result to ensure proper file deletion"]
pub async fn delete_files_with_extension(target_dir: &Path, extension: &str) -> io::Result<()> {
    let mut tasks = Vec::new();

    for entry in WalkDir::new(target_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path().to_owned();
        if path.is_file() {
            if let Some(file_extension) = path.extension() {
                if file_extension.eq_ignore_ascii_case(extension) {
                    tasks.push(tokio::spawn(async move {
                        if let Err(e) = tokio::fs::remove_file(&path).await {
                            eprintln!("Failed to remove {}: {e}", path.display());
                        } else {
                            println!("Removed: {}", path.display());
                        }
                    }));
                }
            }
        }
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}

/// Processes a file and adds it to a list if it contains multiple lines.
///
/// # Arguments
///
/// * `path` - A `PathBuf` representing the file path.
/// * `multi_line_files` - An `Arc<Mutex<Vec<PathBuf>>>` that holds the list of files with multiple lines.
///
/// # Returns
///
/// Returns a `Result<()>` indicating the success or failure of the operation.
///
/// # Errors
///
/// This function will return an error if:
/// * The path is invalid.
/// * The file cannot be read.
pub async fn check_file_for_multiple_lines(
    path: &Path,
    multi_line_files: Arc<Mutex<Vec<PathBuf>>>,
) -> anyhow::Result<()> {
    let content = read_file_content(path).await?;
    let line_count = content.lines().count();

    if line_count > 1 {
        println!("File with multiple lines found: {}", path.display());
        multi_line_files.lock().await.push(path.to_path_buf());
    }

    Ok(())
}

/// Opens a list of files in Neovim.
///
/// # Arguments
///
/// * `files` - A slice of `PathBuf` that holds the paths to the files.
///
/// # Returns
///
/// Returns a `Result<()>` indicating the success or failure of the operation.
///
/// # Errors
///
/// This function will return an error if:
/// * Neovim cannot be spawned.
/// * The process cannot wait for Neovim.
pub async fn open_files_in_neovim(files: &[PathBuf]) -> anyhow::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let mut command = Command::new("nvim");
    for file in files {
        command.arg(file);
    }

    command.spawn()?.wait().await?;
    Ok(())
}

/// Process a file with the given function
/// 
/// # Errors
/// Returns an error if:
/// - The processor function returns an error
pub async fn process_file<F, Fut>(path: &Path, processor: F) -> anyhow::Result<()>
where
    F: FnOnce(&Path) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    processor(path).await
}

/// Process a Rust file and check for pedantic warnings
/// 
/// # Errors
/// Returns an error if:
/// - The file cannot be read
/// - The file cannot be processed
pub async fn process_rust_file(path: &Path, files_without_warning: &mut Vec<PathBuf>) -> io::Result<()> {
    let content = read_file_content(path).await?;
    if !content.contains("#![warn(clippy::all, clippy::pedantic)]") {
        files_without_warning.push(path.to_path_buf());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_process_file() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?;

        let processed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let processed_clone = processed.clone();

        let processor = move |_path: &Path| async move {
            processed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };

        process_file(&file_path, processor).await?;
        assert!(processed.load(std::sync::atomic::Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn test_walk_directory() -> anyhow::Result<()> {
        // Create a non-hidden temporary directory
        let temp_dir = TempDir::new()?;
        let test_dir = temp_dir.path().join("test_dir");
        fs::create_dir(&test_dir)?;
        
        // Create test files with different extensions
        File::create(test_dir.join("test1.txt"))?;
        File::create(test_dir.join("test2.txt"))?;
        File::create(test_dir.join("test3.dat"))?;
        
        // Create a subdirectory with more files
        let sub_dir = test_dir.join("subdir");
        fs::create_dir(&sub_dir)?;
        File::create(sub_dir.join("test4.txt"))?;

        let processed_files = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let processed_files_clone = processed_files.clone();

        let processor = move |_path: &Path| {
            let counter = processed_files_clone.clone();
            async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        };

        walk_directory(&test_dir, "txt", processor).await?;
        
        // Should have processed 3 .txt files
        assert_eq!(processed_files.load(std::sync::atomic::Ordering::SeqCst), 3);
        Ok(())
    }
} 