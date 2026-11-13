# Building and Development

## Build the toolkit (Windows)

```bash
cargo build --release
# -> target/release/HashToolkit.exe
```

Requirements (Windows): the stable Rust toolchain with MSVC (Visual Studio's `link.exe`; see `.cargo/config.toml`).

## Build the toolkit (Linux / Debian)

```bash
cargo build --release
# -> target/release/HashToolkit
```

Requirements (Linux): `gcc`/`cc`, `haxe` (e.g., `/opt/haxe/haxe` with stdlib in `/opt/haxe/std`), and HashLink 1.16+ installed in `/usr/local` (`/usr/local/include/hl.h`, `/usr/local/lib/libhl.so`, `/usr/local/bin/hl`), plus GUI system libraries for `eframe`/`rfd` (Mesa/OpenGL, X11/Wayland, libxkbcommon, gtk3). You can override the HashLink paths with `HASHLINK_INCLUDE`, `HASHLINK_LIB`, and `HASHLINK_BIN` (or rely on pre-staged binaries in `app/hlvm/` and `app/native/linux/` when HashLink is not installed system-wide).

`build.rs` compiles `app/LoaderPayload.hx` (when `haxe` is available), compiles `app/native/hashloader.c` and syncs the Linux bridge to `app/native/linux/hashloader.hdll`, syncs `/usr/local/bin/hl` and `/usr/local/lib/libhl.so` directly into `app/hlvm/hashlink` and `app/hlvm/libhl.so` (without colliding with Windows `hashlink.exe` and `libhl.dll`), generates `logo.png` for the GUI, and embeds everything cleanly so target machines do not need HashLink installed.

---

`build.rs` converts `app/hashloaderICON.png` and `app/hlmodformatICON.png` into multi-resolution `.ico` files (16, 24, 32, 48, 64, 128, and 256 px on Windows), embeds the executable icon, and generates a 128 px `logo.png` for the GUI.

If `app/HashLoader.hl` exists (compiled from `app/LoaderPayload.hx`), it is embedded as the Modding Runtime payload (`cfg has_payload`). The native bridge (`hashloader.hdll`) is embedded as `cfg has_bridge`, and the runner extracts it to `<bin>/.hashloader/` and adds that folder to `PATH` (and `LD_LIBRARY_PATH` on Unix).

> Both Windows (`app/hlvm/hashlink.exe`, `app/hlvm/libhl.dll`, `app/native/hashloader.hdll`) and Linux (`app/hlvm/hashlink`, `app/hlvm/libhl.so`, `app/native/linux/hashloader.hdll`) prebuilt artifacts coexist without colliding, so `cargo build --release` works even when Haxe, a C compiler, or a system-wide HashLink install is absent.

## Rebuild the payload and native bridge manually

### Windows (PowerShell / x64 Native Tools)

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

### Linux (Debian)

```bash
# 1. Haxe payload -> OUT_DIR/payload.hl (handled automatically by build.rs if Haxe is installed)
haxe -cp app -main LoaderPayload -hl app/HashLoader.hl

# 2. Native bridge (optional manual check / staging under app/native/linux/)
mkdir -p app/native/linux
gcc -shared -fPIC -O2 -Wall -Wextra -I/usr/local/include \
    app/native/hashloader.c -o app/native/linux/hashloader.hdll \
    -L/usr/local/lib -Wl,-rpath,'$ORIGIN:$ORIGIN/..:.:/usr/local/lib' -lhl -ldl -lpthread

# 3. Build toolkit
cargo build --release
# -> target/release/HashToolkit
```

- `<hashlink-src>` is the HashLink source tree containing `hl.h` (or `/usr/local/include` on Debian).
- `<libhl>` is the directory containing `libhl.lib` (Windows) or `libhl.so` (`/usr/local/lib` on Debian).
- Deploy the build manually to a game if desired: copy `target/release/HashToolkit.exe` as `HashLoader.exe` (Windows) or `target/release/HashToolkit` as `HashLoader` (Linux).

## Update the mod SDK

`app/sdk/hashloader/HashLoader.hx` is embedded as plain text in the toolkit (`include_str!`), then deployed to `tools/sdk/hashloader/` and `tools/templates/src/hashloader/` when installing the tools. It is not bundled into `HashLoader.hl` or the `.hdll`.

After modifying the SDK:

1. Run `cargo build --release` to update the embedded SDK in the binary.
2. Reinstall the tools/SDK from the toolkit GUI.
3. Recompile dependent mods with `haxe build.hxml`.
