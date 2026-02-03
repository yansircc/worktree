//! Condition expression evaluator for step execution control.
//!
//! Supports:
//! - Logical operators: `&&` (AND), `||` (OR), `!` (NOT)
//! - Comparison: `==`, `!=`, `>`, `<`, `>=`, `<=`
//! - String functions: `contains()`, `startsWith()`, `endsWith()`
//! - Value checks: `empty()`, `defined()`
//! - Parentheses for grouping
//! - Shell command fallback (if expression doesn't match known patterns)
//!
//! # Examples
//!
//! ```ignore
//! "${prev.state}" == "success"
//! "${prev.state}" == "success" && "${phase}" == "developing"
//! !empty("${output}")
//! contains("${message}", "error") || "${exit_code}" != "0"
//! ```

mod ast;
mod error;
mod parser;
mod tokenizer;

use ast::{CompareOp, Expr, Value};
use error::ParseError;
use parser::Parser;
use tokenizer::Tokenizer;

/// Condition evaluator with variable expansion support.
pub struct ConditionEvaluator<'a> {
    /// Function to expand variables in strings
    expand_fn: Box<dyn Fn(&str) -> String + 'a>,
    /// Working directory for shell commands
    working_dir: String,
}

impl<'a> ConditionEvaluator<'a> {
    /// Create a new condition evaluator.
    pub fn new<F>(expand_fn: F, working_dir: &str) -> Self
    where
        F: Fn(&str) -> String + 'a,
    {
        Self {
            expand_fn: Box::new(expand_fn),
            working_dir: working_dir.to_string(),
        }
    }

    /// Evaluate a condition expression.
    ///
    /// Returns `true` if the condition is met, `false` otherwise.
    pub fn evaluate(&self, condition: &str) -> bool {
        let condition = condition.trim();
        if condition.is_empty() {
            return true;
        }

        // Try to parse as expression first
        match self.parse_and_evaluate(condition) {
            Ok(result) => result,
            Err(_) => {
                // Fall back to shell command
                self.evaluate_shell(condition)
            }
        }
    }

    /// Parse and evaluate an expression.
    fn parse_and_evaluate(&self, expr: &str) -> Result<bool, ParseError> {
        let tokens = Tokenizer::new(expr).tokenize()?;
        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_expression()?;
        Ok(self.eval_expr(&ast))
    }

    /// Evaluate an AST expression node.
    fn eval_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Bool(b) => *b,
            Expr::Not(inner) => !self.eval_expr(inner),
            Expr::And(left, right) => self.eval_expr(left) && self.eval_expr(right),
            Expr::Or(left, right) => self.eval_expr(left) || self.eval_expr(right),
            Expr::Compare(left, op, right) => {
                let left_val = self.eval_value(left);
                let right_val = self.eval_value(right);
                self.compare(&left_val, op, &right_val)
            }
            Expr::Value(val) => {
                // A standalone value is truthy if non-empty and not "false"
                let v = self.eval_value(val);
                !v.is_empty() && v != "false" && v != "0"
            }
            Expr::Function(name, args) => self.eval_function(name, args),
        }
    }

    /// Evaluate a value node.
    fn eval_value(&self, val: &Value) -> String {
        match val {
            Value::String(s) => (self.expand_fn)(s),
            Value::Number(n) => n.to_string(),
        }
    }

    /// Perform comparison operation.
    fn compare(&self, left: &str, op: &CompareOp, right: &str) -> bool {
        match op {
            CompareOp::Eq => left == right,
            CompareOp::Ne => left != right,
            CompareOp::Gt => self.numeric_compare(left, right, |a, b| a > b),
            CompareOp::Lt => self.numeric_compare(left, right, |a, b| a < b),
            CompareOp::Ge => self.numeric_compare(left, right, |a, b| a >= b),
            CompareOp::Le => self.numeric_compare(left, right, |a, b| a <= b),
        }
    }

    /// Perform numeric comparison, falling back to string comparison.
    fn numeric_compare<F>(&self, left: &str, right: &str, cmp: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left.parse::<f64>(), right.parse::<f64>()) {
            (Ok(l), Ok(r)) => cmp(l, r),
            _ => {
                // Fall back to string comparison
                match cmp(1.0, 0.0) {
                    true if cmp(0.0, 1.0) => left >= right,
                    true => left > right,
                    false if !cmp(1.0, 1.0) => left < right,
                    false => left <= right,
                }
            }
        }
    }

    /// Evaluate a function call.
    fn eval_function(&self, name: &str, args: &[Value]) -> bool {
        let expanded_args: Vec<String> = args.iter().map(|a| self.eval_value(a)).collect();

        match name {
            "contains" => {
                expanded_args.len() >= 2 && expanded_args[0].contains(&expanded_args[1])
            }
            "startsWith" | "starts_with" => {
                expanded_args.len() >= 2 && expanded_args[0].starts_with(&expanded_args[1])
            }
            "endsWith" | "ends_with" => {
                expanded_args.len() >= 2 && expanded_args[0].ends_with(&expanded_args[1])
            }
            "empty" => expanded_args.first().map(|s| s.is_empty()).unwrap_or(true),
            "defined" => {
                // A variable is defined if it was expanded (doesn't contain ${})
                if let Some(arg) = args.first() {
                    if let Value::String(s) = arg {
                        let expanded = (self.expand_fn)(s);
                        !expanded.contains("${")
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            "matches" => {
                // Simplified regex match (just contains for now)
                expanded_args.len() >= 2 && expanded_args[0].contains(&expanded_args[1])
            }
            _ => false,
        }
    }

    /// Evaluate condition as shell command.
    fn evaluate_shell(&self, condition: &str) -> bool {
        use std::process::{Command, Stdio};

        let expanded = (self.expand_fn)(condition);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&expanded);
        cmd.current_dir(&self.working_dir);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        cmd.status().map(|s| s.success()).unwrap_or(false)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_evaluator() -> ConditionEvaluator<'static> {
        ConditionEvaluator::new(
            |s| {
                s.replace("${task}", "auth")
                    .replace("${phase}", "developing")
                    .replace("${prev.state}", "success")
                    .replace("${exit_code}", "0")
                    .replace("${undefined}", "${undefined}")
            },
            ".",
        )
    }

    // ========== Basic Comparison ==========

    #[test]
    fn test_string_equality() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'hello' == 'hello'"));
        assert!(!eval.evaluate("'hello' == 'world'"));
    }

    #[test]
    fn test_string_inequality() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'hello' != 'world'"));
        assert!(!eval.evaluate("'hello' != 'hello'"));
    }

    #[test]
    fn test_numeric_comparison() {
        let eval = make_evaluator();
        assert!(eval.evaluate("10 > 5"));
        assert!(eval.evaluate("5 < 10"));
        assert!(eval.evaluate("10 >= 10"));
        assert!(eval.evaluate("5 <= 5"));
        assert!(!eval.evaluate("5 > 10"));
    }

    #[test]
    fn test_negative_numbers() {
        let eval = make_evaluator();
        assert!(eval.evaluate("-5 < 0"));
        assert!(eval.evaluate("0 > -10"));
    }

    // ========== Variable Expansion ==========

    #[test]
    fn test_variable_comparison() {
        let eval = make_evaluator();
        assert!(eval.evaluate("\"${task}\" == \"auth\""));
        assert!(eval.evaluate("\"${phase}\" == \"developing\""));
        assert!(eval.evaluate("\"${prev.state}\" == \"success\""));
    }

    #[test]
    fn test_variable_with_single_quotes() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'${task}' == 'auth'"));
    }

    // ========== Logical Operators ==========

    #[test]
    fn test_and_operator() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'a' == 'a' && 'b' == 'b'"));
        assert!(!eval.evaluate("'a' == 'a' && 'b' == 'c'"));
        assert!(!eval.evaluate("'a' == 'x' && 'b' == 'b'"));
    }

    #[test]
    fn test_or_operator() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'a' == 'a' || 'b' == 'c'"));
        assert!(eval.evaluate("'a' == 'x' || 'b' == 'b'"));
        assert!(!eval.evaluate("'a' == 'x' || 'b' == 'y'"));
    }

    #[test]
    fn test_not_operator() {
        let eval = make_evaluator();
        assert!(eval.evaluate("!'a' == 'b'"));
        assert!(!eval.evaluate("!'a' == 'a'"));
    }

    #[test]
    fn test_combined_logic() {
        let eval = make_evaluator();
        assert!(eval.evaluate("'a' == 'a' && ('b' == 'c' || 'c' == 'c')"));
        assert!(eval.evaluate("!('a' == 'b') && 'c' == 'c'"));
    }

    // ========== Functions ==========

    #[test]
    fn test_contains_function() {
        let eval = make_evaluator();
        assert!(eval.evaluate("contains('hello world', 'world')"));
        assert!(!eval.evaluate("contains('hello world', 'foo')"));
    }

    #[test]
    fn test_starts_with_function() {
        let eval = make_evaluator();
        assert!(eval.evaluate("startsWith('hello world', 'hello')"));
        assert!(!eval.evaluate("startsWith('hello world', 'world')"));
    }

    #[test]
    fn test_ends_with_function() {
        let eval = make_evaluator();
        assert!(eval.evaluate("endsWith('hello world', 'world')"));
        assert!(!eval.evaluate("endsWith('hello world', 'hello')"));
    }

    #[test]
    fn test_empty_function() {
        let eval = make_evaluator();
        assert!(eval.evaluate("empty('')"));
        assert!(!eval.evaluate("empty('hello')"));
    }

    #[test]
    fn test_defined_function() {
        let eval = make_evaluator();
        assert!(eval.evaluate("defined('${task}')"));
        assert!(!eval.evaluate("defined('${undefined}')"));
    }

    #[test]
    fn test_function_with_variables() {
        let eval = make_evaluator();
        assert!(eval.evaluate("contains('${task}', 'auth')"));
        assert!(eval.evaluate("startsWith('${phase}', 'develop')"));
    }

    // ========== Complex Expressions ==========

    #[test]
    fn test_complex_expression() {
        let eval = make_evaluator();
        let expr = "\"${prev.state}\" == \"success\" && \"${exit_code}\" == \"0\"";
        assert!(eval.evaluate(expr));
    }

    #[test]
    fn test_nested_parentheses() {
        let eval = make_evaluator();
        assert!(eval.evaluate("((1 == 1))"));
        assert!(eval.evaluate("(1 == 1) && (2 == 2)"));
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_empty_condition() {
        let eval = make_evaluator();
        assert!(eval.evaluate(""));
        assert!(eval.evaluate("   "));
    }

    #[test]
    fn test_shell_fallback() {
        let eval = make_evaluator();
        assert!(eval.evaluate("true"));
        assert!(!eval.evaluate("false"));
        assert!(eval.evaluate("test 1 -eq 1"));
    }

    #[test]
    fn test_boolean_literals() {
        let eval = make_evaluator();
        assert!(eval.evaluate("true == true"));
        assert!(!eval.evaluate("true == false"));
    }
}
