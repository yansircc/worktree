//! Parser for condition expressions.
//!
//! Grammar (simplified):
//! ```text
//! expression = or_expr
//! or_expr    = and_expr ('||' and_expr)*
//! and_expr   = not_expr ('&&' not_expr)*
//! not_expr   = '!' not_expr | compare_expr
//! compare_expr = primary (('==' | '!=' | '>' | '<' | '>=' | '<=') primary)?
//! primary    = '(' expression ')' | function_call | value
//! function_call = identifier '(' arguments ')'
//! ```

use super::ast::{CompareOp, Expr, Value};
use super::error::ParseError;
use super::tokenizer::Token;

/// Parser for condition expressions.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given tokens.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Parse the tokens into an AST expression.
    pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;

        while matches!(self.current(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_not()?;

        while matches!(self.current(), Token::And) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.current(), Token::Not) {
            self.advance();
            let expr = self.parse_not()?;
            Ok(Expr::Not(Box::new(expr)))
        } else {
            self.parse_compare()
        }
    }

    fn parse_compare(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_primary()?;

        let op = match self.current() {
            Token::Eq => Some(CompareOp::Eq),
            Token::Ne => Some(CompareOp::Ne),
            Token::Gt => Some(CompareOp::Gt),
            Token::Lt => Some(CompareOp::Lt),
            Token::Ge => Some(CompareOp::Ge),
            Token::Le => Some(CompareOp::Le),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let right = self.parse_primary()?;

            let left_val = self.expr_to_value(left)?;
            let right_val = self.expr_to_value(right)?;

            Ok(Expr::Compare(left_val, op, right_val))
        } else {
            Ok(left)
        }
    }

    fn expr_to_value(&self, expr: Expr) -> Result<Value, ParseError> {
        match expr {
            Expr::Value(v) => Ok(v),
            Expr::Bool(b) => Ok(Value::String(b.to_string())),
            _ => Err(ParseError::ExpectedValue),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.current().clone() {
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                if !matches!(self.current(), Token::RParen) {
                    return Err(ParseError::ExpectedRParen);
                }
                self.advance();
                Ok(expr)
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr::Value(Value::String(s)))
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Value(Value::Number(n)))
            }
            Token::Identifier(name) => {
                self.advance();
                // Check if it's a function call
                if matches!(self.current(), Token::LParen) {
                    self.advance();
                    let args = self.parse_arguments()?;
                    if !matches!(self.current(), Token::RParen) {
                        return Err(ParseError::ExpectedRParen);
                    }
                    self.advance();
                    Ok(Expr::Function(name, args))
                } else {
                    // Bare identifier treated as string
                    Ok(Expr::Value(Value::String(name)))
                }
            }
            Token::Eof => Ok(Expr::Bool(true)), // Empty expression is true
            _ => Err(ParseError::UnexpectedToken),
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut args = Vec::new();

        if matches!(self.current(), Token::RParen) {
            return Ok(args);
        }

        loop {
            let expr = self.parse_primary()?;
            args.push(self.expr_to_value(expr)?);

            if matches!(self.current(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tokenizer::Tokenizer;
    use super::*;

    fn parse(input: &str) -> Result<Expr, ParseError> {
        let tokens = Tokenizer::new(input).tokenize()?;
        Parser::new(&tokens).parse_expression()
    }

    #[test]
    fn test_parse_comparison() {
        let expr = parse("'a' == 'b'").unwrap();
        assert!(matches!(expr, Expr::Compare(_, CompareOp::Eq, _)));
    }

    #[test]
    fn test_parse_and() {
        let expr = parse("'a' == 'a' && 'b' == 'b'").unwrap();
        assert!(matches!(expr, Expr::And(_, _)));
    }

    #[test]
    fn test_parse_or() {
        let expr = parse("'a' == 'b' || 'c' == 'c'").unwrap();
        assert!(matches!(expr, Expr::Or(_, _)));
    }

    #[test]
    fn test_parse_not() {
        let expr = parse("!'a' == 'b'").unwrap();
        assert!(matches!(expr, Expr::Not(_)));
    }

    #[test]
    fn test_parse_function() {
        let expr = parse("contains('hello', 'ell')").unwrap();
        assert!(matches!(expr, Expr::Function(name, _) if name == "contains"));
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = parse("('a' == 'a')").unwrap();
        assert!(matches!(expr, Expr::Compare(_, _, _)));
    }

    #[test]
    fn test_parse_empty() {
        let expr = parse("").unwrap();
        assert!(matches!(expr, Expr::Bool(true)));
    }
}
