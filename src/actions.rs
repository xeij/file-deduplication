use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use anyhow::{Result, Context};
use console::style;
use humansize::{format_size, DECIMAL};

use crate::{FileInfo, DedupAction};

/// Performs the specified action on duplicate files
pub fn perform_action(
    duplicates: &[FileInfo],
    action: &DedupAction,
    dry_run: bool,
) -> Result<ActionResult> {
    let mut result = ActionResult::new();
    
    // Skip the first file (original) and process duplicates
    for duplicate in duplicates.iter().skip(1) {
        let action_result = match action {
            DedupAction::List => {
                // List action is handled in the main display function
                continue;
            }
            DedupAction::Delete => delete_file(&duplicate.path, dry_run)?,
            DedupAction::Move(target_dir) => move_file(&duplicate.path, target_dir, dry_run)?,
            DedupAction::Hardlink => create_hardlink(&duplicates[0].path, &duplicate.path, dry_run)?,
            DedupAction::Symlink => create_symlink(&duplicates[0].path, &duplicate.path, dry_run)?,
        };
        
        result.add_operation(action_result);
    }
    
    Ok(result)
}

/// Result of performing actions on files
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub operations: Vec<FileOperation>,
    pub total_space_saved: u64,
    pub total_files_processed: usize,
}

/// Represents a single file operation
#[derive(Debug, Clone)]
pub struct FileOperation {
    pub path: PathBuf,
    pub action: String,
    pub success: bool,
    pub error: Option<String>,
    pub space_saved: u64,
}

impl ActionResult {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            total_space_saved: 0,
            total_files_processed: 0,
        }
    }

    pub fn add_operation(&mut self, operation: FileOperation) {
        self.total_space_saved += operation.space_saved;
        self.total_files_processed += 1;
        self.operations.push(operation);
    }

    pub fn success_count(&self) -> usize {
        self.operations.iter().filter(|op| op.success).count()
    }

    pub fn error_count(&self) -> usize {
        self.operations.iter().filter(|op| !op.success).count()
    }

    pub fn print_summary(&self) {
        println!();
        println!("{}", style("📊 Action Summary").green().bold());
        println!("{}", style("-".repeat(20)).green());
        println!("Files processed: {}", self.total_files_processed);
        println!("Successful operations: {}", self.success_count());
        println!("Failed operations: {}", self.error_count());
        println!("Total space saved: {}", format_size(self.total_space_saved, DECIMAL));
        
        if self.error_count() > 0 {
            println!();
            println!("{}", style("❌ Errors:").red().bold());
            for op in &self.operations {
                if !op.success {
                    if let Some(error) = &op.error {
                        println!("  {}: {}", op.path.display(), error);
                    }
                }
            }
        }
    }
}

/// Delete a file
fn delete_file(path: &Path, dry_run: bool) -> Result<FileOperation> {
    let file_size = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {}", path.display()))?
        .len();
    
    if dry_run {
        println!("Would delete: {}", path.display());
        return Ok(FileOperation {
            path: path.to_path_buf(),
            action: "delete".to_string(),
            success: true,
            error: None,
            space_saved: file_size,
        });
    }

    match fs::remove_file(path) {
        Ok(_) => {
            println!("✅ Deleted: {}", path.display());
            Ok(FileOperation {
                path: path.to_path_buf(),
                action: "delete".to_string(),
                success: true,
                error: None,
                space_saved: file_size,
            })
        }
        Err(e) => {
            let error_msg = format!("Failed to delete: {}", e);
            eprintln!("❌ {}: {}", path.display(), error_msg);
            Ok(FileOperation {
                path: path.to_path_buf(),
                action: "delete".to_string(),
                success: false,
                error: Some(error_msg),
                space_saved: 0,
            })
        }
    }
}

/// Move a file to a target directory
fn move_file(source: &Path, target_dir: &Path, dry_run: bool) -> Result<FileOperation> {
    let file_size = fs::metadata(source)
        .with_context(|| format!("Failed to get metadata for {}", source.display()))?
        .len();
    
    // Create target directory if it doesn't exist
    if !dry_run {
        fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create target directory {}", target_dir.display()))?;
    }
    
    // Generate unique filename if file already exists in target
    let filename = source.file_name().unwrap();
    let mut target_path = target_dir.join(filename);
    let mut counter = 1;
    
    while target_path.exists() {
        let stem = source.file_stem().unwrap().to_string_lossy();
        let ext = source.extension().map(|s| s.to_string_lossy()).unwrap_or_default();
        let new_filename = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        target_path = target_dir.join(new_filename);
        counter += 1;
    }
    
    if dry_run {
        println!("Would move: {} -> {}", source.display(), target_path.display());
        return Ok(FileOperation {
            path: source.to_path_buf(),
            action: "move".to_string(),
            success: true,
            error: None,
            space_saved: file_size,
        });
    }

    match fs::rename(source, &target_path) {
        Ok(_) => {
            println!("✅ Moved: {} -> {}", source.display(), target_path.display());
            Ok(FileOperation {
                path: source.to_path_buf(),
                action: "move".to_string(),
                success: true,
                error: None,
                space_saved: file_size,
            })
        }
        Err(e) => {
            let error_msg = format!("Failed to move: {}", e);
            eprintln!("❌ {}: {}", source.display(), error_msg);
            Ok(FileOperation {
                path: source.to_path_buf(),
                action: "move".to_string(),
                success: false,
                error: Some(error_msg),
                space_saved: 0,
            })
        }
    }
}

/// Create a hard link
fn create_hardlink(original: &Path, duplicate: &Path, dry_run: bool) -> Result<FileOperation> {
    let file_size = fs::metadata(duplicate)
        .with_context(|| format!("Failed to get metadata for {}", duplicate.display()))?
        .len();
    
    if dry_run {
        println!("Would create hardlink: {} -> {}", duplicate.display(), original.display());
        return Ok(FileOperation {
            path: duplicate.to_path_buf(),
            action: "hardlink".to_string(),
            success: true,
            error: None,
            space_saved: file_size,
        });
    }

    // Remove duplicate file first
    if let Err(e) = fs::remove_file(duplicate) {
        let error_msg = format!("Failed to remove duplicate before hardlinking: {}", e);
        eprintln!("❌ {}: {}", duplicate.display(), error_msg);
        return Ok(FileOperation {
            path: duplicate.to_path_buf(),
            action: "hardlink".to_string(),
            success: false,
            error: Some(error_msg),
            space_saved: 0,
        });
    }

    // Create hard link
    match fs::hard_link(original, duplicate) {
        Ok(_) => {
            println!("✅ Created hardlink: {} -> {}", duplicate.display(), original.display());
            Ok(FileOperation {
                path: duplicate.to_path_buf(),
                action: "hardlink".to_string(),
                success: true,
                error: None,
                space_saved: file_size,
            })
        }
        Err(e) => {
            let error_msg = format!("Failed to create hardlink: {}", e);
            eprintln!("❌ {}: {}", duplicate.display(), error_msg);
            Ok(FileOperation {
                path: duplicate.to_path_buf(),
                action: "hardlink".to_string(),
                success: false,
                error: Some(error_msg),
                space_saved: 0,
            })
        }
    }
}

/// Create a symbolic link
fn create_symlink(original: &Path, duplicate: &Path, dry_run: bool) -> Result<FileOperation> {
    let file_size = fs::metadata(duplicate)
        .with_context(|| format!("Failed to get metadata for {}", duplicate.display()))?
        .len();
    
    if dry_run {
        println!("Would create symlink: {} -> {}", duplicate.display(), original.display());
        return Ok(FileOperation {
            path: duplicate.to_path_buf(),
            action: "symlink".to_string(),
            success: true,
            error: None,
            space_saved: file_size,
        });
    }

    // Remove duplicate file first
    if let Err(e) = fs::remove_file(duplicate) {
        let error_msg = format!("Failed to remove duplicate before symlinking: {}", e);
        eprintln!("❌ {}: {}", duplicate.display(), error_msg);
        return Ok(FileOperation {
            path: duplicate.to_path_buf(),
            action: "symlink".to_string(),
            success: false,
            error: Some(error_msg),
            space_saved: 0,
        });
    }

    // Create symbolic link
    let result = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original, duplicate)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(original, duplicate)
        }
    };

    match result {
        Ok(_) => {
            println!("✅ Created symlink: {} -> {}", duplicate.display(), original.display());
            Ok(FileOperation {
                path: duplicate.to_path_buf(),
                action: "symlink".to_string(),
                success: true,
                error: None,
                space_saved: file_size,
            })
        }
        Err(e) => {
            let error_msg = format!("Failed to create symlink: {}", e);
            eprintln!("❌ {}: {}", duplicate.display(), error_msg);
            Ok(FileOperation {
                path: duplicate.to_path_buf(),
                action: "symlink".to_string(),
                success: false,
                error: Some(error_msg),
                space_saved: 0,
            })
        }
    }
} 