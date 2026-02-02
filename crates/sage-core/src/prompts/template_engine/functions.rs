//! Built-in template functions
//!
//! Provides built-in functions for template rendering:
//! - date.format(pattern)
//! - str.uppercase(s), str.lowercase(s), str.capitalize(s)
//! - arr.join(separator), arr.length()

use super::types::Value;
use std::collections::HashMap;

/// Built-in function registry
pub struct BuiltinFunctions;

impl BuiltinFunctions {
    /// Execute a built-in function
    pub fn execute(name: &str, args: &[Value], context: &HashMap<String, Value>) -> Option<Value> {
        match name {
            // Date functions
            "date.format" => Self::date_format(args),
            "date.now" => Self::date_now(),
            "date.year" => Self::date_year(),

            // String functions
            "str.uppercase" => Self::str_uppercase(args),
            "str.lowercase" => Self::str_lowercase(args),
            "str.capitalize" => Self::str_capitalize(args),
            "str.trim" => Self::str_trim(args),
            "str.length" => Self::str_length(args),
            "str.replace" => Self::str_replace(args),
            "str.contains" => Self::str_contains(args),
            "str.startsWith" => Self::str_starts_with(args),
            "str.endsWith" => Self::str_ends_with(args),

            // Array functions (these work on context values)
            "arr.join" => Self::arr_join(args, context),
            "arr.length" => Self::arr_length(args, context),
            "arr.first" => Self::arr_first(args, context),
            "arr.last" => Self::arr_last(args, context),

            // Conditional functions
            "if" => Self::if_fn(args),
            "default" => Self::default_fn(args),
            "coalesce" => Self::coalesce_fn(args),

            _ => None,
        }
    }

    // Date functions

    fn date_format(args: &[Value]) -> Option<Value> {
        let pattern = args.first()?.as_str()?;
        let now = chrono::Local::now();

        // Simple pattern replacement
        let result = pattern
            .replace("YYYY", &now.format("%Y").to_string())
            .replace("MM", &now.format("%m").to_string())
            .replace("DD", &now.format("%d").to_string())
            .replace("HH", &now.format("%H").to_string())
            .replace("mm", &now.format("%M").to_string())
            .replace("ss", &now.format("%S").to_string());

        Some(Value::String(result))
    }

    fn date_now() -> Option<Value> {
        Some(Value::String(
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        ))
    }

    fn date_year() -> Option<Value> {
        Some(Value::String(
            chrono::Local::now().format("%Y").to_string(),
        ))
    }

    // String functions

    fn str_uppercase(args: &[Value]) -> Option<Value> {
        let s = args.first()?.as_str()?;
        Some(Value::String(s.to_uppercase()))
    }

    fn str_lowercase(args: &[Value]) -> Option<Value> {
        let s = args.first()?.as_str()?;
        Some(Value::String(s.to_lowercase()))
    }

    fn str_capitalize(args: &[Value]) -> Option<Value> {
        let s = args.first()?.as_str()?;
        let mut chars = s.chars();
        let result = match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        };
        Some(Value::String(result))
    }

    fn str_trim(args: &[Value]) -> Option<Value> {
        let s = args.first()?.as_str()?;
        Some(Value::String(s.trim().to_string()))
    }

    fn str_length(args: &[Value]) -> Option<Value> {
        let s = args.first()?.as_str()?;
        Some(Value::Number(s.len() as f64))
    }

    fn str_replace(args: &[Value]) -> Option<Value> {
        if args.len() < 3 {
            return None;
        }
        let s = args[0].as_str()?;
        let from = args[1].as_str()?;
        let to = args[2].as_str()?;
        Some(Value::String(s.replace(from, to)))
    }

    fn str_contains(args: &[Value]) -> Option<Value> {
        if args.len() < 2 {
            return None;
        }
        let s = args[0].as_str()?;
        let needle = args[1].as_str()?;
        Some(Value::Bool(s.contains(needle)))
    }

    fn str_starts_with(args: &[Value]) -> Option<Value> {
        if args.len() < 2 {
            return None;
        }
        let s = args[0].as_str()?;
        let prefix = args[1].as_str()?;
        Some(Value::Bool(s.starts_with(prefix)))
    }

    fn str_ends_with(args: &[Value]) -> Option<Value> {
        if args.len() < 2 {
            return None;
        }
        let s = args[0].as_str()?;
        let suffix = args[1].as_str()?;
        Some(Value::Bool(s.ends_with(suffix)))
    }

    // Array functions

    fn arr_join(args: &[Value], _context: &HashMap<String, Value>) -> Option<Value> {
        if args.is_empty() {
            return None;
        }

        // First arg should be an array, second is separator
        let arr = args[0].as_array()?;
        let separator = if args.len() > 1 {
            args[1].as_str().unwrap_or(", ")
        } else {
            ", "
        };

        let items: Vec<String> = arr.iter().map(|v| v.to_string_value()).collect();
        Some(Value::String(items.join(separator)))
    }

    fn arr_length(args: &[Value], _context: &HashMap<String, Value>) -> Option<Value> {
        let arr = args.first()?.as_array()?;
        Some(Value::Number(arr.len() as f64))
    }

    fn arr_first(args: &[Value], _context: &HashMap<String, Value>) -> Option<Value> {
        let arr = args.first()?.as_array()?;
        arr.first().cloned()
    }

    fn arr_last(args: &[Value], _context: &HashMap<String, Value>) -> Option<Value> {
        let arr = args.first()?.as_array()?;
        arr.last().cloned()
    }

    // Conditional functions

    fn if_fn(args: &[Value]) -> Option<Value> {
        if args.len() < 2 {
            return None;
        }
        let condition = args[0].is_truthy();
        if condition {
            Some(args[1].clone())
        } else if args.len() > 2 {
            Some(args[2].clone())
        } else {
            Some(Value::Null)
        }
    }

    fn default_fn(args: &[Value]) -> Option<Value> {
        if args.is_empty() {
            return None;
        }
        let value = &args[0];
        if value.is_truthy() {
            Some(value.clone())
        } else if args.len() > 1 {
            Some(args[1].clone())
        } else {
            Some(Value::Null)
        }
    }

    fn coalesce_fn(args: &[Value]) -> Option<Value> {
        for arg in args {
            if arg.is_truthy() {
                return Some(arg.clone());
            }
        }
        Some(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_uppercase() {
        let args = vec![Value::String("hello".to_string())];
        let result = BuiltinFunctions::str_uppercase(&args);
        assert_eq!(result, Some(Value::String("HELLO".to_string())));
    }

    #[test]
    fn test_str_lowercase() {
        let args = vec![Value::String("HELLO".to_string())];
        let result = BuiltinFunctions::str_lowercase(&args);
        assert_eq!(result, Some(Value::String("hello".to_string())));
    }

    #[test]
    fn test_str_capitalize() {
        let args = vec![Value::String("hello world".to_string())];
        let result = BuiltinFunctions::str_capitalize(&args);
        assert_eq!(result, Some(Value::String("Hello world".to_string())));
    }

    #[test]
    fn test_str_trim() {
        let args = vec![Value::String("  hello  ".to_string())];
        let result = BuiltinFunctions::str_trim(&args);
        assert_eq!(result, Some(Value::String("hello".to_string())));
    }

    #[test]
    fn test_str_length() {
        let args = vec![Value::String("hello".to_string())];
        let result = BuiltinFunctions::str_length(&args);
        assert_eq!(result, Some(Value::Number(5.0)));
    }

    #[test]
    fn test_str_replace() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
            Value::String("rust".to_string()),
        ];
        let result = BuiltinFunctions::str_replace(&args);
        assert_eq!(result, Some(Value::String("hello rust".to_string())));
    }

    #[test]
    fn test_str_contains() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ];
        let result = BuiltinFunctions::str_contains(&args);
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_arr_join() {
        let args = vec![
            Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ]),
            Value::String(", ".to_string()),
        ];
        let context = HashMap::new();
        let result = BuiltinFunctions::arr_join(&args, &context);
        assert_eq!(result, Some(Value::String("a, b, c".to_string())));
    }

    #[test]
    fn test_arr_length() {
        let args = vec![Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ])];
        let context = HashMap::new();
        let result = BuiltinFunctions::arr_length(&args, &context);
        assert_eq!(result, Some(Value::Number(2.0)));
    }

    #[test]
    fn test_if_fn() {
        let args = vec![
            Value::Bool(true),
            Value::String("yes".to_string()),
            Value::String("no".to_string()),
        ];
        let result = BuiltinFunctions::if_fn(&args);
        assert_eq!(result, Some(Value::String("yes".to_string())));

        let args = vec![
            Value::Bool(false),
            Value::String("yes".to_string()),
            Value::String("no".to_string()),
        ];
        let result = BuiltinFunctions::if_fn(&args);
        assert_eq!(result, Some(Value::String("no".to_string())));
    }

    #[test]
    fn test_default_fn() {
        let args = vec![Value::String("".to_string()), Value::String("default".to_string())];
        let result = BuiltinFunctions::default_fn(&args);
        assert_eq!(result, Some(Value::String("default".to_string())));

        let args = vec![Value::String("value".to_string()), Value::String("default".to_string())];
        let result = BuiltinFunctions::default_fn(&args);
        assert_eq!(result, Some(Value::String("value".to_string())));
    }

    #[test]
    fn test_coalesce_fn() {
        let args = vec![
            Value::Null,
            Value::String("".to_string()),
            Value::String("found".to_string()),
        ];
        let result = BuiltinFunctions::coalesce_fn(&args);
        assert_eq!(result, Some(Value::String("found".to_string())));
    }

    #[test]
    fn test_date_now() {
        let result = BuiltinFunctions::date_now();
        assert!(result.is_some());
        let s = result.unwrap().to_string_value();
        assert!(s.contains("-")); // Should be YYYY-MM-DD format
    }
}
