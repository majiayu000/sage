//! Run command types and arguments

use std::path::PathBuf;

/// Arguments for the run command
pub struct RunArgs {
    pub task: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// Base URL for the model API (reserved for future SDK support)
    #[allow(dead_code)]
    pub model_base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_steps: Option<u32>,
    pub working_dir: Option<PathBuf>,
    pub config_file: String,
    pub trajectory_file: Option<PathBuf>,
    pub patch_path: Option<PathBuf>,
    pub must_patch: bool,
    pub verbose: bool,
}
