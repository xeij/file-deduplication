# File Deduplication Utility

A fast, safe, and cross-platform command-line tool for file deduplication written in Rust. This utility scans directories, identifies duplicate files using hash comparisons, and provides various options to handle duplicates efficiently.

## Features

- 🔍 **Fast Scanning**: Parallel directory traversal and file hashing using BLAKE3
- 🛡️ **Safe Operations**: Dry-run mode, confirmation prompts, and safety checks
- 🎯 **Multiple Actions**: Delete, move, hardlink, or symlink duplicate files
- 📊 **Detailed Reporting**: Progress bars, summaries, and space savings analysis
- 🔧 **Flexible Filtering**: Filter by file size, extensions, and paths
- 🚀 **Cross-Platform**: Works on Windows, macOS, and Linux
- ⚡ **High Performance**: Multi-threaded processing with configurable thread count

## Installation

### From Source

```bash
git clone https://github.com/yourusername/file-deduplication.git
cd file-deduplication
cargo build --release
```

The binary will be available at `target/release/dedup` (or `dedup.exe` on Windows).

### Using Cargo

```bash
cargo install --path .
```

## Usage

### Basic Usage

```bash
# List duplicate files in a directory
dedup --dir /path/to/directory

# Scan multiple directories
dedup --dir /path/to/dir1 --dir /path/to/dir2

# Delete duplicate files (dry run first!)
dedup --dir /path/to/directory --action delete --dry-run
dedup --dir /path/to/directory --action delete

# Move duplicates to a specific directory
dedup --dir /path/to/directory --action move --move-to /path/to/duplicates

# Create hard links for duplicates
dedup --dir /path/to/directory --action hardlink

# Create symbolic links for duplicates
dedup --dir /path/to/directory --action symlink
```

### Command Line Options

```
Usage: dedup [OPTIONS] --dir <PATH>

Options:
  -d, --dir <PATH>              Directories to scan (can be specified multiple times)
  -a, --action <ACTION>         Action to perform on duplicate files
                                [default: list] [possible values: list, delete, move, hardlink, symlink]
      --move-to <PATH>          Target directory for move action
      --dry-run                 Show what would be done without making changes
      --min-size <SIZE>         Minimum file size in bytes to consider [default: 0]
      --max-size <SIZE>         Maximum file size in bytes to consider
      --include-ext <EXTENSIONS> File extensions to include (comma-separated)
      --exclude-ext <EXTENSIONS> File extensions to exclude (comma-separated)
  -y, --yes                     Skip confirmation prompts (use with caution)
  -v, --verbose                 Enable verbose output
      --threads <COUNT>         Number of threads (0 = auto-detect) [default: 0]
  -h, --help                    Print help
  -V, --version                 Print version
```

## Examples

### Basic Scanning

```bash
# Scan a directory and list duplicates
dedup --dir ~/Documents

# Scan with verbose output
dedup --dir ~/Documents --verbose

# Scan multiple directories
dedup --dir ~/Documents --dir ~/Pictures --dir ~/Downloads
```

### Filtering Files

```bash
# Only scan image files
dedup --dir ~/Pictures --include-ext jpg,jpeg,png,gif

# Exclude temporary files
dedup --dir ~/Documents --exclude-ext tmp,log,bak

# Only scan files larger than 1MB
dedup --dir ~/Documents --min-size 1048576

# Only scan files between 1MB and 100MB
dedup --dir ~/Documents --min-size 1048576 --max-size 104857600
```

### Safe Operations

```bash
# Always use dry-run first to preview changes
dedup --dir ~/Documents --action delete --dry-run

# Then perform the actual operation
dedup --dir ~/Documents --action delete

# Skip confirmation prompts (use with caution)
dedup --dir ~/Documents --action delete --yes
```

### Different Actions

```bash
# Delete duplicates (keeps the first occurrence)
dedup --dir ~/Documents --action delete

# Move duplicates to a backup directory
dedup --dir ~/Documents --action move --move-to ~/duplicates-backup

# Replace duplicates with hard links (saves space)
dedup --dir ~/Documents --action hardlink

# Replace duplicates with symbolic links
dedup --dir ~/Documents --action symlink
```

### Performance Tuning

```bash
# Use specific number of threads
dedup --dir ~/Documents --threads 8

# For large datasets, consider using hardlinks for safety
dedup --dir ~/large-dataset --action hardlink --threads 16
```

## Actions Explained

### List (Default)
Lists all duplicate files without making any changes. Shows file paths, sizes, and potential space savings.

### Delete
Deletes duplicate files, keeping only the first occurrence found. **Use with caution!**

### Move
Moves duplicate files to a specified directory, preserving the originals in their locations.

### Hardlink
Replaces duplicate files with hard links to the first occurrence. This saves space while maintaining multiple file paths.

### Symlink
Replaces duplicate files with symbolic links to the first occurrence. Requires appropriate permissions on Windows.

## Safety Features

- **Dry Run Mode**: Preview changes before applying them
- **Confirmation Prompts**: Ask before performing destructive operations
- **System File Detection**: Avoid operating on system files
- **Path Safety Checks**: Prevent operations on system directories
- **Error Handling**: Graceful handling of file access errors

## Performance

The tool is designed for high performance:

- **Parallel Processing**: Multi-threaded file hashing and directory traversal
- **Efficient Hashing**: Uses BLAKE3 for fast and secure hashing
- **Memory Efficient**: Streams file content for hashing large files
- **Progress Reporting**: Real-time progress bars and ETA

## Output Example

```
🔍 Scanning directories for duplicate files...
✅ Hashing complete

📊 Duplicate Files Found
========================================

Hash: 7d865e959b2f... (2.5 MB)
  📄 /home/user/Documents/photo.jpg
  🔗 /home/user/Pictures/photo.jpg
  🔗 /home/user/Backup/photo.jpg

Hash: 3c4e20a8b5d1... (1.2 MB)
  📄 /home/user/Documents/report.pdf
  🔗 /home/user/Downloads/report.pdf

📈 Summary
--------------------
Total files scanned: 1,234
Duplicate files found: 3
Potential space savings: 3.7 MB
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Uses [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) for fast and secure hashing
- Built with [Rust](https://www.rust-lang.org/) for performance and safety
- CLI interface powered by [clap](https://github.com/clap-rs/clap)

## Changelog

### v0.1.0
- Initial release
- Basic file deduplication functionality
- Support for multiple actions (list, delete, move, hardlink, symlink)
- Comprehensive filtering options
- Cross-platform support 