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
xio = "0.1.2"
```

## Usage Examples

### Walking Directories and Processing Files

Process all files with a specific extension in a directory:

```rust
use xio::{walk_directory, Path};
use anyhow::Result;

async fn process_json_files(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "json",
        |path| async move {
            // Process each JSON file
            println!("Processing: {}", path.display());
            Ok(())
        }
    ).await
}
```

### Multiple Extensions

Process files with multiple extensions:

```rust
use xio::{walk_directory, Path};
use anyhow::Result;

async fn process_image_files(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "jpg;jpeg;png;webp",  // Semicolon-separated list of extensions
        |path| async move {
            println!("Processing image: {}", path.display());
            Ok(())
        }
    ).await
}
```

### Reading and Writing Files

```rust
use xio::{read_file_content, write_to_file, Path};
use anyhow::Result;

async fn modify_file_content(path: &str) -> Result<()> {
    // Read file content
    let content = read_file_content(Path::new(path)).await?;
    
    // Modify content
    let modified = content.to_uppercase();
    
    // Write back to file
    write_to_file(Path::new(path), &modified).await?;
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

### Smart Path Filtering

The library automatically skips:

- Hidden files and directories (except "." and "..")
- Git directories (.git)
- Rust target directories (target)

You can see this in action when using `walk_directory`:

```rust
use xio::{walk_directory, Path};
use anyhow::Result;

async fn process_visible_files(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "txt",
        |path| async move {
            // This will only process visible .txt files
            // Skips .git/, target/, and hidden files
            println!("Processing: {}", path.display());
            Ok(())
        }
    ).await
}
```

## Error Handling

All operations return `Result` types with detailed error information:

- `io::Result` for basic file operations
- `anyhow::Result` for more complex operations with rich error context

Example with error handling:

```rust
use xio::{walk_directory, Path};
use anyhow::{Context, Result};

async fn process_files(dir: &str) -> Result<()> {
    walk_directory(
        dir,
        "dat",
        |path| async move {
            let content = xio::read_file_content(path)
                .await
                .with_context(|| format!("Failed to read {}", path.display()))?;
            
            // Process content...
            Ok(())
        }
    )
    .await
    .with_context(|| format!("Failed to process directory: {}", dir))
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
