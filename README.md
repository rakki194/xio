# xio

A utility library providing common functionality for file system operations and asynchronous file processing in Rust. Designed for efficient file traversal, content manipulation, and batch processing tasks.

## Features

- 🚀 Asynchronous file operations using Tokio
- 📁 Smart directory traversal with customizable filters
- 🔍 Extension-based file filtering
- ⚡ Parallel file processing capabilities
- 🛡️ Robust error handling with anyhow
- 🎯 Skip common unwanted paths (.git, target, hidden files)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xio = "0.1.1"
```

## Usage Examples

### Walking Directories and Processing Files

Process all files with a specific extension in a directory:

```rust
use xio::{walk_directory, Path};
use anyhow::Result;

async fn process_json_files(dir: &str) -> Result<()> {
    walk_directory(dir, "json", |path| async move {
        // Read the file content
        let content = xio::read_file_content(path).await?;
        println!("Processing {}: {} bytes", path.display(), content.len());
        Ok(())
    }).await
}
```

### Reading and Writing Files

```rust
use xio::{Path, read_file_content, write_to_file};
use anyhow::Result;

async fn copy_and_modify_file(src: &str, dest: &str) -> Result<()> {
    // Read source file
    let content = read_file_content(Path::new(src)).await?;
    
    // Modify content
    let modified = content.to_uppercase();
    
    // Write to destination
    write_to_file(Path::new(dest), &modified).await?;
    Ok(())
}
```

### Processing Rust Files

Special utilities for working with Rust source files:

```rust
use xio::{walk_rust_files, Path};
use std::io;

async fn find_long_rust_files(dir: &str) -> io::Result<()> {
    walk_rust_files(dir, |path| async move {
        let content = xio::read_file_content(path).await?;
        if content.lines().count() > 100 {
            println!("Long file found: {}", path.display());
        }
        Ok(())
    }).await
}
```

### Batch File Operations

Delete all files with a specific extension:

```rust
use xio::{delete_files_with_extension, Path};

async fn cleanup_temp_files(dir: &str) -> io::Result<()> {
    delete_files_with_extension(Path::new(dir), "tmp").await
}
```

### Reading File Lines

```rust
use xio::{read_lines, Path};

async fn count_non_empty_lines(path: &str) -> io::Result<usize> {
    let lines = read_lines(Path::new(path)).await?;
    Ok(lines.iter().filter(|line| !line.is_empty()).count())
}
```

## Advanced Features

### Smart Path Filtering

The library automatically skips:
- Hidden files and directories (except "." and "..")
- Git directories (.git)
- Rust target directories (target)

### Error Handling

All operations return `Result` types with detailed error information:
- `io::Result` for basic file operations
- `anyhow::Result` for more complex operations with rich error context

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
