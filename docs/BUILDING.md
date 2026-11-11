# Building and Development

## Build the toolkit

```bash
cargo build --release
# -> target/release/HashToolkit.exe
```

Requirements: the stable Rust toolchain with MSVC (Visual Studio's `link.exe`; see `.cargo/config.toml`).

`build.rs` converts `app/hashloaderICON.png` and `app/hlmodformatICON.png` into multi-resolution `.ico` files (16, 24, 32, 48, 64, 128, and 256 px), embeds the executable icon, and generates a 128 px `logo.png` for the GUI.

If `app/HashLoader.hl` exists (compiled from `app/LoaderPayload.hx`), it is embedded as the Modding Runtime payload (`cfg has_payload`). If `app/native/hashloader.hdll` exists, it is embedded as the native bridge (`cfg has_bridge`). The runner extracts the bridge to `<bin>/.hashloader/` and appends that folder to `PATH`.

> `cargo build --release` does **not** compile Haxe or C. It embeds artifacts that already exist in `app/`. Rebuild those artifacts before building if you changed the payload or native bridge.

## Rebuild the payload and native bridge

```powershell
# 1. Haxe payload: app/LoaderPayload.hx -> app/HashLoader.hl
haxe -cp app -main LoaderPayload -hl app/HashLoader.hl

# 2. Native bridge: app/native/hashloader.c -> app/native/hashloader.hdll
# Run from a Visual Studio "x64 Native Tools" prompt, or after vcvarsall.bat x64.
cl /nologo /LD /MT /O2 /W3 /I "<hashlink-src>\src" `
   app\native\hashloader.c /Fe:app\native\hashloader.hdll `
   /link /NOLOGO /LIBPATH:"<libhl>" libhl.lib

# 3. Embed both artifacts in the executable
cargo build --release
# -> target/release/HashToolkit.exe
```

- `<hashlink-src>` is the HashLink source tree containing `hl.h`.
- `<libhl>` is the directory containing `libhl.lib` (from the HashLink Windows distribution). If you only have `libhl.dll`, generate an import library using `dumpbin /exports` and `lib /def`.
- The artifacts must be named `app/HashLoader.hl` and `app/native/hashloader.hdll`; `build.rs` and the runner expect these paths.
- `build.rs` tracks artifacts via `rerun-if-changed`. If you do not rebuild the `.hl` or `.hdll` files before `cargo build`, the previous artifacts will be embedded.
- Deploy the build to a game with: `Copy-Item target\release\HashToolkit.exe "<game-directory>\HashLoader.exe" -Force`.

## Update the mod SDK

`app/sdk/hashloader/HashLoader.hx` is embedded as plain text in the toolkit (`include_str!`), then deployed to `tools/sdk/hashloader/` and `tools/templates/src/hashloader/` when installing the tools. It is not bundled into `HashLoader.hl` or the `.hdll`.

After modifying the SDK:

1. Run `cargo build --release` to update the embedded SDK in the binary.
2. Reinstall the tools/SDK from the toolkit GUI.
3. Recompile dependent mods with `haxe build.hxml`.
