# 65816 Test ROMs

This directory contains assembly test programs for validating the 65816 CPU emulator.

## addressing_modes_test.asm

Comprehensive test program that validates all 65816 addressing modes including:

### Addressing Modes Tested
- **Immediate**: `#$12`, `#$1234`
- **Direct Page**: `$12`
- **Direct Page Indexed**: `$12,X` and `$12,Y`
- **Absolute**: `$1234`
- **Absolute Indexed**: `$1234,X` and `$1234,Y`
- **Absolute Long**: `$123456`
- **Direct Page Indirect**: `($12)`
- **Direct Page Indexed Indirect**: `($12,X)`
- **Direct Page Indirect Indexed**: `($12),Y`
- **Absolute Indexed Indirect**: `($1234,X)`
- **Stack Relative**: `$12,S`
- **Stack Relative Indirect Indexed**: `($12,S),Y`

### Special Test Cases
- Different Direct Page values (`$0000`, `$2000`, `$FF00`)
- Page crossing conditions
- Bank wrapping behavior

### Test Results Format
Results are stored at `$7E0100-$7E01FF` in memory with the following structure per test:
```
Offset 0: test_id    (1 byte) - Test identifier
Offset 1: expected   (1 byte) - Expected value
Offset 2: actual     (1 byte) - Actual value read/written
Offset 3: pass_flag  (1 byte) - 1 if passed, 0 if failed
```

Each test occupies 4 bytes, allowing for up to 64 tests.

## Assembling the Test ROM

To assemble the test ROM, you need a 65816 assembler. Here are instructions for common assemblers:

### Using WLA-DX
```bash
# Install WLA-DX (if not already installed)
# On Ubuntu/Debian:
sudo apt-get install wla-dx

# Assemble the ROM
wla-65816 -o addressing_modes_test.bin addressing_modes_test.asm
```

### Using ca65 (from cc65 suite)
```bash
# Install cc65 (if not already installed)
# On Ubuntu/Debian:
sudo apt-get install cc65

# Assemble and link
ca65 --cpu 65816 addressing_modes_test.asm -o addressing_modes_test.o
ld65 -t none addressing_modes_test.o -o addressing_modes_test.bin
```

### Using bass
```bash
# Install bass assembler
# Download from: https://github.com/ARM9/bass

# Assemble
bass addressing_modes_test.asm -o addressing_modes_test.bin
```

## Running the Tests

After assembling the ROM, run the test harness:

```bash
# Run all addressing mode tests (requires assembled ROM)
cargo test --test addressing_modes_test --target x86_64-unknown-linux-gnu -- --ignored

# Run individual unit tests (don't require assembled ROM)
cargo test --test addressing_modes_test --target x86_64-unknown-linux-gnu test_immediate_addressing
cargo test --test addressing_modes_test --target x86_64-unknown-linux-gnu test_direct_page_addressing
cargo test --test addressing_modes_test --target x86_64-unknown-linux-gnu test_absolute_addressing
cargo test --test addressing_modes_test --target x86_64-unknown-linux-gnu test_indexed_addressing
```

## Test Details

### Test IDs and Descriptions

| ID | Test Description | Address Calculation |
|----|------------------|---------------------|
| 1  | Immediate 8-bit | Value in instruction stream |
| 2  | Immediate 16-bit (high) | Value in instruction stream |
| 3  | Immediate 16-bit (low) | Value in instruction stream |
| 4  | Direct Page read | DP + $12 |
| 5  | Direct Page write | DP + $12 |
| 6  | DP Indexed X | DP + $12 + X |
| 7  | DP Indexed Y | DP + $12 + Y |
| 8  | Absolute read | DBR:$1234 |
| 9  | Absolute write | DBR:$1234 |
| 10 | Absolute Indexed X | DBR:($1234 + X) |
| 11 | Absolute Indexed Y | DBR:($1234 + Y) |
| 12 | Absolute Long read | $7E2000 |
| 13 | Absolute Long write | $7E2000 |
| 14 | DP Indirect read | DBR:[DP+$30] |
| 15 | DP Indirect write | DBR:[DP+$30] |
| 16 | DP Indexed Indirect read | DBR:[DP+$30+X] |
| 17 | DP Indexed Indirect write | DBR:[DP+$30+X] |
| 18 | DP Indirect Indexed read | DBR:([DP+$50]+Y) |
| 19 | DP Indirect Indexed write | DBR:([DP+$50]+Y) |
| 20 | Absolute Indexed Indirect | [PBR:($1250+X)] |
| 21 | Stack Relative read | SP + $01 |
| 22 | Stack Relative write | SP + $01 |
| 23 | Stack Relative Indirect Indexed read | DBR:([SP+$01]+Y) |
| 24 | Stack Relative Indirect Indexed write | DBR:([SP+$01]+Y) |
| 25 | DP=$2000 test | $2000 + $12 |
| 26 | DP=$FF00 test | $FF00 + $12 |
| 27 | Page crossing (before) | $12FF |
| 28 | Page crossing (after) | $1300 |
| 29 | Bank wrapping | $7EFFFF |
| 30 | Bank wrapping | $7E0000 |

## Notes

- The main comprehensive test (`test_all_addressing_modes`) is marked with `#[ignore]` by default since it requires the assembled ROM binary
- Unit tests can run without the assembled ROM and test individual addressing modes with minimal test cases
- All address calculations are documented in the assembly source with detailed comments
- The test program uses an infinite loop (`JMP END`) to signal completion
