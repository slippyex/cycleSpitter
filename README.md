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
    ./cycleSpitter --input input_file.s --label SCANLINES_LABEL --template template.s --cycles 512 > output_file.s    
   ``` 
## Input Format

Your assembly file should include cycle counts in parentheses in comments:
   ```asm
                lea     charBuffer,a0
                lea     buffer8,a1
                addq.w  #1,delayCounter
.loop:          tst.w d0
                movem.l d0-d7/a1-a3,-(sp)
;---------------------------------------------------------
; SCROLLOOP: Loop that performs the scrolling effect on the bitmap.
;---------------------------------------------------------
                rept 7
                    lsl.w   (a0)+
                    addq.l  #2,a0
add                 set 224
                    rept    28
                        roxl.w  add(a1)
add set add-8
                    endr
                    roxl.w  (a1)
                    lea     SCREEN_WIDTH(a1),a1
                endr
   ``` 

## Template File

The default template (template.s) contains:
   ```asm
; =============================================================
; new scanline
; -------------------------------------------------------------
; left border
		move.b	d7,$ffff8260.w			; 
		move.w	d7,$ffff8260.w			; 
		dcb.w	88,$4e71
; -------------------------------------------------------------
; right border
		move.w	d7,$ffff820a.w			; 
		move.b	d7,$ffff820a.w			; 
		dcb.w	11,$4e71
; -------------------------------------------------------------
; stabilizer
		move.b	d7,$ffff8260.w			; 
		move.w	d7,$ffff8260.w			; 
		dcb.w	11,$4e71
; =============================================================
   ``` 

## Output Example
   ```asm
; ------------------------------------------
; This file is generated using
; cycleSpitter (c) 2025 - slippy / vectronix
; Total scanlines created: 22
; Template used: ./examples/template.s
; ------------------------------------------
SCANLINES_LABEL equ 22
        move.b  d7,$ffff8260.w  ;       (12)    move.b dn,xxx.w [0]
        move.w  d7,$ffff8260.w  ;       (12)    move.w dn,xxx.w [12]
; --- Section 1 section ---
        lea     charBuffer,a0   ;       (12)    lea.l xxx.l,an  [24]
        lea     buffer8,a1      ;       (12)    lea.l xxx.l,an  [36]
        addq.w  #1,delayCounter ;       (20)    addq.w #xxx,xxx.l       [48]
.loop:  tst.w d0        ;       (4)     tst.w dn        [68]
        movem.l d0-d7/a1-a3,-(sp)       ;       (96 -> [base (8) + (reg count (11) * reg (8))]) movem.l reglist,-(an)   [72]
;---------------------------------------------------------
; SCROLLOOP: Loop that performs the scrolling effect on the bitmap.
;---------------------------------------------------------
        lsl.w   (a0)+   ;       (12)    lsl.w (an)+     [168]
        addq.l  #2,a0   ;       (8)     addq.l #xxx,an  [180]
...
...
...
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
