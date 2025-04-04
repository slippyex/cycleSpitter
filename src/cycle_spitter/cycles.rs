// src/cycle_spitter/cycles.rs
//! # Cycles Module
//!
//! This module is responsible for analyzing and normalizing assembly code lines and then
//! looking up execution cycle counts for the normalized instruction. The module leverages
//! regular expressions for pattern matching and a JSON-based cycle database for lookup.
//!
//! ## Static Regular Expressions
//! A set of `Lazy<Regex>` static variables are used to compile and reuse regex patterns for:
//! - **REG_DISPLACEMENT**: Matches instructions with displacement addressing mode (e.g., `d(sp)`).
//! - **REG_INSTRUCTION**: Matches specific instructions like `lea` or `moveq`.
//! - **REG_IMMEDIATE**: Matches immediate values (e.g., `#1234`).
//! - **REG_DATA**: Matches data register patterns (e.g., `d0`, `d7`).
//! - **REG_ADDR**: Matches address registers (e.g., `a0` to `a7` or `sp`).
//! - **REG_ABS_ADDRESS**: Matches absolute addressing tokens.
//! - **REG_SPACES**: Matches spaces and tabs.
//! - **REG_BCC**: Matches branch conditions (`bt`, `bf`, etc.).
//!
//! ## Static Lookup Map
//! - **CYCLES_MAP**: A `HashMap` populated from the `cycles.json` file that contains
//!   instruction-to-cycle information. The JSON file is located at `db/cycles.json` within the project.
//!
//! ## Functions
//! - `lookup_cycles`: Takes an assembly instruction line as input, normalizes it, and retrieves
//!   the corresponding cycle count from the preloaded `CYCLES_MAP`. Throws a warning and
//!   returns 0 in case the instruction is not found in the cycle database.
//! - `normalize_line`: Normalizes an assembly instruction line by stripping extraneous spaces,
//!   adjusting instruction formats, converting specific operands into standardized
//!   placeholders for effective database lookups.
//!
//! ### Usage
//! This module is primarily designed to provide normalized instruction strings and their
//! associated execution cycles in CPU emulation tools or static analysis tools.
//!
//! ## Example
//! ```rust
//! use cycle_spitter::cycles::{lookup_cycles};
//!
//! let line = " moveq #16,d0";
//! let cycles = lookup_cycles(line);
//! println!("Instruction: {}, Cycles: {}", line, cycles);
//! ```

// Detailed Descriptions of Individual Components

//! ### Static Variables
//! - **REG_DISPLACEMENT**:
//!   Matches displacement addressing patterns like `value(sp)` or `value(a3)`.
//! - **REG_INSTRUCTION**:
//!   Matches specific instructions `lea` or `moveq` that require `.l` suffix normalization.
//! - **REG_IMMEDIATE**:
//!   Matches immediate values like `#value` and replaces them with generic `#xxx`.
//! - **REG_DATA**:
//!   Matches data registers (`d0` to `d7`) and replaces them with `dn`.
//! - **REG_ADDR**:
//!   Matches address registers (`a0` to `a7` or `sp`) and replaces them with `an`.
//! - **REG_ABS_ADDRESS**:
//!   Captures and normalizes absolute addresses, with optional `.l` or `.w` suffixes.
//! - **REG_SPACES**:
//!   Collapses multiple spaces or tabs into a single space.
//! - **REG_BCC**:
//!   Matches branch conditions with or without suffixes and ensures a `.b` or `.w` suffix is applied.
//!
//! ### `lookup_cycles` Function
//! Retrieves the number of execution cycles for a given instruction line.
//! - **Parameters**:
//!   - `line`: A `&str` containing the full assembly instruction line.
//! - **Returns**:
//!   - A `usize` representing the number of cycles required for the given instruction.
//! - **Behavior**:
//!   - Normalizes the input instruction using `normalize_line`.
//!   - Performs a lookup in the `CYCLES_MAP`.
//!   - If a match is not found, issues a warning on `stderr` and returns `0`.
//!
//! ### `normalize_line` Function
//! Standardizes an instruction line into a format suitable for efficient database lookup.
//! - **Parameters**:
//!   - `line`: A `&str` containing the raw assembly instruction line.
//! - **Returns**:
//!   - A `String` representing the normalized instruction.
//! - **Normalization Steps**:
//!   - Strips extraneous whitespace.
//!   - Adds `.l` suffix to specific instructions like `lea` or `moveq`.
//!   - Replaces branch conditions with `.b` or `.w` suffixes.
//!   - Replaces certain operand patterns with placeholders (e.g., `dn`, `an`, `#xxx`, etc.).

use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde_json;

use regex::Regex;

static REG_DISPLACEMENT: Lazy<Regex> = Lazy::new(|| {
    // Matches an operand in the format: `<displacement>(<address_register>)`
    // Example matches: `12(a0)`, `-4(sp)`
    // - `[^\s,()]+`: Matches a series of characters that are not whitespace, commas, or parentheses
    // - `\(a[0-7]|sp\)`: Matches an address register (`a0`-`a7`) or the stack pointer (`sp`) inside parentheses
    Regex::new(r"([^\s,()]+)\((a[0-7]|sp)\)").unwrap()
});

static REG_INSTRUCTION: Lazy<Regex> = Lazy::new(|| {
    // Matches specific instructions: `lea` or `moveq` at the start of a line
    // Example matches: `lea`, `moveq`
    // - `^`: Asserts that the match occurs at the beginning of the string
    // - `(lea|moveq)`: Matches either `lea` or `moveq`
    Regex::new(r"^(lea|moveq)$").unwrap()
});

static REG_IMMEDIATE: Lazy<Regex> = Lazy::new(|| {
    // Matches immediate values prefixed with `#`
    // Example matches: `#123`, `#-45`
    // - `#[^,\s]+`: Matches a `#` followed by a series of characters that are not commas or whitespace
    Regex::new(r"(#[^,\s]+)").unwrap()
});

static REG_DATA: Lazy<Regex> = Lazy::new(|| {
    // Matches data registers `d0` through `d7`
    // Example matches: `d0`, `d7`
    // - `\b`: Asserts a word boundary to ensure precise matching
    // - `d[0-7]`: Matches `d` followed by a single digit from 0 to 7
    Regex::new(r"\bd[0-7]\b").unwrap()
});

static REG_ADDR: Lazy<Regex> = Lazy::new(|| {
    // Matches address registers `a0` through `a7` or the stack pointer `sp`
    // Example matches: `a0`, `a7`, `sp`
    // - `\b`: Asserts a word boundary to ensure precise matching
    // - `(a[0-7]|sp)`: Matches `a` followed by a digit from 0 to 7, or `sp`
    Regex::new(r"\b(a[0-7]|sp)\b").unwrap()
});

static REG_ABS_ADDRESS: Lazy<Regex> = Lazy::new(|| {
    // Matches absolute addresses, optionally followed by a `.l` or `.w` suffix
    // Example matches: `label`, `label.l`, `label.w`
    // - `(?P<before>^|[ \t,(\[])`: Matches the start of the string or a space, tab, comma, parenthesis, or square bracket
    // - `(?P<token>[a-zA-Z_][a-zA-Z0-9_]*)`: Matches an identifier (starts with a letter/underscore, followed by letters, digits, or underscores)
    // - `(?P<suffix>\.[lw])?`: Optionally matches a `.l` or `.w` suffix
    Regex::new(
        r"(?P<before>^|[ \t,(\[])(?P<token>[a-zA-Z_][a-zA-Z0-9_]*)(?P<suffix>\.[lw])?\b"
    ).unwrap()
});

static REG_SPACES: Lazy<Regex> = Lazy::new(|| {
    // Matches one or more spaces or tabs
    // Example matches: ` `, `\t`, `    `
    // - `[ \t]+`: Matches one or more occurrences of a space or tab character
    Regex::new(r"[ \t]+").unwrap()
});

static REG_BCC: Lazy<Regex> = Lazy::new(|| {
    // Matches branch instructions beginning with `b` (e.g., `bra`, `beq`) and optionally ending with `.s`
    // Example matches: `bne`, `bra.s`
    // - `^`: Asserts that the match occurs at the beginning of the string
    // - `(b[A-Za-z]{2})`: Matches `b` followed by any two letters
    // - `(\.s)?`: Optionally matches a `.s` suffix
    Regex::new(r"^(b[A-Za-z]{2})(\.s)?").unwrap()
});

static CYCLES_MAP: Lazy<HashMap<String, Vec<usize>>> = Lazy::new(|| {
    let json_str = include_str!("db/cycles.json");
    serde_json::from_str(json_str).expect("Error parsing cycles JSON")
});

pub struct CycleCount {
    pub cycles: usize,
    pub lookup: String
}

pub fn lookup_cycles(line: &str) -> CycleCount {
    let normalized = normalize_line(line);

    let result = if let Some(cycles) = CYCLES_MAP.get(normalized.as_str()) {
        CycleCount {
            cycles: cycles[0],
            lookup: normalized
        }
    } else {
        eprintln!("Warning: No cycle count found for instruction: {}", line);
        CycleCount {
            cycles: 0,
            lookup: normalized
        }
    };
    result
}

pub fn normalize_line(line: &str) -> String {
    let line_without_comment = match line.find(';') {
        Some(idx) => &line[..idx],
        None => line,
    };

    // Remove leading label like `my_label:` if present
    let line_without_label = Regex::new(r"^\s*[a-zA-Z_][a-zA-Z0-9_]*:\s*")
        .unwrap()
        .replace(line_without_comment, "")
        .to_string();

    let trimmed = line_without_label.trim().to_lowercase();

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let first_token = parts.next().unwrap();
    let operand_part = parts.next().unwrap_or("");

    let first_token = {
        if REG_INSTRUCTION.is_match(first_token) {
            // For instructions like "lea" or "moveq", always force a ".l" suffix.
            format!("{}.l", first_token)
        } else if let Some(caps) = REG_BCC.captures(first_token) {
            // Handle branch conditions.
            let trailing = caps.get(3).map_or("", |m| m.as_str());
            if caps.get(2).is_some() {
                format!("{}{}{}", &caps[1], ".b", trailing)
            } else {
                format!("{}{}{}", &caps[1], ".w", trailing)
            }
        } else if !first_token.contains('.') {
            // If no suffix is found, default to ".w"
            format!("{}.w", first_token)
        } else {
            first_token.to_string()
        }
    };

    let mut operands = operand_part.to_string();
    operands = REG_DISPLACEMENT.replace_all(&operands, |captures: &regex::Captures| {
        if &captures[1] == "-" {
            // If the first capture is "-", keep it unchanged
            format!("-({})", &captures[2])
        } else {
            // Otherwise, replace with "d($2)"
            format!("d({})", &captures[2])
        }
    }).into_owned();
    operands = REG_IMMEDIATE.replace_all(&operands, "#xxx").into_owned();
    // c. Replace data registers.
    operands = REG_DATA.replace_all(&operands, "dn").into_owned();
    // d. Replace address registers.
    operands = REG_ADDR.replace_all(&operands, "an").into_owned();
    // e. Replace any remaining absolute addresses.
    operands = REG_ABS_ADDRESS.replace_all(&operands, |caps: &regex::Captures| {
        let before = caps.name("before").unwrap().as_str();
        let token = caps.name("token").unwrap().as_str();
        let suffix = caps.name("suffix").map(|m| m.as_str());
        if token == "an" || token == "dn" || token == "d" {
            caps.get(0).unwrap().as_str().to_string()
        } else {
            if let Some(suf) = suffix {
                if suf == ".w" {
                    format!("{}xxx.w", before)
                } else {
                    format!("{}xxx.l", before)
                }
            } else {
                format!("{}xxx.l", before)
            }
        }
    }).into_owned();
    // f. Collapse multiple spaces.
    operands = REG_SPACES.replace_all(&operands, " ").into_owned();
    let mut operands = operands.trim().to_owned();
    operands = if operands.contains('$') {
        // Use a non-capturing group for the '$'
        let re = Regex::new(r"\$(\w+)(\.w)?([,;\n\t])?").unwrap();
        let result = re.replace_all(&operands, |caps: &regex::Captures| {
            // Capture any punctuation (if present).
            let punctuation = caps.get(3).map_or("", |m| m.as_str());
            // If group 2 (".w") is present, use "xxx.w", else use "xxx.l".
            if caps.get(2).is_some() {
                format!("xxx.w{}", punctuation)
            } else {
                format!("xxx.l{}", punctuation)
            }
        });
        result.into_owned()
    } else {
        operands.to_string()
    };


    if operands.is_empty() {
        first_token
    } else {
        format!("{} {}", first_token, operands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that `lookup_cycles` works with valid instructions present in the `CYCLES_MAP`.
    #[test]
    fn test_lookup_cycles_valid_instruction() {
        let line = "moveq #16,d0";
        let cycles = lookup_cycles(line);
        assert!(cycles.cycles > 0, "Valid instruction should return a positive cycle count.");
    }

    /// Test that `lookup_cycles` returns 0 for unknown instructions.
    #[test]
    fn test_lookup_cycles_unknown_instruction() {
        let line = "unknown_op #42,d1";
        let cycles = lookup_cycles(line);
        assert_eq!(cycles.cycles, 0, "Unknown instructions should return 0 cycles.");
    }

    /// Test that `lookup_cycles` handles instructions with normalized cases.
    #[test]
    fn test_lookup_cycles_normalized_instruction() {
        let line = " moveq #12,d2  "; // Misformatted but equivalent to "moveq #12,d2"
        let normalized_line = normalize_line(line);
        assert_eq!(normalized_line, "moveq.l #xxx,dn");

        let line = " add #12,d2  "; // Misformatted but equivalent to "add.w #12,d2"
        let normalized_line = normalize_line(line);
        assert_eq!(normalized_line, "add.w #xxx,dn");

        let line = " moveq #12,D2";
        let normalized_line = normalize_line(line);
        assert_eq!(normalized_line, "moveq.l #xxx,dn");

        let line = " MOVE.W A1,A2";
        let normalized_line = normalize_line(line);
        assert_eq!(normalized_line, "move.w an,an");

    }

    /// Test normalization of valid lines.
    #[test]
    fn test_normalize_line_valid_cases() {
        assert_eq!(
            normalize_line("move.l d0,a1"),
            "move.l dn,an",
            "Expected `move.l` instruction with displacement to normalize correctly."
        );
        assert_eq!(
            normalize_line("lea $ffff8240.w,a0"),
            "lea.l xxx.w,an",
            "Expected `lea` instruction with absolute.w to normalize correctly."
        );
        assert_eq!(
            normalize_line("lea $ffff8240,a0"),
            "lea.l xxx.l,an",
            "Expected `lea` instruction with absolute.l to normalize correctly."
        );
        assert_eq!(
            normalize_line("move.w $ffff8240.w,d0"),
            "move.w xxx.w,dn",
            "Expected `move.w` instruction with absolute.w to normalize correctly."
        );
        assert_eq!(
            normalize_line("move.w d0,$ffff8240.w"),
            "move.w dn,xxx.w",
            "Expected `move.w` instruction with absolute.w to normalize correctly."
        );
        assert_eq!(
            normalize_line("move.w $ffff8240,d0"),
            "move.w xxx.l,dn",
            "Expected `move.w` instruction with absolute to normalize correctly."
        );
        assert_eq!(
            normalize_line("move.w d0,$ffff8240"),
            "move.w dn,xxx.l",
            "Expected `move.w` instruction with absolute to normalize correctly."
        );
        assert_eq!(
            normalize_line("move.b	d7,$ffff8260.w			;"),
            "move.b dn,xxx.w",
            "Expected `move.w` instruction with absolute to normalize correctly."
        );
        assert_eq!(
            normalize_line("bne.s label.w"),
            "bne.b xxx.w",
            "Expected branch instruction to normalize to `.b` suffix."
        );
    }

    /// Test that normalization leaves already normalized items unchanged.
    #[test]
    fn test_normalize_already_normalized_instructions() {
        let line = "moveq.l #xxx,dn";
        assert_eq!(
            normalize_line(line),
            line,
            "Already normalized instruction should remain unchanged."
        );
    }

    /// Test immediate value normalization.
    #[test]
    fn test_normalize_immediate_values() {
        assert_eq!(
            normalize_line("addq.l #20,d1"),
            "addq.l #xxx,dn",
            "Immediate values should be replaced with #xxx."
        );
    }

    /// Test normalization of displacement addressing.
    #[test]
    fn test_normalize_displacement_addressing() {
        let line = "lea 100(sp),a1";
        let expected = "lea.l d(an),an"; // since sp internally resolves into a7 = an
        assert_eq!(
            normalize_line(line),
            expected,
            "Displacement addressing should be normalized properly."
        );
    }

    /// Test register normalization.
    #[test]
    fn test_normalize_registers() {
        let line = "movem.l d0-d7/a0-a6,-(sp)";
        let expected = "movem.l dn-dn/an-an,-(an)";
        assert_eq!(
            normalize_line(line),
            expected,
            "Registers (data/address) should be replaced with placeholders."
        );

        let line = "movem.l (sp)+,d0-d7/a0-a6";
        let expected = "movem.l (an)+,dn-dn/an-an";
        assert_eq!(
            normalize_line(line),
            expected,
            "Registers (data/address) should be replaced with placeholders."
        );

    }

    /// Test absolute addressing normalization.
    #[test]
    fn test_normalize_absolute_addressing() {
        let line = "movea.l my_label,a0";
        let expected = "movea.l xxx.l,an";
        assert_eq!(
            normalize_line(line),
            expected,
            "Absolute addressing should be normalized to `xxx.l`."
        );
    }

    /// Test that malformed instructions don't cause crashes.
    #[test]
    fn test_normalize_malformed_input() {
        let line = "moveq #16";
        let result = normalize_line(line);
        assert!(!result.is_empty(), "Malformed input should result in a non-empty result.");
    }

    /// Test an instruction with a label
    #[test]
    fn test_normalize_with_label_input() {
        let line = "my_label:\tmoveq #16,d1";
        let expected = "moveq.l #xxx,dn";
        assert_eq!(
            normalize_line(line),
            expected,
            "Absolute addressing should be normalized to `xxx.l`."
        );
    }

    /// Test that whitespace is handled correctly.
    #[test]
    fn test_normalize_whitespace_handling() {
        let line = "   add.l     d0,d1 ";
        let expected = "add.l dn,dn";
        assert_eq!(
            normalize_line(line),
            expected,
            "Whitespace should be handled and normalized correctly."
        );
    }

    /// Test branch condition with missing suffixes.
    #[test]
    fn test_branch_normalization_with_suffix() {
        assert_eq!(
            normalize_line("bne label"),
            "bne.w xxx.l",
            "Branch instructions should be normalized with `.w` suffix for label."
        );

        assert_eq!(
            normalize_line("bne.s dummy.w"),
            "bne.b xxx.w",
            "Branch instructions with `.s` suffix should normalize correctly."
        );
    }

    /// Test fallback for unknown tokens in normalization.
    #[test]
    fn test_normalize_unknown_tokens() {
        let line = "customop $FF,d1";
        let normalized = normalize_line(line);
        assert_ne!(
            normalized, "",
            "Unknown tokens should still produce a normalized line."
        );
    }
}