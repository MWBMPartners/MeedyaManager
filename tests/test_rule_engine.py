# ============================================================================
# File: /tests/test_rule_engine.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Comprehensive test suite for core/rule_engine.py — the MusicBee-style
# template parser and evaluator. Tests cover:
#   - Lexer tokenization
#   - Parser AST construction
#   - Tag resolution
#   - All 20 built-in functions
#   - Deep nesting
#   - Error handling
#   - End-to-end template evaluation
# ============================================================================

import pytest                                      # Test framework
from core.rule_engine import (
    Lexer,                                         # Tokenizer
    Parser,                                        # AST builder
    RuleEngine,                                    # Main evaluator
    TokenType,                                     # Token categories
    TemplateSyntaxError,                           # Syntax error type
    TemplateEvalError,                             # Evaluation error type
    TemplateNode,                                  # AST root node
    LiteralNode,                                   # AST literal text
    TagNode,                                       # AST tag reference
    FunctionNode,                                  # AST function call
    ComparisonNode,                                # AST comparison
)


# ============================================================================
# Sample metadata for testing — representative music file
# ============================================================================
MUSIC_METADATA = {
    "title": "Bohemian Rhapsody",
    "artist": "Queen",
    "album": "A Night at the Opera",
    "album_artist": "Queen",
    "year": "1975",
    "genre": "Rock; Progressive Rock",
    "track_num": "11",
    "disc_num": "1",
    "total_tracks": "12",
    "media_group": "Audio",
    "format_class": "flac",
    "media_class": "Music",
    "quality_type": "Lossless",
    "extension": "flac",
    "filepath": "/music/queen/song.flac",
    "filename": "song",
    "audio_channels": "2",
    "codec": "FLAC",
    "bitrate": "1411",
    "sample_rate": "44100",
    "bit_depth": "16",
    "date_added": "2025-06-15",
    "bpm": "72",
}

# Sample metadata for a TV show episode
TV_METADATA = {
    "title": "Felina",
    "show": "Breaking Bad",
    "season": "5",
    "episode": "16",
    "episode_title": "Felina",
    "director": "Vince Gilligan",
    "resolution": "1080p",
    "media_group": "Video",
    "format_class": "mkv",
    "media_class": "TV Show",
    "quality_type": "Lossy",
    "extension": "mkv",
    "filepath": "/tv/bb.mkv",
    "filename": "bb",
}

# Sample metadata with custom tags
CUSTOM_METADATA = {
    "title": "Test Song",
    "artist": "Test Artist",
    "extension": "mp3",
    "media_class": "Music",
    "custom_spotifyurl": "https://open.spotify.com/track/abc123",
    "custom_myrating": "5",
}


# ============================================================================
# Convenience: create an engine instance for tests
# ============================================================================
@pytest.fixture
def engine():
    """Provide a fresh RuleEngine instance for each test."""
    return RuleEngine()


# ============================================================================
# LEXER TESTS — Tokenization
# ============================================================================

class TestLexer:
    """Tests for the Lexer tokenizer."""

    def test_simple_tag(self):
        """A single tag tokenizes to one TAG token."""
        tokens = Lexer("<Title>").tokenize()
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.TAG
        assert tokens[0].value == "Title"

    def test_literal_text(self):
        """Plain text tokenizes to a LITERAL token."""
        tokens = Lexer("Hello World").tokenize()
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.LITERAL
        assert tokens[0].value == "Hello World"

    def test_tag_with_literal(self):
        """Mixed tag and literal text produces correct token sequence."""
        tokens = Lexer("Music/<Artist>/<Album>").tokenize()
        # Expect: LITERAL "Music/", TAG "Artist", LITERAL "/", TAG "Album"
        assert tokens[0].type == TokenType.LITERAL
        assert tokens[0].value == "Music/"
        assert tokens[1].type == TokenType.TAG
        assert tokens[1].value == "Artist"
        assert tokens[2].type == TokenType.LITERAL
        assert tokens[2].value == "/"
        assert tokens[3].type == TokenType.TAG
        assert tokens[3].value == "Album"

    def test_function_start(self):
        """$FuncName( tokenizes to FUNC_START."""
        tokens = Lexer("$Pad(<Track #>,2)").tokenize()
        assert tokens[0].type == TokenType.FUNC_START
        assert tokens[0].value == "Pad"

    def test_function_with_args(self):
        """Function with tag arg, comma, and literal arg tokenizes correctly."""
        tokens = Lexer("$Pad(<Track #>,2)").tokenize()
        types = [t.type for t in tokens]
        assert TokenType.FUNC_START in types
        assert TokenType.TAG in types
        assert TokenType.COMMA in types
        assert TokenType.RPAREN in types

    def test_nested_function(self):
        """Nested function calls produce correct token stream."""
        tokens = Lexer("$If(<Genre>=Rock,yes,no)").tokenize()
        types = [t.type for t in tokens]
        assert types[0] == TokenType.FUNC_START    # $If(
        assert types[1] == TokenType.TAG            # Genre
        assert types[2] == TokenType.EQUALS         # =
        # "Rock" is literal, then comma, "yes", comma, "no", rparen

    def test_comparison_operators(self):
        """Comparison operators =, > tokenize correctly inside functions."""
        tokens = Lexer("$If(<Year>>2000,modern,classic)").tokenize()
        types = [t.type for t in tokens]
        assert TokenType.GT in types               # > operator

    def test_custom_tag(self):
        """Custom:Name tags tokenize as TAG tokens."""
        tokens = Lexer("<Custom:SpotifyURL>").tokenize()
        assert tokens[0].type == TokenType.TAG
        assert tokens[0].value == "Custom:SpotifyURL"

    def test_invalid_tag_becomes_literal(self):
        """An invalid tag like <FakeTag> is treated as literals."""
        tokens = Lexer("<NotATag>").tokenize()
        # Should NOT be a TAG token since "NotATag" is not in TAG_MAP
        tag_tokens = [t for t in tokens if t.type == TokenType.TAG]
        assert len(tag_tokens) == 0


# ============================================================================
# PARSER TESTS — AST Construction
# ============================================================================

class TestParser:
    """Tests for the recursive descent Parser."""

    def test_parse_literal(self):
        """Literal text parses to a TemplateNode with a LiteralNode child."""
        tokens = Lexer("Hello").tokenize()
        ast = Parser(tokens).parse()
        assert isinstance(ast, TemplateNode)
        assert len(ast.children) == 1
        assert isinstance(ast.children[0], LiteralNode)

    def test_parse_tag(self):
        """A tag parses to a TagNode."""
        tokens = Lexer("<Title>").tokenize()
        ast = Parser(tokens).parse()
        assert isinstance(ast.children[0], TagNode)
        assert ast.children[0].name == "Title"

    def test_parse_function(self):
        """A function call parses to a FunctionNode with args."""
        tokens = Lexer("$Upper(<Title>)").tokenize()
        ast = Parser(tokens).parse()
        assert isinstance(ast.children[0], FunctionNode)
        assert ast.children[0].name == "Upper"
        assert len(ast.children[0].args) == 1

    def test_parse_comparison(self):
        """<Tag>=Value parses to a ComparisonNode."""
        tokens = Lexer("$If(<Genre>=Rock,yes,no)").tokenize()
        ast = Parser(tokens).parse()
        func = ast.children[0]                     # FunctionNode for $If
        assert isinstance(func, FunctionNode)
        # First arg should contain a ComparisonNode
        first_arg = func.args[0]
        assert isinstance(first_arg.children[0], ComparisonNode)

    def test_parse_nested_functions(self):
        """Nested function calls parse correctly."""
        tokens = Lexer("$If($Contains(<Genre>,Rock)=T,yes,no)").tokenize()
        ast = Parser(tokens).parse()
        func = ast.children[0]
        assert func.name == "If"
        # First arg should contain a comparison with $Contains on the left
        first_arg = func.args[0]
        comp = first_arg.children[0]
        assert isinstance(comp, ComparisonNode)
        assert isinstance(comp.left, FunctionNode)
        assert comp.left.name == "Contains"

    def test_parse_deeply_nested(self):
        """Four levels of nesting parse without error."""
        template = "$If(<Media Class>=Music,$If(<Quality Type>=Lossless,FLAC,MP3),Other)"
        tokens = Lexer(template).tokenize()
        ast = Parser(tokens).parse()
        # Should have one FunctionNode at root
        assert isinstance(ast.children[0], FunctionNode)
        assert ast.children[0].name == "If"

    def test_unclosed_function_raises(self):
        """Unclosed function parenthesis raises TemplateSyntaxError."""
        tokens = Lexer("$If(<Title>=test,yes").tokenize()
        with pytest.raises(TemplateSyntaxError):
            Parser(tokens).parse()

    def test_max_depth_guard(self):
        """Exceeding max nesting depth raises TemplateSyntaxError."""
        # Build a deeply nested template that exceeds MAX_DEPTH
        deep = "$If(<Title>=x," * 60 + "leaf" + ",no)" * 60
        tokens = Lexer(deep).tokenize()
        with pytest.raises(TemplateSyntaxError, match="Maximum nesting depth"):
            Parser(tokens).parse()


# ============================================================================
# TAG RESOLUTION TESTS
# ============================================================================

class TestTagResolution:
    """Tests for tag resolution during evaluation."""

    def test_resolve_simple_tag(self, engine):
        """<Title> resolves to the title metadata value."""
        result = engine.evaluate("<Title>", MUSIC_METADATA)
        assert result == "Bohemian Rhapsody"

    def test_resolve_album_artist(self, engine):
        """<Album Artist> resolves to album_artist key."""
        result = engine.evaluate("<Album Artist>", MUSIC_METADATA)
        assert result == "Queen"

    def test_resolve_classification_tag(self, engine):
        """<Media Class> resolves correctly."""
        result = engine.evaluate("<Media Class>", MUSIC_METADATA)
        assert result == "Music"

    def test_resolve_custom_tag(self, engine):
        """<Custom:SpotifyURL> resolves to custom_spotifyurl key."""
        result = engine.evaluate("<Custom:SpotifyURL>", CUSTOM_METADATA)
        assert result == "https://open.spotify.com/track/abc123"

    def test_missing_tag_returns_empty(self, engine):
        """Missing tag resolves to empty string."""
        result = engine.evaluate("<Show>", MUSIC_METADATA)
        assert result == ""


# ============================================================================
# FUNCTION TESTS — Each of the 20 built-in functions
# ============================================================================

class TestConditionalFunctions:
    """Tests for $If, $And, $Or."""

    def test_if_true_branch(self, engine):
        """$If with matching criteria returns true branch."""
        result = engine.evaluate(
            "$If(<Media Class>=Music,music_folder,other_folder)",
            MUSIC_METADATA
        )
        assert result == "music_folder"

    def test_if_false_branch(self, engine):
        """$If with non-matching criteria returns false branch."""
        result = engine.evaluate(
            "$If(<Media Class>=Video,video_folder,other_folder)",
            MUSIC_METADATA
        )
        assert result == "other_folder"

    def test_if_greater_than(self, engine):
        """$If with > operator compares numerically."""
        result = engine.evaluate(
            "$If(<Year>>2000,modern,classic)",
            MUSIC_METADATA  # year = "1975"
        )
        assert result == "classic"

    def test_if_less_than(self, engine):
        """$If with < operator compares numerically."""
        result = engine.evaluate(
            "$If(<Year><2000,classic,modern)",
            MUSIC_METADATA  # year = "1975"
        )
        assert result == "classic"

    def test_and_both_true(self, engine):
        """$And returns truthy when both criteria match."""
        result = engine.evaluate(
            "$If($And(<Media Class>=Music,<Quality Type>=Lossless),hi-res,standard)",
            MUSIC_METADATA
        )
        assert result == "hi-res"

    def test_and_one_false(self, engine):
        """$And returns falsy when one criterion doesn't match."""
        result = engine.evaluate(
            "$If($And(<Media Class>=Music,<Quality Type>=Lossy),lossy-music,other)",
            MUSIC_METADATA  # quality_type = "Lossless"
        )
        assert result == "other"

    def test_or_either_true(self, engine):
        """$Or returns truthy when at least one criterion matches."""
        result = engine.evaluate(
            "$If($Or(<Genre>=Jazz,<Genre>=Rock; Progressive Rock),match,no-match)",
            MUSIC_METADATA  # genre contains "Rock"
        )
        # Genre is "Rock; Progressive Rock" which doesn't exactly equal "Jazz" or
        # "Rock; Progressive Rock" — but it DOES equal the second option
        assert result == "match"

    def test_or_neither_true(self, engine):
        """$Or returns falsy when neither criterion matches."""
        result = engine.evaluate(
            "$If($Or(<Genre>=Jazz,<Genre>=Classical),match,no-match)",
            MUSIC_METADATA
        )
        assert result == "no-match"


class TestLogicFunctions:
    """Tests for $IsNull, $Contains, $IsMatch."""

    def test_is_null_when_present(self, engine):
        """$IsNull returns ifNotNull branch when tag has a value."""
        result = engine.evaluate(
            "$IsNull(<Album Artist>,Unknown,<Album Artist>)",
            MUSIC_METADATA
        )
        assert result == "Queen"

    def test_is_null_when_missing(self, engine):
        """$IsNull returns ifNull branch when tag is missing."""
        result = engine.evaluate(
            "$IsNull(<Show>,No Show,<Show>)",
            MUSIC_METADATA  # "show" not in metadata
        )
        assert result == "No Show"

    def test_contains_found(self, engine):
        """$Contains returns 'T' when search string is found."""
        result = engine.evaluate("$Contains(<Genre>,Rock)", MUSIC_METADATA)
        assert result == "T"

    def test_contains_not_found(self, engine):
        """$Contains returns 'F' when search string is not found."""
        result = engine.evaluate("$Contains(<Genre>,Jazz)", MUSIC_METADATA)
        assert result == "F"

    def test_contains_case_insensitive(self, engine):
        """$Contains search is case-insensitive."""
        result = engine.evaluate("$Contains(<Genre>,rock)", MUSIC_METADATA)
        assert result == "T"

    def test_is_match_found(self, engine):
        """$IsMatch returns 'T' when regex matches."""
        result = engine.evaluate('$IsMatch(<Title>,"^Bohemian")', MUSIC_METADATA)
        assert result == "T"

    def test_is_match_not_found(self, engine):
        """$IsMatch returns 'F' when regex doesn't match."""
        result = engine.evaluate('$IsMatch(<Title>,"^Stairway")', MUSIC_METADATA)
        assert result == "F"


class TestStringFunctions:
    """Tests for $Replace, $RxReplace, $Left, $Right, $Upper, $Lower, $Trim."""

    def test_replace(self, engine):
        """$Replace substitutes all occurrences."""
        result = engine.evaluate(
            "$Replace(<Artist>,Queen,King)",
            MUSIC_METADATA
        )
        assert result == "King"

    def test_rx_replace(self, engine):
        """$RxReplace handles regex substitution."""
        # Remove everything after semicolon in genre
        result = engine.evaluate(
            '$RxReplace(<Genre>,";.*","")',
            MUSIC_METADATA  # genre = "Rock; Progressive Rock"
        )
        assert result == "Rock"

    def test_left(self, engine):
        """$Left returns first n characters."""
        result = engine.evaluate("$Left(<Artist>,1)", MUSIC_METADATA)
        assert result == "Q"

    def test_right(self, engine):
        """$Right returns last n characters."""
        result = engine.evaluate("$Right(<Year>,2)", MUSIC_METADATA)
        assert result == "75"

    def test_upper(self, engine):
        """$Upper converts to uppercase."""
        result = engine.evaluate("$Upper(<Ext>)", MUSIC_METADATA)
        assert result == "FLAC"

    def test_lower(self, engine):
        """$Lower converts to lowercase."""
        result = engine.evaluate("$Lower(<Codec>)", MUSIC_METADATA)
        assert result == "flac"

    def test_trim(self, engine):
        """$Trim strips whitespace."""
        metadata = {**MUSIC_METADATA, "title": "  Spaced Title  "}
        result = engine.evaluate("$Trim(<Title>)", metadata)
        assert result == "Spaced Title"


class TestSplittingFunctions:
    """Tests for $Split, $RSplit, $First."""

    def test_split(self, engine):
        """$Split returns the nth element from left split."""
        result = engine.evaluate(
            "$Split(<Genre>,;,1)",
            MUSIC_METADATA  # genre = "Rock; Progressive Rock"
        )
        assert result == "Rock"

    def test_split_second_element(self, engine):
        """$Split with index 2 returns the second element."""
        result = engine.evaluate(
            "$Split(<Genre>,;,2)",
            MUSIC_METADATA
        )
        assert result == "Progressive Rock"

    def test_rsplit(self, engine):
        """$RSplit returns the nth element from right split."""
        metadata = {**MUSIC_METADATA, "artist": "John Smith"}
        result = engine.evaluate("$RSplit(<Artist>, ,1)", metadata)
        assert result == "Smith"

    def test_first(self, engine):
        """$First returns the first multi-value element."""
        result = engine.evaluate("$First(<Genre>)", MUSIC_METADATA)
        assert result == "Rock"


class TestFormattingFunctions:
    """Tests for $Pad, $Date, $Sort, $Group."""

    def test_pad_single_digit(self, engine):
        """$Pad zero-pads single digit to specified width."""
        metadata = {**MUSIC_METADATA, "track_num": "1"}
        result = engine.evaluate("$Pad(<Track #>,2)", metadata)
        assert result == "01"

    def test_pad_already_wide(self, engine):
        """$Pad doesn't modify values already at or exceeding width."""
        result = engine.evaluate("$Pad(<Track #>,2)", MUSIC_METADATA)
        assert result == "11"                      # Already 2 digits

    def test_pad_three_digits(self, engine):
        """$Pad can pad to 3 digits."""
        metadata = {**MUSIC_METADATA, "track_num": "5"}
        result = engine.evaluate("$Pad(<Track #>,3)", metadata)
        assert result == "005"

    def test_date_format(self, engine):
        """$Date formats a date string."""
        result = engine.evaluate("$Date(<Date Added>,yyyy)", MUSIC_METADATA)
        assert result == "2025"

    def test_sort_strip_the(self, engine):
        """$Sort strips 'The' from the beginning."""
        metadata = {**MUSIC_METADATA, "artist": "The Beatles"}
        result = engine.evaluate("$Sort(<Artist>)", metadata)
        assert result == "Beatles"

    def test_sort_no_article(self, engine):
        """$Sort returns text unchanged if no sort word is present."""
        result = engine.evaluate("$Sort(<Artist>)", MUSIC_METADATA)
        assert result == "Queen"

    def test_sort_strip_a(self, engine):
        """$Sort strips 'A ' from the beginning."""
        metadata = {**MUSIC_METADATA, "album": "A Night at the Opera"}
        result = engine.evaluate("$Sort(<Album>)", metadata)
        assert result == "Night at the Opera"

    def test_group_single_char(self, engine):
        """$Group returns first character uppercase (A-Z grouping)."""
        result = engine.evaluate("$Group(<Artist>,1)", MUSIC_METADATA)
        assert result == "Q"

    def test_group_two_chars(self, engine):
        """$Group returns first n characters uppercase."""
        result = engine.evaluate("$Group(<Artist>,2)", MUSIC_METADATA)
        assert result == "QU"


# ============================================================================
# END-TO-END TEMPLATE TESTS — Full evaluation from template string to result
# ============================================================================

class TestEndToEnd:
    """Full template evaluation tests matching examples from help/rule-syntax.md."""

    def test_basic_music_template(self, engine):
        """Basic music organization template produces correct path."""
        result = engine.evaluate(
            "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",
            MUSIC_METADATA
        )
        assert result == "Music/Queen/A Night at the Opera/11 - Bohemian Rhapsody.flac"

    def test_tv_show_template(self, engine):
        """TV show template with season/episode padding."""
        result = engine.evaluate(
            "TV Shows/<Show>/Season <$Pad(<Season>,2)>/<Show> - S<$Pad(<Season>,2)>E<$Pad(<Episode>,2)> - <Episode Title>.<Ext>",
            TV_METADATA
        )
        assert result == "TV Shows/Breaking Bad/Season 05/Breaking Bad - S05E16 - Felina.mkv"

    def test_lossless_vs_lossy_separation(self, engine):
        """$If separates lossless and lossy into different folders."""
        result = engine.evaluate(
            "$If(<Quality Type>=Lossless,Music/Lossless/<Artist>,Music/Lossy/<Artist>)",
            MUSIC_METADATA  # quality_type = "Lossless"
        )
        assert result == "Music/Lossless/Queen"

    def test_az_folder_grouping(self, engine):
        """A-Z folder grouping with $Group and $Sort."""
        result = engine.evaluate(
            "Music/$Group($Sort(<Artist>),1)/<Artist>",
            MUSIC_METADATA
        )
        assert result == "Music/Q/Queen"

    def test_az_grouping_with_the(self, engine):
        """A-Z grouping strips 'The' before grouping."""
        metadata = {**MUSIC_METADATA, "artist": "The Beatles"}
        result = engine.evaluate(
            "Music/$Group($Sort(<Artist>),1)/<Artist>",
            metadata
        )
        assert result == "Music/B/The Beatles"

    def test_handle_missing_album_artist(self, engine):
        """$IsNull falls back to <Artist> when <Album Artist> is missing."""
        metadata = {**MUSIC_METADATA}
        del metadata["album_artist"]               # Remove album artist
        result = engine.evaluate(
            "$IsNull(<Album Artist>,<Artist>,<Album Artist>)",
            metadata
        )
        assert result == "Queen"                   # Falls back to artist

    def test_multi_type_router(self, engine):
        """Multi-type router routes Music correctly."""
        result = engine.evaluate(
            "$If(<Media Class>=Music,Music/<Artist>,Other/<Filename>)",
            MUSIC_METADATA
        )
        assert result == "Music/Queen"

    def test_multi_type_router_tv(self, engine):
        """Multi-type router routes TV Show correctly."""
        result = engine.evaluate(
            "$If(<Media Class>=Music,Music/<Artist>,$If(<Media Class>=TV Show,TV/<Show>,Other))",
            TV_METADATA
        )
        assert result == "TV/Breaking Bad"

    def test_contains_in_if(self, engine):
        """$Contains used as $If criteria."""
        result = engine.evaluate(
            "$If($Contains(<Genre>,Rock)=T,Rock/<Artist>,Other/<Artist>)",
            MUSIC_METADATA
        )
        assert result == "Rock/Queen"

    def test_deeply_nested_template(self, engine):
        """Four-level nested $If chain evaluates correctly."""
        template = (
            "$If(<Media Class>=Music,"
            "  $If(<Quality Type>=Lossless,"
            "    $If($Contains(<Genre>,Rock)=T,"
            "      Rock/Lossless/<Artist>,"
            "      Other/Lossless/<Artist>),"
            "    Lossy/<Artist>),"
            "  Other/<Filename>)"
        )
        result = engine.evaluate(template, MUSIC_METADATA)
        # Music → Lossless → Contains Rock → "Rock/Lossless/Queen"
        # Note: whitespace from template indentation is preserved as literals
        assert "Rock/Lossless/Queen" in result


# ============================================================================
# VALIDATION TESTS
# ============================================================================

class TestValidation:
    """Tests for template validation."""

    def test_valid_template(self, engine):
        """A valid template returns no errors."""
        errors = engine.validate("<Title>/<Artist>")
        assert errors == []

    def test_valid_function_template(self, engine):
        """A valid function template returns no errors."""
        errors = engine.validate("$Pad(<Track #>,2)")
        assert errors == []

    def test_unknown_function(self, engine):
        """An unknown function name is reported."""
        errors = engine.validate("$Bogus(<Title>)")
        assert any("Unknown function" in e for e in errors)

    def test_unclosed_function(self, engine):
        """Unclosed function parenthesis is reported."""
        errors = engine.validate("$If(<Title>=test,yes")
        assert len(errors) > 0


# ============================================================================
# ERROR HANDLING TESTS
# ============================================================================

class TestErrorHandling:
    """Tests for error conditions."""

    def test_missing_function_name(self):
        """$ without function name raises TemplateSyntaxError."""
        with pytest.raises(TemplateSyntaxError):
            Lexer("$(bad)").tokenize()

    def test_if_insufficient_args(self, engine):
        """$If with fewer than 3 args raises TemplateEvalError."""
        with pytest.raises(TemplateEvalError, match="requires 3 arguments"):
            engine.evaluate("$If(<Title>=test,yes)", MUSIC_METADATA)

    def test_pad_non_numeric_width(self, engine):
        """$Pad with non-numeric width raises TemplateEvalError."""
        with pytest.raises(TemplateEvalError, match="must be a number"):
            engine.evaluate("$Pad(<Track #>,abc)", MUSIC_METADATA)

    def test_left_non_numeric_count(self, engine):
        """$Left with non-numeric count raises TemplateEvalError."""
        with pytest.raises(TemplateEvalError, match="must be a number"):
            engine.evaluate("$Left(<Title>,abc)", MUSIC_METADATA)

    def test_empty_template(self, engine):
        """Empty template evaluates to empty string."""
        result = engine.evaluate("", MUSIC_METADATA)
        assert result == ""

    def test_literal_only_template(self, engine):
        """Template with only literal text returns that text."""
        result = engine.evaluate("just/plain/text.mp3", MUSIC_METADATA)
        assert result == "just/plain/text.mp3"
