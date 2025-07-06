use anyhow::Result;
use console::style;
use humansize::{format_size, DECIMAL};

use crate::{DedupResult, DedupAction};
use crate::actions::{perform_action, ActionResult};

/// Perform deduplication on the scan results
pub fn perform_deduplication(
    scan_result: &DedupResult,
    action: DedupAction,
    dry_run: bool,
) -> Result<()> {
    if matches!(action, DedupAction::List) {
        // List action is already handled in the main display function
        return Ok(());
    }

    let action_name = match action {
        DedupAction::Delete => "Deleting",
        DedupAction::Move(_) => "Moving",
        DedupAction::Hardlink => "Creating hardlinks for",
        DedupAction::Symlink => "Creating symlinks for",
        DedupAction::List => "Listing", // This shouldn't happen due to the check above
    };

    println!();
    println!("{} {}", style(format!("🔄 {} duplicate files...", action_name)).cyan().bold(), 
             if dry_run { style("(DRY RUN)").yellow() } else { style("").clear() });

    let mut total_result = ActionResult::new();
    let mut group_count = 0;

    for (hash, files) in &scan_result.duplicates {
        if files.len() > 1 {
            group_count += 1;
            
            if dry_run || matches!(action, DedupAction::Delete | DedupAction::Move(_)) {
                println!();
                println!("{} {} ({})", 
                    style(format!("Processing group {}:", group_count)).bold(),
                    &hash[..12],
                    format_size(files[0].size, DECIMAL)
                );
                println!("  📄 Keeping: {}", files[0].path.display());
            }

            let result = perform_action(files, &action, dry_run)?;
            
            // Merge results
            for operation in result.operations {
                total_result.add_operation(operation);
            }
        }
    }

    // Print summary
    total_result.print_summary();

    if !dry_run {
        println!();
        println!("{}", style("✅ Deduplication complete!").green().bold());
    }

    Ok(())
}

/// Analyze the scan results and provide recommendations
pub fn analyze_duplicates(scan_result: &DedupResult) -> DedupAnalysis {
    let mut analysis = DedupAnalysis::new();
    
    for (_, files) in &scan_result.duplicates {
        if files.len() > 1 {
            let file_size = files[0].size;
            let duplicate_count = files.len() - 1;
            
            analysis.total_groups += 1;
            analysis.total_duplicates += duplicate_count;
            analysis.total_wasted_space += file_size * duplicate_count as u64;
            
            // Categorize by size
            match file_size {
                0..=1024 => analysis.small_files += duplicate_count,
                1025..=1048576 => analysis.medium_files += duplicate_count,
                _ => analysis.large_files += duplicate_count,
            }
            
            // Track largest waste
            let group_waste = file_size * duplicate_count as u64;
            if group_waste > analysis.largest_waste.1 {
                analysis.largest_waste = (files[0].path.clone(), group_waste);
            }
        }
    }
    
    analysis
}

/// Analysis results for duplicate files
#[derive(Debug)]
pub struct DedupAnalysis {
    pub total_groups: usize,
    pub total_duplicates: usize,
    pub total_wasted_space: u64,
    pub small_files: usize,    // <= 1KB
    pub medium_files: usize,   // 1KB - 1MB
    pub large_files: usize,    // > 1MB
    pub largest_waste: (std::path::PathBuf, u64), // (path, wasted_bytes)
}

impl DedupAnalysis {
    pub fn new() -> Self {
        Self {
            total_groups: 0,
            total_duplicates: 0,
            total_wasted_space: 0,
            small_files: 0,
            medium_files: 0,
            large_files: 0,
            largest_waste: (std::path::PathBuf::new(), 0),
        }
    }

    pub fn print_analysis(&self) {
        println!();
        println!("{}", style("🔍 Duplicate Analysis").cyan().bold());
        println!("{}", style("=".repeat(30)).cyan());
        
        println!("Duplicate groups found: {}", self.total_groups);
        println!("Total duplicate files: {}", self.total_duplicates);
        println!("Total wasted space: {}", format_size(self.total_wasted_space, DECIMAL));
        
        println!();
        println!("{}", style("📊 File Size Distribution:").bold());
        println!("  Small files (≤1KB): {}", self.small_files);
        println!("  Medium files (1KB-1MB): {}", self.medium_files);
        println!("  Large files (>1MB): {}", self.large_files);
        
        if self.largest_waste.1 > 0 {
            println!();
            println!("{}", style("🎯 Largest opportunity:").bold());
            println!("  File: {}", self.largest_waste.0.display());
            println!("  Potential savings: {}", format_size(self.largest_waste.1, DECIMAL));
        }
        
        // Recommendations
        println!();
        println!("{}", style("💡 Recommendations:").green().bold());
        
        if self.large_files > 0 {
            println!("  • Focus on large files first for maximum space savings");
        }
        
        if self.total_duplicates > 100 {
            println!("  • Consider using hardlinks to save space without losing data");
        }
        
        if self.total_wasted_space > 1_000_000_000 { // > 1GB
            println!("  • Significant space savings possible (>1GB)");
        }
        
        println!("  • Always use --dry-run first to preview changes");
        println!("  • Consider backing up important files before deletion");
    }
}

impl Default for DedupAnalysis {
    fn default() -> Self {
        Self::new()
    }
} 