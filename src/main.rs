use clap::{Parser, ValueEnum};
use anyhow::Result;
use std::path::PathBuf;
use console::style;
use file_deduplication::{Scanner, DedupAction, DedupResult, perform_deduplication};

#[derive(Debug, Clone, ValueEnum)]
enum ActionType {
    /// List duplicate files without taking any action
    List,
    /// Delete duplicate files (keeps the first occurrence)
    Delete,
    /// Move duplicate files to a specified directory
    Move,
    /// Create hard links for duplicate files
    Hardlink,
    /// Create symbolic links for duplicate files
    Symlink,
}

#[derive(Parser)]
#[command(
    name = "dedup",
    version,
    about = "A fast, safe, and cross-platform file deduplication utility",
    long_about = "Scan directories, identify duplicate files based on hash comparisons, and provide options to delete, move, or link duplicates to save disk space."
)]
struct Cli {
    /// Directory paths to scan for duplicates
    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Directories to scan (can be specified multiple times)"
    )]
    dir: Vec<PathBuf>,

    /// Action to take on duplicate files
    #[arg(
        short,
        long,
        value_enum,
        default_value = "list",
        help = "Action to perform on duplicate files"
    )]
    action: ActionType,

    /// Directory to move duplicate files to (required for move action)
    #[arg(
        long,
        value_name = "PATH",
        help = "Target directory for move action"
    )]
    move_to: Option<PathBuf>,

    /// Perform a dry run without making actual changes
    #[arg(
        long,
        help = "Show what would be done without making changes"
    )]
    dry_run: bool,

    /// Minimum file size to consider (in bytes)
    #[arg(
        long,
        default_value = "0",
        help = "Minimum file size in bytes to consider"
    )]
    min_size: u64,

    /// Maximum file size to consider (in bytes)
    #[arg(
        long,
        help = "Maximum file size in bytes to consider"
    )]
    max_size: Option<u64>,

    /// File extensions to include (e.g., jpg,png,pdf)
    #[arg(
        long,
        value_delimiter = ',',
        help = "File extensions to include (comma-separated)"
    )]
    include_ext: Vec<String>,

    /// File extensions to exclude (e.g., tmp,log)
    #[arg(
        long,
        value_delimiter = ',',
        help = "File extensions to exclude (comma-separated)"
    )]
    exclude_ext: Vec<String>,

    /// Skip confirmation prompts
    #[arg(
        short,
        long,
        help = "Skip confirmation prompts (use with caution)"
    )]
    yes: bool,

    /// Verbose output
    #[arg(
        short,
        long,
        help = "Enable verbose output"
    )]
    verbose: bool,

    /// Number of threads to use for parallel processing
    #[arg(
        long,
        default_value = "0",
        help = "Number of threads (0 = auto-detect)"
    )]
    threads: usize,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Set up thread pool if specified
    if args.threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()
            .unwrap();
    }

    // Validate arguments
    if args.dir.is_empty() {
        eprintln!("{}", style("Error: At least one directory must be specified").red());
        std::process::exit(1);
    }

    if matches!(args.action, ActionType::Move) && args.move_to.is_none() {
        eprintln!("{}", style("Error: --move-to is required when using move action").red());
        std::process::exit(1);
    }

    // Create scanner with filters
    let mut scanner = Scanner::new();
    scanner.set_min_size(args.min_size);
    if let Some(max_size) = args.max_size {
        scanner.set_max_size(max_size);
    }
    scanner.set_include_extensions(args.include_ext);
    scanner.set_exclude_extensions(args.exclude_ext);
    scanner.set_verbose(args.verbose);

    println!("{}", style("🔍 Scanning directories for duplicate files...").cyan().bold());

    // Scan directories
    let scan_result = scanner.scan_directories(&args.dir)?;
    
    if scan_result.duplicates.is_empty() {
        println!("{}", style("✅ No duplicate files found!").green().bold());
        return Ok(());
    }

    // Display results
    display_results(&scan_result, args.verbose)?;

    // Perform action
    let action = match args.action {
        ActionType::List => DedupAction::List,
        ActionType::Delete => DedupAction::Delete,
        ActionType::Move => DedupAction::Move(args.move_to.unwrap()),
        ActionType::Hardlink => DedupAction::Hardlink,
        ActionType::Symlink => DedupAction::Symlink,
    };

    if !matches!(action, DedupAction::List) {
        if args.dry_run {
            println!("{}", style("🧪 Dry run mode - no changes will be made").yellow().bold());
        } else if !args.yes {
            let proceed = dialoguer::Confirm::new()
                .with_prompt("Do you want to proceed with the selected action?")
                .interact()?;
            
            if !proceed {
                println!("{}", style("Operation cancelled").yellow());
                return Ok(());
            }
        }

        perform_deduplication(&scan_result, action, args.dry_run)?;
    }

    Ok(())
}

fn display_results(result: &DedupResult, verbose: bool) -> Result<()> {
    use humansize::{format_size, DECIMAL};
    
    println!();
    println!("{}", style("📊 Duplicate Files Found").cyan().bold());
    println!("{}", style("=".repeat(40)).cyan());
    
    let mut total_duplicates = 0;
    let mut total_waste = 0u64;
    
    for (hash, files) in &result.duplicates {
        if files.len() > 1 {
            total_duplicates += files.len() - 1; // Don't count the original
            let file_size = files[0].size;
            let waste = file_size * (files.len() - 1) as u64;
            total_waste += waste;
            
            if verbose {
                println!();
                println!("{} {} ({})", 
                    style("Hash:").bold(), 
                    &hash[..16], 
                    format_size(file_size, DECIMAL)
                );
                for (i, file) in files.iter().enumerate() {
                    let marker = if i == 0 { "📄" } else { "🔗" };
                    println!("  {} {}", marker, file.path.display());
                }
            } else {
                println!("{} duplicate files for {} ({})", 
                    files.len() - 1, 
                    files[0].path.file_name().unwrap_or_default().to_string_lossy(),
                    format_size(waste, DECIMAL)
                );
            }
        }
    }
    
    println!();
    println!("{}", style("📈 Summary").green().bold());
    println!("{}", style("-".repeat(20)).green());
    println!("Total files scanned: {}", result.total_files);
    println!("Duplicate files found: {}", total_duplicates);
    println!("Potential space savings: {}", format_size(total_waste, DECIMAL));
    
    Ok(())
} 