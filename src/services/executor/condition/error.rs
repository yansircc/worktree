//! Parse errors for condition expressions.

/// Error types during condition parsing.
///
/// Fields are used for Debug output when diagnosing parse failures.
#[derive(Debug)]
pub enum ParseError {
    /// Unexpected character in input
    UnexpectedChar(#[allow(dead_code)] char),
    /// String literal not closed
    UnterminatedString,
    /// Invalid number format
    InvalidNumber(#[allow(dead_code)] String),
    /// Unexpected token during parsing
    UnexpectedToken,
    /// Expected closing parenthesis
    ExpectedRParen,
    /// Expected a value expression
    ExpectedValue,
}
