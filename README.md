[README.md](https://github.com/user-attachments/files/34617845/README.md)
# HashLoader Setup Toolkit
![Uploading hashloaderICON.png…]()

A portable mod loader kit for games and applications running on **HashLink** (JIT mode).
Built as a single Rust executable with four core features:

1. **Install HashLoader into a game** (primary feature): injects the loader, generates `HashLoader.exe`, 
   `hashloader.toml`, and creates the `hlmods/` directory in the game root (supports `main.hl`, any `*.hl` file, or Lime/HaxeFlixel `hlboot.dat`).
2. **Register `.hlmod` file extension on Windows** (optional): assigns a custom icon, File Explorer integration, and package details viewer on double-click.
3. **Pack mod folder → `.hlmod`** (for mod developers).
4. **Modding & Reverse Engineering Tools**: graphical decompiler and visual `.hl` bytecode inspector, plus automated Haxe stub generator (`.hx`).

Even without a compiled code payload, the loader still works for resource replacement mods (`override/`).

---

## Building

```bash
cargo build --release
# -> target/release/HashToolkit.exe
```

- Requires the stable Rust toolchain with MSVC (Visual Studio's `link.exe`; see `.cargo/config.toml`).
- `build.rs` automatically converts `app/hashloaderICON.png` and `app/hlmodformatICON.png` into multi-resolution `.ico` 
  files (16, 24, 32, 48, 64, 128, and 256 px), embeds the executable icon, 
  and generates a 128 px `logo.png` for the GUI.
- If `app/HashLoader.hl` exists (compiled from `app/LoaderPayload.hx`), it is 
  embedded as the Modding Runtime payload (`cfg has_payload`) and the runner 
  launches with support for code mods.
- If `app/native/hashloader.hdll` exists, it is embedded as the native bridge 
  (`cfg has_bridge`). The runner automatically extracts it to `<bin>/.hashloader/` and appends that 
  folder to the `PATH` environment variable, removing any requirement for mods to compile or ship 
  their own native C code.

> [!NOTE]
> `cargo build --release` does **not** compile Haxe or C: it only embeds the artifacts 
> that already exist inside the `app/` directory. If you modify the payload or the native bridge, build 
> the new artifacts first.

### Rebuilding the payload and native bridge

```powershell
# 1. Haxe Payload: app/LoaderPayload.hx -> app/HashLoader.hl
haxe -cp app -main LoaderPayload -hl app/HashLoader.hl

# 2. Native bridge: app/native/hashloader.c -> app/native/hashloader.hdll
#    (Visual Studio "x64 Native Tools" prompt, or vcvarsall.bat x64)

cl /nologo /LD /MT /O2 /W3 /I "<hashlink-src>\src" ^
   app\native\hashloader.c /Fe:app\native\hashloader.hdll ^
   /link /NOLOGO /LIBPATH:"<libhl>" libhl.lib

# 3. Embed both artifacts into the binary
cargo build --release
# -> target/release/HashToolkit.exe
```

- `<hashlink-src>`: HashLink source tree (where `hl.h` is located).
- `<libhl>`: directory containing `libhl.lib` (HashLink Windows distribution). 
  If you only have `libhl.dll`, generate the import library using `dumpbin /exports` + 
  `lib /def`.
- The payload must be named `app/HashLoader.hl` and the bridge 
  `app/native/hashloader.hdll`: `build.rs` and the runner expect these exact paths.
- `build.rs` tracks artifacts via `rerun-if-changed`. If you do not rebuild the `.hl`/`.hdll` 
  files before running `cargo build`, the previous build artifacts will be embedded.
- To deploy the build into a game: 
  `Copy-Item target\release\HashToolkit.exe "<game-directory>\HashLoader.exe" -Force`.

### Updating the mod SDK

`app/sdk/hashloader/HashLoader.hx` is not bundled into `HashLoader.hl` or the 
`.hdll`: it is embedded as plain text inside the toolkit (`include_str!`) and deployed to 
`tools/sdk/hashloader/` and `tools/templates/src/hashloader/` when installing the 
tools. After modifying it:

1. Run `cargo build --release` to update the embedded SDK in the binary;
2. Reinstall tools/SDK from the toolkit GUI;
3. Recompile any mods that depend on it (`haxe build.hxml`).

---

## Dual Binary Mode

| Executable state | Behavior |
| --- | --- |
| Without `hashloader.toml` alongside executable | **Setup Toolkit (GUI)** with language picker, installer, packager, and RE tools |
| With `hashloader.toml` alongside executable | **Runner**: extracts pending mods and automatically launches the game |
| `HashToolkit.exe --info mod.hlmod` | Displays the package inspector with detailed mod information |
| `HashToolkit.exe --decompiler [file.hl]` | Directly launches the bytecode decompiler and inspector |

### 1. Installing HashLoader (injection)

A file browser opens to select the destination target. Supported targets include the **game root**, its 
`bin/` directory, or direct selection of the `.hl` / `hlboot.dat` file. Target requirements (standard JIT mode, not static HL/C builds):

- Game bytecode file located in `<root>/bin/` or `<root>/`: `main.hl`, `hlboot.dat`, or the largest detected `*.hl` file.
- For Lime/OpenFL/HaxeFlixel builds: the companion executable `<Game>.exe` alongside `hlboot.dat`.
- HashLink JIT runtime: `hashlink.exe` / `hl.exe` + `libhl.dll` next to the bytecode file.

If the destination does not bundle a runtime, the toolkit resolves it in cascade order:

1. Already present in `bin/` or the game root;
2. A `hlvm/` folder located next to `HashToolkit.exe` (portable mode);
3. Runtime embedded within the binary — if built with `app/hlvm/` 
   (`hashlink.exe` + `libhl.dll`): the toolkit automatically extracts it to `bin/`, 
   making the installation 100% self-contained.

Installation produces the following directory layout:

```text
<root>/HashLoader.exe        Toolkit copy configured as the game's launch executable
<root>/hashloader.toml       Loader configuration (bin, args, runtime, mods, payload)
<root>/hlmods/               Installed mods directory + README.txt
<bin>/.hashloader/           Internal cache: extracted mods + logs + loader.json + hashloader.hdll
```

From this point on, launch the game directly via `HashLoader.exe`.

### 2. Windows `.hlmod` Integration (optional, in HKCU)

Registers the file extension in the current user's Windows registry (no administrator privileges required):

```text
Software\Classes\.hlmod                                     (default) = HashLoader.hlmodPackage
Software\Classes\HashLoader.hlmodPackage\DefaultIcon         = %LOCALAPPDATA%\HashLoader\hlmod.ico
Software\Classes\HashLoader.hlmodPackage\shell\open\command  = "<exe-path>" --info "%1"
```

Includes file association update notifications (`SHChangeNotify`) to refresh desktop icons immediately. 
The "Remove Association" button performs a completely clean uninstallation.

**File manager integration:** upon enabling integration, the `.hlmod` package 
is registered as an archive (`PerceivedType=compressed`, 
`ContentType=application/zip`), adding the following to the context menu / "Open with" list:

- **HashLoader Mod Pack** (default double-click action: visual inspector for mod details and dependencies).
- **Compressed (zipped) Folder** (native Windows Explorer view to inspect archive contents).
- **WinRAR / 7-Zip** if installed on the system (detected across `Program Files`, `Program Files (x86)`, and `App Paths`).

### 3. Mod Packager (developers)

Select the mod's development folder and target `.hlmod` output path; the `hlmod.json` manifest is thoroughly validated before 
generating the final compressed package.

---

## `.hlmod` Package Format

A `.hlmod` package is a standard ZIP archive (Deflate compression) containing a mandatory `hlmod.json` manifest at its root.

```json
{
  "hlmod": 1,
  "id": "my-mod",
  "name": "My Mod",
  "version": "1.0.0",
  "author": "your-username",
  "description": "Example mod for HashLoader",
  "main": "code/main.hl",
  "priority": 100,
  "requires": [],
  "website": "https://github.com/username/my-mod"
}
```

| Field | Description |
| --- | --- |
| `hlmod` | Package format version; currently `1` |
| `id` | Standardized technical identifier: `[a-z0-9_-]` |
| `name` | Human-readable display name |
| `version` | Mod version (used in cache paths: `<id>-<version>`) |
| `author` | Mod creator or development team |
| `description` | Brief summary of mod features |
| `main` | Optional: path to the `.hl` plugin within the package (e.g., `code/main.hl`). Omit for asset-only mods |
| `priority` | Execution priority: lower values load earlier; ties are sorted alphabetically by `id` |
| `requires` | List of required mod dependencies |
| `website` | URL to project repository or website |

Typical mod folder layout:

```text
my-mod/
  hlmod.json
  code/
    main.hl
  override/
    img/
      logo.png
```

Processing rules:

- Contents within `override/` are copied over `<bin>/` following the `priority` order. Replaced files 
  are backed up to `<mod>/originals/`.
- Extracted files are stored in `<bin>/.hashloader/mods/<id>-<version>/` and invalidated 
  automatically whenever the `.hlmod` file changes (tracked by modification timestamp and file size).

---

## Runner Execution Contract (Haxe Payload)

The runner outputs `<bin>/.hashloader/loader.json` and invokes the JIT host:

```text
hashlink.exe [payload.hl] <main.hl> [game arguments]
  cwd               = <bin>
  PATH              = <bin>;<hashlink folder>;<libhl folder>;<bin>/.hashloader
  HASHLOADER_CONFIG = <bin>/.hashloader/loader.json
  HASHLOADER_ROOT   = <root>
  Logs              = <bin>/.hashloader/run.log, run.stdout, run.stderr
```

Reference payload (`app/LoaderPayload.hx` compiled to `app/HashLoader.hl`). The payload runs as the 
**primary entry-point bootstrap** and dynamically links the game as a plugin:

```haxe
class LoaderPayload {
  static function main() {
    final cfg = Sys.getEnv("HASHLOADER_CONFIG"); // loader.json
    // 1) Parse loader.json (mods sorted by priority)
    // 2) Initialize native bridge observer: locate hxd.System,
    //    openfl/lime, or engine loop and hook its tick into the main thread
    // 3) Load the game via sys_load_plugin(main_hl)
  }
}
```

Structure of `loader.json` (regenerated on each launch):

```json
{
  "version": 1,
  "bin": "PATH\\bin",
  "main_hl": "PATH\\bin\\main.hl",
  "payload": "PATH\\bin\\.hashloader\\HashLoader.hl",
  "mods": [
    {
      "id": "my-mod",
      "name": "My Mod",
      "version": "1.0.0",
      "author": "your-username",
      "path": "PATH\\bin\\.hashloader\\mods\\my-mod-1.0.0\\code\\main.hl",
      "priority": 100
    }
  ]
}
```

---

## Plugin Quick-Start Guide (Haxe)

```hxml
# plugin.hxml
-cp src
--main MyMod
--hl code/main.hl
# To share types with the game without duplicating definitions:
#   - annotate target methods and fields with @:virtual
#   - exclude host packages: --macro exclude("game.package.GameClass")
```

Compile with:

```bash
haxe plugin.hxml
```

---

## Mod Runtime API (`hashloader.HashLoader`)

The native bridge (`hashloader.hdll`) locates the game's `.hl` or `hlboot.dat` module in memory 
and exposes its classes to the mod without requiring external dependencies. The mod only needs to import the Haxe SDK:

```haxe
import hashloader.HashLoader;

class Main {
  static function main() {
    if (!HashLoader.available()) return;

    var app = HashLoader.gameInstance(); // Main / hxd.App instance in Heaps
    var scene = HashLoader.scene();      // h2d.Scene in Heaps
    HashLoader.setWindowTitle("My Mod with HashLoader");

    // Frame-by-frame logic on the main game thread:
    HashLoader.onFrame(function() {
      // Synchronized game loop execution
    });
  }
}
```

| Function | Description |
| --- | --- |
| `available()` / `version()` | Native bridge availability state and version |
| `typeCount()` | Total count of registered types inside the game module |
| `resolveClass(name)` / `classExists(name)` | Resolves cached game classes (`"hxd.App"`, `"flixel.FlxG"`, `"Main"`, etc.) |
| `gameInstance()` / `scene()` / `window()` | Retrieves engine instances (`Main`, `h2d.Scene`, SDL/Lime window) |
| `flxG()` / `flxGame()` / `flxState()` | Direct access to core HaxeFlixel engine utilities |
| `setWindowTitle(text)` | Safely updates the active window title |
| `onBeforeFrame(cb)` / `onFrame(cb)` | Synchronized callbacks triggered before or after frame rendering |
| `arrayLength(arr)` / `arrayGet(arr, i)` | Safe cross-module access for dynamic and object arrays in HashLink |
| `staticField` / `setStaticField` | Reads and writes static variables and properties |
| `field` / `setField` | Reads and writes instance properties |
| `call` / `callStatic` / `newInstance` | Dynamic invocation for methods and constructors |

---

## Configuration (`hashloader.toml`)

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

---

## License

**MIT** License
