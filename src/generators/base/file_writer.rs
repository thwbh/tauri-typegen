use std::fs;
use std::path::Path;

/// Utility for writing generated TypeScript files with consistent patterns
pub struct FileWriter {
    output_path: String,
    generated_files: Vec<String>,
}

impl FileWriter {
    pub fn new(output_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_path)?;
        Ok(Self {
            output_path: output_path.to_string(),
            generated_files: Vec::new(),
        })
    }

    /// Write a TypeScript file with the given content
    pub fn write_typescript_file(
        &mut self,
        filename: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = format!("{}/{}", self.output_path, filename);
        fs::write(&file_path, content)?;
        self.generated_files.push(filename.to_string());
        Ok(())
    }

    /// Write the types.ts file
    pub fn write_types_file(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_typescript_file("types.ts", content)
    }

    /// Write the commands.ts file  
    pub fn write_commands_file(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_typescript_file("commands.ts", content)
    }

    /// Write the index.ts file
    pub fn write_index_file(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_typescript_file("index.ts", content)
    }

    /// Write the schemas.ts file (for zod generator)
    pub fn write_schemas_file(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_typescript_file("schemas.ts", content)
    }

    /// Write the events.ts file
    pub fn write_events_file(&mut self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.write_typescript_file("events.ts", content)
    }

    /// Get the list of generated files
    pub fn get_generated_files(&self) -> &[String] {
        &self.generated_files
    }

    /// Get the output path
    pub fn get_output_path(&self) -> &str {
        &self.output_path
    }

    /// Create directory if it doesn't exist
    pub fn ensure_directory_exists(path: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    /// Check if a file exists in the output directory
    pub fn file_exists(&self, filename: &str) -> bool {
        let file_path = format!("{}/{}", self.output_path, filename);
        Path::new(&file_path).exists()
    }

    /// Delete a file if it exists (useful for cleanup)
    pub fn delete_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = format!("{}/{}", self.output_path, filename);
        if Path::new(&file_path).exists() {
            fs::remove_file(&file_path)?;
        }
        Ok(())
    }

    /// Get the full path to a file in the output directory
    pub fn get_file_path(&self, filename: &str) -> String {
        format!("{}/{}", self.output_path, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Helper to create a unique temp directory for tests
    fn temp_dir() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = format!("./test_output_{}_{}", std::process::id(), timestamp);
        let _ = fs::remove_dir_all(&dir); // Clean up from previous runs
        dir
    }

    // Helper to cleanup temp directory
    fn cleanup_dir(dir: &str) {
        let _ = fs::remove_dir_all(dir);
    }

    mod initialization {
        use super::*;

        #[test]
        fn test_new_creates_directory() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir);
            assert!(writer.is_ok());
            assert!(Path::new(&dir).exists());
            cleanup_dir(&dir);
        }

        #[test]
        fn test_new_with_nested_path() {
            let dir = format!("{}/nested/path", temp_dir());
            let writer = FileWriter::new(&dir);
            assert!(writer.is_ok());
            assert!(Path::new(&dir).exists());
            cleanup_dir(&temp_dir());
        }

        #[test]
        fn test_new_with_existing_directory() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let writer = FileWriter::new(&dir);
            assert!(writer.is_ok());
            cleanup_dir(&dir);
        }
    }

    mod file_writing {
        use super::*;

        #[test]
        fn test_write_typescript_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_typescript_file("test.ts", "export const x = 1;");
            assert!(result.is_ok());

            let file_path = format!("{}/test.ts", dir);
            assert!(Path::new(&file_path).exists());

            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, "export const x = 1;");

            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_types_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_types_file("export type User = { name: string };");
            assert!(result.is_ok());
            assert!(writer.file_exists("types.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_commands_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_commands_file("export const commands = {};");
            assert!(result.is_ok());
            assert!(writer.file_exists("commands.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_index_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_index_file("export * from './types';");
            assert!(result.is_ok());
            assert!(writer.file_exists("index.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_schemas_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_schemas_file("import { z } from 'zod';");
            assert!(result.is_ok());
            assert!(writer.file_exists("schemas.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_events_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_events_file("export const events = {};");
            assert!(result.is_ok());
            assert!(writer.file_exists("events.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_multiple_files() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();

            writer.write_types_file("types").unwrap();
            writer.write_commands_file("commands").unwrap();
            writer.write_index_file("index").unwrap();

            assert_eq!(writer.get_generated_files().len(), 3);
            assert!(writer
                .get_generated_files()
                .contains(&"types.ts".to_string()));
            assert!(writer
                .get_generated_files()
                .contains(&"commands.ts".to_string()));
            assert!(writer
                .get_generated_files()
                .contains(&"index.ts".to_string()));

            cleanup_dir(&dir);
        }
    }

    mod getters {
        use super::*;

        #[test]
        fn test_get_generated_files_empty() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir).unwrap();
            assert!(writer.get_generated_files().is_empty());
            cleanup_dir(&dir);
        }

        #[test]
        fn test_get_generated_files_after_writing() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            writer.write_types_file("content").unwrap();

            let files = writer.get_generated_files();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0], "types.ts");

            cleanup_dir(&dir);
        }

        #[test]
        fn test_get_output_path() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir).unwrap();
            assert_eq!(writer.get_output_path(), dir);
            cleanup_dir(&dir);
        }

        #[test]
        fn test_get_file_path() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir).unwrap();
            let path = writer.get_file_path("test.ts");
            assert_eq!(path, format!("{}/test.ts", dir));
            cleanup_dir(&dir);
        }
    }

    mod file_operations {
        use super::*;

        #[test]
        fn test_file_exists_false() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir).unwrap();
            assert!(!writer.file_exists("nonexistent.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_file_exists_true() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            writer.write_types_file("content").unwrap();
            assert!(writer.file_exists("types.ts"));
            cleanup_dir(&dir);
        }

        #[test]
        fn test_delete_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            writer.write_types_file("content").unwrap();
            assert!(writer.file_exists("types.ts"));

            let result = writer.delete_file("types.ts");
            assert!(result.is_ok());
            assert!(!writer.file_exists("types.ts"));

            cleanup_dir(&dir);
        }

        #[test]
        fn test_delete_nonexistent_file() {
            let dir = temp_dir();
            let writer = FileWriter::new(&dir).unwrap();
            let result = writer.delete_file("nonexistent.ts");
            assert!(result.is_ok()); // Should not error
            cleanup_dir(&dir);
        }

        #[test]
        fn test_ensure_directory_exists() {
            let dir = format!("{}/ensure_test", temp_dir());
            let result = FileWriter::ensure_directory_exists(&dir);
            assert!(result.is_ok());
            assert!(Path::new(&dir).exists());
            cleanup_dir(&temp_dir());
        }

        #[test]
        fn test_ensure_directory_exists_already_exists() {
            let dir = temp_dir();
            fs::create_dir_all(&dir).unwrap();
            let result = FileWriter::ensure_directory_exists(&dir);
            assert!(result.is_ok());
            cleanup_dir(&dir);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_write_empty_content() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();
            let result = writer.write_typescript_file("empty.ts", "");
            assert!(result.is_ok());

            let content = fs::read_to_string(writer.get_file_path("empty.ts")).unwrap();
            assert_eq!(content, "");

            cleanup_dir(&dir);
        }

        #[test]
        fn test_overwrite_existing_file() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();

            writer.write_types_file("first").unwrap();
            writer.write_types_file("second").unwrap();

            let content = fs::read_to_string(writer.get_file_path("types.ts")).unwrap();
            assert_eq!(content, "second");

            // File should only appear once in generated_files list
            assert_eq!(writer.get_generated_files().len(), 2);

            cleanup_dir(&dir);
        }

        #[test]
        fn test_write_large_content() {
            let dir = temp_dir();
            let mut writer = FileWriter::new(&dir).unwrap();

            let large_content = "x".repeat(100_000);
            let result = writer.write_typescript_file("large.ts", &large_content);
            assert!(result.is_ok());

            let content = fs::read_to_string(writer.get_file_path("large.ts")).unwrap();
            assert_eq!(content.len(), 100_000);

            cleanup_dir(&dir);
        }
    }
}
