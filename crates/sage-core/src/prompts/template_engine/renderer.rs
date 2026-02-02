//! Template renderer
//!
//! Renders parsed templates with variable substitution and function execution.

use super::functions::BuiltinFunctions;
use super::parser::TemplateParser;
use super::types::*;
use crate::prompts::PromptVariables;
use std::collections::HashMap;

/// Template renderer that processes templates with variables
pub struct EnhancedRenderer {
    /// Additional context values beyond PromptVariables
    context: HashMap<String, Value>,
}

impl EnhancedRenderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
        }
    }

    /// Add a context value
    pub fn with_value(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Add multiple context values
    pub fn with_values(mut self, values: HashMap<String, Value>) -> Self {
        self.context.extend(values);
        self
    }

    /// Render a template string with the given variables
    pub fn render(&self, template: &str, vars: &PromptVariables) -> String {
        let parsed = TemplateParser::parse(template);
        self.render_template(&parsed, vars)
    }

    /// Render a parsed template
    fn render_template(&self, template: &Template, vars: &PromptVariables) -> String {
        let mut result = String::new();
        for node in &template.nodes {
            result.push_str(&self.render_node(node, vars));
        }
        result
    }

    /// Render a single template node
    fn render_node(&self, node: &TemplateNode, vars: &PromptVariables) -> String {
        match node {
            TemplateNode::Text(text) => text.clone(),
            TemplateNode::Variable(var_ref) => self.render_variable(var_ref, vars),
            TemplateNode::Conditional(cond) => self.render_conditional(cond, vars),
            TemplateNode::FunctionCall(func) => self.render_function(func, vars),
        }
    }

    /// Render a variable reference
    fn render_variable(&self, var_ref: &VariableRef, vars: &PromptVariables) -> String {
        // First check context for complex values
        if let Some(value) = self.context.get(&var_ref.name) {
            return self.resolve_value_path(value, &var_ref.path);
        }

        // Then check PromptVariables
        if var_ref.path.is_empty() {
            // Simple variable
            vars.get(&var_ref.name)
                .map(|s| s.to_string())
                .or_else(|| vars.custom.get(&var_ref.name).cloned())
                .unwrap_or_default()
        } else {
            // Property path - check custom variables with full path
            let full_path = var_ref.full_path();
            vars.custom.get(&full_path).cloned().unwrap_or_default()
        }
    }

    /// Resolve a value through a property path
    fn resolve_value_path(&self, value: &Value, path: &[String]) -> String {
        if path.is_empty() {
            return value.to_string_value();
        }

        let mut current = value.clone();
        for key in path {
            match current {
                Value::Object(ref obj) => {
                    if let Some(v) = obj.get(key) {
                        current = v.clone();
                    } else {
                        return String::new();
                    }
                }
                _ => return String::new(),
            }
        }
        current.to_string_value()
    }

    /// Render a conditional expression
    fn render_conditional(&self, cond: &ConditionalExpr, vars: &PromptVariables) -> String {
        let condition_met = self.evaluate_condition(&cond.condition, vars);

        let branch = if condition_met {
            &cond.true_branch
        } else {
            &cond.false_branch
        };

        let mut result = String::new();
        for node in branch {
            result.push_str(&self.render_node(node, vars));
        }
        result
    }

    /// Evaluate a condition
    fn evaluate_condition(&self, condition: &ConditionType, vars: &PromptVariables) -> bool {
        match condition {
            ConditionType::HasTool(tool) => {
                // Check various tool name formats
                vars.has_tool(tool)
                    || vars.has_tool(&tool.to_uppercase())
                    || vars.has_tool(&Self::tool_name_from_const(tool))
            }
            ConditionType::BoolVar(name) => match name.as_str() {
                "IS_GIT_REPO" => vars.is_git_repo,
                "IN_PLAN_MODE" => vars.in_plan_mode,
                "PLAN_EXISTS" => vars.plan_exists,
                _ => {
                    // Check custom variables
                    vars.custom
                        .get(name)
                        .map(|v| v == "true" || v == "1")
                        .unwrap_or(false)
                }
            },
            ConditionType::NonEmpty(var_ref) => {
                let value = self.render_variable(var_ref, vars);
                !value.is_empty()
            }
            ConditionType::Custom(expr) => {
                // For custom conditions, check if it's a truthy value
                vars.custom
                    .get(expr)
                    .map(|v| !v.is_empty() && v != "false" && v != "0")
                    .unwrap_or(false)
            }
        }
    }

    /// Convert tool constant name to actual tool name
    fn tool_name_from_const(const_name: &str) -> String {
        match const_name {
            "BASH" => "Bash".to_string(),
            "READ" => "Read".to_string(),
            "EDIT" => "Edit".to_string(),
            "WRITE" => "Write".to_string(),
            "GLOB" => "Glob".to_string(),
            "GREP" => "Grep".to_string(),
            "TASK" => "Task".to_string(),
            "TODOWRITE" => "TodoWrite".to_string(),
            "ASKUSERQUESTION" => "AskUserQuestion".to_string(),
            "WEBFETCH" => "WebFetch".to_string(),
            "WEBSEARCH" => "WebSearch".to_string(),
            "ENTERPLANMODE" => "EnterPlanMode".to_string(),
            "EXITPLANMODE" => "ExitPlanMode".to_string(),
            _ => const_name.to_string(),
        }
    }

    /// Render a function call
    fn render_function(&self, func: &FunctionCall, vars: &PromptVariables) -> String {
        // Evaluate arguments
        let args: Vec<Value> = func
            .args
            .iter()
            .map(|arg| self.evaluate_arg(arg, vars))
            .collect();

        // Execute the function
        let result = BuiltinFunctions::execute(&func.name, &args, &self.context);

        // Apply method chain if present
        if let Some(ref chain) = func.chain {
            if let Some(value) = result {
                return self.apply_method_chain(&value, chain, vars);
            }
        }

        result.map(|v| v.to_string_value()).unwrap_or_default()
    }

    /// Evaluate a function argument
    fn evaluate_arg(&self, arg: &FunctionArg, vars: &PromptVariables) -> Value {
        match arg {
            FunctionArg::String(s) => Value::String(s.clone()),
            FunctionArg::Number(n) => Value::Number(*n),
            FunctionArg::Bool(b) => Value::Bool(*b),
            FunctionArg::Variable(var_ref) => {
                let s = self.render_variable(var_ref, vars);
                Value::String(s)
            }
            FunctionArg::Lambda(_) => {
                // Lambdas are handled specially in method chains
                Value::Null
            }
        }
    }

    /// Apply a method chain to a value
    fn apply_method_chain(
        &self,
        value: &Value,
        chain: &MethodChain,
        vars: &PromptVariables,
    ) -> String {
        let result = match chain.method.as_str() {
            "map" => self.apply_map(value, &chain.args, vars),
            "filter" => self.apply_filter(value, &chain.args, vars),
            "join" => self.apply_join(value, &chain.args),
            "length" => Some(Value::Number(
                value.as_array().map(|a| a.len()).unwrap_or(0) as f64,
            )),
            "first" => value.as_array().and_then(|a| a.first().cloned()),
            "last" => value.as_array().and_then(|a| a.last().cloned()),
            _ => None,
        };

        if let Some(ref next) = chain.next {
            if let Some(ref val) = result {
                return self.apply_method_chain(val, next, vars);
            }
        }

        result.map(|v| v.to_string_value()).unwrap_or_default()
    }

    /// Apply map operation
    fn apply_map(
        &self,
        value: &Value,
        args: &[FunctionArg],
        _vars: &PromptVariables,
    ) -> Option<Value> {
        let arr = value.as_array()?;
        let lambda = args.first().and_then(|a| match a {
            FunctionArg::Lambda(l) => Some(l),
            _ => None,
        })?;

        let mapped: Vec<Value> = arr
            .iter()
            .map(|item| {
                // Simple property access from lambda body
                if let TemplateNode::Variable(var_ref) = lambda.body.as_ref() {
                    if var_ref.name == lambda.param && !var_ref.path.is_empty() {
                        // Access property on item
                        if let Value::Object(obj) = item {
                            if let Some(prop) = var_ref.path.first() {
                                return obj.get(prop).cloned().unwrap_or(Value::Null);
                            }
                        }
                    }
                }
                item.clone()
            })
            .collect();

        Some(Value::Array(mapped))
    }

    /// Apply filter operation
    fn apply_filter(
        &self,
        value: &Value,
        args: &[FunctionArg],
        _vars: &PromptVariables,
    ) -> Option<Value> {
        let arr = value.as_array()?;
        let lambda = args.first().and_then(|a| match a {
            FunctionArg::Lambda(l) => Some(l),
            _ => None,
        })?;

        let filtered: Vec<Value> = arr
            .iter()
            .filter(|item| {
                // Simple property check from lambda body
                if let TemplateNode::Variable(var_ref) = lambda.body.as_ref() {
                    if var_ref.name == lambda.param && !var_ref.path.is_empty() {
                        if let Value::Object(obj) = item {
                            if let Some(prop) = var_ref.path.first() {
                                if let Some(val) = obj.get(prop) {
                                    return val.is_truthy();
                                }
                            }
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();

        Some(Value::Array(filtered))
    }

    /// Apply join operation
    fn apply_join(&self, value: &Value, args: &[FunctionArg]) -> Option<Value> {
        let arr = value.as_array()?;
        let separator = args
            .first()
            .and_then(|a| match a {
                FunctionArg::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or(", ");

        let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
        Some(Value::String(items.join(separator)))
    }
}

impl Default for EnhancedRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function for simple rendering
pub fn render(template: &str, vars: &PromptVariables) -> String {
    EnhancedRenderer::new().render(template, vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable() {
        let vars = PromptVariables::new();
        let result = render("Hello, ${AGENT_NAME}!", &vars);
        assert_eq!(result, "Hello, Sage Agent!");
    }

    #[test]
    fn test_multiple_variables() {
        let vars = PromptVariables::new();
        let result = render("Use ${READ_TOOL_NAME} and ${EDIT_TOOL_NAME}.", &vars);
        assert_eq!(result, "Use Read and Edit.");
    }

    #[test]
    fn test_conditional_true() {
        let mut vars = PromptVariables::new();
        vars.is_git_repo = true;
        let result = render("${IS_GIT_REPO?`Yes`:`No`}", &vars);
        assert_eq!(result, "Yes");
    }

    #[test]
    fn test_conditional_false() {
        let mut vars = PromptVariables::new();
        vars.is_git_repo = false;
        let result = render("${IS_GIT_REPO?`Yes`:`No`}", &vars);
        assert_eq!(result, "No");
    }

    #[test]
    fn test_has_tool_conditional() {
        let mut vars = PromptVariables::new();
        vars.add_tool("Bash");
        let result = render("${HAS_TOOL_BASH?`bash available`:`no bash`}", &vars);
        assert_eq!(result, "bash available");
    }

    #[test]
    fn test_nested_variable_in_conditional() {
        let mut vars = PromptVariables::new();
        vars.is_git_repo = true;
        vars.git_branch = "main".to_string();
        let result = render("${IS_GIT_REPO?`Branch: ${GIT_BRANCH}`:`No git`}", &vars);
        assert_eq!(result, "Branch: main");
    }

    #[test]
    fn test_custom_variable() {
        let mut vars = PromptVariables::new();
        vars.set("CUSTOM", "custom_value");
        let result = render("Custom: ${CUSTOM}", &vars);
        assert_eq!(result, "Custom: custom_value");
    }

    #[test]
    fn test_context_value() {
        let vars = PromptVariables::new();
        let renderer = EnhancedRenderer::new().with_value("NAME", "World");
        let result = renderer.render("Hello, ${NAME}!", &vars);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_function_call() {
        let vars = PromptVariables::new();
        let result = render("${str.uppercase('hello')}", &vars);
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_empty_template() {
        let vars = PromptVariables::new();
        let result = render("", &vars);
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_variables() {
        let vars = PromptVariables::new();
        let result = render("Plain text without variables", &vars);
        assert_eq!(result, "Plain text without variables");
    }

    #[test]
    fn test_plan_mode_conditional() {
        let mut vars = PromptVariables::new();
        vars.in_plan_mode = true;
        let result = render("${IN_PLAN_MODE?`Planning`:`Executing`}", &vars);
        assert_eq!(result, "Planning");
    }
}
