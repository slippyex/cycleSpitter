// src/cycle_spitter/block.rs

/// Processes a block of strings to handle nested REPT (repeat) and ENDR (end repeat) directives.
///
/// This function recursively processes a list of assembly-like textual instructions and expands
/// nested repeating blocks defined by "REPT <count>" ... "ENDR" directives. A REPT block is repeated
/// `count` times, and nested REPT blocks are supported through recursion.
///
/// # Parameters
/// - `lines`: A slice of strings (`&[String]`) representing the input lines to process.
/// - `start_index`: The starting index within the `lines` slice from where processing starts.
///
/// # Returns
/// A tuple containing:
/// - `Vec<String>`: The processed lines with expanded REPT blocks.
/// - `usize`: The index indicating where processing has stopped. This is useful for skipping to the
///   correct position in the parent recursion or in the remaining lines.
///
/// # Behavior
/// - Lines starting with "REPT <count>":
///   - If `<count>` is a valid integer, the function recursively processes the subsequent lines
///     until the corresponding "ENDR" directive.
///   - The resulting block is repeated `<count>` times, and all repeated lines are added to the result.
/// - Lines starting with "ENDR":
///   - Indicates the end of a REPT block and stops further processing for the current recursive call.
/// - Any other line:
///   - Added directly to the result as-is.
///
/// # Examples
///
/// ## Input:
/// Input lines:
/// ```text
/// ["line1", "rept 3", "line2", "endr", "line3"]
/// ```
///
/// ## Output:
/// Processed lines:
/// ```text
/// ["line1", "line2", "line2", "line2", "line3"]
/// ```
///
/// ## Nested Example:
/// Input lines:
/// ```text
/// ["line1", "rept 2", "line2", "rept 2", "line3", "endr", "endr", "line4"]
/// ```
///
/// Processed lines:
/// ```text
/// ["line1", "line2", "line3", "line3", "line2", "line3", "line3", "line4"]
/// ```
///
/// # Notes
/// - If the REPT directive does not have a valid repeat count, the line is added to the results unchanged.
/// - It is assumed that the "REPT" and corresponding "ENDR" directives are properly paired and nested.
///
/// # Panics
/// This function does not perform checks for malformed or mismatched "REPT"/"ENDR" directives,
/// and it is the caller's responsibility to ensure valid input.
pub fn process_block(lines: &[String], start_index: usize) -> (Vec<String>, usize) {
    let mut result = Vec::new();
    let mut index = start_index;
    while index < lines.len() {
        let line = &lines[index];
        let lower = line.to_lowercase();
        if lower.starts_with("rept") {
            let parts: Vec<&str> = lower.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(count) = parts[1].parse::<usize>() {
                    let (block, new_index) = process_block(lines, index + 1);
                    for _ in 0..count {
                        result.extend(block.iter().cloned());
                    }
                    index = new_index;
                    continue;
                } else {
                    result.push(line.clone());
                }
            }
        } else if lower.starts_with("endr") {
            return (result, index + 1);
        } else {
            result.push(line.clone());
        }
        index += 1;
    }
    (result, index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_block_with_single_rept() {
        let lines = vec![
            "line1".to_string(),
            "rept 3".to_string(),
            "line2".to_string(),
            "endr".to_string(),
            "line3".to_string(),
        ];
        let (result, _) = process_block(&lines, 0);

        let expected = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line2".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_rept_blocks() {
        let lines = vec![
            "line1".to_string(),
            "rept 2".to_string(),
            "line2".to_string(),
            "rept 2".to_string(),
            "line3".to_string(),
            "endr".to_string(),
            "endr".to_string(),
            "line4".to_string(),
        ];
        let (result, _) = process_block(&lines, 0);

        let expected = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
            "line3".to_string(),
            "line2".to_string(),
            "line3".to_string(),
            "line3".to_string(),
            "line4".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_empty_input() {
        let lines: Vec<String> = vec![];
        let (result, _) = process_block(&lines, 0);

        assert!(result.is_empty());
    }

    #[test]
    fn test_no_rept_blocks() {
        let lines = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];
        let (result, _) = process_block(&lines, 0);

        let expected = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_rept_with_invalid_count() {
        let lines = vec![
            "line1".to_string(),
            "rept abc".to_string(), // Invalid count
            "line2".to_string(),
            "endr".to_string(),
        ];
        let (result, _) = process_block(&lines, 0);

        let expected = vec![
            "line1".to_string(),
            "rept abc".to_string(),
            "line2".to_string(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_rept_no_endr() {
        // This test might expose undefined behavior since "REPT" blocks without matching "ENDR"
        // are not explicitly handled, and the function assumes valid input.

        let lines = vec![
            "line1".to_string(),
            "rept 2".to_string(),
            "line2".to_string(),
        ];
        let (result, _) = process_block(&lines, 0);

        // Expected behavior: unmatched "REPT" is processed as if the lines end there
        let expected = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line2".to_string(),
        ];

        assert_eq!(result, expected);
    }
}