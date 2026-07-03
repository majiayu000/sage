use anyhow::Result;
use clap::{Parser, ValueEnum};
use sage_eval::{
    EvalRunOptions, EvalRunnerKind, EvalSuite, OfflineRunner, SdkAgentRunner, run_suite,
};
use sage_sdk::SageAgentSdk;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "sage-eval")]
#[command(about = "Run Sage Agent eval suites and emit pass@1/tool-use metrics")]
struct Args {
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/tasks/offline_smoke.json"))]
    tasks: PathBuf,
    #[arg(long, default_value = "offline")]
    runner: RunnerArg,
    #[arg(long, default_value = "target/sage-eval-runs")]
    output_dir: PathBuf,
    #[arg(long)]
    report_json: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RunnerArg {
    Offline,
    Sdk,
}

impl From<RunnerArg> for EvalRunnerKind {
    fn from(value: RunnerArg) -> Self {
        match value {
            RunnerArg::Offline => Self::Offline,
            RunnerArg::Sdk => Self::Sdk,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let suite = EvalSuite::load(&args.tasks)?;
    let options = EvalRunOptions::new(&args.output_dir);
    let report = match EvalRunnerKind::from(args.runner) {
        EvalRunnerKind::Offline => run_suite(&suite, &OfflineRunner, &options).await?,
        EvalRunnerKind::Sdk => {
            let sdk = SageAgentSdk::new()?;
            run_suite(&suite, &SdkAgentRunner::new(sdk), &options).await?
        }
    };
    let output = serde_json::to_string_pretty(&report)?;

    if let Some(path) = args.report_json {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, &output).await?;
    } else {
        println!("{output}");
    }

    Ok(())
}
