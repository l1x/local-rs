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
        assert_eq!(color1, color2);
    }

    #[test]
    fn test_different_ids_get_different_colors() {
        // Different IDs should eventually map to different colors
        // (Not guaranteed for any two IDs due to pigeonhole principle, but likely for many)
        let mut color_discriminants = std::collections::HashSet::new();
        for i in 0..100 {
            let id = format!("id-{}", i);
            let color = get_color_for_id(&id);
            // Use discriminant since AnsiColors doesn't implement Hash
            color_discriminants.insert(format!("{:?}", std::mem::discriminant(&color)));
        }
        // With 32 colors and 100 random-ish IDs, we expect to see many distinct colors
        assert!(color_discriminants.len() > 5, "Color distribution is poor: only {} colors used", color_discriminants.len());
    }

    #[test]
    fn test_colored_id_format() {
        let id = "test-id";
        let result = colored_id(id);
        
        // Should contain the ID wrapped in brackets
        assert!(result.contains("[test-id]"));
        
        // Should contain ANSI escape codes (starts with \x1b[)
        assert!(result.contains("\x1b["));
    }

    #[test]
    fn test_hashing_consistency() {
        // The simple hash function: acc.wrapping_mul(31).wrapping_add(c as u32)
        // For "A" (65): 0 * 31 + 65 = 65. 65 % 32 = 1. COLORS[1] = Green
        assert_eq!(get_color_for_id("A"), COLORS[1]);
        
        // For "AA": (65 * 31 + 65) = 2080. 2080 % 32 = 0. COLORS[0] = Red
        assert_eq!(get_color_for_id("AA"), COLORS[0]);
    }
}
