//! Regression test suite - Backward compatibility tests

// Shared modules
#[path = "common/mod.rs"]
mod common;
#[path = "fixtures/mod.rs"]
mod fixtures;

// Regression test modules
#[path = "regression/test_backward_compat.rs"]
mod test_backward_compat;
