# sage-eval

Minimal eval harness for Sage Agent quality metrics.

## Run

Offline smoke suite, no network or API key:

```sh
cargo run -p sage-eval -- --runner offline
```

Write a report:

```sh
cargo run -p sage-eval -- --runner offline --report-json target/sage-eval/report.json
```

SDK runner:

```sh
cargo run -p sage-eval -- --runner sdk --tasks crates/sage-eval/tasks/offline_smoke.json
```

The SDK runner uses the current Sage configuration and writes metrics from the trajectory produced in the task workspace. Use it for local/manual evals where provider credentials and sandbox permissions are intentionally configured.

## Metrics

The report includes:

- `pass_at_1`: one deterministic run per task.
- `tool_metrics.recognition_accuracy`: based only on `tool_intent` trajectory entries.
- `tool_metrics.execution_accuracy`: based only on `tool_call` trajectory entries.
- mismatch counts for recognized-but-not-executed and executed-without-recognition.

The metrics intentionally do not infer tool-need recognition from tool execution.

## Add Tasks

Add entries to a suite JSON file:

- `prompt`: user task.
- `required_tool_categories`: categories that must be recognized through `tool_intent`.
- `expected_tool_names`: concrete tool calls expected in trajectory.
- `workspace_files`: files created inside the temporary task workspace.
- `assertions`: executable checks such as `output_contains` or `file_contains`.
- `offline`: deterministic trace used by the offline runner.

Paths in task files must be relative and cannot escape the eval workspace.

## Safety

Each task runs in its own temporary workspace under the eval output directory. The default offline runner does not execute agent tools. The SDK runner is intended for local/manual use with a deny-by-default sandbox or permission profile configured outside the harness; do not add slow or credentialed SDK evals to the default PR path.
