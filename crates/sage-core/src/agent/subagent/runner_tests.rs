use super::runner::StepResult;

#[test]
fn test_step_result() {
    let continue_result = StepResult::Continue;
    let completed_result = StepResult::Completed("Done".to_string());

    match continue_result {
        StepResult::Continue => {}
        _ => panic!("Expected Continue"),
    }

    match completed_result {
        StepResult::Completed(output) => assert_eq!(output, "Done"),
        _ => panic!("Expected Completed"),
    }
}
