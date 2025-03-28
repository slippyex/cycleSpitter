/**
 * cycleSpitter (c) slippy / vectronix, 2025
 *
 * A Cycle Splitter tool for Atari ST fullscreen (sync) programming.
 *
 * Expects a 68000 assembly code snippet file where the cycles per instruction
 * are provided in parentheses in the comment.
 *
 * For example, given an input snippet like:
 *
 *                 lea     _3dpnt0,a3                  ;                       (12)
 *                 lea     cubeScreenOffsets,a4        ;                       (12)
 *
 *                 ; preserve the initial screen offset in a5
 *                 movea.l screen_adr_fs,a5            ;                       (20)
 *                 lea 230*140(a5),a5 ; (8)
 *
 *                 REPT 45
 *                     movea.l (a3),a2                 ;                       (12)
 *                     lea     _3dcube,a0              ;                       (12)
 *                     adda.l  (a2)+,a0                ;                       (16) -- is 14 but padded to 16
 *                     move.l  a5,a1       ; screen initial offset preserved   ( 4)
 *                     adda.w  (a4)+,a1                ;                       (12)
 *                     REPT 27
 *                         move.l  (a0)+,(a1)          ;                       (20)
 *                         move.l  (a0)+,8(a1)         ;                       (24)
 *                         lea     SCREEN_WIDTH(a1),a1 ;                       ( 8)
 *                     ENDR
 *                     ; exclude the last rept and save the lea
 *                     move.l  (a0)+,(a1)              ;                       (20)
 *                     move.l  (a0)+,8(a1)             ;                       (24)
 *                     move.l  a2,(a3)+                ;                       (12)
 *                 ENDR
 *
 * The output scanline will include the injection lines (each 12 cycles) for:
 *   - Left border:
 *       move.b	d7,$ffff8260.w	;3 Left border  [0]
 *       move.w	d7,$ffff8260.w	;3             [12]
 *   - Right border:
 *       move.w	d7,$ffff820a.w	;3 Right border [??]
 *       move.b	d7,$ffff820a.w	;3             [??]
 *   - Stabilizer:
 *       move.b	d7,$ffff8260.w	;3 Stabilizer   [??]
 *       move.w	d7,$ffff8260.w	;3             [??]
 *
 * (In each case the first injection line is annotated with the current running
 *  total, and then 12 cycles are added before processing the subsequent code.)
 *
 * Usage: ./cycleSpitter [filename.s] [SCANLINES_CONSUMED_LABEL] > [generated_filename.s]
 * If no filename is provided, it defaults to "sample.s".
 */

use regex::Regex;
use std::env;
use std::fs;
use std::error::Error;

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
        let mut cycles = 0;
        if let Some(cap) = number_re.captures(line) {
            // Capture group 1 is the digits.
            if let Some(m) = cap.get(1) {
                cycles = m.as_str().parse::<usize>().unwrap_or(0);
            }
        }
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
            let annotated = format!("{}\t[{}]", line, local_sum);
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

#[derive(Debug)]
struct TemplateSection {
    injection_code: Vec<(String, usize)>, // (code, cycles)
    nop_cycles: usize,
    label: String,
}

const SCANLINE_CYCLES: usize = 512;

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

        let cycles = number_re.captures(trimmed)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().parse::<usize>().unwrap_or(0))
            .unwrap_or(0);

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

fn main() -> Result<(), Box<dyn Error>> {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 { &args[1] } else { "sample.s" };
    let scanlines_label = if args.len() > 2 { &args[2] } else { "SCANLINES_CONSUMED" };
    let template_file = if args.len() > 3 { &args[3] } else { "template.s" };

    // Compile regexes
    let number_re = Regex::new(r"\(\s*(\d+)\s*\)")?;

    // Parse template file
    let template_content = fs::read_to_string(template_file)?;
    let template_sections = parse_template(&template_content, &number_re)?;

    // Read and process input file
    let content = fs::read_to_string(filename)?;
    let raw_lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).collect();
    let (flat_lines, _) = process_block(&raw_lines, 0);

    let mut final_output: Vec<String> = Vec::new();
    let mut current_index = 0;
    let mut line_count = 0;

    while current_index < flat_lines.len() {
        let mut scanline_offset = 0;
        let mut scanline_cycles = 0;

        // Process each template section
        for section in &template_sections {
            // Add injection code with proper cycle counting
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

            // Add section header
            final_output.push(format!("; --- {} section ---", section.label));

            // Process code for this section
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

        // Pad to exactly 512 cycles
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

    // Output the results
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
