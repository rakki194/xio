# xio

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

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xio = "0.1.5"
```

## Core Functions

### Directory Walking and File Processing

#### `walk_directory`

Asynchronously walks through a directory and processes files with a specific extension.

```rust
use std::path::Path;
use xio::{walk_directory, anyhow};

async fn process_txt_files() -> anyhow::Result<()> {
    walk_directory("./", "txt", |path| {
        let path = path.to_path_buf();
        async move {
            println!("Processing: {}", path.display());
            Ok(())
        }
    }).await
}
```

#### `walk_rust_files`

Specifically designed for processing Rust source files, automatically skipping irrelevant directories.

```rust
use std::path::Path;
use std::io;
use xio::walk_rust_files;

async fn process_rust_files() -> io::Result<()> {
    walk_rust_files("./src", |path| {
        let path = path.to_path_buf();
        async move {
            println!("Found Rust file: {}", path.display());
            Ok(())
        }
    }).await
}
```

### File Operations

#### `read_file_content`

Asynchronously reads the entire content of a file as a string.

```rust
use std::path::Path;
use std::io;
use xio::read_file_content;

async fn read_file() -> io::Result<()> {
    let content = read_file_content(Path::new("example.txt")).await?;
    println!("File content: {}", content);
    Ok(())
}
```

#### `read_lines`

Asynchronously reads a file line by line, returning a vector of strings.

```rust
use std::path::Path;
use std::io;
use xio::read_lines;

async fn read_file_lines() -> io::Result<()> {
    let lines = read_lines(Path::new("example.txt")).await?;
    for line in lines {
        println!("{}", line);
    }
    Ok(())
}
```

#### `write_to_file`

Asynchronously writes content to a file.

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

### File System Utilities

#### `delete_files_with_extension`

Recursively deletes all files with a specific extension in a directory.

```rust
use std::path::Path;
use std::io;
use xio::delete_files_with_extension;

async fn cleanup_temp_files() -> io::Result<()> {
    delete_files_with_extension(Path::new("./"), "tmp").await
}
```

#### `check_file_for_multiple_lines`

Checks if a file contains multiple lines and adds it to a tracked list.

```rust
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use xio::{check_file_for_multiple_lines, anyhow};

async fn find_multi_line_files() -> anyhow::Result<()> {
    let files = Arc::new(Mutex::new(Vec::new()));
    check_file_for_multiple_lines(
        Path::new("example.txt"),
        files.clone()
    ).await?;
    let result = files.lock().await;
    println!("Found {} multi-line files", result.len());
    Ok(())
}
```

### File Extension Utilities

#### `has_extension`

Checks if a file has a specific extension.

```rust
use std::path::Path;
use xio::fs::has_extension;

let path = Path::new("document.pdf");
assert!(has_extension(path, "pdf"));
assert!(!has_extension(Path::new("document"), "pdf"));
assert!(!has_extension(Path::new(".hidden"), "hidden")); // Hidden files return false
```

#### `get_files_with_extension`

Returns an iterator over all files with a specific extension in a directory.

```rust
use std::path::Path;
use xio::fs::get_files_with_extension;

let path = Path::new("./documents");
for pdf_file in get_files_with_extension(path, "pdf") {
    println!("Found PDF: {}", pdf_file.display());
}
```

### Path Filtering Functions

#### `is_hidden`

Determines if a directory entry is hidden (starts with a dot).

```rust
use walkdir::WalkDir;
use xio::is_hidden;

let entry = WalkDir::new(".").into_iter().next().unwrap().unwrap();
assert!(!is_hidden(&entry)); // "." is not considered hidden
```

#### `is_target_dir` and `is_git_dir`

Helper functions to identify Rust target directories and Git repositories.

```rust
use walkdir::WalkDir;
use xio::{is_target_dir, is_git_dir};

let entry = WalkDir::new("src").into_iter().next().unwrap().unwrap();
assert!(!is_target_dir(&entry)); // "src" is not a target directory
assert!(!is_git_dir(&entry)); // "src" is not a git directory
```

### Process Files

#### `process_file`

Generic file processor that takes any async function for custom file processing.

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

#### `process_rust_file`

Process a Rust file and check for pedantic warnings.

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

## Error Handling

The library uses `anyhow` for rich error handling and `io::Result` for basic operations. All functions return appropriate Result types with detailed error context.

## License

This project is licensed under the MIT License.
