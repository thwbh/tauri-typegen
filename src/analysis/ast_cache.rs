use std::collections::HashMap;
use std::path::PathBuf;
use syn::File as SynFile;
use walkdir::WalkDir;

/// Cache entry for a parsed Rust file
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// The parsed AST
    pub ast: SynFile,
    /// File path for reference
    pub path: PathBuf,
    // Last modified time for cache invalidation (if needed later)
    // modified: std::time::SystemTime,
}

impl ParsedFile {
    pub fn new(ast: SynFile, path: PathBuf) -> Self {
        Self { ast, path }
    }
}

/// AST cache for parsed Rust files
#[derive(Debug, Default)]
pub struct AstCache {
    cache: HashMap<PathBuf, ParsedFile>,
}

impl AstCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Parse and cache all Rust files in the given project path
    pub fn parse_and_cache_all_files(
        &mut self,
        project_path: &str,
        verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if verbose {
            println!("🔄 Parsing and caching all Rust files in: {}", project_path);
        }

        for entry in WalkDir::new(project_path) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                // Skip target directory and other build artifacts
                if path.to_string_lossy().contains("/target/")
                    || path.to_string_lossy().contains("/.git/")
                {
                    continue;
                }

                if verbose {
                    println!("📄 Parsing file: {}", path.display());
                }

                let content = std::fs::read_to_string(path)?;
                match syn::parse_file(&content) {
                    Ok(ast) => {
                        let parsed_file = ParsedFile::new(ast, path.to_path_buf());
                        self.cache.insert(path.to_path_buf(), parsed_file);
                        if verbose {
                            println!("✅ Successfully parsed: {}", path.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to parse {}: {}", path.display(), e);
                        // Continue processing other files even if one fails
                    }
                }
            }
        }

        if verbose {
            println!("📊 Cached {} Rust files", self.cache.len());
        }
        Ok(())
    }

    /// Get a parsed file from the cache
    pub fn get(&self, path: &PathBuf) -> Option<&ParsedFile> {
        self.cache.get(path)
    }

    /// Get a cloned parsed file from the cache
    pub fn get_cloned(&self, path: &PathBuf) -> Option<ParsedFile> {
        self.cache.get(path).cloned()
    }

    /// Get all cached file paths
    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, PathBuf, ParsedFile> {
        self.cache.keys()
    }

    /// Get all cached files as an iterator
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, PathBuf, ParsedFile> {
        self.cache.iter()
    }

    /// Check if a file is cached
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.cache.contains_key(path)
    }

    /// Get the number of cached files
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Insert a parsed file into the cache
    pub fn insert(&mut self, path: PathBuf, parsed_file: ParsedFile) -> Option<ParsedFile> {
        self.cache.insert(path, parsed_file)
    }

    /// Parse a single file and add it to the cache
    pub fn parse_and_cache_file(
        &mut self,
        file_path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(file_path)?;
        let ast = syn::parse_file(&content)?;
        let parsed_file = ParsedFile::new(ast, file_path.to_path_buf());
        self.cache.insert(file_path.to_path_buf(), parsed_file);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn temp_dir() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("./test_ast_cache_{}_{}", std::process::id(), timestamp)
    }

    fn cleanup_dir(dir: &str) {
        let _ = fs::remove_dir_all(dir);
    }

    fn create_rust_file(dir: &str, name: &str, content: &str) -> PathBuf {
        let path = PathBuf::from(format!("{}/{}", dir, name));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    mod parsed_file {
        use super::*;

        #[test]
        fn test_new_creates_parsed_file() {
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            let parsed = ParsedFile::new(ast, path.clone());
            assert_eq!(parsed.path, path);
        }

        #[test]
        fn test_clone_works() {
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            let parsed1 = ParsedFile::new(ast, path.clone());
            let parsed2 = parsed1.clone();
            assert_eq!(parsed1.path, parsed2.path);
        }
    }

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_empty_cache() {
            let cache = AstCache::new();
            assert!(cache.is_empty());
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_default_creates_empty_cache() {
            let cache = AstCache::default();
            assert!(cache.is_empty());
        }
    }

    mod single_file_operations {
        use super::*;

        #[test]
        fn test_parse_and_cache_single_file() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = create_rust_file(&dir, "test.rs", "fn main() {}");

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_file(&path);
            assert!(result.is_ok());
            assert_eq!(cache.len(), 1);
            assert!(cache.contains(&path));

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_invalid_syntax_errors() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = create_rust_file(&dir, "invalid.rs", "fn main( {");

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_file(&path);
            assert!(result.is_err());
            assert_eq!(cache.len(), 0);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_nonexistent_file_errors() {
            let mut cache = AstCache::new();
            let path = PathBuf::from("nonexistent.rs");
            let result = cache.parse_and_cache_file(&path);
            assert!(result.is_err());
        }
    }

    mod multi_file_operations {
        use super::*;

        #[test]
        fn test_parse_and_cache_all_files() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();

            create_rust_file(&dir, "lib.rs", "pub fn hello() {}");
            create_rust_file(&dir, "main.rs", "fn main() {}");
            create_rust_file(&dir, "mod/types.rs", "struct User {}");

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_all_files(&dir, false);
            assert!(result.is_ok());
            assert_eq!(cache.len(), 3);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_skips_target_directory() {
            let dir = temp_dir();
            fs::create_dir_all(format!("{}/target", dir)).unwrap();

            create_rust_file(&dir, "lib.rs", "pub fn hello() {}");
            create_rust_file(&dir, "target/debug.rs", "fn debug() {}");

            let mut cache = AstCache::new();
            cache.parse_and_cache_all_files(&dir, false).unwrap();

            // Should only have lib.rs, not target/debug.rs
            assert_eq!(cache.len(), 1);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_skips_git_directory() {
            let dir = temp_dir();
            fs::create_dir_all(format!("{}/.git", dir)).unwrap();

            create_rust_file(&dir, "lib.rs", "pub fn hello() {}");
            create_rust_file(&dir, ".git/hooks.rs", "fn hook() {}");

            let mut cache = AstCache::new();
            cache.parse_and_cache_all_files(&dir, false).unwrap();

            assert_eq!(cache.len(), 1);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_continues_on_syntax_error() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();

            create_rust_file(&dir, "valid.rs", "fn main() {}");
            create_rust_file(&dir, "invalid.rs", "fn main( {");
            create_rust_file(&dir, "valid2.rs", "struct User {}");

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_all_files(&dir, false);
            assert!(result.is_ok());
            // Should have 2 valid files, skip the invalid one
            assert_eq!(cache.len(), 2);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_with_verbose_output() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            create_rust_file(&dir, "lib.rs", "pub fn hello() {}");

            let mut cache = AstCache::new();
            // Just verify it doesn't panic with verbose=true
            let result = cache.parse_and_cache_all_files(&dir, true);
            assert!(result.is_ok());

            cleanup_dir(&dir);
        }
    }

    mod cache_operations {
        use super::*;

        #[test]
        fn test_get_returns_reference() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            let parsed = ParsedFile::new(ast, path.clone());
            cache.insert(path.clone(), parsed);

            let result = cache.get(&path);
            assert!(result.is_some());
            assert_eq!(result.unwrap().path, path);
        }

        #[test]
        fn test_get_returns_none_for_missing() {
            let cache = AstCache::new();
            let path = PathBuf::from("missing.rs");
            assert!(cache.get(&path).is_none());
        }

        #[test]
        fn test_get_cloned_returns_owned() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            let parsed = ParsedFile::new(ast, path.clone());
            cache.insert(path.clone(), parsed);

            let result = cache.get_cloned(&path);
            assert!(result.is_some());
            assert_eq!(result.unwrap().path, path);
        }

        #[test]
        fn test_contains_checks_presence() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            let parsed = ParsedFile::new(ast, path.clone());
            cache.insert(path.clone(), parsed);

            assert!(cache.contains(&path));
            assert!(!cache.contains(&PathBuf::from("other.rs")));
        }

        #[test]
        fn test_len_returns_count() {
            let mut cache = AstCache::new();
            assert_eq!(cache.len(), 0);

            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            cache.insert(
                PathBuf::from("test.rs"),
                ParsedFile::new(ast, PathBuf::from("test.rs")),
            );
            assert_eq!(cache.len(), 1);
        }

        #[test]
        fn test_is_empty_checks_emptiness() {
            let mut cache = AstCache::new();
            assert!(cache.is_empty());

            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            cache.insert(
                PathBuf::from("test.rs"),
                ParsedFile::new(ast, PathBuf::from("test.rs")),
            );
            assert!(!cache.is_empty());
        }

        #[test]
        fn test_clear_empties_cache() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            cache.insert(
                PathBuf::from("test.rs"),
                ParsedFile::new(ast, PathBuf::from("test.rs")),
            );

            assert!(!cache.is_empty());
            cache.clear();
            assert!(cache.is_empty());
        }

        #[test]
        fn test_insert_returns_old_value() {
            let mut cache = AstCache::new();
            let ast1: SynFile = syn::parse_str("fn main() {}").unwrap();
            let ast2: SynFile = syn::parse_str("fn test() {}").unwrap();
            let path = PathBuf::from("test.rs");

            let old = cache.insert(path.clone(), ParsedFile::new(ast1, path.clone()));
            assert!(old.is_none());

            let old = cache.insert(path.clone(), ParsedFile::new(ast2, path.clone()));
            assert!(old.is_some());
        }

        #[test]
        fn test_keys_returns_iterator() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path1 = PathBuf::from("test1.rs");
            let path2 = PathBuf::from("test2.rs");

            cache.insert(path1.clone(), ParsedFile::new(ast.clone(), path1.clone()));
            cache.insert(path2.clone(), ParsedFile::new(ast.clone(), path2.clone()));

            let keys: Vec<_> = cache.keys().collect();
            assert_eq!(keys.len(), 2);
        }

        #[test]
        fn test_iter_returns_iterator() {
            let mut cache = AstCache::new();
            let ast: SynFile = syn::parse_str("fn main() {}").unwrap();
            let path = PathBuf::from("test.rs");
            cache.insert(path.clone(), ParsedFile::new(ast, path.clone()));

            let count = cache.iter().count();
            assert_eq!(count, 1);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_directory() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_all_files(&dir, false);
            assert!(result.is_ok());
            assert_eq!(cache.len(), 0);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_directory_with_only_non_rust_files() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            create_rust_file(&dir, "readme.txt", "Hello");
            create_rust_file(&dir, "config.json", "{}");

            let mut cache = AstCache::new();
            cache.parse_and_cache_all_files(&dir, false).unwrap();
            assert_eq!(cache.len(), 0);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_parse_empty_rust_file() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = create_rust_file(&dir, "empty.rs", "");

            let mut cache = AstCache::new();
            let result = cache.parse_and_cache_file(&path);
            assert!(result.is_ok());
            assert_eq!(cache.len(), 1);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_cache_same_file_twice() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let path = create_rust_file(&dir, "test.rs", "fn main() {}");

            let mut cache = AstCache::new();
            cache.parse_and_cache_file(&path).unwrap();
            cache.parse_and_cache_file(&path).unwrap();

            // Should still be 1 (overwritten)
            assert_eq!(cache.len(), 1);

            cleanup_dir(&dir);
        }
    }
}
