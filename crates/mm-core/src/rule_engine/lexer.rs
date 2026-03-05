// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine Lexer (Tokenizer)
//
// Converts a raw template string into a flat sequence of tokens.
// Recognises:
//   - `<TagName>` — angle-bracket tag references
//   - `$FuncName` — dollar-sign function names
//   - `(`, `)`, `,` — function argument delimiters
//   - `"quoted literals"` — string literals inside function args
//   - bare text — any other character sequence
//   - `{legacy}` — legacy curly-brace syntax (emitted as Literal for compat)
//
// The lexer is a single-pass, character-by-character state machine with no
// backtracking.  It never panics — all error conditions are returned as
// `MmError::RuleEngine`.
//
// License: GPL-2.0-or-later

use crate::error::{MmError, MmResult};

// ───────────────────────────────────────────────────────────────────────────
// Token type
// ───────────────────────────────────────────────────────────────────────────

/// A single token produced by the template lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Bare text or unrecognised characters (including legacy `{key}`)
    Literal(String),
    /// `<TagName>` — the angle brackets are consumed; inner string is the tag name
    Tag(String),
    /// `$FunctionName` — the dollar sign is consumed; inner string is the name
    FuncName(String),
    /// Opening parenthesis `(`
    LParen,
    /// Closing parenthesis `)`
    RParen,
    /// Comma `,` separating function arguments
    Comma,
    /// `"quoted string"` — quotes are consumed; inner string is the value
    QuotedLiteral(String),
    /// End-of-input sentinel
    Eof,
}

// ───────────────────────────────────────────────────────────────────────────
// Lexer struct
// ───────────────────────────────────────────────────────────────────────────

/// Character-by-character tokenizer for MusicBee-inspired templates.
pub struct Lexer {
    /// Source characters (collected from the template string)
    chars: Vec<char>,
    /// Current read position within `chars`
    pos: usize,
}

impl Lexer {
    /// Create a new lexer for the given template string.
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    /// Tokenize the entire input and return all tokens (including final Eof).
    pub fn tokenize(&mut self) -> MmResult<Vec<Token>> {
        // Accumulate tokens from the input stream
        let mut tokens = Vec::new();
        loop {
            // Read the next token from the character stream
            let tok = self.next_token()?;
            // Check if we have reached end of input
            let is_eof = tok == Token::Eof;
            // Always push the token (including Eof) into the output
            tokens.push(tok);
            // Stop after Eof
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// Consume characters and produce the next token.
    fn next_token(&mut self) -> MmResult<Token> {
        // End of input check
        if self.pos >= self.chars.len() {
            return Ok(Token::Eof);
        }

        // Peek at the current character to decide which token to produce
        match self.chars[self.pos] {
            // ── Angle-bracket tag: <TagName> ──
            '<' => self.read_tag(),
            // ── Dollar-sign function: $FuncName ──
            '$' => self.read_func_name(),
            // ── Opening parenthesis ──
            '(' => {
                self.pos += 1; // consume the '('
                Ok(Token::LParen)
            }
            // ── Closing parenthesis ──
            ')' => {
                self.pos += 1; // consume the ')'
                Ok(Token::RParen)
            }
            // ── Comma separator ──
            ',' => {
                self.pos += 1; // consume the ','
                Ok(Token::Comma)
            }
            // ── Quoted literal: "text" ──
            '"' => self.read_quoted_literal(),
            // ── Anything else: accumulate bare text ──
            _ => self.read_literal(),
        }
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Read a `<TagName>` token.  Consumes `<`, inner text, and `>`.
    fn read_tag(&mut self) -> MmResult<Token> {
        // Skip the opening '<'
        self.pos += 1;
        // Collect characters until we find '>'
        let start = self.pos;
        while self.pos < self.chars.len() && self.chars[self.pos] != '>' {
            self.pos += 1;
        }
        // Must have a closing '>'
        if self.pos >= self.chars.len() {
            return Err(MmError::RuleEngine(
                "unclosed tag: missing '>' in template".into(),
            ));
        }
        // Extract the tag name between '<' and '>'
        let name: String = self.chars[start..self.pos].iter().collect();
        // Skip the closing '>'
        self.pos += 1;
        // Empty tag names are an error
        if name.is_empty() {
            return Err(MmError::RuleEngine(
                "empty tag name: '<>' is not allowed".into(),
            ));
        }
        Ok(Token::Tag(name))
    }

    /// Read a `$FuncName` token.  Consumes `$` and then alphanumeric/underscore chars.
    fn read_func_name(&mut self) -> MmResult<Token> {
        // Skip the '$'
        self.pos += 1;
        // Collect alphanumeric and underscore characters for the function name
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_alphanumeric() || self.chars[self.pos] == '_')
        {
            self.pos += 1;
        }
        // Extract the function name
        let name: String = self.chars[start..self.pos].iter().collect();
        // Empty function name is an error (lone '$')
        if name.is_empty() {
            return Err(MmError::RuleEngine(
                "empty function name: '$' must be followed by a name".into(),
            ));
        }
        Ok(Token::FuncName(name))
    }

    /// Read a `"quoted literal"` token.  Consumes opening `"`, content, and closing `"`.
    /// Supports `\"` escape for embedded quotes.
    fn read_quoted_literal(&mut self) -> MmResult<Token> {
        // Skip the opening '"'
        self.pos += 1;
        // Collect characters until we find an unescaped closing '"'
        let mut value = String::new();
        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            // Backslash escape: consume the next character literally
            if ch == '\\' && self.pos + 1 < self.chars.len() {
                self.pos += 1; // skip backslash
                value.push(self.chars[self.pos]); // push escaped char
                self.pos += 1;
                continue;
            }
            // Unescaped closing quote ends the literal
            if ch == '"' {
                self.pos += 1; // skip closing '"'
                return Ok(Token::QuotedLiteral(value));
            }
            // Regular character — accumulate
            value.push(ch);
            self.pos += 1;
        }
        // Reached end of input without a closing quote
        Err(MmError::RuleEngine(
            "unclosed quoted literal: missing closing '\"' in template".into(),
        ))
    }

    /// Read a run of bare text (anything that is not a special character).
    /// Legacy `{key}` syntax is consumed as part of the literal text.
    fn read_literal(&mut self) -> MmResult<Token> {
        let mut text = String::new();
        while self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            // Stop at any special character that starts a different token
            if matches!(ch, '<' | '$' | '(' | ')' | ',' | '"') {
                break;
            }
            // Accumulate the character into the literal
            text.push(ch);
            self.pos += 1;
        }
        Ok(Token::Literal(text))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Convenience function
// ───────────────────────────────────────────────────────────────────────────

/// Tokenize a template string into a vector of tokens.
///
/// This is a convenience wrapper around `Lexer::new(input).tokenize()`.
pub fn tokenize(input: &str) -> MmResult<Vec<Token>> {
    Lexer::new(input).tokenize()
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty string produces only Eof
    #[test]
    fn empty_input() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens, vec![Token::Eof]);
    }

    /// Single literal text
    #[test]
    fn single_literal() {
        let tokens = tokenize("hello world").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Literal("hello world".into()), Token::Eof]
        );
    }

    /// Single angle-bracket tag
    #[test]
    fn single_tag() {
        let tokens = tokenize("<Artist>").unwrap();
        assert_eq!(tokens, vec![Token::Tag("Artist".into()), Token::Eof]);
    }

    /// Tag preserves case and spaces
    #[test]
    fn tag_preserves_content() {
        let tokens = tokenize("<Album Artist>").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Tag("Album Artist".into()), Token::Eof]
        );
    }

    /// Function name token
    #[test]
    fn func_name() {
        let tokens = tokenize("$If").unwrap();
        assert_eq!(tokens, vec![Token::FuncName("If".into()), Token::Eof]);
    }

    /// Function name with parens
    #[test]
    fn func_name_with_parens() {
        let tokens = tokenize("$If()").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::FuncName("If".into()),
                Token::LParen,
                Token::RParen,
                Token::Eof,
            ]
        );
    }

    /// Quoted literal
    #[test]
    fn quoted_literal() {
        let tokens = tokenize("\"hello\"").unwrap();
        assert_eq!(
            tokens,
            vec![Token::QuotedLiteral("hello".into()), Token::Eof]
        );
    }

    /// Quoted literal with escape
    #[test]
    fn quoted_literal_escape() {
        let tokens = tokenize("\"say \\\"hi\\\"\"").unwrap();
        assert_eq!(
            tokens,
            vec![Token::QuotedLiteral("say \"hi\"".into()), Token::Eof]
        );
    }

    /// Comma token
    #[test]
    fn comma_token() {
        let tokens = tokenize(",").unwrap();
        assert_eq!(tokens, vec![Token::Comma, Token::Eof]);
    }

    /// Parenthesis tokens
    #[test]
    fn paren_tokens() {
        let tokens = tokenize("()").unwrap();
        assert_eq!(tokens, vec![Token::LParen, Token::RParen, Token::Eof]);
    }

    /// Mixed content: tag + literal + tag
    #[test]
    fn mixed_tag_literal_tag() {
        let tokens = tokenize("<Artist>/<Album>").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Tag("Artist".into()),
                Token::Literal("/".into()),
                Token::Tag("Album".into()),
                Token::Eof,
            ]
        );
    }

    /// Full function call: $If(<Artist>,"Unknown")
    #[test]
    fn full_function_call() {
        let tokens = tokenize("$If(<Artist>,\"Unknown\")").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::FuncName("If".into()),
                Token::LParen,
                Token::Tag("Artist".into()),
                Token::Comma,
                Token::QuotedLiteral("Unknown".into()),
                Token::RParen,
                Token::Eof,
            ]
        );
    }

    /// Legacy curly-brace syntax passes through as Literal
    #[test]
    fn legacy_curly_brace() {
        let tokens = tokenize("{artist}").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Literal("{artist}".into()), Token::Eof]
        );
    }

    /// Unclosed tag returns error
    #[test]
    fn unclosed_tag_error() {
        let err = tokenize("<Artist").unwrap_err();
        assert!(err.to_string().contains("unclosed tag"));
    }

    /// Empty tag name returns error
    #[test]
    fn empty_tag_error() {
        let err = tokenize("<>").unwrap_err();
        assert!(err.to_string().contains("empty tag name"));
    }

    /// Unclosed quote returns error
    #[test]
    fn unclosed_quote_error() {
        let err = tokenize("\"hello").unwrap_err();
        assert!(err.to_string().contains("unclosed quoted literal"));
    }

    /// Lone dollar sign returns error
    #[test]
    fn lone_dollar_error() {
        let err = tokenize("$").unwrap_err();
        assert!(err.to_string().contains("empty function name"));
    }

    /// Dollar sign followed by non-alphanumeric returns error
    #[test]
    fn dollar_followed_by_special_error() {
        let err = tokenize("$(").unwrap_err();
        assert!(err.to_string().contains("empty function name"));
    }

    /// Unicode in literal text
    #[test]
    fn unicode_literal() {
        let tokens = tokenize("Ärzte/Björk").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Literal("Ärzte/Björk".into()), Token::Eof]
        );
    }

    /// Unicode in tag name
    #[test]
    fn unicode_tag_name() {
        let tokens = tokenize("<Künstler>").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Tag("Künstler".into()), Token::Eof]
        );
    }

    /// Multiple consecutive tags with no separator
    #[test]
    fn consecutive_tags() {
        let tokens = tokenize("<Artist><Album>").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Tag("Artist".into()),
                Token::Tag("Album".into()),
                Token::Eof,
            ]
        );
    }

    /// Nested function call tokenizes correctly
    #[test]
    fn nested_function_tokens() {
        let tokens = tokenize("$If($IsNull(<Disc#>),\"\",$Pad(<Disc#>,\"2\"))").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::FuncName("If".into()),
                Token::LParen,
                Token::FuncName("IsNull".into()),
                Token::LParen,
                Token::Tag("Disc#".into()),
                Token::RParen,
                Token::Comma,
                Token::QuotedLiteral("".into()),
                Token::Comma,
                Token::FuncName("Pad".into()),
                Token::LParen,
                Token::Tag("Disc#".into()),
                Token::Comma,
                Token::QuotedLiteral("2".into()),
                Token::RParen,
                Token::RParen,
                Token::Eof,
            ]
        );
    }

    /// Real-world MusicBee-style template
    #[test]
    fn real_world_template() {
        let tokens = tokenize("<Album Artist>/<Album>/<Title>").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Tag("Album Artist".into()),
                Token::Literal("/".into()),
                Token::Tag("Album".into()),
                Token::Literal("/".into()),
                Token::Tag("Title".into()),
                Token::Eof,
            ]
        );
    }

    /// Backslash path separators in literal text
    #[test]
    fn backslash_separators() {
        let tokens = tokenize("<Artist>\\<Album>").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Tag("Artist".into()),
                Token::Literal("\\".into()),
                Token::Tag("Album".into()),
                Token::Eof,
            ]
        );
    }

    /// Function name with underscores
    #[test]
    fn func_name_with_underscore() {
        let tokens = tokenize("$First_Value").unwrap();
        assert_eq!(
            tokens,
            vec![Token::FuncName("First_Value".into()), Token::Eof]
        );
    }
}
