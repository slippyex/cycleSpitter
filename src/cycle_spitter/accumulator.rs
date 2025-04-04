// src/cycle_spitter/accumulator.rs

use crate::cycle_spitter::cycles::{lookup_cycles, CycleCount};
use crate::cycle_spitter::regexes::REG_NUMBER_RE;

/// Parses and processes lines of assembly-like code to accumulate a target number of execution cycles,
/// annotating the lines with cycle information, and adding padding (NOP instructions) if necessary
/// to achieve the desired target.
///
/// # Arguments
///
/// - `lines`: A slice of strings, each representing a line of code or annotation.
///   These lines may contain assembly-style instructions or comments.
/// - `start_index`: The starting index in the `lines` array to begin processing.
/// - `target`: The target number of cycles to accumulate before stopping or padding.
/// - `initial_offset`: The initial cycle count to start from, used for tracking execution states across blocks.
/// - `number_re`: A compiled `Regex` to extract the cycle count from a line of code.
///
/// # Returns
///
/// A tuple containing:
/// - `chunk`: A `Vec<String>` holding the processed lines, annotated with cycle information
///   and padded with NOP instructions as needed.
/// - `i`: The index in the `lines` slice where processing stopped.
/// - `local_sum`: The total number of cycles accumulated after processing the chunk.
///
/// # Processing Details
///
/// - Lines that are empty or start with a semicolon (`;`, typically used as a comment in assembly code) are added
///   to `chunk` unchanged, but they do not contribute to the cycle count.
/// - For lines with extractable cycle information (as determined by `number_re` capturing group),
///   the cycles are parsed and accumulated. If adding a line's cycle count would exceed the `target`,
///   padding with NOP (`no operation`) instructions is added to reach the `target`, and processing stops.
/// - Lines where parsing fails or no cycle count is found are skipped.
/// - If the accumulated cycles at the end of processing are less than `target`, the remaining cycles are padded
///   with additional NOP instructions.
/// - Line annotations include the cycles consumed by the instruction and the current accumulated cycle count.
///
/// # Warnings
///
/// If the accumulated cycles after processing (`local_sum - initial_offset`) do not match the `target`,
/// a warning message is printed to the standard error output.
///
/// # Example
///
/// ```rust
/// use regex::Regex;
/// use crate::cycle_spitter::accumulator::accumulate_chunk;
///
/// let code_lines = vec![
///     "MOVE.W A1,A2".to_string(),
///     "; Comment line".to_string(),
///     "ADD D1,D3".to_string(),
/// ];
///
/// let regex = Regex::new(r"\b(\d+)\b").unwrap(); // Assume cycles are numbers in the line.
/// let (chunk, next_index, accumulated_cycles) = accumulate_chunk(&code_lines, 0, 10, 0, &regex);
///
/// println!("Processed chunk: {:?}", chunk);
/// println!("Next processing index: {}", next_index);
/// println!("Accumulated cycles: {}", accumulated_cycles);
/// ```
///
/// # Notes
///
/// - This function is designed to handle assembly-like instructions where each line may
///   consume a specific number of CPU cycles.
/// - The cycle values and their annotations (e.g., `; 4 cycles`) are appended to the lines
///   for debugging and traceability purposes.
/// - NOP instructions are assumed to consume 4 cycles each.
pub fn accumulate_chunk(
    lines: &[String],
    start_index: usize,
    target: usize,
    initial_offset: usize,
) -> (Vec<String>, usize, usize) {
    let mut local_sum = initial_offset;
    let mut chunk = Vec::new();
    let mut i = start_index;
    while i < lines.len() && (local_sum - initial_offset) < target {
        let line = &lines[i];
        if line.trim().is_empty() || line.trim().starts_with(";") {
            chunk.push(line.clone());
            i += 1;
            continue;
        }
        // First try to extract cycle count from comment parentheses.
        let cycles = if let Some(cap) = REG_NUMBER_RE.captures(line) {
            CycleCount {
                cycles: cap.get(1).map(|m| m.as_str().parse::<usize>().unwrap_or(0)).unwrap_or(0),
                lookup: String::from("n/a")
            }
        } else {
            if !line.trim().starts_with(";") && !line.contains(" set ") && !line.contains(" equ ") {
                // Otherwise, use the JSON lookup
                lookup_cycles(line)
            } else {
                i += 1;
                continue;
            }
        };
        if (local_sum - initial_offset) + cycles.cycles > target {
            let diff = target - (local_sum - initial_offset);
            let num_nop = diff / 4; // each NOP is 4 cycles
            for _ in 0..num_nop {
                let nop_line = format!("nop\t; 4 cycles\t[{}]", local_sum);
                chunk.push(nop_line);
                local_sum += 4;
            }
            break;
        }
        // If the line contains a cycle count, annotate it with the current offset.
        if cycles.cycles > 0 {
            let annotated = format!("{}\t;\t({})\t{}\t[{}]", line, cycles.cycles, cycles.lookup, local_sum);
            chunk.push(annotated);
            local_sum += cycles.cycles;
        } else {
            chunk.push(line.clone());
        }
        i += 1;
    }
    if (local_sum - initial_offset) < target {
        let diff = target - (local_sum - initial_offset);
        let num_nop = diff / 4;
        for _ in 0..num_nop {
            let nop_line = format!("nop\t; 4 cycles\t[{}]", local_sum);
            chunk.push(nop_line);
            local_sum += 4;
        }
    }
    if (local_sum - initial_offset) != target {
        eprintln!(
            "Warning: Accumulated cycles {} do not equal target {} starting at index {}.",
            local_sum - initial_offset, target, start_index
        );
    }
    (chunk, i, local_sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_accumulation() {
        let lines = vec![
            "MOVE.W A1,A2 ; (2) cycles".to_string(),
            "ADD #2,D3 ; (4) cycles".to_string(),
        ];
        let (chunk, next_index, accumulated) = accumulate_chunk(&lines, 0, 6, 0);

        assert_eq!(chunk.len(), 2);
        assert!(chunk[0].contains("; (2) cycles"));
        assert!(chunk[1].contains("; (4) cycles"));
        assert_eq!(next_index, 2);
        assert_eq!(accumulated, 6);
    }

    #[test]
    fn test_handle_comments_and_empty_lines() {
        let lines = vec![
            "; This is a comment".to_string(),
            "     ".to_string(),
            "ADD #2,D3 ; (4) cycles".to_string(),
        ];
        let (chunk, next_index, accumulated) = accumulate_chunk(&lines, 0, 4, 0);

        assert_eq!(chunk.len(), 3);
        assert_eq!(chunk[0], "; This is a comment");
        assert!(chunk[1].trim().is_empty());
        assert!(chunk[2].contains("; (4) cycles"));
        assert_eq!(next_index, 3);
        assert_eq!(accumulated, 4);
    }

    #[test]
    fn test_padding_with_nops() {
        let lines = vec![
            "MOVE.W A1,A2 ; (2) cycles".to_string(),
            "ADD #2,D3 ; (4) cycles".to_string(),
        ];
        let (chunk, next_index, accumulated) = accumulate_chunk(&lines, 0, 14, 0);

        assert!(chunk.iter().any(|line| line.contains("nop\t; 4 cycles")));
        assert_eq!(next_index, 2);
        assert_eq!(accumulated, 14);
    }

    #[test]
    fn test_overflow_handling() {
        let lines = vec![
            "MOVE.W A1,A2 ; (2) cycles".to_string(),
            "ADD #2,D3 ; (6) cycles".to_string(),
        ];
        let (chunk, next_index, accumulated) = accumulate_chunk(&lines, 0, 6, 0);

        assert!(chunk.iter().any(|line| line.contains("MOVE.W A1,A2")));
        assert!(!chunk.iter().any(|line| line.contains("ADD #2,D3")));
        assert_eq!(next_index, 1);
        assert_eq!(accumulated, 6);
    }

    #[test]
    fn test_mismatch_warning() {
        let lines = vec![
            "MOVE.W A1,A2 ; (2) cycles".to_string(),
        ];
        let (chunk, next_index, accumulated) = accumulate_chunk(&lines, 0, 10, 0);

        assert!(chunk.iter().any(|line| line.contains("nop\t; 4 cycles")));
        assert_eq!(next_index, 1);
        assert_eq!(accumulated, 10);
    }

}