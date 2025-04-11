// src/cycle_spitter/helpers.rs

use crate::cycle_spitter::cycles::{lookup_cycles};
use crate::cycle_spitter::models::CycleCount;
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
        Some(CycleCount::new(
            vec![cap.get(1).map(|m| m.as_str().parse::<usize>().unwrap_or(0)).unwrap_or(0)],
            String::from("n/a"),
            0,
        ))
    } else if should_skip(line) {
        None
    } else {
        Some(lookup_cycles(line))
    }
}

/// Formats an instruction line for the accumulator module, including a given offset.
pub fn format_accumulated_instruction(line: &str, lookup: &str, cycles: &[usize], reg_count: &usize, offset: usize) -> String {
    // Pre-calculate capacity to avoid reallocations
    let cycles_str = if cycles.len() > 1 && !lookup.contains("reglist") {
        format!("{}/{}", cycles[0], cycles[1]) // Format as "not-taken/taken" for branches
    } else if cycles.len() > 1 && lookup.contains("reglist") {
        format!("{} -> [base ({}) + (reg count ({}) * reg ({}))]", cycles[0] + (reg_count * cycles[1]), cycles[0], reg_count, cycles[1])
    } else {
        cycles[0].to_string()
    };

    let mut result = String::with_capacity(
        line.len() + lookup.len() + cycles_str.len() + offset.to_string().len() + 10
    );

    result.push_str(line);
    result.push_str("\t;\t(");
    result.push_str(&cycles_str);
    result.push_str(")\t");
    result.push_str(lookup);
    if offset > 0 {
        result.push_str("\t[");
        result.push_str(&offset.to_string());
        result.push(']');
    }
    result
}
