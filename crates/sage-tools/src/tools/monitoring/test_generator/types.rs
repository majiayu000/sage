//! Types for the test generator tool

/// Test generator tool
#[derive(Debug, Clone)]
pub struct TestGeneratorTool {
    pub(super) name: String,
    pub(super) description: String,
}

impl TestGeneratorTool {
    /// Create a new test generator tool
    pub fn new() -> Self {
        Self {
            name: "test_generator".to_string(),
            description:
                "Test generation tool for creating unit tests, integration tests, and mocks"
                    .to_string(),
        }
    }
}

impl Default for TestGeneratorTool {
    fn default() -> Self {
        Self::new()
    }
}
