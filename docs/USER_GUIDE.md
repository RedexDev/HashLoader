# Installation and Toolkit Guide

## Dual binary mode

| Executable state | Behavior |
| --- | --- |
| No `hashloader.toml` alongside executable | **Setup Toolkit (GUI)** with language picker, installer, packager, and reverse-engineering tools |
| `hashloader.toml` alongside executable | **Runner**: extracts pending mods and automatically launches the game |
| `HashToolkit.exe --info mod.hlmod` | Opens the package inspector with detailed mod information |
| `HashToolkit.exe --decompiler [file.hl]` | Opens the bytecode decompiler and inspector directly |

## Install HashLoader into a game

Use the file browser to select the target: the game root, its `bin/` directory, or the `.hl` / `hlboot.dat` file. Supported targets must use standard HashLink JIT mode (not static HL/C builds) and have:

- A bytecode file in `<root>/bin/` or `<root>/`: `main.hl`, `hlboot.dat`, or the largest detected `*.hl` file.
- For Lime/OpenFL/HaxeFlixel builds, the companion executable `<Game>.exe` alongside `hlboot.dat`.
- A HashLink JIT runtime: `hashlink.exe` / `hl.exe` and `libhl.dll` next to the bytecode file.

If the game does not include a runtime, the toolkit looks for one in this order:

1. `bin/` or the game root.
2. A `hlvm/` folder next to `HashToolkit.exe` (portable mode).
3. Runtime embedded in the binary from `app/hlvm/` (`hashlink.exe` and `libhl.dll`); the toolkit extracts it to `bin/` for a self-contained installation.

Installation creates this layout:

```text
<root>/HashLoader.exe        Toolkit copy configured as the game's launch executable
<root>/hashloader.toml       Loader configuration (bin, args, runtime, mods, payload)
<root>/hlmods/               Installed mods directory + README.txt
<bin>/.hashloader/           Internal cache: extracted mods + logs + loader.json + hashloader.hdll
```

Launch the game directly via `HashLoader.exe` after installation.

## Optional Windows `.hlmod` integration

The toolkit registers the extension for the current Windows user (HKCU; no administrator privileges required):

```text
Software\Classes\.hlmod                                     (default) = HashLoader.hlmodPackage
Software\Classes\HashLoader.hlmodPackage\DefaultIcon         = %LOCALAPPDATA%\HashLoader\hlmod.ico
Software\Classes\HashLoader.hlmodPackage\shell\open\command  = "<exe-path>" --info "%1"
```

File-association update notifications (`SHChangeNotify`) refresh desktop icons. The **Remove Association** button completely uninstalls the association.

The `.hlmod` package is also registered as an archive (`PerceivedType=compressed`, `ContentType=application/zip`) and appears in the context menu / **Open with** list:

- **HashLoader Mod Pack** (default double-click action: visual inspector for mod details and dependencies).
- **Compressed (zipped) Folder** (native Windows Explorer view of archive contents).
- **WinRAR / 7-Zip**, if installed (detected across `Program Files`, `Program Files (x86)`, and `App Paths`).

## Package a mod

Select the mod's development folder and the target `.hlmod` output path in the packager. The `hlmod.json` manifest is validated before the compressed package is generated.

## Configuration

`hashloader.toml` supports these sections:

```toml
[game]
bin   = "bin"                  # Game execution directory (default: bin)
main  = "bin/main.hl"          # Game bytecode file (*.hl or hlboot.dat)
args  = ["-window"]            # Command-line arguments forwarded to the game

[runtime]
hashlink = "bin/hashlink.exe"  # Path to HashLink JIT interpreter (optional)
libhl    = "bin/libhl.dll"     # Path to libhl.dll

[mods]
folder = "hlmods"              # Directory where users place .hlmod files

[run]
payload = ""                   # Runtime .hl payload file (empty = automatic)
```
