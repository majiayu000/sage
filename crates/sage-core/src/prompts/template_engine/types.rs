//! Enhanced template engine types
//!
//! AST types for the template parser supporting:
//! - Simple variables: ${VAR_NAME}
//! - Object property access: ${obj.property}
//! - Conditional expressions: ${COND?`true`:`false`}
//! - Function calls: ${fn(arg)}
//! - Method chains: ${arr.map(x => x.name).join(', ')}

use std::fmt;

/// A parsed template consisting of nodes
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub nodes: Vec<TemplateNode>,
}

impl Template {
    pub fn new(nodes: Vec<TemplateNode>) -> Self {
        Self { nodes }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// A single node in the template AST
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateNode {
    /// Plain text content
    Text(String),
    /// Variable reference: ${VAR} or ${obj.prop}
    Variable(VariableRef),
    /// Conditional expression: ${COND?`true`:`false`}
    Conditional(ConditionalExpr),
    /// Function call: ${fn(args)}
    FunctionCall(FunctionCall),
}

/// A variable reference with optional property path
#[derive(Debug, Clone, PartialEq)]
pub struct VariableRef {
    /// The root variable name
    pub name: String,
    /// Property path for nested access (e.g., ["prop", "nested"])
    pub path: Vec<String>,
}

impl VariableRef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: Vec::new(),
        }
    }

    pub fn with_path(name: impl Into<String>, path: Vec<String>) -> Self {
        Self {
            name: name.into(),
            path,
        }
    }

    /// Get the full path as a string (e.g., "obj.prop.nested")
    pub fn full_path(&self) -> String {
        if self.path.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.name, self.path.join("."))
        }
    }
}

impl fmt::Display for VariableRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${{{}}}", self.full_path())
    }
}

/// A conditional expression with true/false branches
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalExpr {
    /// The condition to evaluate (variable name or expression)
    pub condition: ConditionType,
    /// Content to render if condition is true
    pub true_branch: Vec<TemplateNode>,
    /// Content to render if condition is false
    pub false_branch: Vec<TemplateNode>,
}

/// Types of conditions supported
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Check if a tool is available: HAS_TOOL_BASH
    HasTool(String),
    /// Check a boolean variable: IS_GIT_REPO, IN_PLAN_MODE
    BoolVar(String),
    /// Check if variable is non-empty
    NonEmpty(VariableRef),
    /// Custom condition expression
    Custom(String),
}

impl ConditionType {
    /// Parse a condition string into a ConditionType
    pub fn parse(s: &str) -> Self {
        if let Some(tool) = s.strip_prefix("HAS_TOOL_") {
            ConditionType::HasTool(tool.to_string())
        } else {
            match s {
                "IS_GIT_REPO" | "IN_PLAN_MODE" | "PLAN_EXISTS" => {
                    ConditionType::BoolVar(s.to_string())
                }
                _ => ConditionType::Custom(s.to_string()),
            }
        }
    }
}

/// A function call expression
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Function name (e.g., "date.format", "str.uppercase")
    pub name: String,
    /// Function arguments
    pub args: Vec<FunctionArg>,
    /// Optional method chain
    pub chain: Option<Box<MethodChain>>,
}

/// A function argument
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionArg {
    /// String literal: "value"
    String(String),
    /// Variable reference: VAR_NAME
    Variable(VariableRef),
    /// Number literal
    Number(f64),
    /// Boolean literal
    Bool(bool),
    /// Lambda expression: x => x.name
    Lambda(LambdaExpr),
}

/// A lambda expression for array operations
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaExpr {
    /// Parameter name (e.g., "x", "t")
    pub param: String,
    /// Body expression (e.g., "x.name")
    pub body: Box<TemplateNode>,
}

/// A method chain for fluent operations
#[derive(Debug, Clone, PartialEq)]
pub struct MethodChain {
    /// The method name (e.g., "map", "filter", "join")
    pub method: String,
    /// Method arguments
    pub args: Vec<FunctionArg>,
    /// Next method in chain
    pub next: Option<Box<MethodChain>>,
}

/// Value types that can be stored and manipulated
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
    Object(std::collections::HashMap<String, Value>),
    Null,
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            Value::Null => false,
        }
    }

    pub fn to_string_value(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
                items.join(", ")
            }
            Value::Object(_) => "[object]".to_string(),
            Value::Null => "".to_string(),
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_ref_simple() {
        let var = VariableRef::new("AGENT_NAME");
        assert_eq!(var.full_path(), "AGENT_NAME");
        assert_eq!(var.to_string(), "${AGENT_NAME}");
    }

    #[test]
    fn test_variable_ref_with_path() {
        let var = VariableRef::with_path("config", vec!["model".to_string(), "name".to_string()]);
        assert_eq!(var.full_path(), "config.model.name");
        assert_eq!(var.to_string(), "${config.model.name}");
    }

    #[test]
    fn test_condition_type_parse() {
        assert_eq!(
            ConditionType::parse("HAS_TOOL_BASH"),
            ConditionType::HasTool("BASH".to_string())
        );
        assert_eq!(
            ConditionType::parse("IS_GIT_REPO"),
            ConditionType::BoolVar("IS_GIT_REPO".to_string())
        );
        assert_eq!(
            ConditionType::parse("CUSTOM_COND"),
            ConditionType::Custom("CUSTOM_COND".to_string())
        );
    }

    #[test]
    fn test_value_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(!Value::Null.is_truthy());
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(Value::String("hello".to_string()).to_string_value(), "hello");
        assert_eq!(Value::Number(42.0).to_string_value(), "42");
        assert_eq!(Value::Bool(true).to_string_value(), "true");
        assert_eq!(Value::Null.to_string_value(), "");
    }
}
