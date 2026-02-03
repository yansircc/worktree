use wt::models::task_parser::validate_name;

#[test]
fn test_error_message_empty_name() {
    let err = validate_name("").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("empty"), "Error should mention empty: {}", msg);
}

#[test]
fn test_error_message_whitespace() {
    let err = validate_name("has space").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("whitespace"),
        "Error should mention whitespace: {}",
        msg
    );
}

#[test]
fn test_error_message_invalid_char() {
    let err = validate_name("test~name").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("~"),
        "Error should mention the invalid char: {}",
        msg
    );
}

#[test]
fn test_error_message_invalid_start() {
    let err = validate_name("-test").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("start"), "Error should mention start: {}", msg);
}

#[test]
fn test_error_message_invalid_end() {
    let err = validate_name("test.lock").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(".lock"), "Error should mention .lock: {}", msg);
}

#[test]
fn test_error_message_double_dot() {
    let err = validate_name("a..b").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(".."), "Error should mention ..: {}", msg);
}
