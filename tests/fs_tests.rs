use std::fs::{self, File};
use std::path::Path;
use tempfile::TempDir;
use xio::fs::{has_extension, get_files_with_extension, read_to_string};

#[test]
fn test_has_extension() {
    assert!(has_extension(Path::new("test.txt"), "txt"));
    assert!(!has_extension(Path::new("test.dat"), "txt"));
    assert!(!has_extension(Path::new("test"), "txt"));
    assert!(!has_extension(Path::new(".txt"), "txt")); // Hidden file
    assert!(has_extension(Path::new("path/to/test.txt"), "txt"));
    
    // Additional test cases
    assert!(!has_extension(Path::new(""), "txt")); // Empty path
    assert!(!has_extension(Path::new("test."), "txt")); // Empty extension
    assert!(!has_extension(Path::new("."), "txt")); // Just dot
    assert!(!has_extension(Path::new("test.txt.bak"), "txt")); // Multiple extensions
}

#[test]
fn test_get_files_with_extension() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;

    // Create test files
    File::create(temp_dir.path().join("test1.txt"))?;
    File::create(temp_dir.path().join("test2.txt"))?;
    File::create(temp_dir.path().join("test3.dat"))?;
    File::create(temp_dir.path().join(".hidden.txt"))?; // Hidden file

    // Create subdirectory with more files
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test4.txt"))?;
    File::create(sub_dir.join("test5.dat"))?;

    // Test .txt files
    let files: Vec<_> = get_files_with_extension(temp_dir.path(), "txt").collect();
    assert_eq!(files.len(), 3); // Should not include hidden file
    assert!(files.iter().all(|path| path.extension().unwrap() == "txt"));

    // Test .dat files
    let dat_files: Vec<_> = get_files_with_extension(temp_dir.path(), "dat").collect();
    assert_eq!(dat_files.len(), 2);
    assert!(dat_files.iter().all(|path| path.extension().unwrap() == "dat"));

    // Test non-existent extension
    let no_files: Vec<_> = get_files_with_extension(temp_dir.path(), "xyz").collect();
    assert!(no_files.is_empty());

    // Test empty extension
    let empty_ext: Vec<_> = get_files_with_extension(temp_dir.path(), "").collect();
    assert!(empty_ext.is_empty());

    Ok(())
}

#[test]
fn test_read_to_string() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");

    // Test successful read
    let test_content = "Hello, World!";
    fs::write(&file_path, test_content)?;
    let content = read_to_string(&file_path)?;
    assert_eq!(content, test_content);

    // Test non-existent file
    let non_existent = temp_dir.path().join("nonexistent.txt");
    assert!(read_to_string(&non_existent).is_err());

    // Test empty file
    let empty_file = temp_dir.path().join("empty.txt");
    File::create(&empty_file)?;
    let content = read_to_string(&empty_file)?;
    assert!(content.is_empty());

    // Test file with multiple lines
    let multi_line = temp_dir.path().join("multi.txt");
    fs::write(&multi_line, "Line 1\nLine 2\nLine 3")?;
    let content = read_to_string(&multi_line)?;
    assert_eq!(content.lines().count(), 3);

    // Test directory
    let dir_path = temp_dir.path().join("dir");
    fs::create_dir(&dir_path)?;
    assert!(read_to_string(&dir_path).is_err());

    Ok(())
} 