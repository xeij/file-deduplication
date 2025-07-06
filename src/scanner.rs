use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use anyhow::{Result, Context};
use blake3::Hasher;
use walkdir::WalkDir;
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;

use crate::{FileInfo, DedupResult};

/// Configuration for file scanning
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub min_size: u64,
    pub max_size: Option<u64>,
    pub include_extensions: HashSet<String>,
    pub exclude_extensions: HashSet<String>,
    pub verbose: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            min_size: 0,
            max_size: None,
            include_extensions: HashSet::new(),
            exclude_extensions: HashSet::new(),
            verbose: false,
        }
    }
}

/// Scanner for finding duplicate files
pub struct Scanner {
    config: ScanConfig,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            config: ScanConfig::default(),
        }
    }

    pub fn set_min_size(&mut self, size: u64) {
        self.config.min_size = size;
    }

    pub fn set_max_size(&mut self, size: u64) {
        self.config.max_size = Some(size);
    }

    pub fn set_include_extensions(&mut self, extensions: Vec<String>) {
        self.config.include_extensions = extensions.into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
    }

    pub fn set_exclude_extensions(&mut self, extensions: Vec<String>) {
        self.config.exclude_extensions = extensions.into_iter()
            .map(|ext| ext.to_lowercase())
            .collect();
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.config.verbose = verbose;
    }

    /// Scan directories for duplicate files
    pub fn scan_directories(&self, directories: &[PathBuf]) -> Result<DedupResult> {
        // First pass: collect all files
        let files = self.collect_files(directories)?;
        
        if files.is_empty() {
            return Ok(DedupResult::new());
        }

        // Second pass: hash files and build result
        self.hash_files(files)
    }

    /// Collect all files from directories based on filters
    fn collect_files(&self, directories: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for dir in directories {
            if !dir.exists() {
                eprintln!("{}", style(format!("Warning: Directory {} does not exist", dir.display())).yellow());
                continue;
            }

            if !dir.is_dir() {
                eprintln!("{}", style(format!("Warning: {} is not a directory", dir.display())).yellow());
                continue;
            }

            let walker = WalkDir::new(dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in walker {
                let path = entry.path().to_path_buf();
                
                if self.should_include_file(&path)? {
                    files.push(path);
                }
            }
        }

        if self.config.verbose {
            println!("{} files found matching criteria", files.len());
        }

        Ok(files)
    }

    /// Check if a file should be included based on filters
    fn should_include_file(&self, path: &Path) -> Result<bool> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;

        let size = metadata.len();

        // Size filters
        if size < self.config.min_size {
            return Ok(false);
        }

        if let Some(max_size) = self.config.max_size {
            if size > max_size {
                return Ok(false);
            }
        }

        // Extension filters
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            
            // If include list is specified, file must be in it
            if !self.config.include_extensions.is_empty() {
                if !self.config.include_extensions.contains(&ext_str) {
                    return Ok(false);
                }
            }
            
            // If exclude list is specified, file must not be in it
            if self.config.exclude_extensions.contains(&ext_str) {
                return Ok(false);
            }
        } else if !self.config.include_extensions.is_empty() {
            // No extension, but include list is specified
            return Ok(false);
        }

        Ok(true)
    }

    /// Hash files in parallel and build the result
    fn hash_files(&self, files: Vec<PathBuf>) -> Result<DedupResult> {
        let progress = ProgressBar::new(files.len() as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-")
        );

        let file_infos: Result<Vec<FileInfo>, _> = files
            .into_par_iter()
            .map(|path| {
                let result = self.hash_file(&path);
                progress.inc(1);
                result
            })
            .collect();

        progress.finish_with_message("✅ Hashing complete");

        let mut result = DedupResult::new();
        
        for file_info in file_infos? {
            result.add_file(file_info);
        }

        // Filter out non-duplicates
        result.filter_duplicates();

        Ok(result)
    }

    /// Hash a single file
    fn hash_file(&self, path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;

        let hash = self.calculate_hash(path)?;

        Ok(FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            hash,
            modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        })
    }

    /// Calculate BLAKE3 hash of a file
    fn calculate_hash(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)
            .with_context(|| format!("Failed to open file {}", path.display()))?;
        
        let mut hasher = Hasher::new();
        let mut buffer = vec![0; 8192]; // 8KB buffer
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .with_context(|| format!("Failed to read file {}", path.display()))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(hasher.finalize().to_hex().to_string())
    }
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
} 