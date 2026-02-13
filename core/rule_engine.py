# ============================================================================
# File: /core/rule_engine.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# MusicBee-inspired template engine for MeedyaManager's file renaming system.
# Implements a full recursive descent parser that handles:
#   - Tag references: <Title>, <Album Artist>, <Custom:AnyName>
#   - Function calls: $If(), $Pad(), $Replace(), etc. (20+ functions)
#   - Comparison operators: =, >, < (for $If criteria)
#   - Deep nesting: $If($And(...), ..., $If($Or(...), ..., ...))
#   - Literal text: folder separators, dashes, spaces, etc.
#
# Architecture: Three-stage pipeline
#   Template String → Lexer (tokens) → Parser (AST) → Evaluator (string)
#
# References:
#   - MusicBee template syntax: https://musicbee.fandom.com/wiki/Templates
#   - help/rule-syntax.md (MeedyaManager syntax reference)
# ============================================================================

import re                                          # Regex for $RxReplace, $IsMatch
import logging                                     # Structured logging
from enum import Enum                              # Token type enumeration
from dataclasses import dataclass, field           # Immutable data containers
from typing import Any                             # Type hints

from core.tag_registry import (
    resolve_tag,                                   # Look up tag value in metadata
    is_valid_tag,                                  # Check if tag name is recognized
    TAG_MAP,                                       # Display → internal mapping
)


# Logger for rule engine diagnostics
logger = logging.getLogger("MeedyaManager.RuleEngine")


# ============================================================================
# Error Classes — Distinct errors for syntax vs evaluation problems
# ============================================================================

class TemplateSyntaxError(Exception):
    """Raised when a template string has invalid syntax (parsing stage)."""

    def __init__(self, message, position=None):
        """
        Args:
            message (str): Description of the syntax error
            position (int, optional): Character position in the template where
                                      the error occurred
        """
        self.position = position                   # Character index in template
        if position is not None:
            super().__init__(f"Syntax error at position {position}: {message}")
        else:
            super().__init__(f"Syntax error: {message}")


class TemplateEvalError(Exception):
    """Raised when a template fails during evaluation (runtime stage)."""
    pass


# ============================================================================
# Token Types — Categories of lexical elements in a template
# ============================================================================

class TokenType(Enum):
    """Enumeration of all token types produced by the lexer."""
    LITERAL = "LITERAL"         # Plain text (folder names, dashes, spaces, etc.)
    TAG = "TAG"                 # <TagName> or <Custom:Name> — tag reference
    FUNC_START = "FUNC_START"   # $FuncName( — function call start
    COMMA = "COMMA"             # , — argument separator inside functions
    RPAREN = "RPAREN"           # ) — closes a function call
    EQUALS = "EQUALS"           # = — equality comparison operator
    GT = "GT"                   # > — greater-than comparison operator
    LT = "LT"                  # < — less-than comparison (only in criteria context)


# ============================================================================
# Token — A single lexical element with type, value, and source position
# ============================================================================

@dataclass
class Token:
    """
    Represents a single token from the lexer output.

    Attributes:
        type (TokenType): The category of this token
        value (str): The token's text content (tag name, function name, etc.)
        position (int): Character offset in the original template string
    """
    type: TokenType                                # Token category
    value: str                                     # Token content
    position: int                                  # Source position for error reporting


# ============================================================================
# AST Node Types — Abstract Syntax Tree nodes for parsed templates
# ============================================================================

@dataclass
class LiteralNode:
    """
    AST node for plain text that is emitted verbatim during evaluation.
    Example: The "/" and " - " in "<Artist>/<Album>/<Track #> - <Title>"
    """
    value: str                                     # The literal text content


@dataclass
class TagNode:
    """
    AST node for a tag reference like <Title> or <Custom:SpotifyURL>.
    During evaluation, replaced with the tag's value from metadata.
    """
    name: str                                      # Display name (e.g., "Album Artist")
    is_custom: bool = False                        # True if Custom:* prefix tag


@dataclass
class FunctionNode:
    """
    AST node for a function call like $If(...), $Pad(...), etc.
    Each argument is itself a TemplateNode (supports nesting).
    """
    name: str                                      # Function name (e.g., "If", "Pad")
    args: list = field(default_factory=list)        # List of TemplateNode arguments


@dataclass
class ComparisonNode:
    """
    AST node for a comparison expression like <Genre>=Rock or <Year>>2000.
    Used inside $If() criteria. Left side is a TagNode or FunctionNode,
    right side is a TemplateNode (the comparison value).
    """
    left: Any                                      # TagNode or FunctionNode
    operator: str                                  # "=", ">", or "<"
    right: Any                                     # TemplateNode (comparison value)


@dataclass
class TemplateNode:
    """
    AST node representing a sequence of child nodes. This is the root
    node type and is also used for function arguments (each arg is a
    TemplateNode containing one or more child nodes).
    """
    children: list = field(default_factory=list)    # [LiteralNode|TagNode|FunctionNode|ComparisonNode]


# ============================================================================
# Lexer — Converts template string into a stream of tokens
# ============================================================================

class Lexer:
    """
    Tokenizes a MeedyaManager template string into a sequence of Token objects.

    Handles context-sensitive disambiguation of angle brackets:
    - '<' followed by valid tag name + '>' → TAG token
    - '<' in other contexts → LT comparison operator
    - '>' after tag close is consumed as part of TAG token
    - '>' in other contexts → GT comparison operator

    Also tracks parenthesis depth to distinguish commas inside function
    calls (COMMA tokens) from literal commas in text (LITERAL tokens).
    """

    def __init__(self, text):
        """
        Args:
            text (str): The template string to tokenize
        """
        self.text = text                           # Source template string
        self.pos = 0                               # Current character position
        self.tokens = []                           # Accumulated token list
        self.paren_depth = 0                       # Parenthesis nesting depth
        self._in_angle_wrapper = False             # Tracking <$Func()> wrapper

    def tokenize(self):
        """
        Process the entire template string and return a list of tokens.

        Returns:
            list[Token]: The complete token stream
        """
        while self.pos < len(self.text):
            ch = self.text[self.pos]

            if ch == '<':
                # Check for <$FuncName(...)> pattern — decorative wrapper
                # around a function call (MusicBee convention)
                if self.pos + 1 < len(self.text) and self.text[self.pos + 1] == '$':
                    # Skip the '<' — it's a decorative wrapper for a function
                    self.pos += 1                  # Advance past '<'
                    # The main loop will now see '$' and parse the function
                    # Track that we need to consume the closing '>' wrapper
                    self._in_angle_wrapper = True
                    continue

                # Try to parse as a tag reference <TagName>
                tag = self._try_parse_tag()
                if tag is not None:
                    self.tokens.append(tag)
                else:
                    # Not a valid tag — emit as LT comparison operator
                    self.tokens.append(Token(TokenType.LT, "<", self.pos))
                    self.pos += 1

            elif ch == '$':
                # Parse as a function call $FuncName(
                func = self._parse_function_start()
                self.tokens.append(func)

            elif ch == ',' and self.paren_depth > 0:
                # Comma inside a function call → argument separator
                self.tokens.append(Token(TokenType.COMMA, ",", self.pos))
                self.pos += 1

            elif ch == ')':
                # Close parenthesis → end of function arguments
                self.paren_depth = max(0, self.paren_depth - 1)
                self.tokens.append(Token(TokenType.RPAREN, ")", self.pos))
                self.pos += 1

                # Consume trailing '>' if this closes a <$Func(...)> wrapper
                if (self._in_angle_wrapper and self.paren_depth == 0
                        and self.pos < len(self.text) and self.text[self.pos] == '>'):
                    self.pos += 1                  # Skip the decorative '>'
                    self._in_angle_wrapper = False

            elif ch == '=':
                # Equality comparison operator
                self.tokens.append(Token(TokenType.EQUALS, "=", self.pos))
                self.pos += 1

            elif ch == '>' and self.paren_depth > 0:
                # Greater-than comparison (only meaningful inside function args)
                self.tokens.append(Token(TokenType.GT, ">", self.pos))
                self.pos += 1

            else:
                # Accumulate literal text until a special character is reached
                literal = self._parse_literal()
                if literal.value:                  # Don't emit empty literals
                    self.tokens.append(literal)

        return self.tokens

    def _try_parse_tag(self):
        """
        Attempt to parse a tag reference starting at current '<'.
        Returns a TAG Token if successful, or None if this is not a valid tag.

        A valid tag is: '<' followed by one or more characters (letters, digits,
        spaces, #, :) followed by '>'. The content between < and > must be
        a recognized tag name or a Custom:* tag.
        """
        start = self.pos                           # Remember start position
        # Look for closing '>' — tag names can contain letters, digits, spaces, #, :
        end = self.text.find('>', self.pos + 1)
        if end == -1:
            return None                            # No closing '>' found

        # Extract the tag name between < and >
        tag_name = self.text[self.pos + 1:end].strip()

        # Validate: must not be empty and must look like a tag name
        # (letters, digits, spaces, #, :, but not start with special chars)
        if not tag_name:
            return None

        # Check if it's a known tag or custom tag
        if is_valid_tag(tag_name) or tag_name in TAG_MAP:
            self.pos = end + 1                     # Move past the closing '>'
            return Token(TokenType.TAG, tag_name, start)

        # Not a recognized tag — might be literal '<' text
        return None

    def _parse_function_start(self):
        """
        Parse a function call start: $FuncName(
        Advances position past the opening parenthesis.

        Returns:
            Token: FUNC_START token with the function name as value

        Raises:
            TemplateSyntaxError: If function syntax is invalid
        """
        start = self.pos                           # Remember $ position
        self.pos += 1                              # Skip the '$'

        # Read function name (alphanumeric characters)
        name_start = self.pos
        while self.pos < len(self.text) and self.text[self.pos].isalpha():
            self.pos += 1

        func_name = self.text[name_start:self.pos]
        if not func_name:
            raise TemplateSyntaxError(
                "Expected function name after '$'", start
            )

        # Expect opening parenthesis
        if self.pos >= len(self.text) or self.text[self.pos] != '(':
            raise TemplateSyntaxError(
                f"Expected '(' after function name '${func_name}'", self.pos
            )

        self.pos += 1                              # Skip the '('
        self.paren_depth += 1                      # Track nesting depth

        return Token(TokenType.FUNC_START, func_name, start)

    def _parse_literal(self):
        """
        Parse literal text until a special character is encountered.
        Special characters that stop literal parsing: < $ , ) = >
        (Commas and close-parens only stop literals inside functions.)

        Returns:
            Token: LITERAL token with the accumulated text
        """
        start = self.pos
        # Characters that end a literal segment
        stop_chars = '<$=)'                        # Always stop on these
        if self.paren_depth > 0:
            stop_chars += ',>'                     # Also stop on comma and > inside functions

        while self.pos < len(self.text):
            ch = self.text[self.pos]
            if ch in stop_chars:
                break
            self.pos += 1

        return Token(TokenType.LITERAL, self.text[start:self.pos], start)


# ============================================================================
# Parser — Builds an AST from a token stream via recursive descent
# ============================================================================

class Parser:
    """
    Recursive descent parser that converts a token stream into an Abstract
    Syntax Tree (AST). Handles arbitrarily deep nesting of function calls
    and comparison expressions.

    Max recursion depth is guarded to prevent stack overflow from malformed
    templates (default limit: 50 levels).
    """

    MAX_DEPTH = 50                                 # Maximum nesting depth guard

    def __init__(self, tokens):
        """
        Args:
            tokens (list[Token]): Token stream from the Lexer
        """
        self.tokens = tokens                       # Input token stream
        self.pos = 0                               # Current token index
        self.depth = 0                             # Current nesting depth

    def parse(self):
        """
        Parse the entire token stream into a TemplateNode AST.

        Returns:
            TemplateNode: The root AST node containing all parsed children
        """
        children = self._parse_sequence()
        return TemplateNode(children)

    def _parse_sequence(self, stop_types=None):
        """
        Parse a sequence of nodes until a stop token type is encountered
        or the end of tokens is reached.

        Args:
            stop_types (set[TokenType], optional): Token types that terminate
                this sequence (e.g., COMMA, RPAREN for function args)

        Returns:
            list: List of AST nodes (LiteralNode, TagNode, FunctionNode, etc.)
        """
        if stop_types is None:
            stop_types = set()

        nodes = []
        while self.pos < len(self.tokens):
            token = self.tokens[self.pos]

            # Check if we've hit a stop token (e.g., comma or close-paren)
            if token.type in stop_types:
                break

            # Parse based on token type
            if token.type == TokenType.LITERAL:
                nodes.append(LiteralNode(token.value))
                self.pos += 1

            elif token.type == TokenType.TAG:
                tag_node = TagNode(
                    name=token.value,
                    is_custom=token.value.startswith("Custom:")
                )
                self.pos += 1

                # Check for comparison operator after tag
                if self.pos < len(self.tokens) and self.tokens[self.pos].type in (
                    TokenType.EQUALS, TokenType.GT, TokenType.LT
                ):
                    comp = self._parse_comparison(tag_node)
                    nodes.append(comp)
                else:
                    nodes.append(tag_node)

            elif token.type == TokenType.FUNC_START:
                func_node = self._parse_function()

                # Check for comparison operator after function (e.g., $Contains(...)=T)
                if self.pos < len(self.tokens) and self.tokens[self.pos].type in (
                    TokenType.EQUALS, TokenType.GT, TokenType.LT
                ):
                    comp = self._parse_comparison(func_node)
                    nodes.append(comp)
                else:
                    nodes.append(func_node)

            elif token.type == TokenType.EQUALS:
                # Bare = without preceding tag/function — treat as literal
                nodes.append(LiteralNode("="))
                self.pos += 1

            elif token.type == TokenType.GT:
                # Bare > without preceding tag — treat as literal
                nodes.append(LiteralNode(">"))
                self.pos += 1

            elif token.type == TokenType.LT:
                # < that wasn't a tag — treat as literal
                nodes.append(LiteralNode("<"))
                self.pos += 1

            else:
                # Unexpected token — skip with warning
                logger.warning(f"Unexpected token: {token}")
                self.pos += 1

        return nodes

    def _parse_function(self):
        """
        Parse a function call: the FUNC_START token has already been identified.
        Recursively parses each comma-separated argument as a TemplateNode.

        Returns:
            FunctionNode: AST node for the function call with parsed arguments

        Raises:
            TemplateSyntaxError: If nesting is too deep or parens are unmatched
        """
        # Guard against excessive nesting depth
        self.depth += 1
        if self.depth > self.MAX_DEPTH:
            raise TemplateSyntaxError(
                f"Maximum nesting depth ({self.MAX_DEPTH}) exceeded",
                self.tokens[self.pos].position if self.pos < len(self.tokens) else None
            )

        token = self.tokens[self.pos]              # The FUNC_START token
        func_name = token.value                    # Function name (e.g., "If", "Pad")
        self.pos += 1                              # Move past FUNC_START

        # Parse function arguments (comma-separated TemplateNodes)
        args = []
        # Stop tokens for argument parsing: COMMA (next arg) or RPAREN (end)
        stop_types = {TokenType.COMMA, TokenType.RPAREN}

        while self.pos < len(self.tokens):
            # Parse one argument as a sequence of nodes
            arg_nodes = self._parse_sequence(stop_types)

            # Wrap the argument nodes in a TemplateNode
            if arg_nodes:
                args.append(TemplateNode(arg_nodes))
            else:
                # Empty argument (e.g., between consecutive commas)
                args.append(TemplateNode([LiteralNode("")]))

            # Check what stopped us
            if self.pos < len(self.tokens):
                next_token = self.tokens[self.pos]
                if next_token.type == TokenType.COMMA:
                    self.pos += 1                  # Skip comma, continue to next arg
                elif next_token.type == TokenType.RPAREN:
                    self.pos += 1                  # Skip closing paren, done
                    break
            else:
                # Ran out of tokens without closing paren
                raise TemplateSyntaxError(
                    f"Unclosed function call '${func_name}('",
                    token.position
                )

        self.depth -= 1                            # Unwind nesting depth
        return FunctionNode(func_name, args)

    def _parse_comparison(self, left_node):
        """
        Parse a comparison expression: <Tag>=Value, <Tag>>Value, <Tag><Value
        The left-hand side (tag or function) has already been parsed.

        Args:
            left_node: The left-hand AST node (TagNode or FunctionNode)

        Returns:
            ComparisonNode: AST node for the comparison
        """
        # Read the operator token
        op_token = self.tokens[self.pos]
        operator = op_token.value                  # "=", ">", or "<"
        self.pos += 1

        # Parse the right-hand side (comparison value)
        # Stop at comma, rparen (inside function), or when hitting another
        # tag/function start
        stop_types = {TokenType.COMMA, TokenType.RPAREN}
        right_nodes = self._parse_sequence(stop_types)

        return ComparisonNode(
            left=left_node,
            operator=operator,
            right=TemplateNode(right_nodes)
        )


# ============================================================================
# Evaluator / Rule Engine — Walks AST and produces output string
# ============================================================================

# Sort words to strip in $Sort() function (case-insensitive)
SORT_WORDS = ["the ", "a ", "an "]

# Date format token mapping: MusicBee → Python strftime
DATE_FORMAT_MAP = {
    "yyyy": "%Y",                                  # 4-digit year
    "yy": "%y",                                    # 2-digit year
    "MM": "%m",                                    # 2-digit month
    "dd": "%d",                                    # 2-digit day
    "hh": "%H",                                    # 2-digit hour (24h)
    "mm": "%M",                                    # 2-digit minute
    "ss": "%S",                                    # 2-digit second
}


class RuleEngine:
    """
    MeedyaManager's template evaluation engine.

    Parses MusicBee-style templates and evaluates them against a metadata
    dictionary to produce file paths. Supports 20+ built-in functions,
    conditional logic, string manipulation, and deep nesting.

    Usage:
        engine = RuleEngine()
        result = engine.evaluate(
            "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",
            {"title": "Song", "artist": "Queen", ...}
        )
    """

    def __init__(self):
        """Initialize the rule engine with its function dispatch table."""
        # Function dispatch table: maps function names to handler methods
        # Each handler receives (args: list[TemplateNode], metadata: dict)
        self._functions = {
            # Conditional functions
            "If": self._fn_if,
            "And": self._fn_and,
            "Or": self._fn_or,
            # Logic functions
            "IsNull": self._fn_is_null,
            "Contains": self._fn_contains,
            "IsMatch": self._fn_is_match,
            # String functions
            "Replace": self._fn_replace,
            "RxReplace": self._fn_rx_replace,
            "Left": self._fn_left,
            "Right": self._fn_right,
            "Upper": self._fn_upper,
            "Lower": self._fn_lower,
            "Trim": self._fn_trim,
            # Splitting functions
            "Split": self._fn_split,
            "RSplit": self._fn_rsplit,
            "First": self._fn_first,
            # Formatting functions
            "Pad": self._fn_pad,
            "Date": self._fn_date,
            "Sort": self._fn_sort,
            "Group": self._fn_group,
        }

    def evaluate(self, template, metadata):
        """
        Parse and evaluate a template string against a metadata dictionary.

        Args:
            template (str): MusicBee-style template string
            metadata (dict): Metadata dictionary with internal snake_case keys

        Returns:
            str: The evaluated result string (e.g., a file path)

        Raises:
            TemplateSyntaxError: If the template has invalid syntax
            TemplateEvalError: If evaluation fails (e.g., missing required tags)
        """
        # Stage 1: Tokenize the template string
        lexer = Lexer(template)
        tokens = lexer.tokenize()

        # Stage 2: Parse tokens into an AST
        parser = Parser(tokens)
        ast = parser.parse()

        # Stage 3: Evaluate the AST against metadata
        return self._eval_node(ast, metadata)

    def validate(self, template):
        """
        Validate a template's syntax without evaluating it.
        Returns a list of error messages (empty list = valid).

        Args:
            template (str): Template string to validate

        Returns:
            list[str]: List of error descriptions (empty if valid)
        """
        errors = []
        try:
            # Attempt to tokenize
            lexer = Lexer(template)
            tokens = lexer.tokenize()

            # Attempt to parse
            parser = Parser(tokens)
            ast = parser.parse()

            # Check for unknown function names
            self._validate_functions(ast, errors)

        except TemplateSyntaxError as e:
            errors.append(str(e))

        return errors

    def _validate_functions(self, node, errors):
        """
        Recursively check all function calls in the AST for unknown names.

        Args:
            node: An AST node to validate
            errors (list): Accumulates error messages
        """
        if isinstance(node, TemplateNode):
            for child in node.children:
                self._validate_functions(child, errors)
        elif isinstance(node, FunctionNode):
            if node.name not in self._functions:
                errors.append(f"Unknown function: ${node.name}()")
            for arg in node.args:
                self._validate_functions(arg, errors)
        elif isinstance(node, ComparisonNode):
            self._validate_functions(node.left, errors)
            self._validate_functions(node.right, errors)

    # =========================================================================
    # AST Evaluation — Recursive node evaluation
    # =========================================================================

    def _eval_node(self, node, metadata):
        """
        Evaluate an AST node and return its string result.

        Dispatches to the appropriate handler based on node type:
        - TemplateNode: concatenate children
        - LiteralNode: return text
        - TagNode: resolve from metadata
        - FunctionNode: call registered function handler
        - ComparisonNode: evaluate comparison, return result

        Args:
            node: The AST node to evaluate
            metadata (dict): Metadata dictionary

        Returns:
            str: The evaluated string result
        """
        if isinstance(node, TemplateNode):
            # Concatenate the evaluated results of all children
            return "".join(self._eval_node(child, metadata) for child in node.children)

        elif isinstance(node, LiteralNode):
            # Return literal text verbatim
            return node.value

        elif isinstance(node, TagNode):
            # Resolve tag value from metadata
            value = resolve_tag(node.name, metadata)
            if value is not None:
                return str(value)                  # Convert any type to string
            # Tag not found — return empty string (caller can use $IsNull to handle)
            return ""

        elif isinstance(node, FunctionNode):
            # Look up and call the function handler
            handler = self._functions.get(node.name)
            if handler is None:
                raise TemplateEvalError(f"Unknown function: ${node.name}()")
            return handler(node.args, metadata)

        elif isinstance(node, ComparisonNode):
            # Evaluate the comparison and convert bool to string
            # ("True"/"False") so it can be joined with other nodes.
            # _is_truthy() handles "True" as truthy and "False" as falsy.
            result = self._eval_comparison(node, metadata)
            return str(result)

        else:
            # Fallback for unexpected node types
            return ""

    def _eval_comparison(self, node, metadata):
        """
        Evaluate a comparison expression and return the boolean result.

        Handles three operators:
        - "=": string equality (case-insensitive)
        - ">": numeric greater-than (falls back to string comparison)
        - "<": numeric less-than (falls back to string comparison)

        Args:
            node (ComparisonNode): The comparison to evaluate
            metadata (dict): Metadata dictionary

        Returns:
            bool: True if the comparison is satisfied
        """
        # Evaluate left side (tag or function result)
        left_val = self._eval_node(node.left, metadata)
        # Evaluate right side (comparison value)
        right_val = self._eval_node(node.right, metadata)

        # Strip whitespace for clean comparison
        left_val = left_val.strip()
        right_val = right_val.strip()

        if node.operator == "=":
            # Case-insensitive string equality
            return left_val.lower() == right_val.lower()

        elif node.operator == ">":
            # Try numeric comparison first, fall back to string
            try:
                return float(left_val) > float(right_val)
            except (ValueError, TypeError):
                return left_val > right_val

        elif node.operator == "<":
            # Try numeric comparison first, fall back to string
            try:
                return float(left_val) < float(right_val)
            except (ValueError, TypeError):
                return left_val < right_val

        return False                               # Unknown operator

    # =========================================================================
    # Function Implementations — 20 built-in template functions
    # =========================================================================

    # ---- Conditional Functions ----

    def _fn_if(self, args, metadata):
        """
        $If(criteria, trueResult, falseResult)
        Evaluates criteria; returns trueResult if truthy, falseResult otherwise.

        Args must be: [criteria, trueResult, falseResult]
        """
        if len(args) < 3:
            raise TemplateEvalError("$If() requires 3 arguments: criteria, trueResult, falseResult")

        # Evaluate the criteria — may be a ComparisonNode returning bool,
        # or a plain string (truthy if non-empty)
        criteria_result = self._eval_node(args[0], metadata)

        # Determine truthiness: bool True, non-empty string, "T", "True"
        is_true = self._is_truthy(criteria_result)

        if is_true:
            return self._eval_node(args[1], metadata)  # True branch
        else:
            return self._eval_node(args[2], metadata)  # False branch

    def _fn_and(self, args, metadata):
        """
        $And(criteria1, criteria2)
        Returns True if BOTH criteria are truthy. Used inside $If().
        """
        if len(args) < 2:
            raise TemplateEvalError("$And() requires 2 arguments")

        # Evaluate both criteria
        left = self._eval_node(args[0], metadata)
        right = self._eval_node(args[1], metadata)

        result = self._is_truthy(left) and self._is_truthy(right)
        return str(result)                         # Convert bool to "True"/"False"

    def _fn_or(self, args, metadata):
        """
        $Or(criteria1, criteria2)
        Returns True if EITHER criterion is truthy. Used inside $If().
        """
        if len(args) < 2:
            raise TemplateEvalError("$Or() requires 2 arguments")

        left = self._eval_node(args[0], metadata)
        right = self._eval_node(args[1], metadata)

        result = self._is_truthy(left) or self._is_truthy(right)
        return str(result)                         # Convert bool to "True"/"False"

    # ---- Logic Functions ----

    def _fn_is_null(self, args, metadata):
        """
        $IsNull(tag, ifNullResult, ifNotNullResult)
        Returns ifNullResult if the tag is empty/missing, else ifNotNullResult.
        """
        if len(args) < 3:
            raise TemplateEvalError("$IsNull() requires 3 arguments: tag, ifNull, ifNotNull")

        # Evaluate the tag value
        tag_value = self._eval_node(args[0], metadata)

        # Check if null/empty
        if not tag_value or tag_value.strip() == "":
            return self._eval_node(args[1], metadata)  # Null branch
        else:
            return self._eval_node(args[2], metadata)  # Not-null branch

    def _fn_contains(self, args, metadata):
        """
        $Contains(text, searchString)
        Returns "T" if text contains searchString (case-insensitive), else "F".
        """
        if len(args) < 2:
            raise TemplateEvalError("$Contains() requires 2 arguments: text, search")

        text = self._eval_node(args[0], metadata)
        search = self._eval_node(args[1], metadata)

        if search.lower() in text.lower():
            return "T"
        return "F"

    def _fn_is_match(self, args, metadata):
        """
        $IsMatch(text, regexPattern)
        Returns "T" if text matches the regex pattern, else "F".
        """
        if len(args) < 2:
            raise TemplateEvalError("$IsMatch() requires 2 arguments: text, pattern")

        text = self._eval_node(args[0], metadata)
        pattern = self._eval_node(args[1], metadata)

        # Strip surrounding quotes from pattern if present
        pattern = pattern.strip('"').strip("'")

        try:
            if re.search(pattern, text):
                return "T"
        except re.error as e:
            logger.warning(f"Invalid regex in $IsMatch(): {e}")
        return "F"

    # ---- String Functions ----

    def _fn_replace(self, args, metadata):
        """
        $Replace(text, findString, replaceString)
        Replaces all occurrences of findString with replaceString.
        """
        if len(args) < 3:
            raise TemplateEvalError("$Replace() requires 3 arguments: text, find, replace")

        text = self._eval_node(args[0], metadata)
        find = self._eval_node(args[1], metadata)
        replace = self._eval_node(args[2], metadata)

        return text.replace(find, replace)

    def _fn_rx_replace(self, args, metadata):
        """
        $RxReplace(text, regexPattern, replaceString)
        Regex-based find and replace. Pattern and replacement may be quoted.
        """
        if len(args) < 3:
            raise TemplateEvalError("$RxReplace() requires 3 arguments: text, pattern, replace")

        text = self._eval_node(args[0], metadata)
        pattern = self._eval_node(args[1], metadata).strip('"').strip("'")
        replace = self._eval_node(args[2], metadata).strip('"').strip("'")

        try:
            return re.sub(pattern, replace, text)
        except re.error as e:
            logger.warning(f"Invalid regex in $RxReplace(): {e}")
            return text                            # Return original on regex error

    def _fn_left(self, args, metadata):
        """
        $Left(text, n)
        Returns the first n characters of text.
        """
        if len(args) < 2:
            raise TemplateEvalError("$Left() requires 2 arguments: text, count")

        text = self._eval_node(args[0], metadata)
        count_str = self._eval_node(args[1], metadata).strip()

        try:
            count = int(count_str)
            return text[:count]
        except ValueError:
            raise TemplateEvalError(f"$Left() count must be a number, got '{count_str}'")

    def _fn_right(self, args, metadata):
        """
        $Right(text, n)
        Returns the last n characters of text.
        """
        if len(args) < 2:
            raise TemplateEvalError("$Right() requires 2 arguments: text, count")

        text = self._eval_node(args[0], metadata)
        count_str = self._eval_node(args[1], metadata).strip()

        try:
            count = int(count_str)
            return text[-count:] if count > 0 else ""
        except ValueError:
            raise TemplateEvalError(f"$Right() count must be a number, got '{count_str}'")

    def _fn_upper(self, args, metadata):
        """
        $Upper(text)
        Returns the text in uppercase.
        """
        if len(args) < 1:
            raise TemplateEvalError("$Upper() requires 1 argument: text")
        return self._eval_node(args[0], metadata).upper()

    def _fn_lower(self, args, metadata):
        """
        $Lower(text)
        Returns the text in lowercase.
        """
        if len(args) < 1:
            raise TemplateEvalError("$Lower() requires 1 argument: text")
        return self._eval_node(args[0], metadata).lower()

    def _fn_trim(self, args, metadata):
        """
        $Trim(text)
        Removes leading and trailing whitespace from text.
        """
        if len(args) < 1:
            raise TemplateEvalError("$Trim() requires 1 argument: text")
        return self._eval_node(args[0], metadata).strip()

    # ---- Splitting Functions ----

    def _fn_split(self, args, metadata):
        """
        $Split(text, delimiter, index)
        Splits text by delimiter and returns the element at index (1-based).
        """
        if len(args) < 3:
            raise TemplateEvalError("$Split() requires 3 arguments: text, delimiter, index")

        text = self._eval_node(args[0], metadata)
        delimiter = self._eval_node(args[1], metadata)
        index_str = self._eval_node(args[2], metadata).strip()

        try:
            index = int(index_str) - 1             # Convert 1-based to 0-based
            parts = text.split(delimiter)
            if 0 <= index < len(parts):
                return parts[index].strip()
            return ""                              # Index out of range
        except ValueError:
            raise TemplateEvalError(f"$Split() index must be a number, got '{index_str}'")

    def _fn_rsplit(self, args, metadata):
        """
        $RSplit(text, delimiter, index)
        Splits text by delimiter from the right and returns element at index (1-based).
        """
        if len(args) < 3:
            raise TemplateEvalError("$RSplit() requires 3 arguments: text, delimiter, index")

        text = self._eval_node(args[0], metadata)
        delimiter = self._eval_node(args[1], metadata)
        index_str = self._eval_node(args[2], metadata).strip()

        try:
            index = int(index_str) - 1             # Convert 1-based to 0-based
            # Split from right: reverse parts so index 1 = rightmost segment
            parts = text.rsplit(delimiter)
            parts.reverse()                        # Reverse so index 1 = last element
            if 0 <= index < len(parts):
                return parts[index].strip()
            return ""
        except ValueError:
            raise TemplateEvalError(f"$RSplit() index must be a number, got '{index_str}'")

    def _fn_first(self, args, metadata):
        """
        $First(text)
        Returns the first value from a semicolon-separated multi-value string.
        Example: "Rock; Progressive Rock" → "Rock"
        """
        if len(args) < 1:
            raise TemplateEvalError("$First() requires 1 argument: text")

        text = self._eval_node(args[0], metadata)
        # Split on semicolons (standard multi-value separator)
        parts = text.split(";")
        return parts[0].strip() if parts else text

    # ---- Formatting Functions ----

    def _fn_pad(self, args, metadata):
        """
        $Pad(value, width)
        Zero-pads a numeric value to the specified width.
        Example: $Pad(<Track #>, 2) → "01" for track 1
        """
        if len(args) < 2:
            raise TemplateEvalError("$Pad() requires 2 arguments: value, width")

        value = self._eval_node(args[0], metadata)
        width_str = self._eval_node(args[1], metadata).strip()

        try:
            width = int(width_str)
            # Extract numeric portion for padding (handles "1", "01", etc.)
            # Strip non-numeric leading chars but preserve the number
            numeric = ''.join(c for c in value if c.isdigit())
            if numeric:
                return numeric.zfill(width)
            return value.zfill(width)              # Fallback: pad as-is
        except ValueError:
            raise TemplateEvalError(f"$Pad() width must be a number, got '{width_str}'")

    def _fn_date(self, args, metadata):
        """
        $Date(value, format)
        Formats a date string using MusicBee-style format tokens.
        Tokens: yyyy (year), MM (month), dd (day), hh (hour), mm (minute), ss (second)
        """
        if len(args) < 2:
            raise TemplateEvalError("$Date() requires 2 arguments: value, format")

        value = self._eval_node(args[0], metadata)
        fmt = self._eval_node(args[1], metadata).strip()

        # Convert MusicBee format tokens to Python strftime tokens
        py_fmt = fmt
        for mb_token, py_token in DATE_FORMAT_MAP.items():
            py_fmt = py_fmt.replace(mb_token, py_token)

        try:
            # Try common date formats
            from datetime import datetime
            for parse_fmt in ("%Y-%m-%d", "%Y-%m-%d %H:%M:%S", "%d/%m/%Y", "%m/%d/%Y"):
                try:
                    dt = datetime.strptime(value, parse_fmt)
                    return dt.strftime(py_fmt)
                except ValueError:
                    continue
            # If no format matched, return original value
            return value
        except Exception:
            return value                           # Return original on any error

    def _fn_sort(self, args, metadata):
        """
        $Sort(text)
        Strips common sort words from the beginning of text.
        Removes: "The ", "A ", "An " (case-insensitive)
        Example: "The Beatles" → "Beatles"
        """
        if len(args) < 1:
            raise TemplateEvalError("$Sort() requires 1 argument: text")

        text = self._eval_node(args[0], metadata)
        text_lower = text.lower()

        # Check each sort word and strip if found at the start
        for word in SORT_WORDS:
            if text_lower.startswith(word):
                return text[len(word):]            # Preserve original casing
        return text

    def _fn_group(self, args, metadata):
        """
        $Group(text, n)
        Returns the first n characters of text (for A-Z folder grouping).
        Example: $Group(<Artist>, 1) → "Q" for "Queen"
        """
        if len(args) < 2:
            raise TemplateEvalError("$Group() requires 2 arguments: text, count")

        text = self._eval_node(args[0], metadata)
        count_str = self._eval_node(args[1], metadata).strip()

        try:
            count = int(count_str)
            result = text[:count]
            return result.upper()                  # Uppercase for consistent grouping
        except ValueError:
            raise TemplateEvalError(f"$Group() count must be a number, got '{count_str}'")

    # =========================================================================
    # Helper Methods
    # =========================================================================

    def _is_truthy(self, value):
        """
        Determine if a value is "truthy" in the template engine context.

        Truthy values:
        - Python bool True
        - Non-empty strings (except "F", "False", "0", "")
        - String "T" or "True"

        Args:
            value: The value to check (bool, str, or other)

        Returns:
            bool: True if the value is considered truthy
        """
        if isinstance(value, bool):
            return value
        if isinstance(value, str):
            stripped = value.strip().lower()
            # Explicitly falsy strings
            if stripped in ("", "f", "false", "0"):
                return False
            return True                            # Any non-empty string is truthy
        # Non-string, non-bool: truthy if not None/empty
        return bool(value)
