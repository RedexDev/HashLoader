# HashLoader
<p align="center">
  <img src="app/HashLoader.png" alt="HashLoader" width="470" height="470">
</p>
A portable mod loader kit for games and applications running on **HashLink** (JIT mode), built as a single Rust executable.

HashLoader can install the loader into a game, package mods as `.hlmod` files, register them with Windows, and provide modding and reverse-engineering tools, including resource replacement mods.

## Documentation

Open or download the guides below:

- [Building and development](docs/BUILDING.md)
- [Installation and toolkit guide](docs/USER_GUIDE.md)
- [Mod creation and `.hlmod` format](docs/MODDING.md)
- [Runner and runtime reference](docs/RUNTIME.md)

⚠️ Architecture Compatibility (x86_64 Only)
HashLoader is designed exclusively for x86_64 systems (Windows / Linux) and is not compatible with ARM64 architectures (such as Apple Silicon M1/M2/M3, Raspberry Pi, or mobile devices).

Why doesn't it work on ARM64?
The primary reason lies in the internal design of the HashLink JIT (HL/JIT) engine:

Architecture-Specific Machine Code Generator (JIT): HashLink's official Just-In-Time compiler translates .hl bytecode directly into native x86 / x86_64 instructions at runtime. HashLink does not have an ARM64 JIT backend (on ARM systems, HashLink requires compiling the project to C code via HL/C instead of using the JIT virtual machine).

Bytecode Injection & Low-Level Memory Hooks: Because HashLoader operates by patching .hl bytecode at load time and interacting directly with the JIT runtime environment and native libraries (.hdll), its lifecycle depends on an active JIT-capable VM. Without native ARM64 JIT support in HashLink, there is no dynamic runtime environment into which .hlmod packages can be injected.

Extra Technical Details:
- Calling Conventions (ABI) & FFI: HashLink native dynamic libraries (.hdll) and dynamically generated JIT function pointers assume x86_64 calling conventions and register alignments (System V AMD64 ABI on Linux / Microsoft x64 on Windows).

- HL/JIT vs. HL/C: On ARM64 devices (such as consoles, modern macOS, or Android), Haxe/HashLink games are distributed as precompiled static binaries via HL/C, eliminating dynamic .hl file loading and making runtime bytecode injection impossible without recompiling the entire binary.

[License: MIT](LICENSE)
