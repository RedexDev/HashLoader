# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Generic native bridge `hashloader.hdll` embedded into the executable and deployed by the runner into `<bin>/.hashloader/` (appended to `PATH`). Locates the game module (`*.hl` or `hlboot.dat`) and exposes its classes to mods.
- Support for custom `.hl` filenames and Lime architectures (`hlboot.dat` + `<Game>.exe`) used across HaxeFlixel, OpenFL, Lime, Peote, and Heaps games.
- Runtime Haxe SDK (`hashloader.HashLoader`) featuring cached class resolution, game context (`Main`/`hxd.App`, `FlxG`, `FlxGame`, `FlxState`), scene/window handles, window title control, `onBeforeFrame`/`onFrame` callbacks, reflection helpers (`staticField`, `field`, `call`, `newInstance`, etc.), and safe cross-module array utilities (`arrayLength`, `arrayGet`).
- Payload intercepts the main loop (`hxd.System.setLoop`, `lime.app.Application.window.onRender` / `FlxGame.onEnterFrame`), loads mods on the main thread during the first frame, and stores global game context.
- Mod templates and SDK installable directly through the toolkit interface (`tools/sdk/hashloader/`, `tools/templates/src/hashloader/`).
- Graphical decompiler and visual bytecode inspector for HashLink (`.hl`).
- Automated Haxe stub generator (`.hx`) from `.hl` bytecode files.

### Changed

- Renamed internal loader payload to `LoaderPayload` (`app/LoaderPayload.hx`). The compiled artifact remains `app/HashLoader.hl`, keeping full compatibility with the runner and packager.
- Critical performance optimizations in native bridge `hashloader.c`: readable memory region caching (`VirtualQuery`), accelerated type traversal, and single-pass scoring to prevent frame rate drops.
- Updated GUI with multi-language selector (English by default, Spanish, Russian, and Portuguese) and clickable link to the author's GitHub profile.

### Fixed

- Native signature for `Void->Void` callbacks (`_FUN(_VOID, _NO_ARG)`) and functions returning `Bool` (`_BOOL`).
- Native bridge memory scanning validates each read via `VirtualQuery` (eliminating stale region caches) and suspends background threads during memory traversal, preventing access violations during parallel game initialization.
- Loop wrapper now survives main-loop re-initializations such as `hxd.App.setup()`, which invokes `setLoop(mainLoop)` again after `init()`.
- Cross-module type mismatch (`ArrayObj` to `ArrayDyn`) when reading native engine arrays.
