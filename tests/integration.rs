//! Integration test suite - End-to-end workflow tests

// Shared modules
#[path = "common/mod.rs"]
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

// Integration test modules
#[path = "integration/test_basic_generation.rs"]
mod test_basic_generation;
#[path = "integration/test_channels.rs"]
mod test_channels;
#[path = "integration/test_events.rs"]
mod test_events;
#[path = "integration/test_full_pipeline.rs"]
mod test_full_pipeline;
#[path = "integration/test_serde_support.rs"]
mod test_serde_support;
#[path = "integration/test_type_conversion.rs"]
mod test_type_conversion;
#[path = "integration/test_validation_libraries.rs"]
mod test_validation_libraries;
