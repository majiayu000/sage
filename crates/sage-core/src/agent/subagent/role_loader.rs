//! Path-bounded loading for declarative sub-agent role files.

use super::types::{SubAgentConfig, SubAgentRoleConfig};
use crate::error::{SageError, SageResult};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SubAgentRoleLoader {
    root: PathBuf,
}

impl SubAgentRoleLoader {
    pub fn new(root: impl Into<PathBuf>) -> SageResult<Self> {
        let root = root.into();
        let root = root.canonicalize().map_err(|err| {
            SageError::config(format!(
                "failed to resolve role root '{}': {err}",
                root.display()
            ))
        })?;
        Ok(Self { root })
    }

    pub fn load(&self, path: impl AsRef<Path>) -> SageResult<SubAgentRoleConfig> {
        let candidate = self.resolve_candidate(path.as_ref())?;
        let source = std::fs::read_to_string(&candidate).map_err(|err| {
            SageError::config(format!(
                "failed to read role '{}': {err}",
                candidate.display()
            ))
        })?;
        let role: SubAgentRoleConfig = match candidate.extension().and_then(|ext| ext.to_str()) {
            Some("json") => serde_json::from_str(&source).map_err(|err| {
                SageError::config(format!(
                    "invalid role JSON '{}': {err}",
                    candidate.display()
                ))
            })?,
            Some("toml") => toml::from_str(&source).map_err(|err| {
                SageError::config(format!(
                    "invalid role TOML '{}': {err}",
                    candidate.display()
                ))
            })?,
            Some(other) => {
                return Err(SageError::config(format!(
                    "unsupported role file extension '{other}'"
                )));
            }
            None => {
                return Err(SageError::config(
                    "role file must have .json or .toml extension",
                ));
            }
        };
        role.validate()?;
        Ok(role)
    }

    fn resolve_candidate(&self, path: &Path) -> SageResult<PathBuf> {
        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let candidate = joined.canonicalize().map_err(|err| {
            SageError::config(format!(
                "failed to resolve role '{}': {err}",
                joined.display()
            ))
        })?;
        if !candidate.starts_with(&self.root) {
            return Err(SageError::config(format!(
                "role path '{}' escapes role root '{}'",
                candidate.display(),
                self.root.display()
            )));
        }
        Ok(candidate)
    }
}

pub fn load_custom_role_for_config(
    config: &SubAgentConfig,
) -> SageResult<Option<SubAgentRoleConfig>> {
    let Some(role_path) = &config.role_path else {
        return Ok(None);
    };
    let role_root = if let Some(root) = &config.role_root {
        root.clone()
    } else {
        let parent_cwd = config
            .parent_cwd
            .as_ref()
            .ok_or_else(|| SageError::config("role loading requires parent working directory"))?;
        parent_cwd.join(".sage").join("agents")
    };
    Ok(Some(SubAgentRoleLoader::new(role_root)?.load(role_path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn role_root() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join(".sage").join("agents");
        fs::create_dir_all(&root).expect("role root");
        (temp, root)
    }

    #[test]
    fn subagent_role_loader_reads_valid_toml_role() {
        let (_temp, root) = role_root();
        fs::write(
            root.join("reviewer.toml"),
            r#"
name = "reviewer"
description = "Review code"
prompt = "Review carefully"
tools = ["Read", "Grep"]
model = "haiku"
reasoning = "medium"
profile = "review"
"#,
        )
        .expect("write role");

        let role = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("reviewer.toml")
            .expect("load role");
        assert_eq!(role.name, "reviewer");
        assert_eq!(role.tools, vec!["Read", "Grep"]);
        assert_eq!(role.model.as_deref(), Some("haiku"));
    }

    #[test]
    fn subagent_role_loader_rejects_unknown_fields() {
        let (_temp, root) = role_root();
        fs::write(
            root.join("bad.toml"),
            r#"
name = "bad"
prompt = "bad"
unexpected = true
"#,
        )
        .expect("write role");
        let error = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("bad.toml")
            .expect_err("unknown field must fail");
        assert!(error.to_string().contains("invalid role TOML"));
    }

    #[test]
    fn subagent_role_loader_rejects_path_escape() {
        let (temp, root) = role_root();
        fs::write(temp.path().join("escape.toml"), "name='x'\nprompt='x'\n")
            .expect("write escaped role");
        let error = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("../../escape.toml")
            .expect_err("path escape must fail");
        assert!(error.to_string().contains("escapes role root"));
    }

    #[test]
    fn subagent_role_loader_rejects_bad_reasoning() {
        let (_temp, root) = role_root();
        fs::write(
            root.join("bad_reasoning.toml"),
            "name='x'\nprompt='x'\nreasoning='turbo'\n",
        )
        .expect("write role");
        let error = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("bad_reasoning.toml")
            .expect_err("bad reasoning must fail");
        assert!(error.to_string().contains("unsupported reasoning"));
    }

    #[test]
    fn subagent_role_loader_rejects_unknown_profile() {
        let (_temp, root) = role_root();
        fs::write(
            root.join("bad_profile.toml"),
            "name='x'\nprompt='x'\nprofile='unknown-profile'\n",
        )
        .expect("write role");
        let error = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("bad_profile.toml")
            .expect_err("bad profile must fail");
        assert!(error.to_string().contains("unsupported profile"));
    }

    #[test]
    fn subagent_role_loader_rejects_unknown_model() {
        let (_temp, root) = role_root();
        fs::write(
            root.join("bad_model.toml"),
            "name='x'\nprompt='x'\nmodel='unknown-model'\n",
        )
        .expect("write role");
        let error = SubAgentRoleLoader::new(&root)
            .expect("loader")
            .load("bad_model.toml")
            .expect_err("bad model must fail");
        assert!(error.to_string().contains("unsupported model"));
    }
}
