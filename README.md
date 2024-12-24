# smc-rw-linux
SMC reader/writer for Linux written in Rust

AKA: A Rust rewrite of SmcDumpKey.c found at: https://github.com/floe/smc_util/blob/master/SmcDumpKey.c

## Prerequisites

 - Rust / Cargo
   - Tested: rustc 1.83.0 (90b35a623 2024-11-26) / stable-x86_64-unknown-linux-gnu

## Installation

```
./install.sh
```

Note: Installation needs `sudo` due to I/O port access.

## Usage

```
$ smc_rw -h
Usage: smc_rw <CODE> [VAL]

Arguments:
  <CODE>  
  [VAL]   [default: -1]

Options:
  -h, --help     Print help
  -V, --version  Print version
$
$ smc_rw BCLM 100
data write failed: send_byte(0x64, 0x300) fail: 0x40
$ smc_rw BCLM
100
$ smc_rw BCLM 40
$ smc_rw BCLM
40
```

## Notes

`data write failed: send_byte(0x64, 0x300) fail: 0x40` could occur time to time when writing values to SMC but it seems data are written so the error might be false alert.
