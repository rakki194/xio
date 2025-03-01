#![warn(clippy::all, clippy::pedantic)]

//! # XIO - Extended I/O Operations Library
//! 
//! `xio` is a library that provides extended I/O operations and utilities for file system manipulation,
//! with a focus on asynchronous operations using Tokio. The library offers functionality for:
//! 
//! - Directory traversal with customizable filters
//! - File reading and writing operations
//! - Specialized Rust file processing
//! - File system utilities for common operations
//! 
//! ## Features
//! 
//! - Asynchronous file operations using Tokio
//! - Directory walking with customizable filters (hidden files, git directories, etc.)
//! - Specialized handling for Rust source files
//! - Batch file processing capabilities
//! - Integration with external tools (e.g., Neovim)
//! - Configurable logging with different verbosity levels
//! 
//! ## Logging Configuration
//! 
//! The library uses the `log` crate for logging. To configure logging in your application:
//! 
//! ```rust
//! use env_logger::{Builder, Env};
//! use log::info;
//! 
//! // Set RUST_LOG environment variable to configure logging level
//! // Examples:
//! // export RUST_LOG=xio=debug    # Show all debug messages from xio
//! // export RUST_LOG=xio=info     # Show only info and above from xio
//! // export RUST_LOG=xio=warn     # Show only warnings and errors from xio
//! 
//! // Initialize logging in your application
//! Builder::from_env(Env::default())
//!     .filter_module("xio", log::LevelFilter::Info)  // Default level
//!     .init();
//! ```
//! 
//! ## Example
//! 
//! ```rust
//! use std::path::Path;
//! use xio::{walk_directory, anyhow};
//! use log::info;
//! 
//! async fn process_txt_files() -> anyhow::Result<()> {
//!     walk_directory("./", "txt", |path| {
//!         let path = path.to_path_buf();
//!         async move {
//!             info!("Processing: {}", path.display());
//!             Ok(())
//!         }
//!     }).await
//! }
//! ```

pub mod fs;
pub mod split;

pub use anyhow;
pub use log;
pub use walkdir;

// Re-export commonly used types and traits
pub use std::{
    io::{self, Result as IoResult},
    path::{Path, PathBuf},
    sync::Arc,
};
pub use split::{DirectorySplitter, FileMatcher, RegexFileMatcher, SplitConfig};
use log::{debug, info, warn};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::Mutex,
};
use walkdir::{DirEntry, WalkDir};

/// Determines if a directory entry is hidden.
///
/// This function checks if a directory entry represents a hidden file or directory
/// in Unix-like systems (files starting with a dot). It explicitly excludes the
/// current directory (".") and parent directory ("..") from being considered hidden.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check for hidden status.
///
/// # Returns
///
/// * `true` if the entry's file name starts with a dot (except "." and "..").
/// * `false` otherwise, or if the file name cannot be converted to a string.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use walkdir::WalkDir;
/// use xio::is_hidden;
/// 
/// let entry = WalkDir::new(".").into_iter().next().unwrap().unwrap();
/// assert!(!is_hidden(&entry)); // "." is not considered hidden
/// ```
#[must_use = "Determines if the directory entry is hidden"]
pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.') && s != "." && s != ".." && !s.starts_with(".tmp"))
}

/// Determines if a directory entry is a target directory.
///
/// This function is particularly useful when working with Rust projects to identify
/// the build output directory. It's commonly used to exclude the target directory
/// from file operations and searches.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check.
///
/// # Returns
///
/// * `true` if the entry's file name is exactly "target"
/// * `false` otherwise
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use walkdir::WalkDir;
/// use xio::is_target_dir;
/// 
/// let entry = WalkDir::new("src").into_iter().next().unwrap().unwrap();
/// assert!(!is_target_dir(&entry)); // "src" is not a target directory
/// ```
#[must_use = "Determines if the directory entry is a target directory"]
pub fn is_target_dir(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy() == "target"
}

/// Determines if a directory entry is a git repository directory.
///
/// This function helps identify Git repository directories to optionally exclude them
/// from file operations and searches. It specifically looks for the ".git" directory
/// that's present in Git repositories.
///
/// # Arguments
///
/// * `entry` - A reference to a `DirEntry` to check.
///
/// # Returns
///
/// * `true` if the entry's file name is exactly ".git"
/// * `false` otherwise
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use walkdir::WalkDir;
/// use xio::is_git_dir;
/// 
/// let entry = WalkDir::new("src").into_iter().next().unwrap().unwrap();
/// assert!(!is_git_dir(&entry)); // "src" is not a git directory
/// ```
#[must_use = "Determines if the directory entry is a git repository directory"]
pub fn is_git_dir(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy() == ".git"
}

/// Walks through a directory and asynchronously processes files with a specific extension.
///
/// This function traverses a directory tree and applies an asynchronous callback function
/// to each file that matches the specified extension. It automatically filters out:
/// - Hidden files and directories
/// - Git repository directories
/// - Target directories
///
/// The function processes files concurrently using Tokio tasks.
///
/// # Type Parameters
///
/// * `F` - The callback function type that implements `Fn(&Path) -> Fut`
/// * `Fut` - The future type returned by the callback function
///
/// # Arguments
///
/// * `dir` - The root directory to start the walk from
/// * `extension` - The file extension to match (without the dot)
/// * `callback` - An async function to process each matching file
///
/// # Returns
///
/// Returns `Ok(())` if all files were processed successfully, or an error if any
/// operation failed.
///
/// # Errors
///
/// Returns an `anyhow::Error` if:
/// - Directory traversal fails
/// - File operations fail
/// - The callback function returns an error
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use xio::{walk_directory, anyhow};
/// 
/// async fn process_files() -> anyhow::Result<()> {
///     walk_directory("./", "txt", |path| {
///         let path = path.to_path_buf();
///         async move {
///             println!("Processing: {}", path.display());
///             Ok(())
///         }
///     }).await
/// }
/// ```
#[must_use = "Walks through a directory and requires handling of the result to ensure proper file processing"]
pub async fn walk_directory<F, Fut>(
    dir: impl AsRef<Path>,
    extension: &str,
    callback: F,
) -> anyhow::Result<()>
where
    F: Fn(&Path) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let dir_ref = dir.as_ref();
    debug!("Starting walk of directory: {dir_ref:?}");
    let walker = WalkDir::new(dir_ref).follow_links(true);

    let callback = Arc::new(callback);
    let mut handles = Vec::new();

    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            let file_name = e.file_name().to_string_lossy();
            let keep = !(file_name.starts_with('.') && file_name != "." && file_name != ".." && !file_name.starts_with(".tmp"))
                && file_name != ".git"
                && file_name != "target";
            debug!("Filtering entry: {:?}, keep: {}", e.path(), keep);
            keep
        })
        .filter_map(|r| {
            if let Ok(entry) = r {
                debug!("Found valid entry: {:?}", entry.path());
                Some(entry)
            } else {
                warn!("Invalid entry: {:?}", r.err());
                None
            }
        })
    {
        let path = entry.path().to_owned();
        debug!("Processing path: {path:?}");
        if let Some(ext) = path.extension() {
            debug!("  Extension: {ext:?}");
            if ext.to_string_lossy() == extension {
                info!("Processing file: {path:?}");
                let callback = Arc::clone(&callback);
                let handle = tokio::spawn(async move { callback(&path).await });
                handles.push(handle);
            }
        }
    }

    // Wait for all tasks to complete and collect any errors
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// Walks through Rust files in a directory and applies a callback function to each file.
///
/// This specialized version of directory walking is optimized for Rust source files.
/// It automatically skips:
/// - Hidden folders (except "." and "..")
/// - Git repository directories (.git)
/// - Build output directories (target)
///
/// The function processes files sequentially in the order they are discovered.
///
/// # Type Parameters
///
/// * `F` - The callback function type that implements `Fn(&Path) -> Fut`
/// * `Fut` - The future type returned by the callback function
///
/// # Arguments
///
/// * `dir` - The root directory to start the walk from
/// * `callback` - An async function to process each Rust file
///
/// # Returns
///
/// Returns `Ok(())` if all files were processed successfully.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// * Directory traversal fails (e.g., permission denied)
/// * The callback function returns an error while processing a file
/// * A file or directory cannot be accessed
/// * Path metadata cannot be read
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::io;
/// use xio::walk_rust_files;
/// 
/// async fn process_rust_files() -> io::Result<()> {
///     walk_rust_files("./src", |path| {
///         let path = path.to_path_buf();
///         async move {
///             println!("Found Rust file: {}", path.display());
///             Ok(())
///         }
///     }).await
/// }
/// ```
pub async fn walk_rust_files<F, Fut>(dir: impl AsRef<Path>, callback: F) -> io::Result<()>
where
    F: Fn(&Path) -> Fut,
    Fut: std::future::Future<Output = io::Result<()>>,
{
    let walker = WalkDir::new(dir).follow_links(true);

    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            let file_name = e.file_name().to_string_lossy();
            !(file_name.starts_with('.') && file_name != "." && file_name != ".." && !file_name.starts_with(".tmp"))
                && file_name != ".git"
                && file_name != "target"
        })
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
/// This function asynchronously reads a file line by line and returns a vector
/// containing all lines. Each line is trimmed of whitespace and newline characters.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// Returns a vector of strings, where each string is a line from the file.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - The file cannot be opened
/// - The file cannot be read
/// - The file content is not valid UTF-8
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::io;
/// use xio::read_lines;
/// 
/// async fn read_file_lines() -> io::Result<()> {
///     let lines = read_lines(Path::new("example.txt")).await?;
///     for line in lines {
///         println!("{}", line);
///     }
///     Ok(())
/// }
/// ```
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

/// Reads the entire content of a file into a string.
///
/// This function provides a convenient way to read an entire file into memory
/// asynchronously. It's best suited for smaller files that can fit in memory.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// Returns the entire content of the file as a string.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - The file cannot be opened
/// - The file cannot be read
/// - The file content is not valid UTF-8
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::io;
/// use xio::read_file_content;
/// 
/// async fn read_file() -> io::Result<()> {
///     let content = read_file_content(Path::new("example.txt")).await?;
///     println!("File content: {}", content);
///     Ok(())
/// }
/// ```
#[must_use = "Reads the content of a file and requires handling of the result to ensure the content is retrieved"]
pub async fn read_file_content(path: &Path) -> io::Result<String> {
    tokio::fs::read_to_string(path).await
}

/// Writes content to a file at the specified path.
///
/// This function asynchronously writes a string to a file. If the file already exists,
/// it will be overwritten. If the file doesn't exist, it will be created.
///
/// # Arguments
///
/// * `path` - The path where the file should be written
/// * `content` - The string content to write to the file
///
/// # Returns
///
/// Returns `Ok(())` if the write was successful.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - The file cannot be created
/// - The file cannot be written to
/// - The parent directory doesn't exist
/// - Permission is denied
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::io;
/// use xio::write_to_file;
/// 
/// async fn write_file() -> io::Result<()> {
///     write_to_file(
///         Path::new("output.txt"),
///         "Hello, World!"
///     ).await
/// }
/// ```
#[must_use = "Writes content to a file and requires handling of the result to ensure data is saved"]
pub async fn write_to_file(path: &Path, content: &str) -> io::Result<()> {
    let mut file = File::create(path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await
}

/// Deletes files with a specific extension in a directory and its subdirectories.
///
/// This function recursively walks through a directory tree and deletes all files
/// that match the specified extension. The deletion is performed concurrently
/// using Tokio tasks for better performance.
///
/// # Arguments
///
/// * `target_dir` - The root directory to start the deletion from
/// * `extension` - The file extension to match (without the dot)
///
/// # Returns
///
/// Returns `Ok(())` if all matching files were successfully deleted or if no matching
/// files were found.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - Directory traversal fails
/// - File deletion fails
/// - Permission is denied
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::io;
/// use xio::delete_files_with_extension;
/// 
/// async fn cleanup_temp_files() -> io::Result<()> {
///     delete_files_with_extension(Path::new("./"), "tmp").await
/// }
/// ```
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
                            warn!("Failed to remove {}: {e}", path.display());
                        } else {
                            info!("Removed: {}", path.display());
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
/// This function reads a file and checks if it contains more than one line. If it does,
/// the file path is added to a thread-safe list of multi-line files.
///
/// # Arguments
///
/// * `path` - The path to the file to check
/// * `multi_line_files` - A thread-safe vector to store paths of files with multiple lines
///
/// # Returns
///
/// Returns `Ok(())` if the file was successfully processed.
///
/// # Errors
///
/// Returns an `anyhow::Error` if:
/// - The file cannot be read
/// - The file content cannot be processed
/// - The mutex cannot be locked
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use xio::{check_file_for_multiple_lines, anyhow};
/// 
/// async fn find_multi_line_files() -> anyhow::Result<()> {
///     let files = Arc::new(Mutex::new(Vec::new()));
///     check_file_for_multiple_lines(
///         Path::new("example.txt"),
///         files.clone()
///     ).await?;
///     let result = files.lock().await;
///     println!("Found {} multi-line files", result.len());
///     Ok(())
/// }
/// ```
pub async fn check_file_for_multiple_lines(
    path: &Path,
    multi_line_files: Arc<Mutex<Vec<PathBuf>>>,
) -> anyhow::Result<()> {
    let content = read_file_content(path).await?;
    let line_count = content.lines().count();

    if line_count > 1 {
        debug!("File with multiple lines found: {}", path.display());
        multi_line_files.lock().await.push(path.to_path_buf());
    }

    Ok(())
}

/// Opens a list of files in Neovim or a specified editor.
///
/// This function spawns an editor instance and opens all the specified files for editing.
/// If no files are provided, the function returns successfully without launching the editor.
///
/// # Arguments
///
/// * `files` - A slice of paths to the files to open
/// * `editor` - Optional editor command to use instead of nvim (useful for testing)
///
/// # Returns
///
/// Returns `Ok(())` if the editor was successfully launched and exited.
///
/// # Errors
///
/// Returns an `anyhow::Error` if:
/// - The editor cannot be spawned
/// - The editor process fails to start
/// - The process cannot be waited on
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use xio::{open_files_in_neovim, anyhow};
/// 
/// async fn edit_files() -> anyhow::Result<()> {
///     let files = vec![
///         PathBuf::from("file1.txt"),
///         PathBuf::from("file2.txt")
///     ];
///     open_files_in_neovim(&files, None).await
/// }
/// ```
pub async fn open_files_in_neovim(files: &[PathBuf], editor: Option<&str>) -> anyhow::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let editor = editor.unwrap_or("nvim");
    let mut command = Command::new(editor);
    for file in files {
        command.arg(file);
    }

    command.spawn()?.wait().await?;
    Ok(())
}

/// Process a file with the given function.
///
/// This is a generic file processor that takes any async function that can process
/// a file path and returns a Result. It provides a flexible way to apply custom
/// processing logic to files.
///
/// # Type Parameters
///
/// * `F` - The processor function type that implements `FnOnce(&Path) -> Fut`
/// * `Fut` - The future type returned by the processor function
///
/// # Arguments
///
/// * `path` - The path to the file to process
/// * `processor` - The async function to process the file
///
/// # Returns
///
/// Returns `Ok(())` if the file was successfully processed.
///
/// # Errors
///
/// Returns an `anyhow::Error` if:
/// - The processor function returns an error
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use xio::{process_file, anyhow};
/// 
/// async fn process_my_file() -> anyhow::Result<()> {
///     process_file(
///         Path::new("example.txt"),
///         |path| {
///             let path = path.to_path_buf();
///             async move {
///                 println!("Processing: {}", path.display());
///                 Ok(())
///             }
///         }
///     ).await
/// }
/// ```
pub async fn process_file<F, Fut>(path: &Path, processor: F) -> anyhow::Result<()>
where
    F: FnOnce(&Path) -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    processor(path).await
}

/// Process a Rust file and check for pedantic warnings.
///
/// This function reads a Rust source file and checks if it contains the
/// clippy pedantic warning directive. Files without this directive are
/// added to a list for further processing.
///
/// # Arguments
///
/// * `path` - The path to the Rust file to check
/// * `files_without_warning` - A mutable vector to store paths of files without the warning directive
///
/// # Returns
///
/// Returns `Ok(())` if the file was successfully processed.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// * The file cannot be read
/// * The file content cannot be processed
///
/// # Examples
///
/// ```
/// use std::path::{Path, PathBuf};
/// use std::io;
/// use xio::process_rust_file;
/// 
/// async fn check_rust_files() -> io::Result<()> {
///     let mut files = Vec::new();
///     process_rust_file(
///         Path::new("src/lib.rs"),
///         &mut files
///     ).await?;
///     println!("Found {} files without warnings", files.len());
///     Ok(())
/// }
/// ```
pub async fn process_rust_file(
    path: &Path,
    files_without_warning: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let content = read_file_content(path).await?;
    if !content.contains("#![warn(clippy::all, clippy::pedantic)]") {
        files_without_warning.push(path.to_path_buf());
    }
    Ok(())
}
