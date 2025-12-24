//! Git tool JSON schema definition

use super::types::GitTool;

impl GitTool {
    /// Get the JSON schema for Git tool parameters
    pub fn get_parameters_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": { "type": "null" }
                            },
                            "required": ["status"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "create_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["create_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "switch_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["switch_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "delete_branch": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "force": { "type": "boolean", "default": false }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "required": ["delete_branch"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "list_branches": { "type": "null" }
                            },
                            "required": ["list_branches"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "add": {
                                    "type": "object",
                                    "properties": {
                                        "files": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        }
                                    },
                                    "required": ["files"]
                                }
                            },
                            "required": ["add"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "commit": {
                                    "type": "object",
                                    "properties": {
                                        "message": { "type": "string" },
                                        "all": { "type": "boolean", "default": false }
                                    },
                                    "required": ["message"]
                                }
                            },
                            "required": ["commit"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "push": {
                                    "type": "object",
                                    "properties": {
                                        "remote": { "type": "string" },
                                        "branch": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["push"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "pull": {
                                    "type": "object",
                                    "properties": {
                                        "remote": { "type": "string" },
                                        "branch": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["pull"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "diff": {
                                    "type": "object",
                                    "properties": {
                                        "staged": { "type": "boolean", "default": false },
                                        "file": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["diff"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "log": {
                                    "type": "object",
                                    "properties": {
                                        "count": { "type": "integer", "minimum": 1 },
                                        "oneline": { "type": "boolean", "default": false }
                                    }
                                }
                            },
                            "required": ["log"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "clone": {
                                    "type": "object",
                                    "properties": {
                                        "url": { "type": "string" },
                                        "path": { "type": "string" }
                                    },
                                    "required": ["url"]
                                }
                            },
                            "required": ["clone"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "reset": {
                                    "type": "object",
                                    "properties": {
                                        "hard": { "type": "boolean", "default": false },
                                        "commit": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["reset"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "remote": {
                                    "type": "object",
                                    "properties": {
                                        "verbose": { "type": "boolean", "default": false }
                                    }
                                }
                            },
                            "required": ["remote"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "merge": {
                                    "type": "object",
                                    "properties": {
                                        "branch": { "type": "string" }
                                    },
                                    "required": ["branch"]
                                }
                            },
                            "required": ["merge"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "rebase": {
                                    "type": "object",
                                    "properties": {
                                        "branch": { "type": "string" }
                                    },
                                    "required": ["branch"]
                                }
                            },
                            "required": ["rebase"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "info": { "type": "null" }
                            },
                            "required": ["info"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "stash": {
                                    "type": "object",
                                    "properties": {
                                        "message": { "type": "string" }
                                    }
                                }
                            },
                            "required": ["stash"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "list_stashes": { "type": "null" }
                            },
                            "required": ["list_stashes"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "apply_stash": {
                                    "type": "object",
                                    "properties": {
                                        "index": { "type": "integer", "minimum": 0 }
                                    }
                                }
                            },
                            "required": ["apply_stash"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "blame": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string" }
                                    },
                                    "required": ["file"]
                                }
                            },
                            "required": ["blame"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "file_history": {
                                    "type": "object",
                                    "properties": {
                                        "file": { "type": "string" }
                                    },
                                    "required": ["file"]
                                }
                            },
                            "required": ["file_history"],
                            "additionalProperties": false
                        }
                    ],
                    "description": "Git operation to perform"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory (optional, defaults to current directory)"
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
    }
}
