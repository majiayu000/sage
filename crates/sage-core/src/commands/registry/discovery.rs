//! Command discovery from file system

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::super::types::{CommandSource, SlashCommand};
use super::types::CommandRegistry;

impl CommandRegistry {
    /// Discover commands from a directory
    pub(super) async fn discover_from_dir(
        &mut self,
        dir: &Path,
        source: CommandSource,
    ) -> SageResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read commands directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();

            // Only process .md files
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Some(command) = self.load_command_from_file(&path).await? {
                    // Don't override builtins
                    if !self
                        .commands
                        .get(&command.name)
                        .map_or(false, |(_, src)| *src == CommandSource::Builtin)
                    {
                        // Project commands override user commands
                        let should_register = match source {
                            CommandSource::Project => true,
                            CommandSource::User => !self
                                .commands
                                .get(&command.name)
                                .map_or(false, |(_, src)| *src == CommandSource::Project),
                            CommandSource::Builtin => true,
                        };

                        if should_register {
                            self.register(command, source.clone());
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a command from a markdown file
    async fn load_command_from_file(&self, path: &Path) -> SageResult<Option<SlashCommand>> {
        // Get command name from filename (without .md extension)
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SageError::invalid_input("Invalid command file name".to_string()))?
            .to_string();

        // Read file content
        let mut file = fs::File::open(path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open command file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read command file: {}", e)))?;

        // Parse frontmatter if present
        let (metadata, prompt_template) = self.parse_command_file(&content);

        let mut command =
            SlashCommand::new(name, prompt_template).with_source_path(path.to_path_buf());

        // Apply metadata
        if let Some(desc) = metadata.get("description") {
            command = command.with_description(desc.clone());
        }

        Ok(Some(command))
    }

    /// Parse command file with optional YAML frontmatter
    pub(crate) fn parse_command_file(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();

        // Check for YAML frontmatter (--- ... ---)
        if let Some(after_prefix) = content.strip_prefix("---") {
            if let Some(end) = after_prefix.find("---") {
                let frontmatter = &after_prefix[..end];
                let prompt_template = after_prefix[end + 3..].trim().to_string();

                // Parse simple YAML key: value pairs
                for line in frontmatter.lines() {
                    if let Some(colon) = line.find(':') {
                        let key = line[..colon].trim().to_string();
                        let value = line[colon + 1..].trim().to_string();
                        metadata.insert(key, value);
                    }
                }

                return (metadata, prompt_template);
            }
        }

        (metadata, content.to_string())
    }
}
