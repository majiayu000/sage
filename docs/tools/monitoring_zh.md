# Monitoring Tools

Sage Agent provides comprehensive monitoring tools for log analysis and system observability.

## Log Analyzer Tool

### Overview

- **Tool Name**: `log_analyzer`
- **Purpose**: Advanced log analysis with error detection, pattern matching, and metrics extraction
- **Location**: `crates/sage-tools/src/tools/monitoring/log_analyzer.rs`

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | Analysis command (analyze, search, metrics, errors) |
| `file_path` | string | Yes | Path to log file |
| `pattern` | string | No | Regex pattern to search for |
| `lines` | number | No | Number of lines to analyze (default: all) |
| `log_format` | string | No | Log format (json, combined, common) |

### Supported Commands

#### Analyze Logs
Perform comprehensive log analysis:
```json
{
  "command": "analyze",
  "file_path": "/var/log/app.log",
  "lines": 1000
}
```

#### Search Patterns
Search for specific patterns in logs:
```json
{
  "command": "search",
  "file_path": "/var/log/app.log",
  "pattern": "ERROR|FATAL"
}
```

#### Extract Metrics
Generate metrics from log data:
```json
{
  "command": "metrics",
  "file_path": "/var/log/access.log",
  "log_format": "combined"
}
```

#### Error Detection
Focus on error analysis:
```json
{
  "command": "errors",
  "file_path": "/var/log/app.log",
  "lines": 500
}
```

### Features

- **Error Detection**: Automatically identifies error patterns
- **Metrics Extraction**: Generates statistics from log data
- **Pattern Matching**: Supports regex patterns for custom searches
- **Multiple Formats**: Handles JSON, Apache combined, and common log formats
- **Performance Analysis**: Identifies slow operations and bottlenecks

### Usage Example

```rust
use sage_tools::LogAnalyzerTool;

let analyzer = LogAnalyzerTool::new();

// Analyze application logs
let call = ToolCall::new("1", "log_analyzer", json!({
    "command": "analyze",
    "file_path": "/var/log/myapp.log",
    "lines": 1000
}));

let result = analyzer.execute(&call).await?;
println!("Log analysis: {}", result.content);
```

## Test Generator Tool

### Overview

- **Tool Name**: `test_generator`
- **Purpose**: Automated test generation for unit tests, integration tests, and mocks
- **Location**: `crates/sage-tools/src/tools/monitoring/test_generator.rs`

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | Generation command (unit, integration, mock, benchmark) |
| `language` | string | Yes | Programming language (rust, python, javascript, go) |
| `function_name` | string | No | Function name to test |
| `file_path` | string | No | Source file path |
| `test_framework` | string | No | Test framework (pytest, jest, etc.) |
| `output_path` | string | No | Output file for generated tests |

### Supported Commands

#### Unit Tests
Generate unit tests for functions:
```json
{
  "command": "unit",
  "language": "rust",
  "function_name": "calculate_sum",
  "file_path": "src/math.rs"
}
```

#### Integration Tests
Generate integration test templates:
```json
{
  "command": "integration",
  "language": "python",
  "test_framework": "pytest",
  "output_path": "tests/test_integration.py"
}
```

#### Mock Generation
Create mock objects and stubs:
```json
{
  "command": "mock",
  "language": "javascript",
  "function_name": "apiCall",
  "test_framework": "jest"
}
```

#### Benchmark Tests
Generate performance benchmarks:
```json
{
  "command": "benchmark",
  "language": "rust",
  "function_name": "sort_algorithm",
  "file_path": "src/algorithms.rs"
}
```

### Language Support

#### Rust
- Uses standard `#[test]` and `#[cfg(test)]` attributes
- Supports `assert_eq!`, `assert!`, and `panic!` assertions
- Generates `#[bench]` for benchmark tests

#### Python
- Compatible with pytest, unittest, and nose
- Generates fixtures and parametrized tests
- Supports mock objects with unittest.mock

#### JavaScript/TypeScript
- Jest and Mocha test frameworks
- Async/await test patterns
- Mock functions and modules

#### Go
- Standard testing package
- Table-driven tests
- Benchmark functions

### Usage Example

```rust
use sage_tools::TestGeneratorTool;

let generator = TestGeneratorTool::new();

// Generate Rust unit tests
let call = ToolCall::new("1", "test_generator", json!({
    "command": "unit",
    "language": "rust",
    "function_name": "parse_config",
    "file_path": "src/config.rs"
}));

let result = generator.execute(&call).await?;
println!("Generated test: {}", result.content);
```

## Best Practices

### Log Analysis
1. **Regular Monitoring**: Set up automated log analysis for critical systems
2. **Pattern Tuning**: Customize error patterns for your application
3. **Performance Tracking**: Monitor response times and resource usage
4. **Alert Integration**: Connect analysis results to monitoring systems

### Test Generation
1. **Code Review**: Always review generated tests before integration
2. **Coverage Goals**: Use generated tests as starting points for comprehensive coverage
3. **Framework Consistency**: Stick to established test frameworks in your project
4. **Continuous Integration**: Integrate generated tests into CI/CD pipelines

## Integration Examples

### CI/CD Pipeline
```yaml
# .github/workflows/test.yml
- name: Generate Tests
  run: |
    sage-cli tool test_generator \
      --command unit \
      --language rust \
      --file_path src/lib.rs

- name: Analyze Logs
  run: |
    sage-cli tool log_analyzer \
      --command analyze \
      --file_path logs/test.log
```

### Monitoring Dashboard
```rust
// Automated log monitoring
async fn monitor_logs() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = LogAnalyzerTool::new();
    
    for log_file in get_log_files()? {
        let analysis = analyzer.execute(&ToolCall::new(
            "monitor", 
            "log_analyzer", 
            json!({
                "command": "errors",
                "file_path": log_file,
                "lines": 100
            })
        )).await?;
        
        if analysis.content.contains("CRITICAL") {
            send_alert(&analysis.content).await?;
        }
    }
    
    Ok(())
}
```