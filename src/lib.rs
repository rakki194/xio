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
//! 
//! ## Example
//! 
//! ```rust
//! use xio::{walk_directory, Path};
//! 
//! async fn process_txt_files() -> anyhow::Result<()> {
//!     walk_directory("./", "txt", |path| async move {
//!         println!("Processing: {}", path.display());
//!         Ok(())
//!     }).await
//! }
//! ```

pub mod fs;

pub use anyhow;
pub use walkdir;

// Re-export commonly used types
pub use std::{
    io::{self, Result as IoResult},
    path::{Path, PathBuf},
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
/// use walkdir::DirEntry;
/// use std::path::Path;
/// 
/// let entry = DirEntry::from_path(Path::new(".hidden_file")).unwrap();
/// assert!(is_hidden(&entry));
/// 
/// let entry = DirEntry::from_path(Path::new("visible_file")).unwrap();
/// assert!(!is_hidden(&entry));
/// ```
#[must_use = "Determines if the directory entry is hidden"]
pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.') && s != "." && s != "..")
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
/// use walkdir::DirEntry;
/// use std::path::Path;
/// 
/// let entry = DirEntry::from_path(Path::new("target")).unwrap();
/// assert!(is_target_dir(&entry));
/// 
/// let entry = DirEntry::from_path(Path::new("src")).unwrap();
/// assert!(!is_target_dir(&entry));
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
/// use walkdir::DirEntry;
/// use std::path::Path;
/// 
/// let entry = DirEntry::from_path(Path::new(".git")).unwrap();
/// assert!(is_git_dir(&entry));
/// 
/// let entry = DirEntry::from_path(Path::new("src")).unwrap();
/// assert!(!is_git_dir(&entry));
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
/// 
/// async fn process_files() -> anyhow::Result<()> {
///     walk_directory("./", "rs", |path| async move {
///         println!("Processing Rust file: {}", path.display());
///         Ok(())
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
    println!("Starting walk of directory: {dir_ref:?}");
    let walker = WalkDir::new(dir_ref).follow_links(true);

    let callback = Arc::new(callback);
    let mut handles = Vec::new();

    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            let keep = !is_hidden(e) && !is_git_dir(e) && !is_target_dir(e);
            println!("Filtering entry: {:?}, keep: {}", e.path(), keep);
            keep
        })
        .filter_map(|r| {
            if let Ok(entry) = r {
                println!("Found valid entry: {:?}", entry.path());
                Some(entry)
            } else {
                println!("Invalid entry: {:?}", r.err());
                None
            }
        })
    {
        let path = entry.path().to_owned();
        println!("Processing path: {path:?}");
        if let Some(ext) = path.extension() {
            println!("  Extension: {ext:?}");
            if ext.to_string_lossy() == extension {
                println!("  Processing file: {path:?}");
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
/// Returns `Ok(())` if all files were processed successfully, or an error if any
/// operation failed.
///
/// # Errors
///
/// Returns an `io::Error` if:
/// - Directory traversal fails
/// - File operations fail
/// - The callback function returns an error
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// 
/// async fn process_rust_files() -> io::Result<()> {
///     walk_rust_files("./src", |path| async move {
///         println!("Found Rust file: {}", path.display());
///         Ok(())
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
    file.write_all(content.as_bytes()).await
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
        println!("File with multiple lines found: {}", path.display());
        multi_line_files.lock().await.push(path.to_path_buf());
    }

    Ok(())
}

/// Opens a list of files in Neovim.
///
/// This function spawns a Neovim instance and opens all the specified files for editing.
/// If no files are provided, the function returns successfully without launching Neovim.
///
/// # Arguments
///
/// * `files` - A slice of paths to the files to open
///
/// # Returns
///
/// Returns `Ok(())` if Neovim was successfully launched and exited.
///
/// # Errors
///
/// Returns an `anyhow::Error` if:
/// - Neovim cannot be spawned
/// - The Neovim process fails to start
/// - The process cannot be waited on
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// 
/// async fn edit_files() -> anyhow::Result<()> {
///     let files = vec![
///         PathBuf::from("file1.txt"),
///         PathBuf::from("file2.txt")
///     ];
///     open_files_in_neovim(&files).await
/// }
/// ```
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
/// 
/// async fn custom_processor(path: &Path) -> anyhow::Result<()> {
///     println!("Processing: {}", path.display());
///     Ok(())
/// }
/// 
/// async fn process_my_file() -> anyhow::Result<()> {
///     process_file(
///         Path::new("example.txt"),
///         custom_processor
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
/// - The file cannot be read
/// - The file content cannot be processed
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    #[test]
    fn test_is_hidden() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test hidden file
        let hidden_path = temp_dir.path().join(".hidden_file");
        let entry = walkdir::DirEntry::from_path(&hidden_path).unwrap();
        assert!(is_hidden(&entry));

        // Test visible file
        let visible_path = temp_dir.path().join("visible_file");
        let entry = walkdir::DirEntry::from_path(&visible_path).unwrap();
        assert!(!is_hidden(&entry));

        // Test current directory
        let current_dir = walkdir::DirEntry::from_path(temp_dir.path().join(".")).unwrap();
        assert!(!is_hidden(&current_dir));

        // Test parent directory
        let parent_dir = walkdir::DirEntry::from_path(temp_dir.path().join("..")).unwrap();
        assert!(!is_hidden(&parent_dir));
    }

    #[test]
    fn test_is_target_dir() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test target directory
        let target_path = temp_dir.path().join("target");
        let entry = walkdir::DirEntry::from_path(&target_path).unwrap();
        assert!(is_target_dir(&entry));

        // Test non-target directory
        let non_target_path = temp_dir.path().join("src");
        let entry = walkdir::DirEntry::from_path(&non_target_path).unwrap();
        assert!(!is_target_dir(&entry));
    }

    #[test]
    fn test_is_git_dir() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test git directory
        let git_path = temp_dir.path().join(".git");
        let entry = walkdir::DirEntry::from_path(&git_path).unwrap();
        assert!(is_git_dir(&entry));

        // Test non-git directory
        let non_git_path = temp_dir.path().join("src");
        let entry = walkdir::DirEntry::from_path(&non_git_path).unwrap();
        assert!(!is_git_dir(&entry));
    }

    #[tokio::test]
    async fn test_walk_directory() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test files with different extensions
        File::create(temp_dir.path().join("test1.txt"))?;
        File::create(temp_dir.path().join("test2.txt"))?;
        File::create(temp_dir.path().join("test3.dat"))?;
        
        // Create a subdirectory with more files
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir)?;
        File::create(sub_dir.join("test4.txt"))?;

        let processed_files = Arc::new(Mutex::new(Vec::new()));
        let processed_files_clone = processed_files.clone();

        walk_directory(
            temp_dir.path(),
            "txt",
            move |path| {
                let files = processed_files_clone.clone();
                async move {
                    files.lock().await.push(path.to_path_buf());
                    Ok(())
                }
            },
        )
        .await?;

        let files = processed_files.lock().await;
        assert_eq!(files.len(), 3); // Should find 3 .txt files
        assert!(files.iter().all(|p| p.extension().unwrap() == "txt"));

        Ok(())
    }

    #[tokio::test]
    async fn test_walk_rust_files() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test Rust files
        File::create(temp_dir.path().join("main.rs"))?;
        File::create(temp_dir.path().join("lib.rs"))?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        // Create a subdirectory with more files
        let sub_dir = temp_dir.path().join("src");
        fs::create_dir(&sub_dir)?;
        File::create(sub_dir.join("mod.rs"))?;

        let processed_files = Arc::new(Mutex::new(Vec::new()));
        let processed_files_clone = processed_files.clone();

        walk_rust_files(temp_dir.path(), move |path| {
            let files = processed_files_clone.clone();
            async move {
                files.lock().await.push(path.to_path_buf());
                Ok(())
            }
        })
        .await?;

        let files = processed_files.lock().await;
        assert_eq!(files.len(), 3); // Should find 3 .rs files
        assert!(files.iter().all(|p| p.extension().unwrap() == "rs"));

        Ok(())
    }

    #[tokio::test]
    async fn test_read_lines() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        
        // Create test file with multiple lines
        fs::write(&test_file, "Line 1\nLine 2\nLine 3")?;

        let lines = read_lines(&test_file).await?;
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
        assert_eq!(lines[2], "Line 3");

        Ok(())
    }

    #[tokio::test]
    async fn test_read_file_content() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        
        let test_content = "Hello, World!";
        fs::write(&test_file, test_content)?;

        let content = read_file_content(&test_file).await?;
        assert_eq!(content, test_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_write_to_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        
        let test_content = "Hello, World!";
        write_to_file(&test_file, test_content).await?;

        let content = fs::read_to_string(&test_file)?;
        assert_eq!(content, test_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_files_with_extension() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test files
        File::create(temp_dir.path().join("test1.tmp"))?;
        File::create(temp_dir.path().join("test2.tmp"))?;
        File::create(temp_dir.path().join("test.txt"))?;
        
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir)?;
        File::create(sub_dir.join("test3.tmp"))?;

        delete_files_with_extension(temp_dir.path(), "tmp").await?;

        let remaining_files: Vec<_> = walkdir::WalkDir::new(temp_dir.path())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .collect();

        assert_eq!(remaining_files.len(), 1); // Only test.txt should remain
        assert_eq!(remaining_files[0].file_name(), "test.txt");

        Ok(())
    }

    #[tokio::test]
    async fn test_check_file_for_multiple_lines() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test files
        let single_line = temp_dir.path().join("single.txt");
        let multi_line = temp_dir.path().join("multi.txt");
        
        fs::write(&single_line, "Single line")?;
        fs::write(&multi_line, "Line 1\nLine 2\nLine 3")?;

        let multi_line_files = Arc::new(Mutex::new(Vec::new()));

        // Test single-line file
        check_file_for_multiple_lines(&single_line, multi_line_files.clone()).await?;
        assert_eq!(multi_line_files.lock().await.len(), 0);

        // Test multi-line file
        check_file_for_multiple_lines(&multi_line, multi_line_files.clone()).await?;
        assert_eq!(multi_line_files.lock().await.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_open_files_in_neovim() -> anyhow::Result<()> {
        let files = Vec::new();
        // Test empty file list (should return Ok without launching nvim)
        assert!(open_files_in_neovim(&files).await.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("test.txt");
        File::create(&test_file)?;

        let processed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let processed_clone = processed.clone();

        process_file(&test_file, move |_| {
            let flag = processed_clone.clone();
            async move {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        })
        .await?;

        assert!(processed.load(std::sync::atomic::Ordering::SeqCst));
        Ok(())
    }

    #[tokio::test]
    async fn test_process_rust_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Create test Rust files
        let with_warning = temp_dir.path().join("with_warning.rs");
        let without_warning = temp_dir.path().join("without_warning.rs");
        
        fs::write(&with_warning, "#![warn(clippy::all, clippy::pedantic)]\nfn main() {}")?;
        fs::write(&without_warning, "fn main() {}")?;

        let mut files_without_warning = Vec::new();

        process_rust_file(&with_warning, &mut files_without_warning).await?;
        assert_eq!(files_without_warning.len(), 0);

        process_rust_file(&without_warning, &mut files_without_warning).await?;
        assert_eq!(files_without_warning.len(), 1);
        assert_eq!(files_without_warning[0], without_warning);

        Ok(())
    }
}
