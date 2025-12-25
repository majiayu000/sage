//! Email tool JSON schema definition
//!
//! This module provides the JSON schema for the email tool parameters.

use serde_json::json;

/// Get the JSON schema for email tool parameters
pub fn get_email_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "operation": {
                "type": "object",
                "oneOf": [
                    {
                        "properties": {
                            "send": {
                                "type": "object",
                                "properties": {
                                    "smtp_config": {
                                        "type": "object",
                                        "properties": {
                                            "host": { "type": "string" },
                                            "port": { "type": "integer" },
                                            "username": { "type": "string" },
                                            "password": { "type": "string" },
                                            "use_tls": { "type": "boolean" },
                                            "use_starttls": { "type": "boolean" }
                                        },
                                        "required": ["host", "port", "username", "password"]
                                    },
                                    "message": {
                                        "type": "object",
                                        "properties": {
                                            "from": { "type": "string" },
                                            "to": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            },
                                            "cc": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            },
                                            "bcc": {
                                                "type": "array",
                                                "items": { "type": "string" }
                                            },
                                            "subject": { "type": "string" },
                                            "body_text": { "type": "string" },
                                            "body_html": { "type": "string" },
                                            "attachments": {
                                                "type": "array",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "filename": { "type": "string" },
                                                        "content_type": { "type": "string" },
                                                        "data": { "type": "string" }
                                                    },
                                                    "required": ["filename", "content_type", "data"]
                                                }
                                            }
                                        },
                                        "required": ["from", "to", "subject"]
                                    }
                                },
                                "required": ["smtp_config", "message"]
                            }
                        },
                        "required": ["send"]
                    },
                    {
                        "properties": {
                            "read": {
                                "type": "object",
                                "properties": {
                                    "imap_config": {
                                        "type": "object",
                                        "properties": {
                                            "host": { "type": "string" },
                                            "port": { "type": "integer" },
                                            "username": { "type": "string" },
                                            "password": { "type": "string" },
                                            "use_tls": { "type": "boolean" }
                                        },
                                        "required": ["host", "port", "username", "password"]
                                    },
                                    "folder": { "type": "string", "default": "INBOX" },
                                    "limit": { "type": "integer", "default": 10 },
                                    "unread_only": { "type": "boolean", "default": false }
                                },
                                "required": ["imap_config"]
                            }
                        },
                        "required": ["read"]
                    },
                    {
                        "properties": {
                            "validate_email": {
                                "type": "object",
                                "properties": {
                                    "email": { "type": "string" }
                                },
                                "required": ["email"]
                            }
                        },
                        "required": ["validate_email"]
                    },
                    {
                        "properties": {
                            "process_template": {
                                "type": "object",
                                "properties": {
                                    "template": { "type": "string" },
                                    "variables": {
                                        "type": "object",
                                        "additionalProperties": true
                                    }
                                },
                                "required": ["template", "variables"]
                            }
                        },
                        "required": ["process_template"]
                    },
                    {
                        "properties": {
                            "parse_email": {
                                "type": "object",
                                "properties": {
                                    "raw_email": { "type": "string" }
                                },
                                "required": ["raw_email"]
                            }
                        },
                        "required": ["parse_email"]
                    }
                ]
            }
        },
        "required": ["operation"],
        "additionalProperties": false
    })
}
