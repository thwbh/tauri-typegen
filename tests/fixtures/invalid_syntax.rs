// File with intentional syntax errors for testing error handling
use serde::{Deserialize, Serialize};

#[command]
pub fn broken_function( {
    // Missing parameter and body
}