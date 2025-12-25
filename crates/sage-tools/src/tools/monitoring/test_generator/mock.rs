//! Mock generation

use tracing::debug;

use sage_core::tools::base::ToolError;

use super::types::TestGeneratorTool;

impl TestGeneratorTool {
    /// Generate mock objects
    pub(super) async fn generate_mock(
        &self,
        trait_name: &str,
        language: &str,
    ) -> Result<String, ToolError> {
        debug!("Generating mock for trait: {}", trait_name);

        let mock_code = match language {
            "rust" => format!(
                r#"
use mockall::{{automock, predicate::*}};

#[automock]
pub trait {} {{
    // TODO: Add trait methods that need to be mocked
    fn example_method(&self, param: String) -> Result<String, Error>;
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_with_mock_{}() {{
        // Arrange
        let mut mock = Mock{}.new();
        mock.expect_example_method()
            .with(eq("test_input"))
            .times(1)
            .returning(|_| Ok("mocked_output".to_string()));

        // Act
        let result = mock.example_method("test_input".to_string());

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mocked_output");
    }}
}}
"#,
                trait_name, trait_name, trait_name
            ),
            "typescript" => format!(
                r#"
// Mock for {}
export class Mock{} implements {} {{
    // Mock implementation
    private mockData: any = {{}};

    // Method to set up mock behavior
    setup(method: string, returnValue: any): void {{
        this.mockData[method] = returnValue;
    }}

    // TODO: Implement interface methods with mock behavior
    exampleMethod(param: string): Promise<string> {{
        return Promise.resolve(this.mockData['exampleMethod'] || 'default_mock_value');
    }}
}}

// Test using the mock
describe('{} tests', () => {{
    let mock: Mock{};

    beforeEach(() => {{
        mock = new Mock{}();
    }});

    it('should work with mock', async () => {{
        // Arrange
        mock.setup('exampleMethod', 'mocked_result');

        // Act
        const result = await mock.exampleMethod('test_input');

        // Assert
        expect(result).toBe('mocked_result');
    }});
}});
"#,
                trait_name, trait_name, trait_name, trait_name, trait_name, trait_name
            ),
            _ => format!(
                "// Mock for {} in {}\n// TODO: Implement mock for your specific language",
                trait_name, language
            ),
        };

        Ok(format!(
            "Generated mock for trait '{}':\n{}",
            trait_name, mock_code
        ))
    }
}
