# elf2uf2

Tool to convert ELF binaries to UF2 format for microcontroller flashing.

`elf2uf2` is a command line interface (CLI) tool that parses ELF files and generates UF2 firmware images, suitable for flashing onto microcontrollers such as the RP2040/2350.

## Quick Start

Generate `.uf2` firmware image from `.elf` binary:

### Linux
```bash
./elf2uf2 firmware.elf -o firmware.uf2
```

### Windows
```bash
elf2uf2.exe firmware.elf -o firmware.uf2
```

Then copy the generated `.uf2` file to your board's USB mass storage device. 

## Features

- Supports ELF32 binaries
- Parses and validates ELF headers and program headers
- Converts `LOAD` segments with non-zero file size into UF2 blocks
- Uses physical address (`paddr`) as the target address for UF2 blocks
- Structured logging using `tracing`

## Binaries

Prebuilt binaries are available on the GitHub Releases page: https://github.com/FranciscoSrocha/elf2uf2/releases

### Linux

- **Portable (static)** - `x86_64-unknown-linux-musl`
    - Full static binary, works on most distributions, no dependencies
    - Recommended for general use 
- **Native (dynamic)** - `x86_64-unknown-linux-gnu`
    - Smaller binary, links against system libraries (glibc required)

### Windows

- **Static CRT** - `x86_64-pc-windows-msvc`
    - No Visual C++ Redistributable required (fully self-contained)
    - Recommended for general use 
- **Dynamic CRT** - `x86_64-pc-windows-msvc`
    - Smaller binary, requires MSVC runtime installed
    - Compiled with MSVC toolset 19.44 (Visual Studio 2022)

## Build from source
```bash
git clone https://github.com/FranciscoSrocha/elf2uf2.git
cd elf2uf2
cargo build --release
```

Binary will be available at:
```bash
target/release/elf2uf2
```

## Usage
```bash
elf2uf2 <input.elf> [OPTIONS]
```

### Options

| Option                  | Description                                                                |
| ----------------------- | -------------------------------------------------------------------------- |
| `-o, --output <FILE>`   | Output UF2 file (default: `a.uf2`)                                         |
| `--payload-size <SIZE>` | Payload size per UF2 block (default: 256, max: 476, must be multiple of 4) |
| `--family-id <ID>`      | Optional MCU family ID (supports hex, e.g. `0xE48BFF56`)                   |
| `-v, --verbose`         | Increase logging verbosity (`-v` = info, `-vv` = debug, `-vvv` = trace)    |

## How it works

1. **ELF Parsing**

    - Reads ELF ident and header
    - Validates class (ELF32) and file type (EXEC)
    - Iterates program headers

2. **Segment Processing**

    - Only `LOAD` segments are processed
    - Segments with `file_size == 0` are skipped
    - Target address is derived from `paddr`
    - Data is split into fixed-sized UF2 blocks (default: 256 byte)

3. **Memory Mapping**

    - Segment data is mapped into aligned UF2 blocks
    - Blocks are aligned to `payload_size`
    - In case of overlapping segments the last one overwrites the previous one

4. **UF2 Generation**

    - Each block is written with:
        - proper UF2 headers
        - payload data
        - padding
    - Output is written atomically via a temporary file:
        - The tool writes to a temporary file first: ``` uf2-tool.<name>.<pid>.tmp ```
        - These are automatically cleaned up on startup.

## Error Handling

The tool provides strict validation and will fail if:
- ELF file is not `ELF32`
- File type is not `EXEC`
- Addresses do not fit into 32-bit UF2 space
- Invalid segment layouts are detected

## Limitations

- Only supports **ELF32**
- Only processes **`LOAD` segments**
- Assumes target addresses fit into **u32 (UF2 address space)**

## License

MIT License

## Contributing

Contributions are welcome!

If you find bugs or want features:

- open an issue
- or submit a PR

