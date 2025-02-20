use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use walkdir::DirEntry;
use xio::{
    check_file_for_multiple_lines, delete_files_with_extension, is_git_dir, is_hidden,
    is_target_dir, open_files_in_neovim, process_file, process_rust_file, read_file_content,
    read_lines, walk_directory, walk_rust_files, write_to_file,
};

fn get_dir_entry(path: &Path) -> walkdir::DirEntry {
    walkdir::WalkDir::new(path)
        .into_iter()
        .next()
        .unwrap()
        .unwrap()
}

#[test]
fn test_is_hidden() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test hidden file
    let hidden_path = temp_dir.path().join(".hidden");
    std::fs::File::create(&hidden_path).unwrap();
    let entry = get_dir_entry(&hidden_path);
    assert!(is_hidden(&entry));

    // Test visible file
    let visible_path = temp_dir.path().join("visible");
    std::fs::File::create(&visible_path).unwrap();
    let entry = get_dir_entry(&visible_path);
    assert!(!is_hidden(&entry));

    // Test current directory
    let entry = get_dir_entry(Path::new("."));
    assert!(!is_hidden(&entry));
}

#[test]
fn test_is_target_dir() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test target directory
    let target_path = temp_dir.path().join("target");
    std::fs::create_dir(&target_path).unwrap();
    let entry = get_dir_entry(&target_path);
    assert!(is_target_dir(&entry));

    // Test non-target directory
    let non_target_path = temp_dir.path().join("src");
    std::fs::create_dir(&non_target_path).unwrap();
    let entry = get_dir_entry(&non_target_path);
    assert!(!is_target_dir(&entry));
}

#[test]
fn test_is_git_dir() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test git directory
    let git_path = temp_dir.path().join(".git");
    std::fs::create_dir(&git_path).unwrap();
    let entry = get_dir_entry(&git_path);
    assert!(is_git_dir(&entry));

    // Test non-git directory
    let non_git_path = temp_dir.path().join("src");
    std::fs::create_dir(&non_git_path).unwrap();
    let entry = get_dir_entry(&non_git_path);
    assert!(!is_git_dir(&entry));
}

#[tokio::test]
async fn test_walk_directory() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let processed_files = Arc::new(Mutex::new(Vec::new()));

    // Create test files
    std::fs::File::create(temp_dir.path().join("test1.txt"))?;
    std::fs::File::create(temp_dir.path().join("test2.txt"))?;
    std::fs::File::create(temp_dir.path().join(".hidden.txt"))?;

    let processed_files_clone = Arc::clone(&processed_files);
    walk_directory(temp_dir.path(), "txt", move |path: &Path| {
        let processed_files = Arc::clone(&processed_files_clone);
        let path_buf = path.to_path_buf();
        async move {
            let mut files = processed_files.lock().await;
            files.push(path_buf);
            Ok(())
        }
    })
    .await?;

    let processed = processed_files.lock().await;
    assert_eq!(processed.len(), 2); // Should not include hidden file
    assert!(processed.iter().all(|p| p.extension().unwrap() == "txt"));

    Ok(())
}

#[tokio::test]
async fn test_walk_rust_files() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let processed_files = Arc::new(Mutex::new(Vec::new()));

    // Create test files
    std::fs::File::create(temp_dir.path().join("test1.rs"))?;
    std::fs::File::create(temp_dir.path().join("test2.rs"))?;
    std::fs::File::create(temp_dir.path().join("test.txt"))?;

    let processed_files_clone = Arc::clone(&processed_files);
    walk_rust_files(temp_dir.path(), move |path: &Path| {
        let processed_files = Arc::clone(&processed_files_clone);
        let path_buf = path.to_path_buf();
        async move {
            let mut files = processed_files.lock().await;
            files.push(path_buf);
            Ok(())
        }
    })
    .await?;

    let processed = processed_files.lock().await;
    assert_eq!(processed.len(), 2);
    assert!(processed.iter().all(|p| p.extension().unwrap() == "rs"));

    Ok(())
}

#[tokio::test]
async fn test_read_lines() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    
    std::fs::write(&file_path, "Line 1\nLine 2\nLine 3")?;
    
    let lines = read_lines(&file_path).await?;
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "Line 1");
    assert_eq!(lines[1], "Line 2");
    assert_eq!(lines[2], "Line 3");
    
    Ok(())
}

#[tokio::test]
async fn test_read_file_content() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    
    let content = "Test content\nwith multiple lines";
    std::fs::write(&file_path, content)?;
    
    let read_content = read_file_content(&file_path).await?;
    assert_eq!(read_content, content);
    
    Ok(())
}

#[tokio::test]
async fn test_write_to_file() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    
    let content = "Test content";
    write_to_file(&file_path, content).await?;
    
    let read_content = std::fs::read_to_string(&file_path)?;
    assert_eq!(read_content, content);
    
    Ok(())
}

#[tokio::test]
async fn test_delete_files_with_extension() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create test files
    std::fs::File::create(temp_dir.path().join("test1.txt"))?;
    std::fs::File::create(temp_dir.path().join("test2.txt"))?;
    std::fs::File::create(temp_dir.path().join("test.rs"))?;
    
    delete_files_with_extension(temp_dir.path(), "txt").await?;
    
    let entries: Vec<_> = std::fs::read_dir(temp_dir.path())?
        .filter_map(Result::ok)
        .collect();
    
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].path().extension().unwrap().to_string_lossy(),
        "rs"
    );
    
    Ok(())
}

#[tokio::test]
async fn test_check_file_for_multiple_lines() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let multi_line_files = Arc::new(Mutex::new(Vec::new()));
    
    // Create test files
    let single_line = temp_dir.path().join("single.txt");
    std::fs::write(&single_line, "Single line")?;
    
    let multi_line = temp_dir.path().join("multi.txt");
    std::fs::write(&multi_line, "Line 1\nLine 2")?;
    
    check_file_for_multiple_lines(&single_line, Arc::clone(&multi_line_files)).await?;
    check_file_for_multiple_lines(&multi_line, Arc::clone(&multi_line_files)).await?;
    
    let files = multi_line_files.lock().await;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0], multi_line);
    
    Ok(())
}

#[tokio::test]
async fn test_open_files_in_neovim() -> anyhow::Result<()> {
    let files = vec![PathBuf::from("test1.txt"), PathBuf::from("test2.txt")];
    open_files_in_neovim(&files).await?;
    Ok(())
}

#[tokio::test]
async fn test_process_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Test content")?;
    
    let processed = Arc::new(Mutex::new(false));
    let processed_clone = Arc::clone(&processed);
    
    process_file(&file_path, move |_| {
        let processed = Arc::clone(&processed_clone);
        async move {
            let mut p = processed.lock().await;
            *p = true;
            Ok(())
        }
    })
    .await?;
    
    assert!(*processed.lock().await);
    Ok(())
}

#[tokio::test]
async fn test_process_rust_file() -> std::io::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.rs");
    std::fs::write(&file_path, "#![warn(clippy::all)]\nfn main() {}")?;
    
    let mut files_without_warning = Vec::new();
    process_rust_file(&file_path, &mut files_without_warning).await?;
    
    assert_eq!(files_without_warning.len(), 0);
    Ok(())
} 