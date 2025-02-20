#![warn(clippy::all, clippy::pedantic)]

use std::path::Path;

/// Check if a file has a specific extension
#[must_use]
pub fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension().is_some_and(|ext| ext == extension)
}

/// Get all files in a directory with a specific extension
pub fn get_files_with_extension<'a>(
    dir: &'a Path,
    extension: &'a str,
) -> impl Iterator<Item = std::path::PathBuf> + 'a {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(move |e| has_extension(e.path(), extension))
        .map(|e| e.path().to_path_buf())
}

/// Read a file to string with proper error handling
///
/// # Errors
/// Returns an error if:
/// - The file cannot be read
/// - The file contains invalid UTF-8
pub fn read_to_string(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_has_extension() {
        assert!(has_extension(Path::new("test.txt"), "txt"));
        assert!(!has_extension(Path::new("test.dat"), "txt"));
        assert!(!has_extension(Path::new("test"), "txt"));
        assert!(!has_extension(Path::new(".txt"), "txt")); // Hidden file
        assert!(has_extension(Path::new("path/to/test.txt"), "txt"));
    }

    #[test]
    fn test_get_files_with_extension() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;

        // Create test files
        File::create(temp_dir.path().join("test1.txt"))?;
        File::create(temp_dir.path().join("test2.txt"))?;
        File::create(temp_dir.path().join("test3.dat"))?;

        // Create subdirectory with more files
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir)?;
        File::create(sub_dir.join("test4.txt"))?;

        let files: Vec<_> = get_files_with_extension(temp_dir.path(), "txt").collect();

        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|path| path.extension().unwrap() == "txt"));

        let dat_files: Vec<_> = get_files_with_extension(temp_dir.path(), "dat").collect();
        assert_eq!(dat_files.len(), 1);

        Ok(())
    }

    #[test]
    fn test_read_to_string() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");

        // Test successful read
        fs::write(&file_path, "Hello, World!")?;
        let content = read_to_string(&file_path)?;
        assert_eq!(content, "Hello, World!");

        // Test non-existent file
        let result = read_to_string(Path::new("nonexistent.txt"));
        assert!(result.is_err());

        Ok(())
    }
}
