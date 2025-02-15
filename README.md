# Common Library

A utility library providing common functionality for file system operations and asynchronous file processing.

## Features

- Asynchronous file processing utilities
- Directory walking with extension filtering
- File system utilities for common operations
- Error handling with `anyhow`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xio = { path = "../xio" }
```

## Usage

### Processing Files

```rust
use xio::{process_file, Path};

async fn process_my_file(path: &Path) -> anyhow::Result<()> {
    let processor = |file_path: &Path| async move {
        // Process the file here
        Ok(())
    };
    
    process_file(path, processor).await
}
```

### Walking Directories

```rust
use xio::{walk_directory, Path};

async fn process_txt_files(dir: &Path) -> anyhow::Result<()> {
    let processor = |file_path: &Path| async move {
        // Process each .txt file here
        Ok(())
    };
    
    walk_directory(dir, "txt", processor).await
}
```

### File System Utilities

```rust
use xio::fs::{has_extension, get_files_with_extension, read_to_string};

// Check file extension
let is_txt = has_extension(Path::new("file.txt"), "txt");

// Get all files with specific extension
let txt_files = get_files_with_extension(Path::new("./"), "txt");

// Read file contents
let content = read_to_string(Path::new("file.txt"))?;
```

## Testing

Run the test suite:

```bash
cargo test
```

## License

This project is licensed under the MIT License.
