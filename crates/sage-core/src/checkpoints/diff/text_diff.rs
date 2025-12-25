//! Simple text diff implementation

/// Simple text diff implementation
#[derive(Debug, Clone)]
pub struct TextDiff {
    pub hunks: Vec<DiffHunk>,
}

/// A diff hunk
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

/// A diff line
#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Added(String),
    Removed(String),
}

impl TextDiff {
    /// Compute diff between two strings
    pub fn compute(old: &str, new: &str) -> Self {
        let old_lines: Vec<_> = old.lines().collect();
        let new_lines: Vec<_> = new.lines().collect();

        // Simple LCS-based diff
        let hunks = Self::compute_hunks(&old_lines, &new_lines);
        Self { hunks }
    }

    /// Compute hunks using simple LCS algorithm
    fn compute_hunks(old: &[&str], new: &[&str]) -> Vec<DiffHunk> {
        // For simplicity, use a basic approach
        // In production, would use proper LCS/Myers diff
        let mut hunks = Vec::new();
        let mut lines = Vec::new();

        let mut old_idx = 0;
        let mut new_idx = 0;

        while old_idx < old.len() || new_idx < new.len() {
            if old_idx < old.len() && new_idx < new.len() {
                if old[old_idx] == new[new_idx] {
                    lines.push(DiffLine::Context(old[old_idx].to_string()));
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    // Simple: mark old as removed, new as added
                    lines.push(DiffLine::Removed(old[old_idx].to_string()));
                    old_idx += 1;
                    if new_idx < new.len() {
                        lines.push(DiffLine::Added(new[new_idx].to_string()));
                        new_idx += 1;
                    }
                }
            } else if old_idx < old.len() {
                lines.push(DiffLine::Removed(old[old_idx].to_string()));
                old_idx += 1;
            } else {
                lines.push(DiffLine::Added(new[new_idx].to_string()));
                new_idx += 1;
            }
        }

        if !lines.is_empty() {
            hunks.push(DiffHunk {
                old_start: 1,
                old_count: old.len(),
                new_start: 1,
                new_count: new.len(),
                lines,
            });
        }

        hunks
    }

    /// Format diff as unified diff string
    pub fn format_unified(&self) -> String {
        let mut output = String::new();

        for hunk in &self.hunks {
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
            ));

            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) => output.push_str(&format!(" {}\n", s)),
                    DiffLine::Added(s) => output.push_str(&format!("+{}\n", s)),
                    DiffLine::Removed(s) => output.push_str(&format!("-{}\n", s)),
                }
            }
        }

        output
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.hunks.iter().any(|h| {
            h.lines
                .iter()
                .any(|l| matches!(l, DiffLine::Added(_) | DiffLine::Removed(_)))
        })
    }

    /// Count added lines
    pub fn added_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Added(_)))
            .count()
    }

    /// Count removed lines
    pub fn removed_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Removed(_)))
            .count()
    }
}
