use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use anyhow::{Result, Context};
use humansize::{format_size, DECIMAL};

/// Format file size in human-readable format
pub fn format_file_size(size: u64) -> String {
    format_size(size, DECIMAL)
}

/// Check if a path is safe to operate on (basic safety checks)
pub fn is_safe_path(path: &Path) -> bool {
    // Don't operate on system directories
    let system_dirs = [
        "/bin", "/sbin", "/usr/bin", "/usr/sbin",
        "/System", "/Library", "/Applications",
        "C:\\Windows", "C:\\Program Files", "C:\\Program Files (x86)",
    ];
    
    let path_str = path.to_string_lossy();
    for sys_dir in &system_dirs {
        if path_str.starts_with(sys_dir) {
            return false;
        }
    }
    
    true
}

/// Get file creation time (fallback to modified time if not available)
pub fn get_file_creation_time(path: &Path) -> Result<SystemTime> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {}", path.display()))?;
    
    // Try to get creation time, fallback to modified time
    metadata.created()
        .or_else(|_| metadata.modified())
        .with_context(|| format!("Failed to get creation time for {}", path.display()))
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {}", path.display()))?;
    }
    Ok(())
}

/// Generate a unique filename in a directory
pub fn generate_unique_filename(dir: &Path, original_name: &str) -> PathBuf {
    let mut path = dir.join(original_name);
    let mut counter = 1;
    
    while path.exists() {
        let (stem, ext) = split_filename(original_name);
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        path = dir.join(new_name);
        counter += 1;
    }
    
    path
}

/// Split filename into stem and extension
fn split_filename(filename: &str) -> (String, String) {
    if let Some(dot_pos) = filename.rfind('.') {
        let stem = filename[..dot_pos].to_string();
        let ext = filename[dot_pos + 1..].to_string();
        (stem, ext)
    } else {
        (filename.to_string(), String::new())
    }
}

/// Check if two paths point to the same file
pub fn are_same_file(path1: &Path, path2: &Path) -> Result<bool> {
    let meta1 = fs::metadata(path1)?;
    let meta2 = fs::metadata(path2)?;
    
    // On Unix systems, we can compare inode numbers
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(meta1.dev() == meta2.dev() && meta1.ino() == meta2.ino())
    }
    
    // On Windows, we compare file attributes and times
    #[cfg(windows)]
    {
        Ok(meta1.len() == meta2.len() && 
           meta1.modified().unwrap_or(SystemTime::UNIX_EPOCH) == 
           meta2.modified().unwrap_or(SystemTime::UNIX_EPOCH))
    }
}

/// Format duration in human-readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

/// Check if a file is likely to be a system file
pub fn is_system_file(path: &Path) -> bool {
    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    
    // Common system files to avoid
    let system_files = [
        "desktop.ini", "thumbs.db", ".ds_store", "pagefile.sys",
        "hiberfil.sys", "swapfile.sys", "bootmgr", "ntldr",
    ];
    
    let filename_lower = filename.to_lowercase();
    system_files.iter().any(|&sys_file| filename_lower == sys_file)
}

/// Calculate the percentage of one number relative to another
pub fn calculate_percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

/// Validate that a file extension is in the allowed list
pub fn is_extension_allowed(path: &Path, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        allowed.iter().any(|allowed_ext| allowed_ext.to_lowercase() == ext_str)
    } else {
        false
    }
}

/// Check if a file is readable
pub fn is_readable(path: &Path) -> bool {
    match fs::File::open(path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Get the relative path between two paths
pub fn get_relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let from_absolute = from.canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", from.display()))?;
    let to_absolute = to.canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", to.display()))?;
    
    Ok(pathdiff::diff_paths(&to_absolute, &from_absolute).unwrap_or_else(|| to_absolute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;
    
    #[test]
    fn test_split_filename() {
        assert_eq!(split_filename("test.txt"), ("test".to_string(), "txt".to_string()));
        assert_eq!(split_filename("test"), ("test".to_string(), String::new()));
        assert_eq!(split_filename("test.tar.gz"), ("test.tar".to_string(), "gz".to_string()));
    }
    
    #[test]
    fn test_generate_unique_filename() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();
        
        // Create a file
        File::create(dir_path.join("test.txt")).unwrap();
        
        // Generate unique filename
        let unique_path = generate_unique_filename(dir_path, "test.txt");
        assert_eq!(unique_path.file_name().unwrap(), "test_1.txt");
        
        // Create that file too
        File::create(&unique_path).unwrap();
        
        // Generate another unique filename
        let unique_path2 = generate_unique_filename(dir_path, "test.txt");
        assert_eq!(unique_path2.file_name().unwrap(), "test_2.txt");
    }
    
    #[test]
    fn test_is_system_file() {
        assert!(is_system_file(&PathBuf::from("desktop.ini")));
        assert!(is_system_file(&PathBuf::from("THUMBS.DB")));
        assert!(!is_system_file(&PathBuf::from("my_file.txt")));
    }
    
    #[test]
    fn test_calculate_percentage() {
        assert_eq!(calculate_percentage(50, 100), 50.0);
        assert_eq!(calculate_percentage(0, 100), 0.0);
        assert_eq!(calculate_percentage(100, 0), 0.0);
    }
} 