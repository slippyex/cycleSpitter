// src/cycle_spitter/template.rs

use regex::Regex;
use std::error::Error;
use crate::cycle_spitter::helpers::{extract_cycle_count, format_accumulated_instruction};
use once_cell::sync::Lazy;

/// Represents a section of a parsed template.
/// Each section contains:
/// - `injection_code`: A vector of tuples, where each tuple contains assembly code (String)
///   and its associated cycle count (usize).
/// - `nop_cycles`: The number of NOP (No Operation Placeholder) cycles in the section.
/// - `label`: A label identifying the section.
#[derive(Debug)]
pub struct TemplateSection {
    pub injection_code: Vec<(String, usize)>, // (code, cycles)
    pub nop_cycles: usize,
    pub label: String,
}

static NOP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"dcb\.w\s*(\d+),\s*\$4e71").unwrap()
});

static COMMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r";\s*(.*)").unwrap()
});

static PAREN_NUM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\(\s*\d+\s*\)").unwrap()
});

/// Parses the given template content into a vector of `TemplateSection` objects.
///
/// # Arguments
/// - `template_content`: A string slice containing the content of the template to parse.
///
/// # Returns
/// A `Result` containing:
/// - A `Vec` of `TemplateSection` objects on successful parsing.
/// - A boxed `dyn Error` if any errors occur during parsing.
///
/// # Functionality
/// The function processes the template content line by line:
/// - Lines containing NOP (No Operation Placeholder) instructions, identified by the pattern
///   `dcb.w <count>, $4e71`, are used to calculate the associated cycles (`count * 4`). Each
///   NOP section closes the previous block of code, and a new section is created.
/// - Lines containing other types of instructions are associated with a cycle count extracted
///   using the provided `number_re` pattern (if it matches).
/// - Inline comments are used to identify and assign labels to sections.
/// - Unrecognized or empty lines are ignored.
///
/// At the end of the process, any remaining code block is added as the last section.
///
/// # Key Regular Expressions
/// - `nop_re`: Matches NOP instructions of the form `dcb.w <count>, $4e71`.
/// - `comment_re`: Captures inline comments starting with `;`.
///
/// # Behavior
/// - Splits the template into logical sections based on NOP instructions.
/// - Calculates the cycle counts for instructions and NOPs.
/// - Assigns either meaningful labels from comments or generates default labels for sections.
///
/// # Example Usage
/// ```rust
/// use regex::Regex;
/// use your_crate::cycle_spitter::template::{parse_template, TemplateSection};
///
/// let content = r#"
///     dcb.w 5, $4e71
///     move.w #$1234, D0 ; Move instruction
///     dcb.w 3, $4e71
/// "#;
/// let sections = parse_template(content)?;
/// for section in sections {
///     println!("{:?}", section);
/// }
/// ```
///
/// # Errors
/// The function returns an error in the following cases:
/// - If the `Regex` cannot be compiled or fails to capture required groups.
/// - If parsing a numeric value (e.g., cycle count) from captured groups fails.
pub fn parse_template(template_content: &str) -> Result<Vec<TemplateSection>, Box<dyn Error>> {
    // Pre-allocate vectors based on estimated size
    let line_count = template_content.lines().count();
    let mut sections = Vec::with_capacity(line_count / 4); // Rough estimate: one section per 4 lines
    let mut current_code = Vec::with_capacity(4); // Most sections have a few instructions
    let mut current_label = String::with_capacity(32); // Reasonable size for labels

    let mut cycle_offset: usize = 0;
    for line in template_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle set lines first, before any cycle extraction
        if trimmed.contains(" set ") {
            if current_label.is_empty() {
                current_label = COMMENT_RE.captures(trimmed)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("Section {}", sections.len() + 1));
            }
            current_code.push((trimmed.to_string(), 0));
            continue;
        }

        if let Some(caps) = NOP_RE.captures(trimmed) {
            let count = caps.get(1).unwrap().as_str().parse::<usize>()?;
            let cycles = count * 4;

            if !current_code.is_empty() {
                sections.push(TemplateSection {
                    injection_code: current_code,
                    nop_cycles: cycles,
                    label: current_label,
                });
                current_code = Vec::with_capacity(4);
                current_label = String::with_capacity(32);
            }
            continue;
        }

        // Define a predicate for template-specific lines.
        let skip_predicate = |l: &str| {
            l.trim().starts_with(";") ||
                l.contains("dcb.w") ||
                l.contains(" equ ") ||
                PAREN_NUM_RE.is_match(l)
        };

        if let Some(cycle_count) = extract_cycle_count(trimmed, skip_predicate) {
            if current_label.is_empty() {
                current_label = COMMENT_RE.captures(trimmed)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| format!("Section {}", sections.len() + 1));
            }

            let commented_output = format_accumulated_instruction(
                trimmed,
                &cycle_count.lookup,
                &cycle_count.cycles,
                &cycle_count.reg_count,
                cycle_offset
            );
            let caclucated_cycles = if cycle_count.reg_count > 1 {
                cycle_count.cycles[0] + (cycle_count.cycles[1] * cycle_count.reg_count)
            } else {
                cycle_count.cycles[0]
            };
            current_code.push((commented_output, caclucated_cycles));
            cycle_offset += caclucated_cycles
        } else {
            continue;
        }
    }

    if !current_code.is_empty() {
        sections.push(TemplateSection {
            injection_code: current_code,
            nop_cycles: 0,
            label: current_label,
        });
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_instruction_with_cycles() {
        let content = r#"
            move.w #$1323,D0 ; Move Instruction
            dcb.w 2,$4e71
        "#;
        // Using a regex that captures only decimal numbers.
        let sections = parse_template(content).unwrap();

        // Expect one section, whose injection code was built from the move instruction.
        // The move instruction gets normalized to append the cycle count extracted from it.
        // For lines with an inline comment, the output uses " [cycles]" appended.
        // The NOP line (dcb.w) assigns nop_cycles = 2 * 4 = 8.
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].nop_cycles, 8);
        assert_eq!(sections[0].injection_code.len(), 1);
        assert_eq!(
            sections[0].injection_code[0].0,
            "move.w #$1323,D0 ; Move Instruction	;	(8)	move.w #xxx,dn"
        );
        assert_eq!(sections[0].label, "Move Instruction");
    }

    #[test]
    fn test_parse_template_multiple_sections() {
        let content = r#"
            move.w #$5678,D1
            dcb.w 4,$4e71
            move.w #$9,D2 ; Label for section
            dcb.w 6,$4e71
        "#;
        let sections = parse_template(content).unwrap();

        // Expect two sections.
        //
        // Section 1 is created from the first move instruction.
        // Since it has no inline comment the label is auto-generated ("Section 1")
        // and its normalized output appends "\t; [cycles]".
        // The NOP line assigns nop_cycles = 4 * 4 = 16.
        assert_eq!(sections.len(), 2);

        // Section 1
        assert_eq!(sections[0].nop_cycles, 16);
        assert_eq!(sections[0].injection_code.len(), 1);
        assert_eq!(
            sections[0].injection_code[0].0,
            "move.w #$5678,D1	;	(8)	move.w #xxx,dn"
        );
        assert_eq!(sections[0].label, "Section 1");

        // Section 2 is built from the second move instruction.
        // It has an inline comment, so the normalized output uses " [cycles]" (without a tab)
        // and the label is taken from the comment.
        // The subsequent NOP line assigns nop_cycles = 6 * 4 = 24.
        assert_eq!(sections[1].nop_cycles, 24);
        assert_eq!(sections[1].injection_code.len(), 1);
        assert_eq!(
            sections[1].injection_code[0].0,
            "move.w #$9,D2 ; Label for section	;	(8)	move.w #xxx,dn	[8]"
        );
        assert_eq!(sections[1].label, "Label for section");
    }

    #[test]
    fn test_parse_template_ignore_empty_lines() {
        let content = r#"

            move.w #$123,D3 ; Some work

            dcb.w 1,$4e71
        "#;
        let sections = parse_template(content).unwrap();

        // There should be one section with one instruction and nop_cycles = 1 * 4 = 4.
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].nop_cycles, 4);
        assert_eq!(sections[0].injection_code.len(), 1);
        assert_eq!(
            sections[0].injection_code[0].0,
            "move.w #$123,D3 ; Some work	;	(8)	move.w #xxx,dn"
        );
        assert_eq!(sections[0].label, "Some work");
    }

    #[test]
    fn test_parse_template_with_inline_comments() {
        let content = r#"
            move.w #$100,D4 ; Inline comment
            dcb.w 7,$4e71 ; Another comment
        "#;
        let sections = parse_template(content).unwrap();

        // Expect one section with the inline comment determining the label.
        // NOP cycles should equal 7 * 4 = 28.
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].nop_cycles, 28);
        assert_eq!(sections[0].injection_code.len(), 1);
        assert_eq!(
            sections[0].injection_code[0].0,
            "move.w #$100,D4 ; Inline comment	;	(8)	move.w #xxx,dn"
        );
        assert_eq!(sections[0].label, "Inline comment");
    }

    #[test]
    fn test_parse_template_no_valid_sections() {
        let content = r#"
            ; This is a comment line
            ; Another comment line
        "#;
        let sections = parse_template(content).unwrap();

        // Only comment lines are provided. As they are filtered out,
        // no sections should be created.
        assert_eq!(sections.len(), 0);
    }
}
