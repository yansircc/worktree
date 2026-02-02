use super::*;

#[test]
fn test_completions_generate_bash() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["completions", "generate", "bash"]);

    assert!(ok);
    assert!(stdout.contains("complete"));
    assert!(stdout.contains("wt"));
}

#[test]
fn test_completions_generate_zsh() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["completions", "generate", "zsh"]);

    assert!(ok);
    assert!(stdout.contains("#compdef"));
    assert!(stdout.contains("wt"));
}

#[test]
fn test_completions_generate_fish() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["completions", "generate", "fish"]);

    assert!(ok);
    assert!(stdout.contains("complete"));
    assert!(stdout.contains("wt"));
}

#[test]
fn test_completions_help() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, stdout, _) = run_wt(dir.path(), &["completions", "--help"]);

    assert!(ok);
    assert!(stdout.contains("generate"));
    assert!(stdout.contains("install"));
}
