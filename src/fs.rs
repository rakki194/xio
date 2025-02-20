#![warn(clippy::all, clippy::pedantic)]

//! File system utility functions for working with files and directories.
//! 
//! This module provides a collection of utilities for common file system operations,
//! particularly focused on file extension handling and safe file reading operations.
//! All functions are designed with proper error handling and type safety in mind.
//!
//! # Examples
//!
//! ```
//! use std::path::Path;
//! use xio::fs;
//!
//! let path = Path::new("example.txt");
//! if fs::has_extension(path, "txt") {
//!     println!("Found a text file!");
//! }
//! ```

use std::path::Path;

/// Checks if a file has a specific extension.
///
/// This function compares the file extension case-sensitively with the provided extension.
/// The extension comparison is done without the leading dot.
///
/// # Arguments
///
/// * `path` - The path to check
/// * `extension` - The extension to check for, without the leading dot (e.g., "txt" not ".txt")
///
/// # Returns
///
/// Returns `true` if the file has the specified extension, `false` otherwise.
/// Also returns `false` if the path has no extension or if the path points to a hidden file
/// that starts with a dot.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use xio::fs::has_extension;
///
/// assert!(has_extension(Path::new("document.pdf"), "pdf"));
/// assert!(!has_extension(Path::new("document"), "pdf"));
/// assert!(!has_extension(Path::new(".hidden"), "hidden")); // Hidden files return false
/// ```
#[must_use]
pub fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|ext| ext == extension)
}

/// Recursively finds all files with a specific extension in a directory and its subdirectories.
///
/// This function walks through the directory tree and returns an iterator of paths to files
/// that match the specified extension. The search is case-sensitive.
///
/// # Arguments
///
/// * `dir` - The root directory to start the search from
/// * `extension` - The extension to filter files by, without the leading dot (e.g., "txt" not ".txt")
///
/// # Returns
///
/// Returns an iterator that yields `PathBuf` instances for each matching file found.
/// The iterator automatically handles any permissions errors or inaccessible directories
/// by silently skipping them.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use xio::fs::get_files_with_extension;
///
/// let path = Path::new("./documents");
/// for pdf_file in get_files_with_extension(path, "pdf") {
///     println!("Found PDF: {}", pdf_file.display());
/// }
/// ```
pub fn get_files_with_extension<'a>(
    dir: &'a Path,
    extension: &'a str,
) -> impl Iterator<Item = std::path::PathBuf> + 'a {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(move |e| {
            let file_name = e.file_name().to_str();
            file_name.is_some_and(|s| !s.starts_with('.')) && has_extension(e.path(), extension)
        })
        .map(|e| e.path().to_path_buf())
}

/// Reads a file's contents into a String with comprehensive error handling.
///
/// This function provides a convenient wrapper around `std::fs::read_to_string`
/// with improved error messages that include the file path in case of failure.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// Returns a `Result` containing either the file contents as a `String` or
/// an error with a detailed message including the file path.
///
/// # Errors
///
/// This function will return an error in the following situations:
/// * The file does not exist
/// * The process lacks permissions to read the file
/// * The file contains invalid UTF-8 data
/// * Any other I/O error occurs during reading
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use xio::fs::read_to_string;
///
/// match read_to_string(Path::new("config.toml")) {
///     Ok(contents) => println!("File contents: {}", contents),
///     Err(e) => eprintln!("Error reading file: {}", e),
/// }
/// ```
pub fn read_to_string(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
}
