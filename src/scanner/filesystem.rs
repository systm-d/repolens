//! File system scanning utilities

use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::Path;

/// Information about a file in the repository
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Relative path from repository root
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Whether the file is a directory
    #[allow(dead_code)]
    pub is_dir: bool,
}

/// Scan a directory and return information about all files
///
/// Uses parallel processing for better performance on large repositories.
/// Only returns regular files, not directories.
pub fn scan_directory(root: &Path) -> Vec<FileInfo> {
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .build();

    walker
        .into_iter()
        .par_bridge()
        .filter_map(|entry_result| {
            let entry = entry_result.ok()?;
            let path = entry.path();

            // Skip the root directory itself
            if path == root {
                return None;
            }

            // Skip .git directory
            if path.components().any(|c| c.as_os_str() == ".git") {
                return None;
            }

            // Get file metadata
            let metadata = entry.metadata().ok()?;

            // Skip directories - we only want files
            if metadata.is_dir() {
                return None;
            }

            // Get relative path - handle errors gracefully
            let relative_path = match path.strip_prefix(root) {
                Ok(stripped) => stripped
                    .to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| stripped.to_string_lossy().to_string()),
                Err(_) => {
                    return None;
                }
            };

            if relative_path.is_empty() {
                return None;
            }

            Some(FileInfo {
                path: relative_path,
                size: metadata.len(),
                is_dir: false, // Always false now since we filter directories
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create test files
        fs::write(root.join("test.txt"), "hello").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir/nested.txt"), "world").unwrap();

        let files = scan_directory(root);

        assert!(files.iter().any(|f| f.path == "test.txt"));
        assert!(
            files
                .iter()
                .any(|f| f.path == "subdir/nested.txt" || f.path == "subdir\\nested.txt")
        );
    }

    #[test]
    fn test_scan_directory_file_size() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a file with known content
        let content = "hello world";
        fs::write(root.join("sized.txt"), content).unwrap();

        let files = scan_directory(root);
        let sized_file = files.iter().find(|f| f.path == "sized.txt").unwrap();

        assert_eq!(sized_file.size, content.len() as u64);
    }

    #[test]
    fn test_scan_directory_excludes_git() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create .git directory (simulating a git repo)
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "git config").unwrap();
        fs::write(root.join("regular.txt"), "regular file").unwrap();

        let files = scan_directory(root);

        // Should not include .git files
        assert!(!files.iter().any(|f| f.path.contains(".git")));
        // Should include regular files
        assert!(files.iter().any(|f| f.path == "regular.txt"));
    }

    #[test]
    fn test_scan_directory_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let files = scan_directory(root);
        // Empty directory should return no files
        assert!(files.is_empty());
    }

    #[test]
    fn test_scan_directory_nested_structure() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create deeply nested structure
        fs::create_dir_all(root.join("a/b/c/d")).unwrap();
        fs::write(root.join("a/b/c/d/deep.txt"), "deep file").unwrap();

        let files = scan_directory(root);
        assert!(files.iter().any(|f| f.path.contains("deep.txt")));
    }

    #[test]
    fn test_scan_excludes_directories() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("testdir")).unwrap();
        fs::write(root.join("testfile.txt"), "content").unwrap();
        fs::write(root.join("testdir/nested.txt"), "nested").unwrap();

        let files = scan_directory(root);

        // Directories should not be in the results
        assert!(!files.iter().any(|f| f.path == "testdir"));

        // Files should be included
        assert!(files.iter().any(|f| f.path == "testfile.txt"));
        assert!(
            files
                .iter()
                .any(|f| f.path == "testdir/nested.txt" || f.path == "testdir\\nested.txt")
        );

        // All entries should have is_dir = false
        assert!(files.iter().all(|f| !f.is_dir));
    }
}
