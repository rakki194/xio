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
xio = "0.1.4"
```

## Core Functions

### Directory Walking and File Processing

#### `walk_directory`

Asynchronously walks through a directory and processes files with a specific extension.

```rust
use xio::{walk_directory, Path};
use anyhow::Result;

async fn process_json_files(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "json",
        |path| async move {
            println!("Processing: {}", path.display());
            Ok(())
        }
    ).await
}
```

#### `walk_rust_files`

Specifically designed for processing Rust source files, automatically skipping irrelevant directories.

```rust
use xio::{walk_rust_files, Path};
use anyhow::Result;

async fn analyze_rust_code(dir: &str) -> Result<()> {
    walk_rust_files(dir, |path| async move {
        let content = xio::read_file_content(path).await?;
        println!("Analyzing Rust file: {}, size: {} bytes", 
                path.display(), content.len());
        Ok(())
    }).await
}
```

### File Operations

#### `read_file_content`

Asynchronously reads the entire content of a file as a string.

```rust
use xio::{read_file_content, Path};
use anyhow::Result;

async fn read_and_process(path: &str) -> Result<()> {
    let content = read_file_content(Path::new(path)).await?;
    println!("File content length: {}", content.len());
    Ok(())
}
```

#### `read_lines`

Asynchronously reads a file line by line, returning a vector of strings.

```rust
use xio::{read_lines, Path};
use anyhow::Result;

async fn process_lines(path: &str) -> Result<()> {
    let lines = read_lines(Path::new(path)).await?;
    for (i, line) in lines.iter().enumerate() {
        println!("Line {}: {}", i + 1, line);
    }
    Ok(())
}
```

#### `write_to_file`

Asynchronously writes content to a file.

```rust
use xio::{write_to_file, Path};
use anyhow::Result;

async fn save_content(path: &str, content: &str) -> Result<()> {
    write_to_file(Path::new(path), content).await?;
    println!("Content saved successfully!");
    Ok(())
}
```

### File System Utilities

#### `delete_files_with_extension`

Recursively deletes all files with a specific extension in a directory.

```rust
use xio::{delete_files_with_extension, Path};
use anyhow::Result;

async fn cleanup_temp_files(dir: &str) -> Result<()> {
    delete_files_with_extension(Path::new(dir), "tmp").await?;
    println!("Temporary files cleaned up!");
    Ok(())
}
```

#### `check_file_for_multiple_lines`

Checks if a file contains multiple lines and adds it to a tracked list.

```rust
use xio::{check_file_for_multiple_lines, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

async fn find_multiline_files(path: &str) -> Result<()> {
    let multi_line_files = Arc::new(Mutex::new(Vec::new()));
    check_file_for_multiple_lines(
        Path::new(path),
        Arc::clone(&multi_line_files)
    ).await?;
    
    let files = multi_line_files.lock().await;
    println!("Found {} files with multiple lines", files.len());
    Ok(())
}
```

### File Extension Utilities

#### `has_extension`

Checks if a file has a specific extension.

```rust
use xio::fs::has_extension;
use std::path::Path;

fn check_file_type(path: &str) {
    let path = Path::new(path);
    if has_extension(path, "rs") {
        println!("This is a Rust source file!");
    }
}
```

#### `get_files_with_extension`

Returns an iterator over all files with a specific extension in a directory.

```rust
use xio::fs::get_files_with_extension;
use std::path::Path;

fn list_markdown_files(dir: &str) {
    let files = get_files_with_extension(Path::new(dir), "md");
    for file in files {
        println!("Found markdown file: {}", file.display());
    }
}
```

### Path Filtering Functions

#### `is_hidden`

Determines if a directory entry is hidden (starts with a dot).

```rust
use xio::is_hidden;
use walkdir::WalkDir;

fn list_visible_files(dir: &str) {
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !is_hidden(&entry) {
            println!("Visible entry: {}", entry.path().display());
        }
    }
}
```

#### `is_target_dir` and `is_git_dir`

Helper functions to identify Rust target directories and Git repositories.

```rust
use xio::{is_target_dir, is_git_dir};
use walkdir::WalkDir;

fn list_project_files(dir: &str) {
    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        if !is_target_dir(&entry) && !is_git_dir(&entry) {
            println!("Project file: {}", entry.path().display());
        }
    }
}
```

## Error Handling

The library uses `anyhow` for rich error handling and `io::Result` for basic operations. All functions return appropriate Result types with detailed error context.

```rust
use xio::{walk_directory, Path};
use anyhow::{Context, Result};

async fn process_with_context(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "log",
        |path| async move {
            let content = xio::read_file_content(path)
                .await
                .with_context(|| format!("Failed to read log file: {}", path.display()))?;
            
            // Process content...
            Ok(())
        }
    )
    .await
    .with_context(|| format!("Failed to process log files in directory: {}", dir))
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
