# Mod Creation Guide

## `.hlmod` package format

A `.hlmod` is a standard ZIP archive using Deflate compression. It must contain an `hlmod.json` manifest at its root.

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
| `main` | Optional path to the `.hl` plugin (e.g. `code/main.hl`). Omit for asset-only mods |
| `priority` | Execution priority: lower values load earlier; ties are sorted alphabetically by `id` |
| `requires` | List of required mod dependencies |
| `website` | URL to the project repository or website |

Example mod folder:

```text
my-mod/
  hlmod.json
  code/
    main.hl
  override/
    img/
      logo.png
```

Files in `override/` are copied over `<bin>/` in `priority` order. Replaced files are backed up to `<mod>/originals/`. Extracted files are stored in `<bin>/.hashloader/mods/<id>-<version>/` and invalidated automatically when the `.hlmod` changes (tracked by modification timestamp and file size).

## Plugin quick start (Haxe)

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

## Mod Runtime API (`hashloader.HashLoader`)

The native bridge (`hashloader.hdll`) locates the game's `.hl` or `hlboot.dat` module in memory and exposes its classes to the mod without external dependencies. A mod only needs to import the Haxe SDK:

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
| `onBeforeFrame(cb)` / `onFrame(cb)` | Synchronized callbacks before or after frame rendering |
| `arrayLength(arr)` / `arrayGet(arr, i)` | Safe cross-module access for dynamic and object arrays in HashLink |
| `staticField` / `setStaticField` | Reads and writes static variables and properties |
| `field` / `setField` | Reads and writes instance properties |
| `call` / `callStatic` / `newInstance` | Dynamic invocation for methods and constructors |
