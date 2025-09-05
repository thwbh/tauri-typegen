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
    pub fn parse_and_cache_all_files(&mut self, project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Parsing and caching all Rust files in: {}", project_path);
        
        for entry in WalkDir::new(project_path) {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                // Skip target directory and other build artifacts
                if path.to_string_lossy().contains("/target/") 
                    || path.to_string_lossy().contains("/.git/") {
                    continue;
                }
                
                println!("📄 Parsing file: {}", path.display());
                
                let content = std::fs::read_to_string(path)?;
                match syn::parse_file(&content) {
                    Ok(ast) => {
                        let parsed_file = ParsedFile::new(ast, path.to_path_buf());
                        self.cache.insert(path.to_path_buf(), parsed_file);
                        println!("✅ Successfully parsed: {}", path.display());
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to parse {}: {}", path.display(), e);
                        // Continue processing other files even if one fails
                    }
                }
            }
        }
        
        println!("📊 Cached {} Rust files", self.cache.len());
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
    pub fn keys(&self) -> std::collections::hash_map::Keys<PathBuf, ParsedFile> {
        self.cache.keys()
    }

    /// Get all cached files as an iterator
    pub fn iter(&self) -> std::collections::hash_map::Iter<PathBuf, ParsedFile> {
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
    pub fn parse_and_cache_file(&mut self, file_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(file_path)?;
        let ast = syn::parse_file(&content)?;
        let parsed_file = ParsedFile::new(ast, file_path.to_path_buf());
        self.cache.insert(file_path.to_path_buf(), parsed_file);
        Ok(())
    }
}