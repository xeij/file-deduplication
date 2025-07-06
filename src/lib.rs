pub mod scanner;
pub mod dedup;
pub mod actions;
pub mod utils;

use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;

pub use scanner::Scanner;
pub use dedup::perform_deduplication;

/// Represents a file with metadata used for deduplication
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub hash: String,
    pub modified: std::time::SystemTime,
}

/// Results of a directory scan for duplicate files
#[derive(Debug)]
pub struct DedupResult {
    pub duplicates: HashMap<String, Vec<FileInfo>>,
    pub total_files: usize,
    pub total_size: u64,
}

/// Actions that can be performed on duplicate files
#[derive(Debug, Clone)]
pub enum DedupAction {
    /// List duplicate files without taking any action
    List,
    /// Delete duplicate files (keeps the first occurrence)
    Delete,
    /// Move duplicate files to a specified directory
    Move(PathBuf),
    /// Create hard links for duplicate files
    Hardlink,
    /// Create symbolic links for duplicate files
    Symlink,
}

impl DedupResult {
    pub fn new() -> Self {
        Self {
            duplicates: HashMap::new(),
            total_files: 0,
            total_size: 0,
        }
    }

    pub fn add_file(&mut self, file: FileInfo) {
        self.total_files += 1;
        self.total_size += file.size;
        
        self.duplicates
            .entry(file.hash.clone())
            .or_insert_with(Vec::new)
            .push(file);
    }

    pub fn get_duplicate_count(&self) -> usize {
        self.duplicates
            .values()
            .map(|files| if files.len() > 1 { files.len() - 1 } else { 0 })
            .sum()
    }

    pub fn get_wasted_space(&self) -> u64 {
        self.duplicates
            .values()
            .map(|files| {
                if files.len() > 1 {
                    files[0].size * (files.len() - 1) as u64
                } else {
                    0
                }
            })
            .sum()
    }

    /// Filter out groups that don't have actual duplicates
    pub fn filter_duplicates(&mut self) {
        self.duplicates.retain(|_, files| files.len() > 1);
    }
} 