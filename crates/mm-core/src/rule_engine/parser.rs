// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine Parser (Recursive Descent)
//
// Transforms a flat token stream from the lexer into an abstract syntax tree
// (AST) of `Node` variants.  The parser is a hand-written recursive descent
// parser — no parser-generator crate is used.
//
// Key features:
//   - Produces `Node::Literal`, `Node::Tag`, `Node::FuncCall`, `Node::Sequence`
//   - 50-level nesting depth guard to prevent stack overflow on pathological input
//   - Legacy `{placeholder}` detection with `tracing::warn!()` guidance
//
// License: GPL-2.0-or-later

use regex::Regex;
use std::sync::OnceLock;

use crate::error::{MmError, MmResult};
use super::lexer::{Token, tokenize};

// ───────────────────────────────────────────────────────────────────────────
// Constants
// ───────────────────────────────────────────────────────────────────────────

/// Maximum nesting depth for function calls.  Templates deeper than this
/// return an `MmError::RuleEngine` to prevent stack overflow.
const MAX_DEPTH: usize = 50;

// ───────────────────────────────────────────────────────────────────────────
// AST node types
// ───────────────────────────────────────────────────────────────────────────

/// A node in the template AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A run of literal text (from `Token::Literal` or `Token::QuotedLiteral`)
    Literal(String),
    /// A `<TagName>` reference — name is the raw tag name string
    Tag(String),
    /// A `$Func(arg1, arg2, ...)` call — name is the function name, args are sub-trees
    FuncCall {
        /// Function name (e.g. "If", "Pad", "Upper")
        name: String,
        /// Evaluated argument sub-trees (each arg is itself a Node)
        args: Vec<Node>,
    },
    /// An ordered sequence of sibling nodes (the root of a parsed template)
    Sequence(Vec<Node>),
}

// ───────────────────────────────────────────────────────────────────────────
// Parser struct
// ───────────────────────────────────────────────────────────────────────────

/// Recursive descent parser that converts a token stream into an AST.
struct Parser {
    /// Token stream from the lexer
    tokens: Vec<Token>,
    /// Current read position within `tokens`
    pos: usize,
}

impl Parser {
    /// Create a new parser from a pre-lexed token stream.
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Peek at the current token without consuming it.
    fn peek(&self) -> &Token {
        // Return Eof if we've exhausted the stream
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    /// Consume the current token and advance the position.
    fn advance(&mut self) -> Token {
        // Clone and advance (tokens are small enums, Clone is cheap)
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    /// Parse a sequence of nodes until we hit `Eof`, `)`, or `,`.
    /// The `depth` parameter tracks nesting for the depth guard.
    fn parse_sequence(&mut self, depth: usize) -> MmResult<Vec<Node>> {
        let mut nodes = Vec::new();
        loop {
            // Check what token is next — stop at boundaries
            match self.peek() {
                // End of input or end of function arg — stop
                Token::Eof | Token::RParen | Token::Comma => break,
                // Otherwise, parse the next node
                _ => {
                    let node = self.parse_node(depth)?;
                    nodes.push(node);
                }
            }
        }
        Ok(nodes)
    }

    /// Parse a single node from the token stream.
    fn parse_node(&mut self, depth: usize) -> MmResult<Node> {
        match self.peek().clone() {
            // Literal text — consume and wrap
            Token::Literal(text) => {
                self.advance(); // consume the literal token
                Ok(Node::Literal(text))
            }
            // Quoted literal — consume and wrap as Literal node
            Token::QuotedLiteral(text) => {
                self.advance(); // consume the quoted literal token
                Ok(Node::Literal(text))
            }
            // Tag reference — consume and wrap
            Token::Tag(name) => {
                self.advance(); // consume the tag token
                Ok(Node::Tag(name))
            }
            // Function call — parse name + args
            Token::FuncName(name) => {
                // Check depth guard before recursing
                if depth >= MAX_DEPTH {
                    return Err(MmError::RuleEngine(format!(
                        "template nesting too deep: exceeded {MAX_DEPTH} levels"
                    )));
                }
                self.advance(); // consume the function name token
                // Expect an opening parenthesis
                match self.peek() {
                    Token::LParen => {
                        self.advance(); // consume '('
                    }
                    _ => {
                        return Err(MmError::RuleEngine(format!(
                            "expected '(' after function name '${name}'"
                        )));
                    }
                }
                // Parse function arguments (comma-separated sequences)
                let args = self.parse_func_args(depth + 1)?;
                // Expect a closing parenthesis
                match self.peek() {
                    Token::RParen => {
                        self.advance(); // consume ')'
                    }
                    _ => {
                        return Err(MmError::RuleEngine(format!(
                            "expected ')' to close function '${name}'"
                        )));
                    }
                }
                Ok(Node::FuncCall { name, args })
            }
            // Unexpected tokens
            Token::LParen => {
                self.advance();
                Err(MmError::RuleEngine(
                    "unexpected '(' outside of a function call".into(),
                ))
            }
            tok => {
                self.advance();
                Err(MmError::RuleEngine(format!(
                    "unexpected token in template: {tok:?}"
                )))
            }
        }
    }

    /// Parse the argument list of a function call.
    /// Each argument is a sequence of nodes; arguments are separated by commas.
    fn parse_func_args(&mut self, depth: usize) -> MmResult<Vec<Node>> {
        let mut args = Vec::new();

        // Handle empty argument list: $Func()
        if matches!(self.peek(), Token::RParen) {
            return Ok(args);
        }

        // Parse the first argument
        let nodes = self.parse_sequence(depth)?;
        args.push(Self::wrap_sequence(nodes));

        // Parse subsequent comma-separated arguments
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume ','
            let nodes = self.parse_sequence(depth)?;
            args.push(Self::wrap_sequence(nodes));
        }

        Ok(args)
    }

    /// Wrap a vector of nodes: if exactly one node, return it directly;
    /// otherwise wrap in a `Node::Sequence`.  Empty vectors become empty literals.
    fn wrap_sequence(nodes: Vec<Node>) -> Node {
        match nodes.len() {
            // Empty argument → empty literal
            0 => Node::Literal(String::new()),
            // Single node → unwrap
            1 => nodes.into_iter().next().unwrap(),
            // Multiple nodes → wrap in Sequence
            _ => Node::Sequence(nodes),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Legacy syntax detection
// ───────────────────────────────────────────────────────────────────────────

/// Compiled regex for detecting legacy `{placeholder}` syntax.
/// Uses `OnceLock` to compile only once across all calls.
fn legacy_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\{([^}]+)\}").expect("legacy regex must compile")
    })
}

/// Detect legacy `{placeholder}` patterns in a template string.
/// Returns a list of the placeholder names found (without braces).
pub fn detect_legacy_syntax(template: &str) -> Vec<String> {
    legacy_regex()
        .captures_iter(template)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
// Public API
// ───────────────────────────────────────────────────────────────────────────

/// Parse a template string into an AST.
///
/// This function lexes the input, detects (and warns about) legacy `{key}`
/// syntax, and then runs the recursive descent parser to produce a `Node` tree.
///
/// # Errors
///
/// Returns `MmError::RuleEngine` if:
/// - The lexer encounters unclosed tags or quotes
/// - The parser encounters unexpected tokens or unclosed function calls
/// - Nesting depth exceeds 50 levels
pub fn parse_template(template: &str) -> MmResult<Node> {
    // Detect and warn about legacy {key} syntax
    let legacy = detect_legacy_syntax(template);
    for key in &legacy {
        tracing::warn!(
            key = %key,
            "legacy {{key}} syntax detected: migrate to <{key}> angle-bracket syntax"
        );
    }

    // Tokenize the input
    let tokens = tokenize(template)?;

    // Run the recursive descent parser
    let mut parser = Parser::new(tokens);
    let nodes = parser.parse_sequence(0)?;

    // Wrap the top-level node list
    Ok(Parser::wrap_sequence(nodes))
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty template produces an empty literal
    #[test]
    fn empty_template() {
        let node = parse_template("").unwrap();
        assert_eq!(node, Node::Literal(String::new()));
    }

    /// Pure literal text
    #[test]
    fn pure_literal() {
        let node = parse_template("hello world").unwrap();
        assert_eq!(node, Node::Literal("hello world".into()));
    }

    /// Single tag reference
    #[test]
    fn single_tag() {
        let node = parse_template("<Artist>").unwrap();
        assert_eq!(node, Node::Tag("Artist".into()));
    }

    /// Sequence of tag + literal + tag
    #[test]
    fn tag_literal_tag_sequence() {
        let node = parse_template("<Artist>/<Album>").unwrap();
        assert_eq!(
            node,
            Node::Sequence(vec![
                Node::Tag("Artist".into()),
                Node::Literal("/".into()),
                Node::Tag("Album".into()),
            ])
        );
    }

    /// Function call with no arguments
    #[test]
    fn func_no_args() {
        let node = parse_template("$MediaClass()").unwrap();
        assert_eq!(
            node,
            Node::FuncCall {
                name: "MediaClass".into(),
                args: vec![],
            }
        );
    }

    /// Function call with one literal argument
    #[test]
    fn func_one_literal_arg() {
        let node = parse_template("$Upper(\"hello\")").unwrap();
        assert_eq!(
            node,
            Node::FuncCall {
                name: "Upper".into(),
                args: vec![Node::Literal("hello".into())],
            }
        );
    }

    /// Function call with one tag argument
    #[test]
    fn func_one_tag_arg() {
        let node = parse_template("$Upper(<Artist>)").unwrap();
        assert_eq!(
            node,
            Node::FuncCall {
                name: "Upper".into(),
                args: vec![Node::Tag("Artist".into())],
            }
        );
    }

    /// Function call with multiple arguments
    #[test]
    fn func_multiple_args() {
        let node = parse_template("$If(<Artist>,\"yes\",\"no\")").unwrap();
        assert_eq!(
            node,
            Node::FuncCall {
                name: "If".into(),
                args: vec![
                    Node::Tag("Artist".into()),
                    Node::Literal("yes".into()),
                    Node::Literal("no".into()),
                ],
            }
        );
    }

    /// Nested function call
    #[test]
    fn nested_function() {
        let node = parse_template("$If($IsNull(<Artist>),\"Unknown\",<Artist>)").unwrap();
        assert_eq!(
            node,
            Node::FuncCall {
                name: "If".into(),
                args: vec![
                    Node::FuncCall {
                        name: "IsNull".into(),
                        args: vec![Node::Tag("Artist".into())],
                    },
                    Node::Literal("Unknown".into()),
                    Node::Tag("Artist".into()),
                ],
            }
        );
    }

    /// Mixed sequence: literal + tag + function + literal
    #[test]
    fn mixed_sequence() {
        let node = parse_template("Music/<Artist>/$Upper(<Album>)/track").unwrap();
        assert_eq!(
            node,
            Node::Sequence(vec![
                Node::Literal("Music/".into()),
                Node::Tag("Artist".into()),
                Node::Literal("/".into()),
                Node::FuncCall {
                    name: "Upper".into(),
                    args: vec![Node::Tag("Album".into())],
                },
                Node::Literal("/track".into()),
            ])
        );
    }

    /// Depth guard triggers at 51 levels
    #[test]
    fn depth_guard_triggers() {
        // Build a template 51 levels deep: $A($A($A(...)))
        let mut template = "<X>".to_string();
        for _ in 0..=MAX_DEPTH {
            template = format!("$A({template})");
        }
        let err = parse_template(&template).unwrap_err();
        assert!(err.to_string().contains("nesting too deep"));
    }

    /// Depth guard does NOT trigger at exactly MAX_DEPTH levels
    #[test]
    fn depth_guard_at_limit() {
        // Build a template exactly MAX_DEPTH levels deep
        let mut template = "<X>".to_string();
        for _ in 0..MAX_DEPTH {
            template = format!("$A({template})");
        }
        // This should succeed (we haven't exceeded the limit)
        let result = parse_template(&template);
        assert!(result.is_ok());
    }

    /// Unclosed function paren returns error
    #[test]
    fn unclosed_func_paren() {
        let err = parse_template("$If(<Artist>").unwrap_err();
        assert!(err.to_string().contains("expected ')'"));
    }

    /// Missing opening paren after function name
    #[test]
    fn missing_func_paren() {
        let err = parse_template("$If<Artist>").unwrap_err();
        assert!(err.to_string().contains("expected '('"));
    }

    /// Legacy {key} syntax detected but does not error
    #[test]
    fn legacy_syntax_no_error() {
        let node = parse_template("{artist}/{album}").unwrap();
        // Legacy braces are treated as literal text
        assert_eq!(
            node,
            Node::Literal("{artist}/{album}".into())
        );
    }

    /// detect_legacy_syntax finds curly-brace placeholders
    #[test]
    fn detect_legacy_finds_keys() {
        let keys = detect_legacy_syntax("{artist}/{album}/{title}");
        assert_eq!(keys, vec!["artist", "album", "title"]);
    }

    /// detect_legacy_syntax returns empty for modern syntax
    #[test]
    fn detect_legacy_empty_for_modern() {
        let keys = detect_legacy_syntax("<Artist>/<Album>");
        assert!(keys.is_empty());
    }

    /// Function arg containing tag + literal concatenation
    #[test]
    fn func_arg_concat() {
        let node = parse_template("$If(<Artist>,<Artist>\" - \"<Album>,\"Unknown\")").unwrap();
        // The second arg is a Sequence of Tag + Literal + Tag
        match &node {
            Node::FuncCall { name, args } => {
                assert_eq!(name, "If");
                assert_eq!(args.len(), 3);
                assert_eq!(
                    args[1],
                    Node::Sequence(vec![
                        Node::Tag("Artist".into()),
                        Node::Literal(" - ".into()),
                        Node::Tag("Album".into()),
                    ])
                );
            }
            _ => panic!("expected FuncCall, got {:?}", node),
        }
    }

    /// Adjacent functions
    #[test]
    fn adjacent_functions() {
        let node = parse_template("$Upper(<A>)$Lower(<B>)").unwrap();
        assert_eq!(
            node,
            Node::Sequence(vec![
                Node::FuncCall {
                    name: "Upper".into(),
                    args: vec![Node::Tag("A".into())],
                },
                Node::FuncCall {
                    name: "Lower".into(),
                    args: vec![Node::Tag("B".into())],
                },
            ])
        );
    }

    /// Real-world MusicBee template
    #[test]
    fn real_world_musicbee_template() {
        let template = "<Album Artist>/<Album>/$If($IsNull(<Disc#>),\"\",\"Disc \"$Pad(<Disc#>,\"2\")\" - \")<Title>";
        let node = parse_template(template).unwrap();
        // Just verify it parses without error and produces the right structure
        match &node {
            Node::Sequence(nodes) => {
                // Should have: Tag, Literal("/"), Tag, Literal("/"), FuncCall, Tag
                assert!(nodes.len() >= 5);
                assert_eq!(nodes[0], Node::Tag("Album Artist".into()));
            }
            _ => panic!("expected Sequence, got {:?}", node),
        }
    }

    /// Whitespace-only literal preserved
    #[test]
    fn whitespace_literal() {
        let node = parse_template("   ").unwrap();
        assert_eq!(node, Node::Literal("   ".into()));
    }

    /// Function name case preservation
    #[test]
    fn func_name_case_preserved() {
        let node = parse_template("$IsNull(<X>)").unwrap();
        match node {
            Node::FuncCall { name, .. } => assert_eq!(name, "IsNull"),
            _ => panic!("expected FuncCall"),
        }
    }

    /// Comma inside quoted literal is NOT an argument separator
    #[test]
    fn comma_in_quoted_literal() {
        let node = parse_template("$If(<X>,\"a, b\",\"c\")").unwrap();
        match &node {
            Node::FuncCall { args, .. } => {
                assert_eq!(args.len(), 3);
                assert_eq!(args[1], Node::Literal("a, b".into()));
            }
            _ => panic!("expected FuncCall"),
        }
    }

    /// Deeply nested but within limit
    #[test]
    fn deeply_nested_within_limit() {
        // 10 levels deep — well within the 50-level limit
        let template = "$A($B($C($D($E($F($G($H($I($J(<X>))))))))))";
        let node = parse_template(template).unwrap();
        // Verify outermost is FuncCall named "A"
        match &node {
            Node::FuncCall { name, .. } => assert_eq!(name, "A"),
            _ => panic!("expected FuncCall"),
        }
    }

    /// Multiple tags in a function argument
    #[test]
    fn multiple_tags_in_arg() {
        let node = parse_template("$Upper(<First><Last>)").unwrap();
        match &node {
            Node::FuncCall { args, .. } => {
                assert_eq!(args.len(), 1);
                assert_eq!(
                    args[0],
                    Node::Sequence(vec![
                        Node::Tag("First".into()),
                        Node::Tag("Last".into()),
                    ])
                );
            }
            _ => panic!("expected FuncCall"),
        }
    }
}
