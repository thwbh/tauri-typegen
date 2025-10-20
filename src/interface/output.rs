use indicatif::{ProgressBar, ProgressStyle};
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Verbose,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Verbose => write!(f, "VERBOSE"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    verbose: bool,
    debug: bool,
}

impl Logger {
    pub fn new(verbose: bool, debug: bool) -> Self {
        Self { verbose, debug }
    }

    pub fn should_log(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Error | LogLevel::Warning | LogLevel::Info => true,
            LogLevel::Debug => self.debug || self.verbose,
            LogLevel::Verbose => self.verbose,
        }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        if self.should_log(level) {
            let icon = match level {
                LogLevel::Error => "❌",
                LogLevel::Warning => "⚠️",
                LogLevel::Info => "",
                LogLevel::Debug => "🔍",
                LogLevel::Verbose => "💬",
            };
            if icon.is_empty() {
                println!("{}", message);
            } else {
                println!("{} {}", icon, message);
            }
        }
    }

    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    pub fn warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn verbose(&self, message: &str) {
        self.log(LogLevel::Verbose, message);
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

pub struct ProgressReporter {
    logger: Logger,
    progress_bar: Option<ProgressBar>,
    current_step: usize,
    total_steps: usize,
    step_name: String,
}

impl ProgressReporter {
    pub fn new(logger: Logger, total_steps: usize) -> Self {
        let progress_bar = if !logger.is_verbose() {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            Some(pb)
        } else {
            None
        };

        Self {
            logger,
            progress_bar,
            current_step: 0,
            total_steps,
            step_name: String::new(),
        }
    }

    pub fn start_step(&mut self, step_name: &str) {
        self.current_step += 1;
        self.step_name = step_name.to_string();

        if self.logger.is_verbose() {
            // Verbose mode: use old-style logging
            let progress = if self.total_steps > 0 {
                format!(" ({}/{})", self.current_step, self.total_steps)
            } else {
                String::new()
            };
            self.logger.info(&format!("🚀 {}{}", step_name, progress));
        } else {
            // Non-verbose mode: update the single progress bar
            if let Some(ref pb) = self.progress_bar {
                pb.set_message(format!(
                    "{} ({}/{})",
                    step_name, self.current_step, self.total_steps
                ));
            }
        }
    }

    pub fn complete_step(&mut self, message: Option<&str>) {
        if self.logger.is_verbose() {
            // Verbose mode: use old-style logging
            if let Some(msg) = message {
                self.logger
                    .info(&format!("✅ {} - {}", self.step_name, msg));
            } else {
                self.logger.info(&format!("✅ {}", self.step_name));
            }
        }
        // In non-verbose mode, we just continue to the next step (no need to "complete")
    }

    pub fn fail_step(&mut self, error: &str) {
        if let Some(ref pb) = self.progress_bar {
            pb.finish_with_message(format!("✗ {} - {}", self.step_name, error));
        }
        self.logger
            .error(&format!("Failed {}: {}", self.step_name, error));
    }

    pub fn update_progress(&self, message: &str) {
        // Only log in verbose mode
        self.logger.verbose(message);
    }

    pub fn finish(&self, total_message: &str) {
        if let Some(ref pb) = self.progress_bar {
            pb.finish_and_clear();
        }
        println!("✓ {}", total_message);
    }
}

impl Drop for ProgressReporter {
    fn drop(&mut self) {
        // Ensure progress bar is cleared when reporter is dropped
        if let Some(ref pb) = self.progress_bar {
            pb.finish_and_clear();
        }
    }
}

pub fn print_usage_info(output_path: &str, generated_files: &[String], command_count: usize) {
    println!(
        "\n✓ Generated TypeScript bindings for {} command{}",
        command_count,
        if command_count == 1 { "" } else { "s" }
    );
    println!("📁 Location: {}", output_path);

    println!("\n💡 Import in your frontend:");
    for file in generated_files {
        if file.ends_with("index.ts") || file.ends_with("index.js") {
            println!(
                "  import {{ /* commands */ }} from '{}'",
                output_path.trim_end_matches('/')
            );
            break;
        }
    }
}

pub fn print_dependency_visualization_info(output_path: &str) {
    println!("\n🌐 Dependency visualization generated:");
    println!("  📄 {}/dependency-graph.txt", output_path);
    println!("  📄 {}/dependency-graph.dot", output_path);
    println!(
        "\n💡 To generate a visual graph: dot -Tpng {}/dependency-graph.dot -o graph.png",
        output_path
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_verbose_mode() {
        let logger = Logger::new(true, false);
        assert!(logger.should_log(LogLevel::Verbose));
        assert!(logger.should_log(LogLevel::Info));
        assert!(logger.should_log(LogLevel::Error));
        assert!(logger.should_log(LogLevel::Debug)); // Verbose enables debug
    }

    #[test]
    fn test_logger_normal_mode() {
        let logger = Logger::new(false, false);
        assert!(!logger.should_log(LogLevel::Verbose));
        assert!(!logger.should_log(LogLevel::Debug));
        assert!(logger.should_log(LogLevel::Info));
        assert!(logger.should_log(LogLevel::Error));
    }

    #[test]
    fn test_logger_debug_mode() {
        let logger = Logger::new(false, true);
        assert!(!logger.should_log(LogLevel::Verbose));
        assert!(logger.should_log(LogLevel::Debug));
        assert!(logger.should_log(LogLevel::Info));
    }

    #[test]
    fn test_progress_reporter() {
        let logger = Logger::new(false, false);
        let mut reporter = ProgressReporter::new(logger, 3);

        // Test step progression
        assert_eq!(reporter.current_step, 0);

        reporter.start_step("First Step");
        assert_eq!(reporter.current_step, 1);
        assert_eq!(reporter.step_name, "First Step");

        reporter.start_step("Second Step");
        assert_eq!(reporter.current_step, 2);
        assert_eq!(reporter.step_name, "Second Step");
    }
}
