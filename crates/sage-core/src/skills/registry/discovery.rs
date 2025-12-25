//! Skill discovery from file system

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::super::types::{Skill, SkillSource, SkillTrigger, ToolAccess};
use super::types::SkillRegistry;

impl SkillRegistry {
    /// Discover skills from a directory
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

            // Process .md files
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Some(skill) = self.load_skill_from_file(&path, is_project).await? {
                    // Don't override builtins
                    let is_builtin = self
                        .skills
                        .get(&skill.name)
                        .map_or(false, |s| s.source == SkillSource::Builtin);

                    if !is_builtin {
                        self.register(skill);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a skill from a markdown file
    async fn load_skill_from_file(
        &self,
        path: &Path,
        is_project: bool,
    ) -> SageResult<Option<Skill>> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SageError::invalid_input("Invalid skill file name".to_string()))?
            .to_string();

        let mut file = fs::File::open(path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open skill file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read skill file: {}", e)))?;

        let (metadata, prompt) = self.parse_skill_file(&content);

        let source = if is_project {
            SkillSource::Project(path.to_path_buf())
        } else {
            SkillSource::User(path.to_path_buf())
        };

        let mut skill = Skill::new(
            name,
            metadata.get("description").cloned().unwrap_or_default(),
        )
        .with_prompt(prompt)
        .with_source(source);

        // Parse triggers from metadata
        if let Some(triggers) = metadata.get("triggers") {
            for trigger in triggers.split(',') {
                let trigger = trigger.trim();
                if let Some(keyword) = trigger.strip_prefix("keyword:") {
                    skill = skill.with_trigger(SkillTrigger::Keyword(keyword.to_string()));
                } else if let Some(extension) = trigger.strip_prefix("extension:") {
                    skill = skill.with_trigger(SkillTrigger::FileExtension(extension.to_string()));
                } else if let Some(regex) = trigger.strip_prefix("regex:") {
                    skill = skill.with_trigger(SkillTrigger::Regex(regex.to_string()));
                }
            }
        }

        // Parse priority
        if let Some(priority) = metadata.get("priority") {
            if let Ok(p) = priority.parse() {
                skill = skill.with_priority(p);
            }
        }

        // Parse model
        if let Some(model) = metadata.get("model") {
            skill = skill.with_model(model.clone());
        }

        // Parse tools
        if let Some(tools) = metadata.get("tools") {
            let tool_access = if tools == "all" {
                ToolAccess::All
            } else if tools == "readonly" {
                ToolAccess::ReadOnly
            } else {
                ToolAccess::Only(tools.split(',').map(|s| s.trim().to_string()).collect())
            };
            skill = skill.with_tools(tool_access);
        }

        Ok(Some(skill))
    }

    /// Parse skill file with YAML frontmatter
    fn parse_skill_file(&self, content: &str) -> (HashMap<String, String>, String) {
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
