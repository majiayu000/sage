//! Enhanced template engine
//!
//! A powerful template engine supporting:
//! - Simple variables: `${VAR_NAME}`
//! - Object property access: `${obj.property}`
//! - Conditional expressions: `${COND?`true`:`false`}`
//! - Function calls: `${fn(arg)}`
//! - Method chains: `${arr.map(x => x.name).join(', ')}`
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::template_engine::{render, EnhancedRenderer};
//! use sage_core::prompts::PromptVariables;
//!
//! let vars = PromptVariables::new();
//!
//! // Simple rendering
//! let result = render("Hello, ${AGENT_NAME}!", &vars);
//!
//! // With additional context
//! let renderer = EnhancedRenderer::new()
//!     .with_value("custom_key", "custom_value");
//! let result = renderer.render("${custom_key}", &vars);
//! ```

mod arguments;
mod expressions;
mod functions;
mod parser;
mod renderer;
mod types;

pub use functions::BuiltinFunctions;
pub use parser::TemplateParser;
pub use renderer::{render, EnhancedRenderer};
pub use types::{
    ConditionType, ConditionalExpr, FunctionArg, FunctionCall, LambdaExpr, MethodChain, Template,
    TemplateNode, Value, VariableRef,
};
