// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine Template Functions
//
// Implements 24 template functions callable from `$FuncName(args)` syntax.
// All functions receive pre-evaluated string arguments and return a string
// result.  Functions are grouped into categories:
//
//   Logical (6):  $If, $And, $Or, $Not, $IsNull, $Contains
//   String  (8):  $Replace, $Upper, $Lower, $Left, $Right, $Mid, $Trim, $Split
//   Numeric (4):  $Pad, $Date, $Format, $Count
//   Lookup  (3):  $Sort, $IsMatch, $Lookup
//   Extensions(3): $MediaClass, $MediaGroup, $FirstValue
//
// License: GPL-2.0-or-later

use std::collections::HashMap;
use std::sync::Mutex;

use regex::Regex;

use super::evaluator::EvalContext;
use crate::error::{MmError, MmResult};

// ───────────────────────────────────────────────────────────────────────────
// Constants
// ───────────────────────────────────────────────────────────────────────────

/// Truthy string value returned by boolean functions
const TRUE_VAL: &str = "1";
/// Falsy string value (empty string = false)
const FALSE_VAL: &str = "";

// ───────────────────────────────────────────────────────────────────────────
// Regex cache
// ───────────────────────────────────────────────────────────────────────────

/// Cached compiled regex patterns for `$IsMatch`.  Compiling a regex per
/// evaluation call is expensive, so we cache them in a thread-safe map.
fn regex_cache() -> &'static Mutex<HashMap<String, Regex>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, Regex>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compile a regex pattern, using the cache to avoid recompilation.
fn compile_regex(pattern: &str) -> MmResult<Regex> {
    // Try to get a cached copy first
    let cache = regex_cache();
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(re) = guard.get(pattern) {
        return Ok(re.clone());
    }
    drop(guard); // release lock before compiling

    // Compile the pattern
    let re = Regex::new(pattern)
        .map_err(|e| MmError::RuleEngine(format!("invalid regex pattern '{pattern}': {e}")))?;

    // Cache for future use
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.insert(pattern.to_string(), re.clone());

    Ok(re)
}

// ───────────────────────────────────────────────────────────────────────────
// Truthiness helper
// ───────────────────────────────────────────────────────────────────────────

/// Determine if a string value is "truthy".
/// Empty strings, "0", and "false" (case-insensitive) are falsy.
/// Everything else is truthy.
fn is_truthy(val: &str) -> bool {
    !val.is_empty() && val != "0" && !val.eq_ignore_ascii_case("false")
}

// ───────────────────────────────────────────────────────────────────────────
// Argument validation helper
// ───────────────────────────────────────────────────────────────────────────

/// Validate that the argument count is within the expected range.
fn check_args(name: &str, args: &[String], min: usize, max: usize) -> MmResult<()> {
    if args.len() < min || args.len() > max {
        if min == max {
            return Err(MmError::RuleEngine(format!(
                "${name} expects exactly {min} argument(s), got {}",
                args.len()
            )));
        }
        return Err(MmError::RuleEngine(format!(
            "${name} expects {min}–{max} arguments, got {}",
            args.len()
        )));
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Main dispatch function
// ───────────────────────────────────────────────────────────────────────────

/// Evaluate a template function by name with pre-evaluated string arguments.
///
/// # Arguments
///
/// * `name` — The function name (e.g. "If", "Upper", "Pad")
/// * `args` — Pre-evaluated string arguments
/// * `ctx`  — Evaluation context (for functions that need classification data)
///
/// # Errors
///
/// Returns `MmError::RuleEngine` for unknown function names, wrong argument
/// counts, or invalid arguments (e.g. non-numeric values where numbers expected).
pub fn eval_func(name: &str, args: &[String], ctx: &EvalContext<'_>) -> MmResult<String> {
    // Dispatch by function name (case-insensitive)
    match name.to_lowercase().as_str() {
        // ── Logical functions ────────────────────────────────────────
        "if" => func_if(args),
        "and" => func_and(args),
        "or" => func_or(args),
        "not" => func_not(args),
        "isnull" => func_is_null(args),
        "contains" => func_contains(args),

        // ── String functions ─────────────────────────────────────────
        "replace" => func_replace(args),
        "upper" => func_upper(args),
        "lower" => func_lower(args),
        "left" => func_left(args),
        "right" => func_right(args),
        "mid" => func_mid(args),
        "trim" => func_trim(args),
        "split" => func_split(args),

        // ── Numeric / formatting functions ───────────────────────────
        "pad" => func_pad(args),
        "date" => func_date(args),
        "format" => func_format(args),
        "count" => func_count(args),

        // ── Lookup functions ─────────────────────────────────────────
        "sort" => func_sort(args),
        "ismatch" => func_is_match(args),
        "lookup" => func_lookup(args),

        // ── MeedyaManager extension functions ────────────────────────
        "mediaclass" => func_media_class(ctx),
        "mediagroup" => func_media_group(ctx),
        "firstvalue" => func_first_value(args),

        // ── Unknown function ─────────────────────────────────────────
        _ => Err(MmError::RuleEngine(format!(
            "unknown template function: ${name}"
        ))),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Logical functions (6)
// ───────────────────────────────────────────────────────────────────────────

/// `$If(condition, then_value, else_value?)`
/// If condition is truthy, return then_value; otherwise return else_value (default "").
fn func_if(args: &[String]) -> MmResult<String> {
    check_args("If", args, 2, 3)?;
    let condition = &args[0];
    let then_val = &args[1];
    let else_val = args.get(2).map_or("", std::string::String::as_str);
    if is_truthy(condition) {
        Ok(then_val.clone())
    } else {
        Ok(else_val.to_string())
    }
}

/// `$And(value1, value2, ...)`
/// Returns the last value if ALL values are truthy; otherwise returns "".
fn func_and(args: &[String]) -> MmResult<String> {
    if args.is_empty() {
        return Err(MmError::RuleEngine(
            "$And expects at least 1 argument".into(),
        ));
    }
    // All must be truthy
    if args.iter().all(|a| is_truthy(a)) {
        Ok(args.last().unwrap().clone())
    } else {
        Ok(FALSE_VAL.into())
    }
}

/// `$Or(value1, value2, ...)`
/// Returns the first truthy value; otherwise returns "".
fn func_or(args: &[String]) -> MmResult<String> {
    if args.is_empty() {
        return Err(MmError::RuleEngine(
            "$Or expects at least 1 argument".into(),
        ));
    }
    // Return first truthy value
    for arg in args {
        if is_truthy(arg) {
            return Ok(arg.clone());
        }
    }
    Ok(FALSE_VAL.into())
}

/// `$Not(value)`
/// Returns "1" if value is falsy; "" if value is truthy.
fn func_not(args: &[String]) -> MmResult<String> {
    check_args("Not", args, 1, 1)?;
    if is_truthy(&args[0]) {
        Ok(FALSE_VAL.into())
    } else {
        Ok(TRUE_VAL.into())
    }
}

/// `$IsNull(value)`
/// Returns "1" if value is empty; "" otherwise.
fn func_is_null(args: &[String]) -> MmResult<String> {
    check_args("IsNull", args, 1, 1)?;
    if args[0].is_empty() {
        Ok(TRUE_VAL.into())
    } else {
        Ok(FALSE_VAL.into())
    }
}

/// `$Contains(haystack, needle)`
/// Returns "1" if haystack contains needle (case-insensitive); "" otherwise.
fn func_contains(args: &[String]) -> MmResult<String> {
    check_args("Contains", args, 2, 2)?;
    let haystack = args[0].to_lowercase();
    let needle = args[1].to_lowercase();
    if haystack.contains(&needle) {
        Ok(TRUE_VAL.into())
    } else {
        Ok(FALSE_VAL.into())
    }
}

// ───────────────────────────────────────────────────────────────────────────
// String functions (8)
// ───────────────────────────────────────────────────────────────────────────

/// `$Replace(string, search, replacement)`
/// Replace all occurrences of `search` in `string` with `replacement`.
fn func_replace(args: &[String]) -> MmResult<String> {
    check_args("Replace", args, 3, 3)?;
    Ok(args[0].replace(&*args[1], &args[2]))
}

/// `$Upper(string)`
/// Convert to uppercase.
fn func_upper(args: &[String]) -> MmResult<String> {
    check_args("Upper", args, 1, 1)?;
    Ok(args[0].to_uppercase())
}

/// `$Lower(string)`
/// Convert to lowercase.
fn func_lower(args: &[String]) -> MmResult<String> {
    check_args("Lower", args, 1, 1)?;
    Ok(args[0].to_lowercase())
}

/// `$Left(string, n)`
/// Return the first `n` characters.  Clamps to string length.
fn func_left(args: &[String]) -> MmResult<String> {
    check_args("Left", args, 2, 2)?;
    let n = parse_usize("Left", &args[1])?;
    let chars: Vec<char> = args[0].chars().collect();
    let end = n.min(chars.len());
    Ok(chars[..end].iter().collect())
}

/// `$Right(string, n)`
/// Return the last `n` characters.  Clamps to string length.
fn func_right(args: &[String]) -> MmResult<String> {
    check_args("Right", args, 2, 2)?;
    let n = parse_usize("Right", &args[1])?;
    let chars: Vec<char> = args[0].chars().collect();
    let start = chars.len().saturating_sub(n);
    Ok(chars[start..].iter().collect())
}

/// `$Mid(string, start, length?)`
/// Return a substring starting at `start` (0-indexed) with optional `length`.
fn func_mid(args: &[String]) -> MmResult<String> {
    check_args("Mid", args, 2, 3)?;
    let start = parse_usize("Mid", &args[1])?;
    let chars: Vec<char> = args[0].chars().collect();
    // Clamp start to string length
    let start = start.min(chars.len());
    // Optional length parameter
    let end = if args.len() == 3 {
        let len = parse_usize("Mid", &args[2])?;
        (start + len).min(chars.len())
    } else {
        chars.len()
    };
    Ok(chars[start..end].iter().collect())
}

/// `$Trim(string)`
/// Strip leading and trailing whitespace.
fn func_trim(args: &[String]) -> MmResult<String> {
    check_args("Trim", args, 1, 1)?;
    Ok(args[0].trim().to_string())
}

/// `$Split(string, separator, index)`
/// Split `string` on `separator` and return the element at `index` (0-based).
/// Returns "" if index is out of range.
fn func_split(args: &[String]) -> MmResult<String> {
    check_args("Split", args, 3, 3)?;
    let index = parse_usize("Split", &args[2])?;
    let parts: Vec<&str> = args[0].split(&*args[1]).collect();
    Ok(parts.get(index).unwrap_or(&"").to_string())
}

// ───────────────────────────────────────────────────────────────────────────
// Numeric / formatting functions (4)
// ───────────────────────────────────────────────────────────────────────────

/// `$Pad(string, width, fill_char?)`
/// Left-pad `string` with `fill_char` (default "0") to at least `width` characters.
fn func_pad(args: &[String]) -> MmResult<String> {
    check_args("Pad", args, 2, 3)?;
    let width = parse_usize("Pad", &args[1])?;
    let fill = if args.len() == 3 {
        args[2].chars().next().unwrap_or('0')
    } else {
        '0'
    };
    let input = &args[0];
    if input.len() >= width {
        Ok(input.clone())
    } else {
        let padding: String = std::iter::repeat_n(fill, width - input.len()).collect();
        Ok(format!("{padding}{input}"))
    }
}

/// `$Date(format?)`
/// Returns the current date formatted with the given format string.
/// Default format: "%Y-%m-%d".  Uses chrono format specifiers.
fn func_date(args: &[String]) -> MmResult<String> {
    check_args("Date", args, 0, 1)?;
    let fmt = if args.is_empty() {
        "%Y-%m-%d"
    } else {
        &args[0]
    };
    let now = chrono::Local::now();
    Ok(now.format(fmt).to_string())
}

/// `$Format(number, decimals?)`
/// Format a numeric string with the given number of decimal places (default 0).
fn func_format(args: &[String]) -> MmResult<String> {
    check_args("Format", args, 1, 2)?;
    let decimals = if args.len() == 2 {
        parse_usize("Format", &args[1])?
    } else {
        0
    };
    // Parse the input as f64
    let num: f64 = args[0].parse().map_err(|_| {
        MmError::RuleEngine(format!("$Format: '{}' is not a valid number", args[0]))
    })?;
    Ok(format!("{num:.decimals$}"))
}

/// `$Count(string, separator?)`
/// Count the number of items when splitting on `separator` (default "; ").
fn func_count(args: &[String]) -> MmResult<String> {
    check_args("Count", args, 1, 2)?;
    let sep = if args.len() == 2 { &args[1] } else { "; " };
    // Empty string has zero items
    if args[0].is_empty() {
        return Ok("0".into());
    }
    let count = args[0].split(sep).count();
    Ok(count.to_string())
}

// ───────────────────────────────────────────────────────────────────────────
// Lookup functions (3)
// ───────────────────────────────────────────────────────────────────────────

/// `$Sort(string, separator?)`
/// Sort items in a multi-value string alphabetically.
/// Default separator: "; ".
fn func_sort(args: &[String]) -> MmResult<String> {
    check_args("Sort", args, 1, 2)?;
    let sep = if args.len() == 2 { &args[1] } else { "; " };
    if args[0].is_empty() {
        return Ok(String::new());
    }
    let mut parts: Vec<&str> = args[0].split(sep).collect();
    parts.sort_unstable();
    Ok(parts.join(sep))
}

/// `$IsMatch(string, regex_pattern)`
/// Returns "1" if `string` matches `regex_pattern`; "" otherwise.
/// Regex patterns are cached to avoid recompilation.
fn func_is_match(args: &[String]) -> MmResult<String> {
    check_args("IsMatch", args, 2, 2)?;
    let re = compile_regex(&args[1])?;
    if re.is_match(&args[0]) {
        Ok(TRUE_VAL.into())
    } else {
        Ok(FALSE_VAL.into())
    }
}

/// `$Lookup(key, table_name)`
/// Look up a key in a built-in mapping table.  Returns "" on miss.
/// Currently supported tables: "genre_folder", "quality_folder".
fn func_lookup(args: &[String]) -> MmResult<String> {
    check_args("Lookup", args, 2, 2)?;
    let key = args[0].to_lowercase();
    let table = args[1].to_lowercase();
    match table.as_str() {
        // ── Genre → folder name mapping ──
        "genre_folder" => {
            let result = match key.as_str() {
                "rock" | "alternative" | "indie" | "punk" | "grunge" => "Rock",
                "pop" | "dance pop" | "synth pop" => "Pop",
                "hip hop" | "rap" | "hip-hop" | "trap" => "Hip-Hop",
                "electronic" | "edm" | "house" | "techno" | "trance" | "dubstep" => "Electronic",
                "jazz" | "smooth jazz" | "bebop" | "swing" => "Jazz",
                "classical" | "baroque" | "romantic" | "opera" => "Classical",
                "country" | "bluegrass" | "folk" => "Country & Folk",
                "r&b" | "soul" | "funk" | "motown" => "R&B & Soul",
                "blues" | "delta blues" | "chicago blues" => "Blues",
                "metal" | "heavy metal" | "thrash" | "death metal" | "black metal" => "Metal",
                "reggae" | "ska" | "dub" => "Reggae",
                "latin" | "salsa" | "bossa nova" | "samba" => "Latin",
                "world" | "afrobeat" | "celtic" => "World",
                "soundtrack" | "score" | "ost" => "Soundtracks",
                _ => "",
            };
            Ok(result.into())
        }
        // ── Quality → folder name mapping ──
        "quality_folder" => {
            let result = match key.as_str() {
                "lossless" | "hi-res" => "Lossless",
                "320 kbps" | "lossy 320" => "High Quality",
                "256 kbps" | "lossy 256" => "Standard Quality",
                _ => "Other",
            };
            Ok(result.into())
        }
        // ── Unknown table ──
        _ => Ok(String::new()),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// MeedyaManager extension functions (3)
// ───────────────────────────────────────────────────────────────────────────

/// `$MediaClass()`
/// Returns the MediaClass display string from the evaluation context.
fn func_media_class(ctx: &EvalContext<'_>) -> MmResult<String> {
    match &ctx.classification {
        Some(c) => Ok(c.class.to_string()),
        None => Ok(String::new()),
    }
}

/// `$MediaGroup()`
/// Returns the MediaGroup display string from the evaluation context.
fn func_media_group(ctx: &EvalContext<'_>) -> MmResult<String> {
    match &ctx.classification {
        Some(c) => Ok(c.group.to_string()),
        None => Ok(String::new()),
    }
}

/// `$FirstValue(string, separator?)`
/// Return the first element when splitting on `separator` (default "; ").
fn func_first_value(args: &[String]) -> MmResult<String> {
    check_args("FirstValue", args, 1, 2)?;
    let sep = if args.len() == 2 { &args[1] } else { "; " };
    if args[0].is_empty() {
        return Ok(String::new());
    }
    let first = args[0].split(sep).next().unwrap_or("");
    Ok(first.to_string())
}

// ───────────────────────────────────────────────────────────────────────────
// Numeric parsing helper
// ───────────────────────────────────────────────────────────────────────────

/// Parse a string as `usize`, returning a descriptive error on failure.
fn parse_usize(func_name: &str, s: &str) -> MmResult<usize> {
    s.parse::<usize>().map_err(|_| {
        MmError::RuleEngine(format!(
            "${func_name}: '{s}' is not a valid non-negative integer"
        ))
    })
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{MediaClass, MediaClassification, MediaFormat, MediaGroup, MediaQuality};
    use crate::metadata::TagMap;

    /// Helper: build a minimal EvalContext for testing
    fn test_ctx() -> EvalContext<'static> {
        // Use a leaked TagMap so we can get a 'static reference
        let tags: &'static TagMap = Box::leak(Box::new(TagMap::new()));
        EvalContext {
            tags,
            audio_props: None,
            classification: None,
            file_path: None,
            path_mode: false,
            missing_tag_mode: super::super::evaluator::MissingTagMode::Empty,
        }
    }

    /// Helper: build args from string slices
    fn args(vals: &[&str]) -> Vec<String> {
        vals.iter().map(std::string::ToString::to_string).collect()
    }

    // ── $If ─────────────────────────────────────────────────────────

    #[test]
    fn if_truthy() {
        let ctx = test_ctx();
        let result = eval_func("If", &args(&["yes", "then", "else"]), &ctx).unwrap();
        assert_eq!(result, "then");
    }

    #[test]
    fn if_falsy_empty() {
        let ctx = test_ctx();
        let result = eval_func("If", &args(&["", "then", "else"]), &ctx).unwrap();
        assert_eq!(result, "else");
    }

    #[test]
    fn if_falsy_zero() {
        let ctx = test_ctx();
        let result = eval_func("If", &args(&["0", "then", "else"]), &ctx).unwrap();
        assert_eq!(result, "else");
    }

    #[test]
    fn if_default_else() {
        let ctx = test_ctx();
        let result = eval_func("If", &args(&["", "then"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $And ────────────────────────────────────────────────────────

    #[test]
    fn and_all_truthy() {
        let ctx = test_ctx();
        let result = eval_func("And", &args(&["a", "b", "c"]), &ctx).unwrap();
        assert_eq!(result, "c");
    }

    #[test]
    fn and_one_empty() {
        let ctx = test_ctx();
        let result = eval_func("And", &args(&["a", "", "c"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $Or ─────────────────────────────────────────────────────────

    #[test]
    fn or_first_truthy() {
        let ctx = test_ctx();
        let result = eval_func("Or", &args(&["", "b", "c"]), &ctx).unwrap();
        assert_eq!(result, "b");
    }

    #[test]
    fn or_all_empty() {
        let ctx = test_ctx();
        let result = eval_func("Or", &args(&["", "", ""]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $Not ────────────────────────────────────────────────────────

    #[test]
    fn not_truthy() {
        let ctx = test_ctx();
        let result = eval_func("Not", &args(&["yes"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn not_empty() {
        let ctx = test_ctx();
        let result = eval_func("Not", &args(&[""]), &ctx).unwrap();
        assert_eq!(result, "1");
    }

    // ── $IsNull ─────────────────────────────────────────────────────

    #[test]
    fn is_null_empty() {
        let ctx = test_ctx();
        let result = eval_func("IsNull", &args(&[""]), &ctx).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn is_null_non_empty() {
        let ctx = test_ctx();
        let result = eval_func("IsNull", &args(&["x"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $Contains ───────────────────────────────────────────────────

    #[test]
    fn contains_found() {
        let ctx = test_ctx();
        let result = eval_func("Contains", &args(&["Hello World", "world"]), &ctx).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn contains_not_found() {
        let ctx = test_ctx();
        let result = eval_func("Contains", &args(&["Hello", "xyz"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $Replace ────────────────────────────────────────────────────

    #[test]
    fn replace_basic() {
        let ctx = test_ctx();
        let result = eval_func("Replace", &args(&["foo bar foo", "foo", "baz"]), &ctx).unwrap();
        assert_eq!(result, "baz bar baz");
    }

    #[test]
    fn replace_no_match() {
        let ctx = test_ctx();
        let result = eval_func("Replace", &args(&["hello", "xyz", "abc"]), &ctx).unwrap();
        assert_eq!(result, "hello");
    }

    // ── $Upper / $Lower ─────────────────────────────────────────────

    #[test]
    fn upper() {
        let ctx = test_ctx();
        let result = eval_func("Upper", &args(&["hello"]), &ctx).unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn lower() {
        let ctx = test_ctx();
        let result = eval_func("Lower", &args(&["HELLO"]), &ctx).unwrap();
        assert_eq!(result, "hello");
    }

    // ── $Left / $Right ──────────────────────────────────────────────

    #[test]
    fn left_within_bounds() {
        let ctx = test_ctx();
        let result = eval_func("Left", &args(&["Hello", "3"]), &ctx).unwrap();
        assert_eq!(result, "Hel");
    }

    #[test]
    fn left_beyond_bounds() {
        let ctx = test_ctx();
        let result = eval_func("Left", &args(&["Hi", "10"]), &ctx).unwrap();
        assert_eq!(result, "Hi");
    }

    #[test]
    fn right_within_bounds() {
        let ctx = test_ctx();
        let result = eval_func("Right", &args(&["Hello", "3"]), &ctx).unwrap();
        assert_eq!(result, "llo");
    }

    #[test]
    fn right_beyond_bounds() {
        let ctx = test_ctx();
        let result = eval_func("Right", &args(&["Hi", "10"]), &ctx).unwrap();
        assert_eq!(result, "Hi");
    }

    // ── $Mid ────────────────────────────────────────────────────────

    #[test]
    fn mid_with_length() {
        let ctx = test_ctx();
        let result = eval_func("Mid", &args(&["Hello World", "6", "5"]), &ctx).unwrap();
        assert_eq!(result, "World");
    }

    #[test]
    fn mid_without_length() {
        let ctx = test_ctx();
        let result = eval_func("Mid", &args(&["Hello World", "6"]), &ctx).unwrap();
        assert_eq!(result, "World");
    }

    // ── $Trim ───────────────────────────────────────────────────────

    #[test]
    fn trim_whitespace() {
        let ctx = test_ctx();
        let result = eval_func("Trim", &args(&["  hello  "]), &ctx).unwrap();
        assert_eq!(result, "hello");
    }

    // ── $Split ──────────────────────────────────────────────────────

    #[test]
    fn split_in_range() {
        let ctx = test_ctx();
        let result = eval_func("Split", &args(&["a; b; c", "; ", "1"]), &ctx).unwrap();
        assert_eq!(result, "b");
    }

    #[test]
    fn split_out_of_range() {
        let ctx = test_ctx();
        let result = eval_func("Split", &args(&["a; b", "; ", "5"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $Pad ────────────────────────────────────────────────────────

    #[test]
    fn pad_zero_fill() {
        let ctx = test_ctx();
        let result = eval_func("Pad", &args(&["3", "2"]), &ctx).unwrap();
        assert_eq!(result, "03");
    }

    #[test]
    fn pad_custom_char() {
        let ctx = test_ctx();
        let result = eval_func("Pad", &args(&["3", "4", " "]), &ctx).unwrap();
        assert_eq!(result, "   3");
    }

    #[test]
    fn pad_already_wide() {
        let ctx = test_ctx();
        let result = eval_func("Pad", &args(&["123", "2"]), &ctx).unwrap();
        assert_eq!(result, "123");
    }

    // ── $Date ───────────────────────────────────────────────────────

    #[test]
    fn date_default_format() {
        let ctx = test_ctx();
        let result = eval_func("Date", &args(&[]), &ctx).unwrap();
        // Should be in YYYY-MM-DD format
        assert!(result.len() == 10);
        assert!(result.contains('-'));
    }

    #[test]
    fn date_custom_format() {
        let ctx = test_ctx();
        let result = eval_func("Date", &args(&["%Y"]), &ctx).unwrap();
        // Should be a 4-digit year
        assert!(result.len() == 4);
        assert!(result.parse::<u32>().is_ok());
    }

    // ── $Format ─────────────────────────────────────────────────────

    #[test]
    fn format_integer() {
        let ctx = test_ctx();
        let result = eval_func("Format", &args(&["3.14159"]), &ctx).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn format_decimal() {
        let ctx = test_ctx();
        let result = eval_func("Format", &args(&["3.14159", "2"]), &ctx).unwrap();
        assert_eq!(result, "3.14");
    }

    #[test]
    fn format_not_a_number() {
        let ctx = test_ctx();
        let result = eval_func("Format", &args(&["abc"]), &ctx);
        assert!(result.is_err());
    }

    // ── $Count ──────────────────────────────────────────────────────

    #[test]
    fn count_default_separator() {
        let ctx = test_ctx();
        let result = eval_func("Count", &args(&["a; b; c"]), &ctx).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn count_custom_separator() {
        let ctx = test_ctx();
        let result = eval_func("Count", &args(&["a,b,c", ","]), &ctx).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn count_empty() {
        let ctx = test_ctx();
        let result = eval_func("Count", &args(&[""]), &ctx).unwrap();
        assert_eq!(result, "0");
    }

    // ── $Sort ───────────────────────────────────────────────────────

    #[test]
    fn sort_alphabetical() {
        let ctx = test_ctx();
        let result = eval_func("Sort", &args(&["c; a; b"]), &ctx).unwrap();
        assert_eq!(result, "a; b; c");
    }

    #[test]
    fn sort_empty() {
        let ctx = test_ctx();
        let result = eval_func("Sort", &args(&[""]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $IsMatch ────────────────────────────────────────────────────

    #[test]
    fn is_match_true() {
        let ctx = test_ctx();
        let result = eval_func("IsMatch", &args(&["Song 01", r"^\w+ \d+"]), &ctx).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn is_match_false() {
        let ctx = test_ctx();
        let result = eval_func("IsMatch", &args(&["abc", r"^\d+$"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn is_match_invalid_regex() {
        let ctx = test_ctx();
        let result = eval_func("IsMatch", &args(&["abc", r"[invalid"]), &ctx);
        assert!(result.is_err());
    }

    // ── $Lookup ─────────────────────────────────────────────────────

    #[test]
    fn lookup_genre_known() {
        let ctx = test_ctx();
        let result = eval_func("Lookup", &args(&["Rock", "genre_folder"]), &ctx).unwrap();
        assert_eq!(result, "Rock");
    }

    #[test]
    fn lookup_genre_unknown() {
        let ctx = test_ctx();
        let result = eval_func("Lookup", &args(&["Polka", "genre_folder"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn lookup_unknown_table() {
        let ctx = test_ctx();
        let result = eval_func("Lookup", &args(&["key", "nonexistent"]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $MediaClass / $MediaGroup ───────────────────────────────────

    #[test]
    fn media_class_with_classification() {
        let tags: &'static TagMap = Box::leak(Box::new(TagMap::new()));
        let classification = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::MP3,
            MediaClass::Music,
            MediaQuality::Lossy320,
        );
        let classification_ref: &'static MediaClassification = Box::leak(Box::new(classification));
        let ctx = EvalContext {
            tags,
            audio_props: None,
            classification: Some(classification_ref),
            file_path: None,
            path_mode: false,
            missing_tag_mode: super::super::evaluator::MissingTagMode::Empty,
        };
        let result = eval_func("MediaClass", &[], &ctx).unwrap();
        assert_eq!(result, "Music");
    }

    #[test]
    fn media_group_with_classification() {
        let tags: &'static TagMap = Box::leak(Box::new(TagMap::new()));
        let classification = MediaClassification::new(
            MediaGroup::Video,
            MediaFormat::MKV,
            MediaClass::Movie,
            MediaQuality::Standard,
        );
        let classification_ref: &'static MediaClassification = Box::leak(Box::new(classification));
        let ctx = EvalContext {
            tags,
            audio_props: None,
            classification: Some(classification_ref),
            file_path: None,
            path_mode: false,
            missing_tag_mode: super::super::evaluator::MissingTagMode::Empty,
        };
        let result = eval_func("MediaGroup", &[], &ctx).unwrap();
        assert_eq!(result, "Video");
    }

    #[test]
    fn media_class_without_classification() {
        let ctx = test_ctx();
        let result = eval_func("MediaClass", &[], &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── $FirstValue ─────────────────────────────────────────────────

    #[test]
    fn first_value_default_separator() {
        let ctx = test_ctx();
        let result = eval_func("FirstValue", &args(&["Rock; Pop; Jazz"]), &ctx).unwrap();
        assert_eq!(result, "Rock");
    }

    #[test]
    fn first_value_custom_separator() {
        let ctx = test_ctx();
        let result = eval_func("FirstValue", &args(&["a,b,c", ","]), &ctx).unwrap();
        assert_eq!(result, "a");
    }

    #[test]
    fn first_value_empty() {
        let ctx = test_ctx();
        let result = eval_func("FirstValue", &args(&[""]), &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── Error cases ─────────────────────────────────────────────────

    #[test]
    fn unknown_function_error() {
        let ctx = test_ctx();
        let result = eval_func("NonExistent", &args(&["a"]), &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown template function")
        );
    }

    #[test]
    fn wrong_arg_count_error() {
        let ctx = test_ctx();
        // $Upper expects exactly 1 arg
        let result = eval_func("Upper", &args(&["a", "b"]), &ctx);
        assert!(result.is_err());
    }

    // ── Case-insensitive dispatch ───────────────────────────────────

    #[test]
    fn case_insensitive_function_name() {
        let ctx = test_ctx();
        let r1 = eval_func("UPPER", &args(&["hi"]), &ctx).unwrap();
        let r2 = eval_func("upper", &args(&["hi"]), &ctx).unwrap();
        let r3 = eval_func("Upper", &args(&["hi"]), &ctx).unwrap();
        assert_eq!(r1, "HI");
        assert_eq!(r2, "HI");
        assert_eq!(r3, "HI");
    }
}
