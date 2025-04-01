/**
 * cycleSpitter (c) slippy / vectronix, 2025
 *
 * A Cycle Splitter tool for Atari ST fullscreen (sync) programming.
 *
 * This version now loads the instruction cycles JSON from an external file at compile time
 * and initializes it to a static HashMap (via once_cell::sync::Lazy) so that lookups
 * are optimized for speed.
 *
 * Usage: ./cycleSpitter [filename.s] [SCANLINES_CONSUMED_LABEL] > [generated_filename.s]
 * If no filename is provided, it defaults to "sample.s".
 */

use regex::Regex;
use std::env;
use std::fs;
use std::error::Error;
use std::collections::HashMap;
use serde_json;
use once_cell::sync::Lazy;

// Load the cycles JSON at compile time from a file named "cycles.json".
static CYCLES_MAP: Lazy<HashMap<String, Vec<usize>>> = Lazy::new(|| {
    let json_str = include_str!("cycles.json");
    serde_json::from_str(json_str).expect("Error parsing cycles JSON")
});

const SCANLINE_CYCLES: usize = 512;

#[derive(Debug)]
struct TemplateSection {
    injection_code: Vec<(String, usize)>, // (code, cycles)
    nop_cycles: usize,
    label: String,
}

/// Recursively expands REPT/ENDR blocks.
/// Returns a tuple: (expanded lines, new index).
fn process_block(lines: &[String], start_index: usize) -> (Vec<String>, usize) {
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

/// Looks up the cycle count for a given code line (after normalization).
/// Returns the first cycle count found in the JSON map (or 0 if not found).
fn lookup_cycles(line: &str) -> usize {
  //  let normalized = normalize_line(line);
    let normalized = parse_line(line);
    if let Some(cycles) = CYCLES_MAP.get(normalized.as_str()) {
        // If there are multiple cycle counts (e.g. taken/not-taken), choose the first.
        cycles[0]
    } else {
        eprintln!("Warning: No cycle count found for instruction: {}", line);
        0
    }
}

/// Accumulates lines from `lines[start_index...]` until the sum of cycle counts (relative to initial_offset)
/// reaches the target. For each line that contains a cycle count (extracted from a comment),
/// the current cumulative offset is appended (in square brackets).
/// If adding the next line would overrun the target, NOP lines (each 4 cycles) are inserted.
/// Returns a tuple: (annotated chunk, new index, final cumulative offset).
fn accumulate_chunk(
    lines: &[String],
    start_index: usize,
    target: usize,
    initial_offset: usize,
    number_re: &Regex,
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
        let cycles = if let Some(cap) = number_re.captures(line) {
            cap.get(1).map(|m| m.as_str().parse::<usize>().unwrap_or(0)).unwrap_or(0)
        } else {
            if !line.trim().starts_with(";") && !line.contains(" set ") && !line.contains(" equ ") {
                // Otherwise, use the JSON lookup
                lookup_cycles(line)
            } else {
                i += 1;
                continue;
            }
        };
        if (local_sum - initial_offset) + cycles > target {
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
        if cycles > 0 {
            let annotated = format!("{}\t;\t({})\t[{}]", line, cycles, local_sum);
            chunk.push(annotated);
            local_sum += cycles;
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

/// Parses the template file into sections. For each non-empty line that does not match a NOP pattern,
/// we try to extract its cycle count (either from a "(...)" comment or via lookup in the JSON).
fn parse_template(template_content: &str, number_re: &Regex) -> Result<Vec<TemplateSection>, Box<dyn Error>> {
    let mut sections = Vec::new();
    let nop_re = Regex::new(r"dcb\.w\s*(\d+),\s*\$4e71")?;
    let comment_re = Regex::new(r";\s*(.*)")?;

    let mut current_code = Vec::new();
    let mut current_label = String::new();

    for line in template_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(caps) = nop_re.captures(trimmed) {
            let count = caps.get(1).unwrap().as_str().parse::<usize>()?;
            let cycles = count * 4;

            if !current_code.is_empty() {
                sections.push(TemplateSection {
                    injection_code: current_code,
                    nop_cycles: cycles,
                    label: current_label,
                });
                current_code = Vec::new();
                current_label = String::new();
            }
            continue;
        }

        let cycles = if let Some(cap) = number_re.captures(trimmed) {
            cap.get(1).map(|m| m.as_str().parse::<usize>().unwrap_or(0)).unwrap_or(0)
        } else {
            if trimmed.trim().starts_with(";") && trimmed.contains(" set ") && trimmed.contains(" equ ") {
                lookup_cycles(trimmed)
            } else {
                continue;
            }
        };

        if current_label.is_empty() {
            current_label = comment_re.captures(trimmed)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| format!("Section {}", sections.len() + 1));
        }

        current_code.push((trimmed.to_string(), cycles));
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

static REG_DISPLACEMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"([^\s,()]+)\((a[0-7]|sp)\)").unwrap());
static REG_INSTRUCTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(lea|moveq)$").unwrap());
static REG_IMMEDIATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(#[^,\s]+)").unwrap());
static REG_DATA: Lazy<Regex> = Lazy::new(|| Regex::new(r"\bd[0-7]\b").unwrap());
static REG_ADDR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(a[0-7]|sp)\b").unwrap());
static REG_ABS_ADDRESS: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?P<before>^|[ \t,(\[])(?P<token>[a-zA-Z_][a-zA-Z0-9_]*)(?P<suffix>\.(?:l|w))?\b"
).unwrap());
static REG_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+").unwrap());

fn parse_line(line: &str) -> String {
    let trimmed = line.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let first_token = parts.next().unwrap();
    let operand_part = parts.next().unwrap_or("");

    let first_token = if REG_INSTRUCTION.is_match(first_token) {
        format!("{}.l", first_token)
    } else {
        first_token.to_string()
    };

    let mut operands = operand_part.to_string();
    // a. Replace displacement addressing.
    operands = REG_DISPLACEMENT.replace_all(&operands, "d($2)").into_owned();
    // b. Replace immediate values.
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
    let operands = operands.trim();

    if operands.is_empty() {
        first_token
    } else {
        format!("{} {}", first_token, operands)
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    // Get command-line arguments.
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 { &args[1] } else { "sample.s" };
    let scanlines_label = if args.len() > 2 { &args[2] } else { "SCANLINES_CONSUMED" };
    let template_file = if args.len() > 3 { &args[3] } else { "template.s" };

    // Compile regexes.
    let number_re = Regex::new(r"\(\s*(\d+)\s*\)")?;

    // Parse the template file.
    let template_content = fs::read_to_string(template_file)?;
    let template_sections = parse_template(&template_content, &number_re)?;

    // Read and process the input file.
    let content = fs::read_to_string(filename)?;
    let raw_lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).collect();
    let (flat_lines, _) = process_block(&raw_lines, 0);

    let mut final_output: Vec<String> = Vec::new();
    let mut current_index = 0;
    let mut line_count = 0;

    while current_index < flat_lines.len() {
        let mut scanline_offset = 0;
        let mut scanline_cycles = 0;

        // Process each template section.
        for section in &template_sections {
            // Add injection code with proper cycle counting.
            for (i, (code, cycles)) in section.injection_code.iter().enumerate() {
                let annotated = if i == 0 {
                    format!("{}\t[{}]", code, scanline_offset)
                } else {
                    code.clone()
                };
                final_output.push(annotated);
                scanline_offset += cycles;
                scanline_cycles += cycles;
            }

            // Add section header.
            final_output.push(format!("; --- {} section ---", section.label));

            // Process code for this section.
            if section.nop_cycles > 0 && current_index < flat_lines.len() {
                let (chunk, new_idx, new_offset) = accumulate_chunk(
                    &flat_lines,
                    current_index,
                    section.nop_cycles,
                    scanline_offset,
                    &number_re,
                );
                scanline_offset = new_offset;
                scanline_cycles += section.nop_cycles;
                current_index = new_idx;
                final_output.extend(chunk);
            }
            final_output.push(format!("; Calculated cycles: {}", scanline_offset));
        }

        // Pad to exactly 512 cycles.
        if scanline_cycles < SCANLINE_CYCLES {
            let remaining = SCANLINE_CYCLES - scanline_cycles;
            let nop_count = remaining / 4;
            if nop_count > 0 {
                final_output.push(format!("\tdcb.w\t{},$4e71\t; Pad to 512 cycles ({} cycles)", nop_count, remaining));
            }
            scanline_cycles = SCANLINE_CYCLES;
        } else if scanline_cycles > SCANLINE_CYCLES {
            eprintln!("Warning: Scanline overflow by {} cycles!", scanline_cycles - SCANLINE_CYCLES);
        }

        final_output.push(format!("; Total cycles for scanline: {}", scanline_cycles));
        line_count += 1;
    }

    // Output the results.
    println!("; ------------------------------------------");
    println!("; This file is generated using");
    println!("; cycleSpitter (c) 2025 - slippy / vectronix");
    println!("; Total scanlines created: {}", line_count);
    println!("; Template used: {}", template_file);
    println!("; ------------------------------------------");
    println!("{}\tequ {}", scanlines_label, line_count);
    for line in final_output {
        if line.trim().starts_with(";") || line.contains(" equ ") || line.contains(" set ") {
            println!("{}", line);
        } else {
            println!("\t{}", line);
        }
    }

    Ok(())
}
