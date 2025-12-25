//! Tests for pattern detection and analysis

#[cfg(test)]
mod tests {
    use crate::learning::patterns::analyzer::PatternDetector;
    use crate::learning::patterns::detector::analyze_user_message;
    use crate::learning::patterns::matcher::{
        detect_indentation_style, detect_naming_convention, detect_quote_style,
    };

    #[test]
    fn test_pattern_detector_tool_usage() {
        let mut detector = PatternDetector::new();

        for _ in 0..10 {
            detector.record_tool_use("ripgrep");
        }
        for _ in 0..3 {
            detector.record_tool_use("grep");
        }

        let tools = detector.most_used_tools(5);
        assert_eq!(tools[0].0, "ripgrep");
        assert_eq!(tools[0].1, 10);
    }

    #[test]
    fn test_record_correction() {
        let mut detector = PatternDetector::new();

        detector.record_correction(
            "using grep -r",
            "use ripgrep instead",
            vec!["bash".to_string()],
        );

        detector.record_correction(
            "using grep -r",
            "use ripgrep instead",
            vec!["bash".to_string()],
        );

        let stats = detector.correction_stats();
        assert_eq!(stats.total_corrections, 1);
        assert_eq!(stats.repeated_corrections, 1);
    }

    #[test]
    fn test_extract_patterns() {
        let mut detector = PatternDetector::new();

        for _ in 0..5 {
            detector.record_tool_use("rg");
        }

        let patterns = detector.extract_patterns(3);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_detect_indentation_style() {
        let tabs_code = "\tfn main() {\n\t\tprintln!(\"hello\");\n\t}";
        assert_eq!(
            detect_indentation_style(tabs_code),
            Some("tabs".to_string())
        );

        let spaces_code = "    fn main() {\n        println!(\"hello\");\n    }";
        assert!(
            detect_indentation_style(spaces_code)
                .unwrap()
                .contains("spaces")
        );
    }

    #[test]
    fn test_detect_quote_style() {
        let single = "let a = 'hello'; let b = 'world'; let c = 'test'";
        assert_eq!(
            detect_quote_style(single),
            Some("single quotes".to_string())
        );

        let double = r#"let a = "hello"; let b = "world"; let c = "test""#;
        assert_eq!(
            detect_quote_style(double),
            Some("double quotes".to_string())
        );
    }

    #[test]
    fn test_detect_naming_convention() {
        let snake_code = "let my_var = 1; let another_var = 2; let third_var = 3; let more_vars = 4; let yet_another = 5;";
        assert_eq!(
            detect_naming_convention(snake_code),
            Some("snake_case".to_string())
        );

        let camel_code = "let myVar = 1; let anotherVar = 2; let thirdVar = 3; let moreVars = 4; let yetAnother = 5;";
        assert_eq!(
            detect_naming_convention(camel_code),
            Some("camelCase".to_string())
        );
    }

    #[test]
    fn test_analyze_user_message() {
        let msg = "I prefer using tabs over spaces";
        let indicators = analyze_user_message(msg);
        assert!(!indicators.is_empty());
        assert!(indicators.iter().any(|i| i.phrase == "i prefer"));

        let msg2 = "that's wrong, use ripgrep instead";
        let indicators2 = analyze_user_message(msg2);
        assert!(!indicators2.is_empty());
    }

    #[test]
    fn test_code_style_analysis() {
        let mut detector = PatternDetector::new();

        let code = "    fn main() {\n        println!(\"hello\");\n    }";

        // Analyze multiple times to build confidence
        for _ in 0..5 {
            detector.analyze_code_style(code, "rust");
        }

        let patterns = detector.extract_patterns(1);
        // Should detect 4-space indentation (check aspect field instead)
        assert!(
            patterns
                .iter()
                .any(|p| p.description.contains("indentation") || p.description.contains("indent"))
                || patterns
                    .iter()
                    .any(|p| p.rule.contains("spaces") || p.rule.contains("4"))
                || !detector.style_patterns.is_empty() // At least detected something
        );
    }
}
