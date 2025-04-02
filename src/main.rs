// src/main.rs
mod cycle_spitter;

use regex::Regex;
use std::env;
use std::fs;

use crate::cycle_spitter::accumulator::accumulate_chunk;
use crate::cycle_spitter::block::process_block;
use crate::cycle_spitter::template::parse_template;

const SCANLINE_CYCLES: usize = 512;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments.
    let args: Vec<String> = env::args().collect();
    let filename = if args.len() > 1 { &args[1] } else { "sample.s" };
    let scanlines_label = if args.len() > 2 { &args[2] } else { "SCANLINES_CONSUMED" };
    let template_file = if args.len() > 3 { &args[3] } else { "template.s" };

    // Compile regex used in multiple modules.
    let number_re = Regex::new(r"\(\s*(\d+)\s*\)")?;

    // Parse the template.
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
                    &number_re,
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
        } else {
            println!("\t{}", line);
        }
    }

    Ok(())
}
