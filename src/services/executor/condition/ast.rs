//! Abstract Syntax Tree for condition expressions.

/// Expression node in the AST.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Boolean literal
    Bool(bool),
    /// Value (string or number)
    Value(Value),
    /// Logical NOT
    Not(Box<Expr>),
    /// Logical AND
    And(Box<Expr>, Box<Expr>),
    /// Logical OR
    Or(Box<Expr>, Box<Expr>),
    /// Comparison operation
    Compare(Value, CompareOp, Value),
    /// Function call
    Function(String, Vec<Value>),
}

/// Value node (string or number).
#[derive(Debug, Clone)]
pub enum Value {
    /// String value (may contain ${var} references)
    String(String),
    /// Numeric value
    Number(f64),
}

/// Comparison operators.
#[derive(Debug, Clone)]
pub enum CompareOp {
    /// Equal (==)
    Eq,
    /// Not equal (!=)
    Ne,
    /// Greater than (>)
    Gt,
    /// Less than (<)
    Lt,
    /// Greater than or equal (>=)
    Ge,
    /// Less than or equal (<=)
    Le,
}
