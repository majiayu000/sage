//! Skill discovery from file system
//!
//! This module provides skill discovery compatible with Claude Code's skill format.
//! Skills can be defined as:
//! - `skill-name.md` files directly in the skills directory
//! - `skill-name/SKILL.md` files in subdirectories (Claude Code format)
//!
//! ## Skill File Format
//!
//! ```markdown
//! ---
//! description: Short description of the skill
//! when_to_use: Condition for AI to auto-invoke this skill
//! allowed_tools:
//!   - Read
//!   - Grep
//!   - Glob
//! user_invocable: true
//! argument_hint: "[file path]"
//! model: inherit
//! version: "1.0"
//! ---
//!
//! Your skill prompt here. Use $ARGUMENTS for user-provided arguments.
//! ```

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tracing::debug;

use super::super::types::{Skill, SkillSourceType, SkillTrigger, ToolAccess};
use super::types::SkillRegistry;

/// Parsed skill frontmatter
#[derive(Debug, Default)]
pub struct SkillFrontmatter {
    pub description: Option<String>,
    pub display_name: Option<String>,
    pub when_to_use: Option<String>,
    pub allowed_tools: Vec<String>,
    pub user_invocable: bool,
    pub disable_model_invocation: bool,
    pub argument_hint: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
    pub priority: Option<i32>,
    pub triggers: Vec<String>,
}

impl SkillFrontmatter {
    /// Parse YAML frontmatter from content
    ///
    /// Supports both simple key: value format and YAML array syntax for allowed_tools
    pub fn parse(content: &str) -> (Self, String) {
        let mut frontmatter = Self::default();

        if let Some(after_prefix) = content.strip_prefix("---") {
            if let Some(end) = after_prefix.find("---") {
                let yaml_content = &after_prefix[..end];
                let prompt = after_prefix[end + 3..].trim().to_string();

                let mut current_array_key: Option<String> = None;
                let mut array_values: Vec<String> = Vec::new();

                for line in yaml_content.lines() {
                    let trimmed = line.trim();

                    // Handle array items (lines starting with -)
                    if trimmed.starts_with('-') && current_array_key.is_some() {
                        let value = trimmed.trim_start_matches('-').trim();
                        // Remove quotes if present
                        let value = value.trim_matches('"').trim_matches('\'');
                        array_values.push(value.to_string());
                        continue;
                    }

                    // Save accumulated array values
                    if let Some(ref key) = current_array_key {
                        frontmatter.apply_array(key, &array_values);
                        current_array_key = None;
                        array_values.clear();
                    }

                    // Parse key: value
                    if let Some(colon_pos) = trimmed.find(':') {
                        let key = trimmed[..colon_pos].trim();
                        let value = trimmed[colon_pos + 1..].trim();

                        // Check if this is an array start (empty value or [])
                        if value.is_empty() || value == "[]" {
                            current_array_key = Some(key.to_string());
                            continue;
                        }

                        // Handle inline array [item1, item2]
                        if value.starts_with('[') && value.ends_with(']') {
                            let items: Vec<String> = value[1..value.len() - 1]
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            frontmatter.apply_array(key, &items);
                            continue;
                        }

                        // Handle simple value
                        frontmatter.apply_value(key, value);
                    }
                }

                // Don't forget trailing array values
                if let Some(ref key) = current_array_key {
                    frontmatter.apply_array(key, &array_values);
                }

                return (frontmatter, prompt);
            }
        }

        (frontmatter, content.to_string())
    }

    fn apply_value(&mut self, key: &str, value: &str) {
        // Remove surrounding quotes
        let value = value.trim_matches('"').trim_matches('\'');

        match key {
            "description" => self.description = Some(value.to_string()),
            "name" | "display_name" => self.display_name = Some(value.to_string()),
            "when_to_use" => self.when_to_use = Some(value.to_string()),
            "user_invocable" | "user-invocable" => {
                self.user_invocable = value == "true" || value == "yes" || value == "1"
            }
            "disable_model_invocation" | "disable-model-invocation" => {
                self.disable_model_invocation = value == "true" || value == "yes" || value == "1"
            }
            "argument_hint" | "argument-hint" => self.argument_hint = Some(value.to_string()),
            "model" => {
                if value != "inherit" {
                    self.model = Some(value.to_string());
                }
            }
            "version" => self.version = Some(value.to_string()),
            "priority" => {
                if let Ok(p) = value.parse() {
                    self.priority = Some(p);
                }
            }
            "allowed_tools" | "allowed-tools" | "tools" => {
                // Handle comma-separated inline list
                if !value.is_empty() {
                    self.allowed_tools = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "triggers" => {
                self.triggers = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {}
        }
    }

    fn apply_array(&mut self, key: &str, values: &[String]) {
        match key {
            "allowed_tools" | "allowed-tools" | "tools" => {
                self.allowed_tools = values.to_vec();
            }
            "triggers" => {
                self.triggers = values.to_vec();
            }
            _ => {}
        }
    }
}

impl SkillRegistry {
    /// Discover skills from a directory
    ///
    /// Scans for both:
    /// - `*.md` files directly in the directory
    /// - `*/SKILL.md` files in subdirectories (Claude Code format)
    pub(super) async fn discover_from_dir(
        &mut self,
        dir: &Path,
        is_project: bool,
    ) -> SageResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read skills directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .await
                .map_err(|e| SageError::storage(format!("Failed to get file type: {}", e)))?;

            if file_type.is_dir() {
                // Check for SKILL.md in subdirectory (Claude Code format)
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    let dir_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");

                    if let Some(skill) = self
                        .load_skill_from_file(&skill_md, dir_name, Some(&path), is_project)
                        .await?
                    {
                        if !self.is_builtin(skill.name()) {
                            debug!("Loaded skill '{}' from {:?}", skill.name(), skill_md);
                            self.register(skill);
                            count += 1;
                        }
                    }
                }
            } else if path.extension().map_or(false, |ext| ext == "md") {
                // Process .md files directly in skills directory
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                if let Some(skill) = self
                    .load_skill_from_file(&path, name, None, is_project)
                    .await?
                {
                    if !self.is_builtin(skill.name()) {
                        debug!("Loaded skill '{}' from {:?}", skill.name(), path);
                        self.register(skill);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Check if a skill is builtin
    fn is_builtin(&self, name: &str) -> bool {
        self.skills
            .get(name)
            .map_or(false, |s| *s.source() == SkillSourceType::Builtin)
    }

    /// Load a skill from a markdown file
    async fn load_skill_from_file(
        &self,
        path: &Path,
        name: &str,
        base_dir: Option<&Path>,
        is_project: bool,
    ) -> SageResult<Option<Skill>> {
        let mut file = fs::File::open(path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open skill file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read skill file: {}", e)))?;

        let (frontmatter, prompt) = SkillFrontmatter::parse(&content);

        let source = if is_project {
            SkillSourceType::Project(path.to_path_buf())
        } else {
            SkillSourceType::User(path.to_path_buf())
        };

        let mut skill = Skill::new(
            name.to_string(),
            frontmatter
                .description
                .clone()
                .unwrap_or_else(|| format!("{} skill", name)),
        )
        .with_prompt(prompt)
        .with_source(source);

        // Apply frontmatter fields
        if let Some(display_name) = frontmatter.display_name {
            skill = skill.with_display_name(display_name);
        }

        if let Some(when_to_use) = frontmatter.when_to_use {
            skill = skill.with_when_to_use(when_to_use);
        }

        if frontmatter.user_invocable {
            skill = skill.set_user_invocable();
        }

        if frontmatter.disable_model_invocation {
            skill = skill.disable_model_invocation();
        }

        if let Some(hint) = frontmatter.argument_hint {
            skill = skill.with_argument_hint(hint);
        }

        if let Some(model) = frontmatter.model {
            skill = skill.with_model(model);
        }

        if let Some(version) = frontmatter.version {
            skill = skill.with_version(version);
        }

        if let Some(priority) = frontmatter.priority {
            skill = skill.with_priority(priority);
        }

        if let Some(base_dir) = base_dir {
            skill = skill.with_base_dir(base_dir.to_path_buf());
        }

        // Parse allowed tools
        if !frontmatter.allowed_tools.is_empty() {
            let tool_access = if frontmatter.allowed_tools.len() == 1 {
                match frontmatter.allowed_tools[0].to_lowercase().as_str() {
                    "all" | "*" => ToolAccess::All,
                    "readonly" | "read-only" => ToolAccess::ReadOnly,
                    _ => ToolAccess::Only(frontmatter.allowed_tools.clone()),
                }
            } else {
                ToolAccess::Only(frontmatter.allowed_tools.clone())
            };
            skill = skill.with_tools(tool_access);
        }

        // Parse triggers
        for trigger_str in &frontmatter.triggers {
            let trigger_str = trigger_str.trim();
            if let Some(keyword) = trigger_str.strip_prefix("keyword:") {
                skill = skill.with_trigger(SkillTrigger::Keyword(keyword.to_string()));
            } else if let Some(extension) = trigger_str.strip_prefix("extension:") {
                skill = skill.with_trigger(SkillTrigger::FileExtension(extension.to_string()));
            } else if let Some(regex) = trigger_str.strip_prefix("regex:") {
                skill = skill.with_trigger(SkillTrigger::Regex(regex.to_string()));
            } else if let Some(tool) = trigger_str.strip_prefix("tool:") {
                skill = skill.with_trigger(SkillTrigger::ToolUsage(tool.to_string()));
            } else {
                // Default to keyword trigger
                skill = skill.with_trigger(SkillTrigger::Keyword(trigger_str.to_string()));
            }
        }

        Ok(Some(skill))
    }

    /// Parse skill file with YAML frontmatter (legacy method for compatibility)
    #[allow(dead_code)]
    pub(crate) fn parse_skill_file(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();

        if let Some(after_prefix) = content.strip_prefix("---") {
            if let Some(end) = after_prefix.find("---") {
                let frontmatter = &after_prefix[..end];
                let prompt = after_prefix[end + 3..].trim().to_string();

                for line in frontmatter.lines() {
                    if let Some(colon) = line.find(':') {
                        let key = line[..colon].trim().to_string();
                        let value = line[colon + 1..].trim().to_string();
                        metadata.insert(key, value);
                    }
                }

                return (metadata, prompt);
            }
        }

        (metadata, content.to_string())
    }
}
