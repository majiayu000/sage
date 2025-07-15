//! Sage Agent CLI application

mod commands;
mod console;
mod progress;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use sage_core::error::SageResult;

#[derive(Parser)]
#[command(name = "sage")]
#[command(about = "Sage Agent - LLM-based agent for software engineering tasks")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file (used when no subcommand is provided)
    #[arg(long, default_value = "sage_config.json")]
    config_file: String,

    /// Path to save trajectory file (used when no subcommand is provided)
    #[arg(long)]
    trajectory_file: Option<PathBuf>,

    /// Working directory for the agent (used when no subcommand is provided)
    #[arg(long)]
    working_dir: Option<PathBuf>,

    /// Enable verbose output (used when no subcommand is provided)
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a task using Sage Agent
    Run {
        /// The task description or path to a file containing the task
        task: String,
        
        /// LLM provider to use (openai, anthropic, google, ollama)
        #[arg(long)]
        provider: Option<String>,
        
        /// Model to use
        #[arg(long)]
        model: Option<String>,
        
        /// Base URL for the model API
        #[arg(long)]
        model_base_url: Option<String>,
        
        /// API key for the provider
        #[arg(long)]
        api_key: Option<String>,
        
        /// Maximum number of execution steps
        #[arg(long)]
        max_steps: Option<u32>,
        
        /// Working directory for the agent
        #[arg(long)]
        working_dir: Option<PathBuf>,
        
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
        
        /// Path to save trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,
        
        /// Path to patch file
        #[arg(long)]
        patch_path: Option<PathBuf>,
        
        /// Whether to create a patch
        #[arg(long)]
        must_patch: bool,
        
        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,
    },
    
    /// Interactive mode
    Interactive {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
        
        /// Path to save trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,
        
        /// Working directory for the agent
        #[arg(long)]
        working_dir: Option<PathBuf>,

        /// Enable verbose output
        #[arg(long, short)]
        verbose: bool,
    },
    
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// Trajectory management
    Trajectory {
        #[command(subcommand)]
        action: TrajectoryAction,
    },

    /// Show available tools and their descriptions
    Tools,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },
    
    /// Validate configuration
    Validate {
        /// Path to configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
    },
    
    /// Create a sample configuration file
    Init {
        /// Path for the new configuration file
        #[arg(long, default_value = "sage_config.json")]
        config_file: String,
        
        /// Overwrite existing file
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum TrajectoryAction {
    /// List trajectory files
    List {
        /// Directory to search for trajectories
        #[arg(long, default_value = ".")]
        directory: PathBuf,
    },
    
    /// Show trajectory details
    Show {
        /// Path to trajectory file
        trajectory_file: PathBuf,
    },
    
    /// Analyze trajectory statistics
    Stats {
        /// Path to trajectory file or directory
        path: PathBuf,
    },

    /// Analyze trajectory patterns and performance
    Analyze {
        /// Path to trajectory file or directory
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> SageResult<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        // If no subcommand is provided, default to interactive mode
        None => {
            commands::interactive::execute(commands::interactive::InteractiveArgs {
                config_file: cli.config_file,
                trajectory_file: cli.trajectory_file,
                working_dir: cli.working_dir,
            })
            .await
        }
        Some(Commands::Run {
            task,
            provider,
            model,
            model_base_url,
            api_key,
            max_steps,
            working_dir,
            config_file,
            trajectory_file,
            patch_path,
            must_patch,
            verbose,
        }) => {
            commands::run::execute(commands::run::RunArgs {
                task,
                provider,
                model,
                model_base_url,
                api_key,
                max_steps,
                working_dir,
                config_file,
                trajectory_file,
                patch_path,
                must_patch,
                verbose,
            })
            .await
        }

        Some(Commands::Interactive {
            config_file,
            trajectory_file,
            working_dir,
            verbose: _,
        }) => {
            commands::interactive::execute(commands::interactive::InteractiveArgs {
                config_file,
                trajectory_file,
                working_dir,
            })
            .await
        }

        Some(Commands::Config { action }) => match action {
            ConfigAction::Show { config_file } => {
                commands::config::show(&config_file).await
            }
            ConfigAction::Validate { config_file } => {
                commands::config::validate(&config_file).await
            }
            ConfigAction::Init { config_file, force } => {
                commands::config::init(&config_file, force).await
            }
        },

        Some(Commands::Trajectory { action }) => match action {
            TrajectoryAction::List { directory } => {
                commands::trajectory::list(&directory).await
            }
            TrajectoryAction::Show { trajectory_file } => {
                commands::trajectory::show(&trajectory_file).await
            }
            TrajectoryAction::Stats { path } => {
                commands::trajectory::stats(&path).await
            }
            TrajectoryAction::Analyze { path } => {
                commands::trajectory::analyze(&path).await
            }
        },

        Some(Commands::Tools) => {
            commands::tools::show_tools().await
        },
    }
}
