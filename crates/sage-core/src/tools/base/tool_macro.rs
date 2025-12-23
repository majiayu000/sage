//! Macro helpers for tool implementation

/// Macro to help implement the Tool trait.
///
/// This macro generates boilerplate implementations for:
/// - `new()` constructor
/// - `Default` trait
/// - Basic `Tool` trait methods (`name()` and `description()`)
///
/// You still need to implement:
/// - `schema()` - Define parameter schema
/// - `execute()` - Implement tool logic
/// - Optional trait methods (validation, permissions, etc.)
///
/// # Examples
///
/// ```ignore
/// use sage_core::impl_tool;
/// use sage_core::tools::Tool;
///
/// struct HelloTool;
///
/// // This macro implements new(), Default, and basic Tool trait (name, description)
/// impl_tool!(HelloTool, "hello", "Says hello");
///
/// // NOTE: You still need to implement schema() and execute() for Tool trait
/// // The macro only provides the name() and description() methods.
///
/// let tool = HelloTool::new();
/// assert_eq!(tool.name(), "hello");
/// assert_eq!(tool.description(), "Says hello");
/// ```
#[macro_export]
macro_rules! impl_tool {
    ($tool_type:ty, $name:expr, $description:expr) => {
        impl $tool_type {
            pub fn new() -> Self {
                Self {}
            }
        }

        impl Default for $tool_type {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait::async_trait]
        impl $crate::tools::Tool for $tool_type {
            fn name(&self) -> &str {
                $name
            }

            fn description(&self) -> &str {
                $description
            }
        }
    };
}
