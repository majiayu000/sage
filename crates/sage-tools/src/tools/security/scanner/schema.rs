//! JSON schema definitions for security scanner

/// Get the JSON schema for security scanner parameters
pub fn get_parameters_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "object",
                "oneOf": [
                    {
                        "properties": {
                            "scan": {
                                "type": "object",
                                "properties": {
                                    "scan_type": {
                                        "type": "string",
                                        "enum": ["sast", "dependencies", "secrets", "licenses", "full"],
                                        "description": "Type of security scan to perform"
                                    },
                                    "path": {
                                        "type": "string",
                                        "description": "Path to scan"
                                    },
                                    "output_format": {
                                        "type": "string",
                                        "enum": ["json", "xml", "html", "csv"],
                                        "description": "Output format"
                                    },
                                    "exclude_paths": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "Paths to exclude from scan"
                                    }
                                },
                                "required": ["scan_type", "path"]
                            }
                        },
                        "required": ["scan"]
                    },
                    {
                        "properties": {
                            "audit_dependencies": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "package_manager": {
                                        "type": "string",
                                        "enum": ["cargo", "npm", "pip", "maven", "gradle"],
                                        "description": "Package manager to use for audit"
                                    }
                                },
                                "required": ["path", "package_manager"]
                            }
                        },
                        "required": ["audit_dependencies"]
                    },
                    {
                        "properties": {
                            "secret_scan": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "patterns": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "Custom regex patterns to search for"
                                    }
                                },
                                "required": ["path"]
                            }
                        },
                        "required": ["secret_scan"]
                    },
                    {
                        "properties": {
                            "check_vulnerability": {
                                "type": "object",
                                "properties": {
                                    "cve_id": { "type": "string" },
                                    "path": { "type": "string" }
                                },
                                "required": ["cve_id", "path"]
                            }
                        },
                        "required": ["check_vulnerability"]
                    },
                    {
                        "properties": {
                            "generate_report": {
                                "type": "object",
                                "properties": {
                                    "scan_results": { "type": "string" },
                                    "format": {
                                        "type": "string",
                                        "enum": ["html", "pdf", "json", "xml"]
                                    }
                                },
                                "required": ["scan_results", "format"]
                            }
                        },
                        "required": ["generate_report"]
                    }
                ]
            },
            "working_dir": {
                "type": "string",
                "description": "Working directory for scan operations"
            }
        },
        "required": ["operation"],
        "additionalProperties": false
    })
}
