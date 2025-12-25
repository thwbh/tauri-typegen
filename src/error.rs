use serde::{ser::Serializer, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Command analysis failed: {0}")]
    CommandAnalysis(String),

    #[error("Code generation failed: {0}")]
    CodeGeneration(String),

    #[error("Invalid project path: {0}")]
    InvalidProjectPath(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    mod error_variants {
        use super::*;

        #[test]
        fn test_io_error_creation() {
            let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
            let err = Error::from(io_err);
            assert!(matches!(err, Error::Io(_)));
            assert!(err.to_string().contains("file not found"));
        }

        #[test]
        fn test_command_analysis_error() {
            let err = Error::CommandAnalysis("failed to parse command".to_string());
            assert!(matches!(err, Error::CommandAnalysis(_)));
            assert_eq!(
                err.to_string(),
                "Command analysis failed: failed to parse command"
            );
        }

        #[test]
        fn test_code_generation_error() {
            let err = Error::CodeGeneration("template rendering failed".to_string());
            assert!(matches!(err, Error::CodeGeneration(_)));
            assert_eq!(
                err.to_string(),
                "Code generation failed: template rendering failed"
            );
        }

        #[test]
        fn test_invalid_project_path_error() {
            let err = Error::InvalidProjectPath("/invalid/path".to_string());
            assert!(matches!(err, Error::InvalidProjectPath(_)));
            assert_eq!(err.to_string(), "Invalid project path: /invalid/path");
        }
    }

    mod error_display {
        use super::*;

        #[test]
        fn test_command_analysis_display() {
            let err = Error::CommandAnalysis("test error".to_string());
            let display = format!("{}", err);
            assert!(display.contains("Command analysis failed"));
            assert!(display.contains("test error"));
        }

        #[test]
        fn test_code_generation_display() {
            let err = Error::CodeGeneration("test error".to_string());
            let display = format!("{}", err);
            assert!(display.contains("Code generation failed"));
            assert!(display.contains("test error"));
        }

        #[test]
        fn test_invalid_project_path_display() {
            let err = Error::InvalidProjectPath("test path".to_string());
            let display = format!("{}", err);
            assert!(display.contains("Invalid project path"));
            assert!(display.contains("test path"));
        }

        #[test]
        fn test_debug_format() {
            let err = Error::CommandAnalysis("test".to_string());
            let debug = format!("{:?}", err);
            assert!(debug.contains("CommandAnalysis"));
        }
    }

    mod serialization {
        use super::*;

        #[test]
        fn test_serialize_command_analysis_error() {
            let err = Error::CommandAnalysis("test error".to_string());
            let serialized = serde_json::to_string(&err).unwrap();
            assert!(serialized.contains("Command analysis failed"));
            assert!(serialized.contains("test error"));
        }

        #[test]
        fn test_serialize_code_generation_error() {
            let err = Error::CodeGeneration("generation failed".to_string());
            let serialized = serde_json::to_string(&err).unwrap();
            assert!(serialized.contains("Code generation failed"));
            assert!(serialized.contains("generation failed"));
        }

        #[test]
        fn test_serialize_invalid_project_path_error() {
            let err = Error::InvalidProjectPath("/bad/path".to_string());
            let serialized = serde_json::to_string(&err).unwrap();
            assert!(serialized.contains("Invalid project path"));
            assert!(serialized.contains("/bad/path"));
        }

        #[test]
        fn test_serialize_io_error() {
            let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
            let err = Error::from(io_err);
            let serialized = serde_json::to_string(&err).unwrap();
            assert!(serialized.contains("not found"));
        }
    }

    mod result_type {
        use super::*;

        #[test]
        fn test_result_err() {
            let result: Result<i32> = Err(Error::CommandAnalysis("test".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn test_result_with_question_mark() {
            fn test_fn() -> Result<String> {
                let err = Error::CommandAnalysis("test".to_string());
                Err(err)?;
                Ok("success".to_string())
            }

            let result = test_fn();
            assert!(result.is_err());
        }
    }

    mod from_conversions {
        use super::*;

        #[test]
        fn test_from_io_error() {
            let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
            let err: Error = io_err.into();
            assert!(matches!(err, Error::Io(_)));
        }

        #[test]
        fn test_io_error_kind_preserved() {
            let io_err = io::Error::new(io::ErrorKind::NotFound, "missing");
            let err = Error::from(io_err);
            if let Error::Io(inner) = err {
                assert_eq!(inner.kind(), io::ErrorKind::NotFound);
            } else {
                panic!("Expected Io error variant");
            }
        }
    }
}
