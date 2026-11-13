use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::imageops;

/// Convierte un PNG en un .ico multi-resolución de Windows (entradas BMP 32bpp clásicas).
#[cfg(target_os = "windows")]
fn build_ico(png_path: &Path, out_path: &Path, sizes: &[u32]) -> Result<(), String> {
    let img =
        image::open(png_path).map_err(|e| format!("failed to open {}: {e}", png_path.display()))?;
    let mut entries: Vec<(u8, u8, Vec<u8>)> = Vec::new();

    for &size in sizes {
        let wh = if size == 256 {
            0u8
        } else {
            size.min(255) as u8
        };
        let resized = img.resize_exact(size, size, imageops::FilterType::Lanczos3);
        let rgba = resized.into_rgba8();

        let pixels = size as usize * size as usize;
        let and_stride = ((size as usize + 31) / 32) * 4;
        let mut bmp: Vec<u8> = Vec::with_capacity(40 + pixels * 5);
        // BITMAPINFOHEADER (40 bytes)
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&(size as i32).to_le_bytes());
        bmp.extend_from_slice(&((size as i32) * 2).to_le_bytes()); // altura incluye máscara AND
        bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
        bmp.extend_from_slice(&32u16.to_le_bytes()); // bpp
        bmp.extend_from_slice(&0u32.to_le_bytes()); // compresión: BI_RGB
        bmp.extend_from_slice(&((pixels * 4) as u32).to_le_bytes()); // tamaño XOR
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        // máscara XOR (bottom-up BGRA)
        for y in (0..size as usize).rev() {
            for x in 0..size as usize {
                let p = rgba.get_pixel(x as u32, y as u32);
                bmp.push(p.0[2]);
                bmp.push(p.0[1]);
                bmp.push(p.0[0]);
                bmp.push(p.0[3]);
            }
        }
        // máscara AND (ceros; el canal alfa manda)
        bmp.extend(std::iter::repeat(0u8).take(and_stride * size as usize));
        entries.push((wh, wh, bmp));
    }

    let mut ico: Vec<u8> = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    ico.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * entries.len();
    for &(w, h, ref data) in &entries {
        ico.push(w);
        ico.push(h);
        ico.push(0); // colores
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&(data.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, _, data) in &entries {
        ico.extend_from_slice(data);
    }

    fs::write(out_path, ico).map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
    Ok(())
}

fn build_logo(png_path: &Path, out_path: &Path, size: u32) -> Result<(), String> {
    let img = image::open(png_path).map_err(|e| format!("{e}"))?;
    let resized = img.resize_exact(size, size, imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut out));
    resized
        .write_with_encoder(encoder)
        .map_err(|e| format!("{e}"))?;
    fs::write(out_path, out).map_err(|e| format!("{e}"))?;
    Ok(())
}

fn find_haxe() -> Option<PathBuf> {
    if Command::new("haxe").arg("--version").output().is_ok() {
        return Some(PathBuf::from("haxe"));
    }
    #[cfg(not(windows))]
    for cand in ["/opt/haxe/haxe", "/usr/local/bin/haxe", "/usr/bin/haxe"] {
        let p = Path::new(cand);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_c_compiler() -> Option<PathBuf> {
    for cc in ["cc", "gcc", "clang"] {
        if Command::new(cc).arg("--version").output().is_ok() {
            return Some(PathBuf::from(cc));
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_hl_sdk_unix() -> (Option<PathBuf>, Option<PathBuf>) {
    let inc = env::var_os("HASHLINK_INCLUDE")
        .map(PathBuf::from)
        .filter(|p| p.join("hl.h").is_file())
        .or_else(|| {
            ["/usr/local/include", "/usr/include"]
                .into_iter()
                .map(PathBuf::from)
                .find(|p| p.join("hl.h").is_file())
        });
    let lib = env::var_os("HASHLINK_LIB")
        .map(PathBuf::from)
        .filter(|p| p.join("libhl.so").is_file())
        .or_else(|| {
            [
                "/usr/local/lib",
                "/usr/lib",
                "/usr/lib/x86_64-linux-gnu",
                "/usr/lib/aarch64-linux-gnu",
            ]
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.join("libhl.so").is_file())
        });
    (inc, lib)
}

#[cfg(not(target_os = "windows"))]
fn copy_if_different(src: &Path, dst: &Path, executable: bool) -> bool {
    let same = match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(sm), Ok(dm)) if sm.len() == dm.len() => {
            matches!((fs::read(src), fs::read(dst)), (Ok(sb), Ok(db)) if sb == db)
        }
        _ => false,
    };
    if !same {
        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::copy(src, dst).is_err() {
            return false;
        }
    }
    if executable {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(dst) {
                let mut perm = meta.permissions();
                let mode = perm.mode();
                if mode & 0o111 == 0 {
                    perm.set_mode(mode | 0o755);
                    let _ = fs::set_permissions(dst, perm);
                }
            }
        }
    }
    true
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_payload)");
    println!("cargo::rustc-check-cfg=cfg(has_runtime)");
    println!("cargo::rustc-check-cfg=cfg(has_bridge)");
    println!("cargo:rerun-if-changed=app/LoaderPayload.hx");
    println!("cargo:rerun-if-changed=app/HashLoader.hl");
    println!("cargo:rerun-if-changed=app/native/hashloader.c");
    println!("cargo:rerun-if-changed=app/hashloaderICON.png");
    println!("cargo:rerun-if-changed=app/hlmodformatICON.png");
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=app/native/hashloader.hdll");
        println!("cargo:rerun-if-changed=app/hlvm/hashlink.exe");
        println!("cargo:rerun-if-changed=app/hlvm/libhl.dll");
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("cargo:rerun-if-changed=app/native/linux/hashloader.hdll");
        println!("cargo:rerun-if-changed=app/hlvm/hashlink");
        println!("cargo:rerun-if-changed=app/hlvm/libhl.so");
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let out_dir_str = env::var("OUT_DIR").unwrap_or_default();
    let root = Path::new(&manifest_dir);
    let out_dir = Path::new(&out_dir_str);

    // 1. Logo PNG multiplataforma (icono de ventana + UI) e icono .hlmod para Linux/Unix
    let app_dir = root.join("app");
    let main_icon = app_dir.join("hashloaderICON.png");
    let mod_icon = app_dir.join("hlmodformatICON.png");
    if main_icon.exists() {
        let logo = out_dir.join("logo.png");
        if let Err(msg) = build_logo(&main_icon, &logo, 128) {
            println!("cargo:warning=HashLoader logo pipeline: {msg}");
        }
    }
    #[cfg(not(target_os = "windows"))]
    if mod_icon.exists() {
        for (size, name) in [(128, "hlmod_128.png"), (256, "hlmod_256.png")] {
            if let Err(msg) = build_logo(&mod_icon, &out_dir.join(name), size) {
                println!("cargo:warning=HashLoader .hlmod icon pipeline ({name}): {msg}");
            }
        }
    }

    // 2. Iconos y recursos PE solo en Windows
    #[cfg(target_os = "windows")]
    let icons_result = (|| -> Result<(), String> {
        if !main_icon.exists() {
            return Err(format!("missing icon {}", main_icon.display()));
        }
        if !mod_icon.exists() {
            return Err(format!("missing icon {}", mod_icon.display()));
        }

        let exe_ico = out_dir.join("hashloader.ico");
        let mod_ico = out_dir.join("hlmod.ico");

        build_ico(&main_icon, &exe_ico, &[16, 24, 32, 48, 64, 128, 256])?;
        build_ico(&mod_icon, &mod_ico, &[16, 24, 32, 48, 64, 128, 256])?;

        winresource::WindowsResource::new()
            .set_icon(&exe_ico.to_string_lossy())
            .set("FileDescription", "HashLoader Mod Loader")
            .set("ProductName", "HashLoader")
            .compile()
            .map_err(|e| format!("winresource: {e}"))?;

        Ok(())
    })();

    #[cfg(target_os = "windows")]
    if let Err(msg) = icons_result {
        println!("cargo:warning=HashLoader icon pipeline: {msg}");
    }

    // 3. Payload Haxe del Modding Runtime (LoaderPayload.hx -> payload.hl)
    let payload_dst = out_dir.join("payload.hl");
    let payload_hx = app_dir.join("LoaderPayload.hx");
    let payload_app = app_dir.join("HashLoader.hl");
    let mut payload_ready = false;

    if payload_hx.is_file() {
        if let Some(haxe_bin) = find_haxe() {
            #[cfg(target_os = "windows")]
            let hl_out = payload_app.clone();
            #[cfg(not(target_os = "windows"))]
            let hl_out = payload_dst.clone();

            let mut cmd = Command::new(&haxe_bin);
            cmd.current_dir(root)
                .args(["-cp", "app", "-main", "LoaderPayload", "-hl"])
                .arg(&hl_out);
            #[cfg(not(target_os = "windows"))]
            if env::var_os("HAXE_STD_PATH").is_none() && Path::new("/opt/haxe/std").is_dir() {
                cmd.env("HAXE_STD_PATH", "/opt/haxe/std");
            }
            if let Ok(s) = cmd.status() {
                if !s.success() {
                    println!("cargo:warning=haxe fallo al compilar app/LoaderPayload.hx");
                } else {
                    #[cfg(not(target_os = "windows"))]
                    {
                        payload_ready = payload_dst.is_file();
                    }
                }
            }
        }
    }
    if !payload_ready && payload_app.is_file() {
        if fs::copy(&payload_app, &payload_dst).is_ok() {
            payload_ready = true;
        }
    }
    if payload_ready {
        println!("cargo:rustc-cfg=has_payload");
        println!("cargo:warning=Payload del Modding Runtime embebido");
    }

    // 4. Puente nativo (app/native/hashloader.c -> OUT_DIR/hashloader.hdll)
    let native_c = app_dir.join("native").join("hashloader.c");
    let bridge_dst = out_dir.join("hashloader.hdll");
    let mut bridge_ready = false;

    #[cfg(target_os = "windows")]
    {
        let native_hdll = app_dir.join("native").join("hashloader.hdll");
        let cl_exe = Path::new(
            r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\cl.exe",
        );
        let hl_dir = Path::new(r"C:\hashlink-1.16.0-win");
        if native_c.is_file() && cl_exe.is_file() && hl_dir.is_dir() {
            let obj_out = out_dir.join("hashloader.obj");
            let status = Command::new(cl_exe)
                .current_dir(root)
                .arg("/nologo")
                .arg("/LD")
                .arg("/MT")
                .arg("/O2")
                .arg("/W3")
                .arg(format!("/I{}", hl_dir.join("include").display()))
                .arg(r"/IC:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.44.35207\include")
                .arg(r"/IC:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\ucrt")
                .arg(r"/IC:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\um")
                .arg(r"/IC:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\shared")
                .arg(&native_c)
                .arg(format!("/Fe:{}", native_hdll.display()))
                .arg(format!("/Fo:{}", obj_out.display()))
                .arg("/link")
                .arg("/NOLOGO")
                .arg(format!("/LIBPATH:{}", hl_dir.display()))
                .arg(r"/LIBPATH:C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.44.35207\lib\x64")
                .arg(r"/LIBPATH:C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22621.0\ucrt\x64")
                .arg(r"/LIBPATH:C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22621.0\um\x64")
                .arg("libhl.lib")
                .status();
            let _ = fs::remove_file(app_dir.join("native").join("hashloader.exp"));
            let _ = fs::remove_file(app_dir.join("native").join("hashloader.lib"));
            if let Ok(s) = status {
                if !s.success() {
                    println!("cargo:warning=cl.exe fallo al compilar app/native/hashloader.c");
                }
            }
        }
        if native_hdll.is_file() && fs::copy(&native_hdll, &bridge_dst).is_ok() {
            bridge_ready = true;
            println!("cargo:warning=Puente nativo embebido (app/native/hashloader.hdll)");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let prebuilt = app_dir.join("native").join("linux").join("hashloader.hdll");
        if native_c.is_file() {
            let (inc_opt, lib_opt) = find_hl_sdk_unix();
            if let (Some(cc), Some(inc), Some(lib)) = (find_c_compiler(), inc_opt, lib_opt) {
                let status = Command::new(&cc)
                    .current_dir(root)
                    .args(["-shared", "-fPIC", "-O2", "-Wall", "-Wextra"])
                    .arg(format!("-I{}", inc.display()))
                    .arg(&native_c)
                    .arg("-o")
                    .arg(&bridge_dst)
                    .arg(format!("-L{}", lib.display()))
                    .arg("-Wl,-rpath,$ORIGIN:$ORIGIN/..:.:/usr/local/lib")
                    .args(["-lhl", "-ldl", "-lpthread"])
                    .status();
                match status {
                    Ok(s) if s.success() && bridge_dst.is_file() => {
                        bridge_ready = true;
                        copy_if_different(&bridge_dst, &prebuilt, true);
                        println!("cargo:warning=Puente nativo compilado y embebido (app/native/linux/hashloader.hdll)");
                    }
                    Ok(_) => {
                        println!("cargo:warning={} fallo al compilar app/native/hashloader.c", cc.display());
                    }
                    Err(e) => {
                        println!("cargo:warning=No se pudo ejecutar {}: {e}", cc.display());
                    }
                }
            }
        }
        if !bridge_ready {
            if prebuilt.is_file() && fs::copy(&prebuilt, &bridge_dst).is_ok() {
                bridge_ready = true;
                println!("cargo:warning=Puente nativo precompilado embebido (app/native/linux/hashloader.hdll)");
            }
        }
    }

    if bridge_ready {
        println!("cargo:rustc-cfg=has_bridge");
    }

    // 5. Runtime JIT opcional embebido (Windows: hashlink.exe + libhl.dll; Linux: hashlink + libhl.so)
    #[cfg(target_os = "windows")]
    {
        let hlvm = app_dir.join("hlvm");
        let _ = fs::create_dir_all(&hlvm);
        let hl_exe = hlvm.join("hashlink.exe");
        let hl_dll = hlvm.join("libhl.dll");

        let sys_hl = Path::new(r"C:\hashlink-1.16.0-win");
        let sys_hl_exe = sys_hl.join("hl.exe");
        let sys_hl_dll = sys_hl.join("libhl.dll");
        if sys_hl_exe.is_file() {
            let _ = fs::copy(&sys_hl_exe, &hl_exe);
        }
        if sys_hl_dll.is_file() {
            let _ = fs::copy(&sys_hl_dll, &hl_dll);
        }

        if hl_exe.is_file()
            && hl_dll.is_file()
            && fs::copy(&hl_exe, out_dir.join("hashlink.exe")).is_ok()
            && fs::copy(&hl_dll, out_dir.join("libhl.dll")).is_ok()
        {
            println!("cargo:rustc-cfg=has_runtime");
            println!("cargo:warning=Runtime JIT embebido (app/hlvm/hashlink.exe + libhl.dll)");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let hlvm = app_dir.join("hlvm");
        let _ = fs::create_dir_all(&hlvm);
        let hl_bin_dst = hlvm.join("hashlink");
        let hl_so_dst = hlvm.join("libhl.so");

        let sys_bin = env::var_os("HASHLINK_BIN")
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| {
                [
                    "/usr/local/bin/hl",
                    "/usr/bin/hl",
                    "/usr/local/bin/hashlink",
                    "/usr/bin/hashlink",
                ]
                .into_iter()
                .map(PathBuf::from)
                .find(|p| p.is_file())
            });
        if let Some(src_bin) = sys_bin {
            copy_if_different(&src_bin, &hl_bin_dst, true);
        }

        if let Some(lib_dir) = find_hl_sdk_unix().1 {
            copy_if_different(&lib_dir.join("libhl.so"), &hl_so_dst, true);
        }

        if hl_bin_dst.is_file()
            && hl_so_dst.is_file()
            && fs::copy(&hl_bin_dst, out_dir.join("hashlink")).is_ok()
            && fs::copy(&hl_so_dst, out_dir.join("libhl.so")).is_ok()
        {
            println!("cargo:rustc-cfg=has_runtime");
            println!("cargo:warning=Runtime JIT embebido (app/hlvm/hashlink + libhl.so)");
        }
    }
}
