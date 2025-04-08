# cycleSpitter

A cycle-accurate scanline splitter tool for Atari ST fullscreen (sync) programming.

## Description

`cycleSpitter` is a utility designed to help Atari ST demoscene programmers achieve 
perfect cycle-accurate timing for fullscreen effects. It analyzes 68000 assembly code 
with cycle annotations and automatically:

1. Expands `REPT`/`ENDR` blocks
2. Determines cycle usage per instruction and allows manual override 
3. Splits code into scanline-sized chunks (512 cycles each)
4. Injects border removal and stabilizer code
5. Pads with NOPs when necessary
6. Generates cycle-accurate annotations
7. Handles dynamic cycle usages (i.e. movem instructions with multiple regs)

## Features

- Processes assembly files with cycle annotations in comments (e.g., `move.l (a0)+,(a1) ; (20)`)
- Handles `REPT`/`ENDR` block expansion
- Automatically injects border removal code:
    - Left border removal
    - Right border removal
    - Stabilizer code
- Pads scanlines to exactly 512 cycles with NOPs
- Generates detailed cycle annotations
- Supports custom templates for injection code

## Installation

1. Ensure you have Rust installed (https://rustup.rs)
2. Clone this repository or download the source
3. Build with:
   ```sh
   cargo build --release
    ``` 
## Usage

Basic usage:
   ```sh
    ./cycleSpitter [input_file.s] [SCANLINES_LABEL] > output_file.s    
   ``` 
## Input Format

Your assembly file should include cycle counts in parentheses in comments:
   ```asm
    lea     _3dpnt0,a3                  ; (12)
    lea     cubeScreenOffsets,a4        ; (12)
    
    ; preserve the initial screen offset in a5
    movea.l screen_adr_fs,a5            ; (20)
    lea 230*140(a5),a5                  ; (8)
   ``` 

## Template File

The default template (template.s) contains:
   ```asm
; =============================================================
; new scanline
; -------------------------------------------------------------
; left border
		move.b	d7,$ffff8260.w			; (12)
		move.w	d7,$ffff8260.w			; (12)
		dcb.w	88,$4e71
; -------------------------------------------------------------
; right border
		move.w	d7,$ffff820a.w			; (12)
		move.b	d7,$ffff820a.w			; (12)
		dcb.w	11,$4e71
; -------------------------------------------------------------
; stabilizer
		move.b	d7,$ffff8260.w			; (12)
		move.w	d7,$ffff8260.w			; (12)
		dcb.w	11,$4e71
; =============================================================
   ``` 

## Output Example
   ```asm
; ------------------------------------------
; This file is generated using
; cycleSpitter (c) 2025 - slippy / vectronix
; Total scanlines created: 45
; Template used: template.s
; ------------------------------------------
SCANLINES_CONSUMED equ 45

    ; --- Left border section ---
    move.b d7,$ffff8260.w ;3 Left border [0]
    move.w d7,$ffff8260.w ;3             [12]
    ; Calculated cycles: 24
    
    lea     _3dpnt0,a3                  ; (12) [24]
    lea     cubeScreenOffsets,a4        ; (12) [36]
    nop     ; 4 cycles     [48]
    ; Calculated cycles: 48
   ``` 

## Requirements

    Rust 1.70 or newer

    For development: regex crate (included in Cargo.toml)

## License

Copyright (c) 2025 slippy / vectronix

This tool is provided as-is for the Atari ST demoscene community. Use freely in your productions.
Contributing

Pull requests and bug reports are welcome! Please include test cases for any changes.

## Acknowledgements

    The Atari ST demoscene community

    All fullscreen pioneers who figured out these timings
