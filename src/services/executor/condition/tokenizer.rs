//! Lexical analyzer (tokenizer) for condition expressions.

use std::iter::Peekable;
use std::str::Chars;

use super::error::ParseError;

/// Token types for condition expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Values
    /// String literal
    String(String),
    /// Numeric literal
    Number(f64),
    /// Identifier (function name, etc.)
    Identifier(String),

    // Operators
    /// Logical AND (&&)
    And,
    /// Logical OR (||)
    Or,
    /// Logical NOT (!)
    Not,
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

    // Delimiters
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    /// Comma
    Comma,

    // End
    /// End of input
    Eof,
}

/// Tokenizer for condition expressions.
pub struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer for the given input.
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    /// Tokenize the input into a vector of tokens.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        while let Some(&ch) = self.chars.peek() {
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    self.chars.next();
                }
                '&' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'&') {
                        self.chars.next();
                        tokens.push(Token::And);
                    } else {
                        return Err(ParseError::UnexpectedChar('&'));
                    }
                }
                '|' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'|') {
                        self.chars.next();
                        tokens.push(Token::Or);
                    } else {
                        return Err(ParseError::UnexpectedChar('|'));
                    }
                }
                '!' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'=') {
                        self.chars.next();
                        tokens.push(Token::Ne);
                    } else {
                        tokens.push(Token::Not);
                    }
                }
                '=' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'=') {
                        self.chars.next();
                        tokens.push(Token::Eq);
                    } else {
                        return Err(ParseError::UnexpectedChar('='));
                    }
                }
                '>' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'=') {
                        self.chars.next();
                        tokens.push(Token::Ge);
                    } else {
                        tokens.push(Token::Gt);
                    }
                }
                '<' => {
                    self.chars.next();
                    if self.chars.peek() == Some(&'=') {
                        self.chars.next();
                        tokens.push(Token::Le);
                    } else {
                        tokens.push(Token::Lt);
                    }
                }
                '(' => {
                    self.chars.next();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.chars.next();
                    tokens.push(Token::RParen);
                }
                ',' => {
                    self.chars.next();
                    tokens.push(Token::Comma);
                }
                '"' => {
                    tokens.push(self.read_string('"')?);
                }
                '\'' => {
                    tokens.push(self.read_string('\'')?);
                }
                '0'..='9' => {
                    tokens.push(self.read_number()?);
                }
                '-' if self.chars.clone().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                    tokens.push(self.read_number()?);
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(self.read_identifier());
                }
                '$' => {
                    tokens.push(self.read_variable()?);
                }
                _ => {
                    return Err(ParseError::UnexpectedChar(ch));
                }
            }
        }

        tokens.push(Token::Eof);
        Ok(tokens)
    }

    fn read_string(&mut self, quote: char) -> Result<Token, ParseError> {
        self.chars.next(); // consume opening quote
        let mut s = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch == quote {
                self.chars.next();
                return Ok(Token::String(s));
            } else if ch == '\\' {
                self.chars.next();
                if let Some(&escaped) = self.chars.peek() {
                    self.chars.next();
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        c if c == quote => s.push(c),
                        c => {
                            s.push('\\');
                            s.push(c);
                        }
                    }
                }
            } else {
                s.push(ch);
                self.chars.next();
            }
        }

        Err(ParseError::UnterminatedString)
    }

    fn read_number(&mut self) -> Result<Token, ParseError> {
        let mut s = String::new();

        if self.chars.peek() == Some(&'-') {
            s.push('-');
            self.chars.next();
        }

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                s.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        s.parse::<f64>()
            .map(Token::Number)
            .map_err(|_| ParseError::InvalidNumber(s))
    }

    fn read_identifier(&mut self) -> Token {
        let mut s = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        // Check for boolean literals
        match s.as_str() {
            "true" => Token::String("true".to_string()),
            "false" => Token::String("false".to_string()),
            _ => Token::Identifier(s),
        }
    }

    fn read_variable(&mut self) -> Result<Token, ParseError> {
        let mut s = String::new();

        // Read ${...} or $var
        s.push(self.chars.next().unwrap()); // $

        if self.chars.peek() == Some(&'{') {
            s.push(self.chars.next().unwrap()); // {
            while let Some(&ch) = self.chars.peek() {
                s.push(ch);
                self.chars.next();
                if ch == '}' {
                    break;
                }
            }
        } else {
            // Simple $var
            while let Some(&ch) = self.chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    s.push(ch);
                    self.chars.next();
                } else {
                    break;
                }
            }
        }

        Ok(Token::String(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_comparison() {
        let mut tokenizer = Tokenizer::new("'a' == 'b'");
        let tokens = tokenizer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4); // String, Eq, String, Eof
        assert!(matches!(tokens[0], Token::String(_)));
        assert!(matches!(tokens[1], Token::Eq));
    }

    #[test]
    fn test_tokenize_logical() {
        let mut tokenizer = Tokenizer::new("a && b || !c");
        let tokens = tokenizer.tokenize().unwrap();
        assert!(tokens.contains(&Token::And));
        assert!(tokens.contains(&Token::Or));
        assert!(tokens.contains(&Token::Not));
    }

    #[test]
    fn test_tokenize_numbers() {
        let mut tokenizer = Tokenizer::new("10 >= -5");
        let tokens = tokenizer.tokenize().unwrap();
        assert!(matches!(tokens[0], Token::Number(n) if n == 10.0));
        assert!(matches!(tokens[1], Token::Ge));
        assert!(matches!(tokens[2], Token::Number(n) if n == -5.0));
    }

    #[test]
    fn test_tokenize_function() {
        let mut tokenizer = Tokenizer::new("contains('hello', 'ell')");
        let tokens = tokenizer.tokenize().unwrap();
        assert!(matches!(&tokens[0], Token::Identifier(s) if s == "contains"));
        assert!(matches!(tokens[1], Token::LParen));
    }

    #[test]
    fn test_tokenize_variable() {
        let mut tokenizer = Tokenizer::new("${task}");
        let tokens = tokenizer.tokenize().unwrap();
        assert!(matches!(&tokens[0], Token::String(s) if s == "${task}"));
    }
}
