use crate::{walk_directory, Path, PathBuf};
use anyhow::{Context, Result};
use fancy_regex::Regex;
use futures::future::try_join_all;
use log::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

/// Type alias for a matcher function that determines if a file should be processed
pub type MatcherFn = Box<dyn Fn(&Path) -> Result<bool> + Send + Sync>;

/// Configuration for directory splitting operations
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// Source directory to split
    pub source_dir: PathBuf,
    /// Output directory (if different from source)
    pub output_dir: Option<PathBuf>,
    /// Number of subdirectories to create
    pub num_dirs: usize,
    /// Format string for directory prefix (e.g., "part_{}")
    pub prefix_format: String,
    /// Format string for directory suffix (e.g., "_batch")
    pub suffix_format: String,
    /// Optional regex patterns for finding accompanying files
    pub regex_patterns: Option<Vec<Regex>>,
}

impl SplitConfig {
    /// Creates a new `SplitConfig` with minimum required parameters
    pub fn new(source_dir: impl Into<PathBuf>, num_dirs: usize) -> Self {
        Self {
            source_dir: source_dir.into(),
            output_dir: None,
            num_dirs,
            prefix_format: "part_{}".to_string(),
            suffix_format: String::new(),
            regex_patterns: None,
        }
    }

    /// Sets the output directory
    #[must_use]
    pub fn with_output_dir(mut self, output_dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(output_dir.into());
        self
    }

    /// Sets the directory naming format
    #[must_use]
    pub fn with_naming(mut self, prefix_format: impl Into<String>, suffix_format: impl Into<String>) -> Self {
        self.prefix_format = prefix_format.into();
        self.suffix_format = suffix_format.into();
        self
    }

    /// Sets regex patterns for finding accompanying files
    #[must_use]
    pub fn with_regex_patterns(mut self, patterns: Vec<Regex>) -> Self {
        self.regex_patterns = Some(patterns);
        self
    }
}

/// Represents a file matcher that determines which files to process
#[async_trait::async_trait]
pub trait FileMatcher: Send + Sync {
    /// Returns true if the file should be processed
    async fn is_match(&self, path: &Path) -> Result<bool>;
    /// Finds accompanying files for a matched file
    async fn find_accompanying_files(&self, path: &Path) -> Result<Vec<PathBuf>>;
}

/// A directory splitter that distributes files across multiple directories
pub struct DirectorySplitter<M: FileMatcher> {
    config: SplitConfig,
    matcher: M,
}

impl<M: FileMatcher + Clone + 'static> DirectorySplitter<M> {
    /// Creates a new `DirectorySplitter` with the given configuration and matcher
    pub fn new(config: SplitConfig, matcher: M) -> Self {
        Self { config, matcher }
    }

    /// Splits the directory according to the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Creating directories fails
    /// - Reading from source directory fails
    /// - Copying files fails
    ///
    /// # Panics
    ///
    /// Panics if a file name cannot be extracted from a path,
    /// which should not happen for valid file paths.
    pub async fn split(&self) -> Result<Vec<PathBuf>> {
        let mut created_dirs = Vec::new();
        debug!("Grouping files from source directory");
        let file_groups = Arc::new(Mutex::new(HashMap::new()));
        
        // First, find all matching files and create groups
        info!("Scanning for files...");
        self.find_files(file_groups.clone()).await?;
        
        // Create output directories
        let output_dir = self.config.output_dir.as_ref()
            .unwrap_or(&self.config.source_dir);
            
        for i in 0..self.config.num_dirs {
            let dir_name = format!(
                "{}{}",
                self.config.prefix_format.replace("{}", &i.to_string()),
                self.config.suffix_format
            );
            let dir_path = output_dir.join(&dir_name);
            debug!("Creating directory: {}", dir_path.display());
            fs::create_dir_all(&dir_path).await?;
            created_dirs.push(dir_path);
        }

        // Distribute files across directories
        let mut current_dir = 0;
        let groups = file_groups.lock().await;
        info!("Distributing {} file groups across directories", groups.len());
        
        for files in groups.values() {
            let target_dir = &created_dirs[current_dir];
            debug!("Processing {} files into directory: {}", files.len(), target_dir.display());
            
            for file in files {
                let file_name = file.file_name().unwrap();
                let target_path = target_dir.join(file_name);
                debug!("Copying {} to {}", file.display(), target_path.display());
                fs::copy(file, &target_path).await?;
            }
            current_dir = (current_dir + 1) % self.config.num_dirs;
        }

        Ok(created_dirs)
    }

    /// Cleans up the created directories
    ///
    /// # Errors
    ///
    /// Returns an error if removing any of the directories fails.
    pub async fn cleanup(&self, dirs: Vec<PathBuf>) -> Result<()> {
        info!("Starting cleanup of {} directories", dirs.len());
        try_join_all(dirs.into_iter().map(|dir| async move {
            debug!("Removing directory: {}", dir.display());
            fs::remove_dir_all(&dir)
                .await
                .context(format!("Failed to remove directory: {}", dir.display()))
        }))
        .await?;
        Ok(())
    }

    async fn find_files(&self, file_groups: Arc<Mutex<HashMap<PathBuf, Vec<PathBuf>>>>) -> Result<()> {
        let config = self.config.clone();
        let matcher = self.matcher.clone();
        
        walk_directory(&config.source_dir, "*", move |path| {
            let path = path.to_path_buf();
            let file_groups = file_groups.clone();
            let matcher = matcher.clone();
            
            async move {
                if matcher.is_match(&path).await? {
                    debug!("Found matching file: {}", path.display());
                    let mut groups = file_groups.lock().await;
                    let group = groups.entry(path.clone()).or_default();
                    group.push(path.clone());

                    // Find accompanying files
                    let accompanying = matcher.find_accompanying_files(&path).await?;
                    for accompanying_path in accompanying {
                        debug!("Found accompanying file: {}", accompanying_path.display());
                        group.push(accompanying_path);
                    }
                }
                Ok(())
            }
        })
        .await?;

        Ok(())
    }
}

/// A regex-based file matcher that can find accompanying files using patterns
pub struct RegexFileMatcher {
    /// Function to determine if a file should be processed
    pub matcher_fn: MatcherFn,
    /// Optional regex patterns for finding accompanying files
    pub regex_patterns: Option<Vec<Regex>>,
}

#[async_trait::async_trait]
impl FileMatcher for RegexFileMatcher {
    async fn is_match(&self, path: &Path) -> Result<bool> { 
        (self.matcher_fn)(path) 
    }

    async fn find_accompanying_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut accompanying = Vec::new();
        
        if let Some(patterns) = &self.regex_patterns {
            let dir = path.parent().unwrap();
            let mut dir_entries = fs::read_dir(dir).await?;
            
            while let Some(entry) = dir_entries.next_entry().await? {
                let accompanying_path = entry.path();
                if accompanying_path.is_file() {
                    let file_name = accompanying_path.to_str().unwrap();
                    for pattern in patterns {
                        if pattern.is_match(file_name)? {
                            accompanying.push(accompanying_path.clone());
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(accompanying)
    }
} 