// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — CLI Output Formatting
//
// Centralised output helpers for the `meedya` CLI. Provides consistent
// formatting across all commands: human-readable coloured tables or
// machine-parseable JSON output.

use colored::Colorize;
use serde::Serialize;

// ─── Output format ──────────────────────────────────────────────────────────

/// Controls how command results are rendered to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable coloured output with tables and status indicators
    Human,
    /// Machine-parseable JSON output (one object per result)
    Json,
}

// ─── Exit codes ─────────────────────────────────────────────────────────────

/// Standard process exit codes used across all CLI commands.
pub struct ExitCode;

impl ExitCode {
    /// Command completed successfully
    pub const SUCCESS: i32 = 0;
    /// Command failed with an error
    pub const ERROR: i32 = 1;
    /// Command completed with partial results (some items succeeded, some failed)
    pub const PARTIAL: i32 = 2;
}

// ─── Table output ───────────────────────────────────────────────────────────

/// Print a coloured, column-aligned table to stdout.
///
/// Headers are printed in bold cyan. Columns are separated by two spaces and
/// padded to the widest value in each column.
///
/// # Arguments
/// * `headers` — Column header labels
/// * `rows` — Row data; each inner `Vec` must have the same length as `headers`
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    // Nothing to display if there are no headers
    if headers.is_empty() {
        return;
    }

    // Calculate the maximum width for each column (header or data)
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            // Guard against rows shorter than the header count
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Print header row — bold cyan with column padding
    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{}", header_line.bold().cyan());

    // Print separator line — dashes under each column
    let separator: String = widths
        .iter()
        .map(|w| "─".repeat(*w))
        .collect::<Vec<_>>()
        .join("──");
    println!("{}", separator.dimmed());

    // Print each data row with column padding
    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                // Pad cell to column width (use 0 if index out of range)
                let width = widths.get(i).copied().unwrap_or(0);
                format!("{cell:<width$}")
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!("{line}");
    }
}

// ─── Key-value output ───────────────────────────────────────────────────────

/// Print a labelled key-value pair.
///
/// The key is printed in bold with a colon separator, followed by the value.
pub fn print_key_value(key: &str, value: &str) {
    println!("{}: {}", key.bold(), value);
}

// ─── JSON output ────────────────────────────────────────────────────────────

/// Serialize a value as pretty-printed JSON and write to stdout.
///
/// This is used in JSON output mode (`--json`) to produce machine-parseable
/// results. Panics are avoided — serialization errors are printed as error
/// messages instead.
pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => print_error(&format!("Failed to serialize JSON: {e}")),
    }
}

// ─── Status messages ────────────────────────────────────────────────────────

/// Print a success message in green with a check mark prefix.
pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg.green());
}

/// Print a warning message in yellow with a warning prefix.
pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".yellow().bold(), msg.yellow());
}

/// Print an error message in red with an X prefix.
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg.red());
}

// ─── Section headers ────────────────────────────────────────────────────────

/// Print a section header with an underline.
///
/// Used to visually separate different parts of command output (e.g.,
/// "Classification", "Tags", "Audio Properties" in the debug command).
pub fn print_header(msg: &str) {
    println!();
    println!("{}", msg.bold().underline());
}

// ─── Progress indicator ─────────────────────────────────────────────────────

#[allow(dead_code)] // Will be used by scan/watch commands in future iterations
/// Print a simple progress line using carriage return (overwrites current line).
///
/// Suitable for operations that process many files sequentially. The line
/// is overwritten on each call to show updated progress.
///
/// # Arguments
/// * `current` — Number of items processed so far
/// * `total` — Total number of items
/// * `label` — Short description of the operation (e.g., "Scanning files")
pub fn print_progress(current: usize, total: usize, label: &str) {
    // Use \r to overwrite the line; flush is handled by the next print
    eprint!("\r{label}: {current}/{total}");
    // If we've reached the end, print a newline to finalize the line
    if current >= total {
        eprintln!();
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that OutputFormat enum variants are distinct
    #[test]
    fn output_format_equality() {
        assert_eq!(OutputFormat::Human, OutputFormat::Human);
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Human, OutputFormat::Json);
    }

    /// Verify exit code constants have correct values
    #[test]
    fn exit_code_values() {
        assert_eq!(ExitCode::SUCCESS, 0);
        assert_eq!(ExitCode::ERROR, 1);
        assert_eq!(ExitCode::PARTIAL, 2);
    }

    /// Verify print_json produces valid JSON for a simple struct
    #[test]
    fn json_serialization() {
        // This test verifies the serialization path works without panicking.
        // We can't easily capture stdout in a unit test, but we verify the
        // function doesn't panic on a valid serializable value.
        #[derive(Serialize)]
        struct TestData {
            name: String,
            count: u32,
        }
        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };
        // Should not panic
        print_json(&data);
    }

    /// Verify print_table handles empty input without panicking
    #[test]
    fn table_empty_input() {
        // Empty headers — should print nothing
        print_table(&[], &[]);
        // Headers but no rows — should print header + separator only
        print_table(&["Name", "Value"], &[]);
    }
}
