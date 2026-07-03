//! Minimal eval harness for Sage Agent quality metrics.

pub mod metrics;
pub mod report;
pub mod runner;
pub mod task;
pub mod trace;

pub use metrics::{TaskToolMetrics, ToolMetricsSummary, evaluate_tool_metrics};
pub use report::{EvalReport, TaskReport};
pub use runner::{
    EvalRunOptions, EvalRunner, EvalRunnerKind, OfflineRunner, SdkAgentRunner, TaskRunOutput,
    run_suite,
};
pub use task::{Assertion, EvalSuite, EvalTask, OfflineTrace, ToolIntentSpec, WorkspaceFile};
