// src/cycle_spitter/format_helpers.rs

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
