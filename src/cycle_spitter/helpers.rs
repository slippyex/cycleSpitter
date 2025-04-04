// src/cycle_spitter/helpers.rs

use crate::cycle_spitter::cycles::{lookup_cycles, CycleCount};
use crate::cycle_spitter::regexes::REG_NUMBER_RE;

/// Extracts the cycle count from a line of code. It first attempts to match a numeric value
/// using REG_NUMBER_RE. If that fails, it applies the provided `should_skip` predicate. If the
/// predicate returns true, the function returns `None` (indicating that the line should be skipped).
/// Otherwise, it calls `lookup_cycles` on the line.
///
/// # Arguments
/// - `line`: The line to extract cycle information from.
/// - `should_skip`: A predicate function that returns `true` if the line should be skipped.
///
/// # Returns
/// An `Option<CycleCount>` if a cycle count was extracted, or `None` if the line meets a skip condition.
pub fn extract_cycle_count<F>(line: &str, should_skip: F) -> Option<CycleCount>
where
    F: Fn(&str) -> bool,
{
    if let Some(cap) = REG_NUMBER_RE.captures(line) {
        Some(CycleCount {
            cycles: cap
                .get(1)
                .map(|m| m.as_str().parse::<usize>().unwrap_or(0))
                .unwrap_or(0),
            lookup: String::from("n/a"),
        })
    } else if should_skip(line) {
        None
    } else {
        Some(lookup_cycles(line))
    }
}

/// Formats an instruction line for the template module.
/// If the line already contains a semicolon, it uses one style; otherwise, it inserts a tab and semicolon.
pub fn format_template_instruction(line: &str, lookup: &str, cycles: usize) -> String {
    if line.contains(";") {
        format!("{} {} [{}]", line, lookup, cycles)
    } else {
        format!("{}\t; {} [{}]", line, lookup, cycles)
    }
}

/// Formats an instruction line for the accumulator module, including a given offset.
pub fn format_accumulated_instruction(line: &str, lookup: &str, cycles: usize, offset: usize) -> String {
    format!("{}\t;\t({})\t{}\t[{}]", line, cycles, lookup, offset)
}
