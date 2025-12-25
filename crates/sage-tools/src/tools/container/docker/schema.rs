//! Docker tool JSON schema definitions

/// Generate the JSON schema for Docker tool parameters
pub fn parameters_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "object",
                "oneOf": [
                    {
                        "properties": {
                            "list_containers": {
                                "type": "object",
                                "properties": {
                                    "all": { "type": "boolean", "default": false }
                                }
                            }
                        },
                        "required": ["list_containers"]
                    },
                    {
                        "properties": {
                            "run_container": {
                                "type": "object",
                                "properties": {
                                    "image": { "type": "string" },
                                    "name": { "type": "string" },
                                    "ports": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    },
                                    "volumes": {
                                        "type": "array",
                                        "items": { "type": "string" }
                                    },
                                    "environment": {
                                        "type": "object",
                                        "additionalProperties": { "type": "string" }
                                    },
                                    "detach": { "type": "boolean", "default": false },
                                    "remove": { "type": "boolean", "default": false },
                                    "command": { "type": "string" }
                                },
                                "required": ["image"]
                            }
                        },
                        "required": ["run_container"]
                    },
                    {
                        "properties": {
                            "stop_container": {
                                "type": "object",
                                "properties": {
                                    "container": { "type": "string" }
                                },
                                "required": ["container"]
                            }
                        },
                        "required": ["stop_container"]
                    },
                    {
                        "properties": {
                            "build_image": {
                                "type": "object",
                                "properties": {
                                    "dockerfile_path": { "type": "string" },
                                    "tag": { "type": "string" },
                                    "context": { "type": "string" },
                                    "build_args": {
                                        "type": "object",
                                        "additionalProperties": { "type": "string" }
                                    }
                                },
                                "required": ["dockerfile_path", "tag"]
                            }
                        },
                        "required": ["build_image"]
                    },
                    {
                        "properties": {
                            "system_info": { "type": "null" }
                        },
                        "required": ["system_info"]
                    }
                ]
            },
            "working_dir": {
                "type": "string",
                "description": "Working directory for Docker commands"
            }
        },
        "required": ["operation"],
        "additionalProperties": false
    })
}
