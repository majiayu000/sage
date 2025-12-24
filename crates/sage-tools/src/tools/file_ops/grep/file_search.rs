//! Single file search logic

use super::output::GrepOutputMode;
use crate::tools::file_ops::grep::GrepTool;
use regex::Regex;
use sage_core::tools::base::{FileSystemTool, ToolError};
use std::path::Path;

impl GrepTool {
    /// Search a single file
    pub fn search_file(
        &self,
        path: &Path,
        regex: &Regex,
        show_line_numbers: bool,
        lines_before: usize,
        lines_after: usize,
        output_mode: GrepOutputMode,
    ) -> Result<Option<String>, ToolError> {
        // Read file content, silently skip binary files
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                // Skip files that can't be read as text (binary files, encoding issues)
                if e.kind() == std::io::ErrorKind::InvalidData {
                    return Ok(None);
                }
                // Skip permission denied errors silently
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return Ok(None);
                }
                return Err(ToolError::Io(e));
            }
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut matching_lines = Vec::new();
        let mut match_count = 0;

        for (i, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                match_count += 1;

                if output_mode == GrepOutputMode::Content {
                    // Add context lines before
                    let start = i.saturating_sub(lines_before);

                    for (idx, ctx_line) in lines[start..i].iter().enumerate() {
                        let line_num = start + idx + 1;
                        if show_line_numbers {
                            matching_lines.push(format!("{}:\t{}", line_num, ctx_line));
                        } else {
                            matching_lines.push(ctx_line.to_string());
                        }
                    }

                    // Add the matching line
                    if show_line_numbers {
                        matching_lines.push(format!("{}:\t{}", i + 1, line));
                    } else {
                        matching_lines.push(line.to_string());
                    }

                    // Add context lines after
                    let end = std::cmp::min(i + lines_after + 1, lines.len());
                    for (idx, ctx_line) in lines[(i + 1)..end].iter().enumerate() {
                        let line_num = i + 2 + idx;
                        if show_line_numbers {
                            matching_lines.push(format!("{}:\t{}", line_num, ctx_line));
                        } else {
                            matching_lines.push(ctx_line.to_string());
                        }
                    }

                    if lines_before > 0 || lines_after > 0 {
                        matching_lines.push("--".to_string());
                    }
                }
            }
        }

        if match_count == 0 {
            return Ok(None);
        }

        let relative_path = path
            .strip_prefix(self.working_directory())
            .unwrap_or(path);

        let result = match output_mode {
            GrepOutputMode::Content => {
                format!(
                    "{}:\n{}",
                    relative_path.display(),
                    matching_lines.join("\n")
                )
            }
            GrepOutputMode::FilesWithMatches => relative_path.display().to_string(),
            GrepOutputMode::Count => {
                format!("{}:{}", relative_path.display(), match_count)
            }
        };

        Ok(Some(result))
    }
}
