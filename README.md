# XIO - Extended I/O Operations Library

A utility library providing common functionality for file system operations and asynchronous file processing in Rust. Designed for efficient file traversal, content manipulation, and batch processing tasks.

## Features

- 🚀 Asynchronous file operations using Tokio
- 📁 Smart directory traversal with customizable filters
  - Skip hidden files and directories
  - Skip .git directories
  - Skip Rust target directories
- 🔍 Extension-based file filtering
- ⚡ Parallel file processing capabilities
- 🛡️ Robust error handling with anyhow
- 📝 Configurable logging levels

## Installation

```bash
cargo add xio
```

## Logging Configuration

XIO uses the `log` crate for logging. You can configure the logging level in two ways:

### Using Environment Variables

```bash
# Show all debug messages from xio
export RUST_LOG=xio=debug

# Show only info and above from xio
export RUST_LOG=xio=info

# Show only warnings and errors from xio
export RUST_LOG=xio=warn

# Show debug messages from xio and errors from other crates
export RUST_LOG=error,xio=debug
```

### Programmatically

```rust
use env_logger::{Builder, Env};

// Initialize with default info level
Builder::from_env(Env::default())
    .filter_module("xio", log::LevelFilter::Info)
    .init();

// Or use debug level for more verbose output
Builder::from_env(Env::default())
    .filter_module("xio", log::LevelFilter::Debug)
    .init();
```

## Core Function Reference

### Directory Walking and File Processing

#### `walk_directory`

Traverses a directory structure and processes files with a specified extension. The function uses asynchronous I/O with Tokio to efficiently walk through directory trees while applying smart filtering to skip hidden files, `.git` directories, and Rust `target` directories. Files are processed concurrently using Tokio tasks for maximum performance.

```rust
use std::path::Path;
use xio::{walk_directory, anyhow};
use log::info;

async fn process_txt_files() -> anyhow::Result<()> {
    walk_directory("./", "txt", |path| {
        let path = path.to_path_buf();
        async move {
            info!("Processing: {}", path.display());
            Ok(())
        }
    }).await
}
```

This function is ideal for batch processing of files across directory structures, providing smart filtering out-of-the-box. It allows for custom callback functions to handle each matching file, with integrated error handling and context propagation.

#### `walk_rust_files`

Specialized function for processing Rust source files throughout a codebase. This function automatically identifies `.rs` files while intelligently skipping irrelevant directories. It uses sequential processing to ensure order-dependent operations work correctly when analyzing Rust code.

```rust
use std::path::Path;
use std::io;
use xio::walk_rust_files;
use log::info;

async fn process_rust_files() -> io::Result<()> {
    walk_rust_files("./src", |path| {
        let path = path.to_path_buf();
        async move {
            info!("Found Rust file: {}", path.display());
            Ok(())
        }
    }).await
}
```

Perfect for code analysis tools, linters, and Rust codebase transformation utilities. This function streamlines the process of working with Rust source files across projects of any size.

### File Operations

#### `read_file_content`

Asynchronously reads an entire file into memory as a string. This function provides a simple, memory-efficient way to access file contents, with proper error handling built in.

```rust
use std::path::Path;
use std::io;
use xio::read_file_content;
use log::info;

async fn read_file() -> io::Result<()> {
    let content = read_file_content(Path::new("example.txt")).await?;
    info!("File content: {}", content);
    Ok(())
}
```

This function is built on Tokio's async I/O system, making it efficient for concurrent file access patterns. Best suited for small to medium files that fit easily in memory, it provides a clean and reliable way to access file content without blocking operations.

#### `read_lines`

Reads a file line by line and returns a vector containing each line as a string. This function trims whitespace from each line, providing clean, ready-to-use data. It's more memory-efficient than reading the entire file when you need to process lines individually.

```rust
use std::path::Path;
use std::io;
use xio::read_lines;
use log::info;

async fn read_file_lines() -> io::Result<()> {
    let lines = read_lines(Path::new("example.txt")).await?;
    for line in lines {
        info!("{}", line);
    }
    Ok(())
}
```

Perfect for processing configuration files, data files, logs, and any text format organized by lines. It handles UTF-8 encoding and automatically deals with different newline conventions (CR, LF, CRLF).

#### `write_to_file`

Asynchronously writes string content to a file. This function creates or overwrites the target file with the provided content, ensuring all data is properly written using async file operations.

```rust
use std::path::Path;
use std::io;
use xio::write_to_file;

async fn write_file() -> io::Result<()> {
    write_to_file(
        Path::new("output.txt"),
        "Hello, World!"
    ).await
}
```

This function automatically handles file creation, writing all content, and flushing the data to ensure it's properly saved. It's well-suited for writing configuration files, logs, and text outputs from your application.

### File System Utilities

#### `delete_files_with_extension`

Recursively finds and deletes all files with a specific extension in a directory tree. This function processes deletions concurrently using Tokio tasks, making it efficient even on large directory structures.

```rust
use std::path::Path;
use std::io;
use xio::delete_files_with_extension;

async fn cleanup_temp_files() -> io::Result<()> {
    delete_files_with_extension(Path::new("./"), "tmp").await
}
```

Ideal for cleanup operations, cache management, and removing temporary files. The function automatically logs both successful deletions and failures, providing visibility into the cleanup process. It's safe to use for targeted file type cleanup without affecting other files.

#### `check_file_for_multiple_lines`

Analyzes a file to determine if it contains multiple lines of text. If multiple lines are found, the file path is added to a thread-safe collection. This is useful for identifying files that meet specific structural criteria.

```rust
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use xio::{check_file_for_multiple_lines, anyhow};
use log::info;

async fn find_multi_line_files() -> anyhow::Result<()> {
    let files = Arc::new(Mutex::new(Vec::new()));
    check_file_for_multiple_lines(
        Path::new("example.txt"),
        files.clone()
    ).await?;
    let result = files.lock().await;
    info!("Found {} multi-line files", result.len());
    Ok(())
}
```

This function is particularly useful for validating file formats, identifying potential issues in data files, or filtering files for further processing based on their structure.

#### `open_files_in_neovim`

Opens multiple files in Neovim (or another specified editor) for interactive editing. This function launches the editor as a subprocess and waits for it to complete, making it useful for integrating with interactive workflows.

```rust
use std::path::PathBuf;
use xio::{open_files_in_neovim, anyhow};

async fn edit_files() -> anyhow::Result<()> {
    let files = vec![
        PathBuf::from("file1.txt"),
        PathBuf::from("file2.txt")
    ];
    open_files_in_neovim(&files, None).await
}
```

Perfect for CLI tools that need to offer interactive editing capabilities, this function integrates smoothly with terminal-based workflows. It can be customized to use different editors based on user preferences or environment variables.

#### `process_file`

Generic function that applies a custom processor function to a single file. This provides a flexible foundation for implementing file transformation operations with custom logic.

```rust
use std::path::Path;
use xio::{process_file, anyhow};

async fn process_my_file() -> anyhow::Result<()> {
    process_file(
        Path::new("example.txt"),
        |path| {
            let path = path.to_path_buf();
            async move {
                println!("Processing: {}", path.display());
                Ok(())
            }
        }
    ).await
}
```

This function serves as a building block for more complex file operations, allowing you to encapsulate custom processing logic while handling asynchronous file access patterns correctly.

#### `process_rust_file`

Analyzes a Rust source file to check for the presence of specific linter directives. It specifically checks for `#![warn(clippy::all, clippy::pedantic)]` warnings and collects files that don't include these directives.

```rust
use std::path::{Path, PathBuf};
use std::io;
use xio::process_rust_file;

async fn check_rust_files() -> io::Result<()> {
    let mut files = Vec::new();
    process_rust_file(
        Path::new("src/lib.rs"),
        &mut files
    ).await?;
    println!("Found {} files without warnings", files.len());
    Ok(())
}
```

This function is useful for ensuring coding standards across a Rust codebase, identifying files that may need linter configuration updates, and maintaining consistent code quality settings.

### File Extension Utilities

#### `has_extension`

Checks if a file has a specific extension in a case-sensitive manner. This function provides a reliable way to filter files based on their extension, properly handling path components and edge cases.

```rust
use std::path::Path;
use xio::fs::has_extension;

let path = Path::new("document.pdf");
assert!(has_extension(path, "pdf"));
assert!(!has_extension(Path::new("document"), "pdf"));
assert!(!has_extension(Path::new(".hidden"), "hidden")); // Hidden files return false
```

This utility function correctly handles special cases (no extension, hidden files) and provides a consistent interface for extension checking across your application.

#### `get_files_with_extension`

Returns an iterator over all files with a specific extension in a directory tree. This function walks the directory recursively, skipping hidden files, and returns paths to all matching files.

```rust
use std::path::Path;
use xio::fs::get_files_with_extension;
use log::info;

let path = Path::new("./documents");
for pdf_file in get_files_with_extension(path, "pdf") {
    info!("Found PDF: {}", pdf_file.display());
}
```

This function provides an efficient way to collect files of a specific type across a directory structure, without the complexity of manually implementing directory traversal logic. It's memory-efficient as it returns an iterator rather than collecting all paths.

#### `read_to_string`

Reads a file's contents into a String with comprehensive error handling. This synchronous function enhances the standard library's `read_to_string` with better error messages that include the file path in case of failure.

```rust
use std::path::Path;
use xio::fs::read_to_string;

match read_to_string(Path::new("config.toml")) {
    Ok(contents) => println!("File contents: {}", contents),
    Err(e) => eprintln!("Error reading file: {}", e),
}
```

This function provides clear, contextual error messages that include the file path, making it easier to diagnose issues. It's a drop-in replacement for the standard library function with improved error reporting.

### Path Filtering Functions

#### `is_hidden`

Determines if a directory entry represents a hidden file or directory. This function follows Unix conventions where files starting with a dot are considered hidden, with special handling for "." and ".." directory entries.

```rust
use walkdir::WalkDir;
use xio::is_hidden;

let entry = WalkDir::new(".").into_iter().next().unwrap().unwrap();
assert!(!is_hidden(&entry)); // "." is not considered hidden
```

This utility function is used internally by directory walking functions to implement smart filtering, but can also be used directly for custom directory traversal logic.

#### `is_target_dir` and `is_git_dir`

Helper functions to identify specific directory types that are typically excluded from file operations. These functions recognize Rust build output directories and Git repository metadata directories.

```rust
use walkdir::WalkDir;
use xio::{is_target_dir, is_git_dir};

let entry = WalkDir::new("src").into_iter().next().unwrap().unwrap();
assert!(!is_target_dir(&entry)); // "src" is not a target directory
assert!(!is_git_dir(&entry));    // "src" is not a git directory
```

These utility functions help implement smart exclusion policies when traversing directory structures, preventing processing of irrelevant technical directories.

## Directory Splitting Utilities

The `split` module provides advanced functionality for distributing files across multiple directories according to configurable patterns.

### `DirectorySplitter`

Distributes files across multiple output directories according to configured rules. This utility helps organize large collections of files into more manageable groups based on custom matching criteria.

```rust
use xio::split::{DirectorySplitter, SplitConfig, RegexFileMatcher};
use fancy_regex::Regex;
use std::path::Path;

async fn split_files() -> anyhow::Result<()> {
    // Create a matcher that processes all txt files
    let matcher = RegexFileMatcher {
        matcher_fn: Box::new(|path| {
            Ok(path.extension().map_or(false, |ext| ext == "txt"))
        }),
        regex_patterns: None,
    };
    
    // Configure how to split the directory
    let config = SplitConfig::new("./source", 5)
        .with_output_dir("./output")
        .with_naming("batch_{}", "_files");
    
    // Create and run the splitter
    let splitter = DirectorySplitter::new(config, matcher);
    let created_dirs = splitter.split().await?;
    
    // Later, clean up if needed
    splitter.cleanup(created_dirs).await?;
    
    Ok(())
}
```

This system provides a powerful way to distribute large sets of files across multiple directories for parallel processing, balancing storage, or organization purposes.

### `SplitConfig`

Configures the directory splitting operation with fine-grained control over:

- Source and output directory locations
- Number of output directories to create
- Naming patterns for output directories
- Rules for finding related files that should be kept together

### `FileMatcher` and `RegexFileMatcher`

Interface and implementation for determining which files to process during splitting operations. The `RegexFileMatcher` allows for powerful pattern matching using regular expressions to:

- Select which files to distribute across directories
- Find related files that should be kept together in the same target directory

These components combine to create a flexible system for distributing files in complex directory structures, particularly useful for data processing pipelines that need to partition large datasets.

## Examples

### Basic File Processing

```rust
use xio::{walk_directory, read_file_content, write_to_file};
use std::path::Path;

async fn process_all_json_files() -> anyhow::Result<()> {
    walk_directory("./data", "json", |path| {
        let path = path.to_path_buf();
        async move {
            // Read the JSON file
            let content = read_file_content(&path).await?;
            
            // Process the content (e.g., prettify JSON)
            let processed = content.replace(",", ", ");
            
            // Write back to the file
            write_to_file(&path, &processed).await?;
            
            Ok(())
        }
    }).await
}
```

### Recursive File Searching

```rust
use xio::fs::get_files_with_extension;
use std::path::Path;

fn find_all_images() {
    let root_dir = Path::new("./documents");
    
    // Find all PNG files
    let png_files: Vec<_> = get_files_with_extension(root_dir, "png").collect();
    println!("Found {} PNG files", png_files.len());
    
    // Find all JPG files
    let jpg_files: Vec<_> = get_files_with_extension(root_dir, "jpg").collect();
    println!("Found {} JPG files", jpg_files.len());
}
```

### File Cleanup Utilities

```rust
use xio::delete_files_with_extension;
use std::path::Path;

async fn cleanup_temporary_files() -> anyhow::Result<()> {
    // Delete all temporary files
    delete_files_with_extension(Path::new("./cache"), "tmp").await?;
    delete_files_with_extension(Path::new("./cache"), "temp").await?;
    
    println!("Cleanup complete!");
    Ok(())
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
