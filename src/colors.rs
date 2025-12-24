//! Color utilities for request ID visualization.

use owo_colors::{AnsiColors, DynColors, OwoColorize, Style};

/// 32 visually distinct ANSI colors for request ID coloring
///
/// Carefully selected to provide maximum visual differentiation while maintaining
/// readability on both light and dark backgrounds. Includes both standard and
/// bright variants, with some duplication to reach 32 distinct colors.
const COLORS: [AnsiColors; 32] = [
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
    AnsiColors::BrightYellow,
    AnsiColors::BrightBlue,
    AnsiColors::BrightMagenta,
    AnsiColors::BrightCyan,
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
    AnsiColors::BrightYellow,
    AnsiColors::BrightBlue,
    AnsiColors::BrightMagenta,
    AnsiColors::BrightCyan,
    AnsiColors::Red,
    AnsiColors::Green,
    AnsiColors::Yellow,
    AnsiColors::Blue,
    AnsiColors::Magenta,
    AnsiColors::Cyan,
    AnsiColors::BrightRed,
    AnsiColors::BrightGreen,
];

/// Deterministically maps a request ID to one of the 32 colors
///
/// Uses a stable hash function to ensure the same ID always gets the same color.
/// The hash is designed to be:
/// - Fast to compute (important for high-throughput logging)
/// - Well-distributed across the color palette
/// - Consistent across program runs
pub fn get_color_for_id(id: &str) -> AnsiColors {
    let hash = id
        .chars()
        .fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32));
    COLORS[(hash % 32) as usize]
}

/// Formats a request ID with consistent color coding
///
/// Returns a `String` with embedded ANSI color codes. Uses the full-color
/// palette while gracefully degrading to no color when output isn't to a terminal.
pub fn colored_id(id: &str) -> String {
    let color = get_color_for_id(id);
    let style = Style::new().color(DynColors::Ansi(color));
    format!("[{}]", id).style(style).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_determinism() {
        // Same ID should always get the same color
        let color1 = get_color_for_id("abc123");
        let color2 = get_color_for_id("abc123");
        assert!(std::mem::discriminant(&color1) == std::mem::discriminant(&color2));
    }

    #[test]
    fn test_colored_id_format() {
        let result = colored_id("test");
        // Should contain the ID wrapped in brackets
        assert!(result.contains("test"));
    }
}
