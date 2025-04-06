// src/main.rs
mod cycle_spitter;
/// Main program for the "cycleSpitter" generation tool.
///
/// This program reads a source 68000 assembly file, processes its content to split it
/// into scanlines of a defined cycle length, and outputs the generated result
/// with annotations and structure based on a provided template file.
///
/// Key functionalities:
/// 1. **Input Parsing and Validation**:
///    - Reads the input assembly file, output scanlines label, and template file.
///    - Defaults to using "sample.s", "SCANLINES_CONSUMED", and "template.s" if
///      no input arguments are provided.
/// 2. **Template Parsing**:
///    - Processes the predefined template file to organize the layout of injected
///      code for each scanline, handling sections and nop cycles.
/// 3. **Assembly File Processing**:
///    - Reads the input assembly file line by line, trims it, and preprocesses it
///      into a flat structure for easier processing (via `process_block`).
///    - Breaks the input into chunks that fit into scanlines of `SCANLINE_CYCLES`
///      cycles, inserting padding and annotations when necessary.
/// 4. **Output Generation**:
///    - Assembles the final output lines based on the parsed template and scanline
///      processing.
///    - Adds metadata, including the total number of scanlines, template used, and
///      the specific padding details for scanline alignment.
///    - Outputs the processed and annotated result in an assembly-compatible format.
///
/// ### Constants:
/// - `SCANLINE_CYCLES`: Defines the number of cycles a single scanline should consist of (512 by default).
///
/// ### Command-line Arguments:
/// - `[1]`: Path to the input source file (default: "sample.s").
/// - `[2]`: The label for the total scanlines summary (default: "SCANLINES_CONSUMED").
/// - `[3]`: Path to the template file for code layout (default: "template.s").
///
/// ### Example Usage:
/// ```sh
/// $ ./cycle_spitter input.s SCANLINES_OUTPUT template.s
/// ```
///
/// ### Output Notes:
/// - Outputs generated assembly with annotations for padding and cycle calculations.
/// - Total cycles per scanline and issues like overflow are highlighted for debugging.
///
/// ### Error Handling:
/// - Gracefully handles file I/O errors and invalid or missing templates.
/// - Warns if a scanline exceeds the defined cycle limit.
///
/// The program is modularized into different components:
/// - `accumulate_chunk`: Handles grouping of assembly lines into cycle-aligned chunks.
/// - `process_block`: Flattens and preprocesses the input structure.
/// - `parse_template`: Reads and interprets the injection template sections.
///
/// Author: slippy / vectronix (c) 2025
use std::env;
use std::fs;

use crate::cycle_spitter::accumulator::accumulate_chunk;
use crate::cycle_spitter::block::process_block;
use crate::cycle_spitter::regexes::REG_LABEL_RE;
use crate::cycle_spitter::template::parse_template;

const SCANLINE_CYCLES: usize = 512;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments.
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 { &args[1] } else { "sample.s" };
    let scanlines_label = if args.len() > 2 { &args[2] } else { "SCANLINES_CONSUMED" };
    let template_file = if args.len() > 3 { &args[3] } else { "template.s" };

    // Parse the template.
    let template_content = fs::read_to_string(template_file)?;
    let template_sections = parse_template(&template_content)?;

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

        for section in &template_sections {
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

            final_output.push(format!("; --- {} section ---", section.label));

            if section.nop_cycles > 0 && current_index < flat_lines.len() {
                let (chunk, new_idx, new_offset) = accumulate_chunk(
                    &flat_lines,
                    current_index,
                    section.nop_cycles,
                    scanline_offset,
                );
                scanline_offset = new_offset;
                scanline_cycles += section.nop_cycles;
                current_index = new_idx;
                final_output.extend(chunk);
            }
            final_output.push(format!("; Calculated cycles: {}", scanline_offset));
        }

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
        } else if let Some(caps) = REG_LABEL_RE.captures(&line) {
            println!("{}\t{}", &caps[1], caps[2].to_string().clone().trim());
        } else {
            println!("\t{}", line);
        }
    }

    Ok(())
}
