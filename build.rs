use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::imageops;

/// Convierte un PNG en un .ico multi-resolución de Windows (entradas BMP 32bpp clásicas).
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

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_payload)");
    println!("cargo::rustc-check-cfg=cfg(has_runtime)");
    println!("cargo::rustc-check-cfg=cfg(has_bridge)");
    println!("cargo:rerun-if-changed=app/LoaderPayload.hx");
    println!("cargo:rerun-if-changed=app/HashLoader.hl");
    println!("cargo:rerun-if-changed=app/native/hashloader.c");
    println!("cargo:rerun-if-changed=app/native/hashloader.hdll");
    println!("cargo:rerun-if-changed=app/hlvm/hashlink.exe");
    println!("cargo:rerun-if-changed=app/hlvm/libhl.dll");

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let root = Path::new(&manifest_dir);
        let payload_hx = root.join("app").join("LoaderPayload.hx");
        if payload_hx.is_file() {
            let status = Command::new("haxe")
                .current_dir(root)
                .args([
                    "-cp",
                    "app",
                    "-main",
                    "LoaderPayload",
                    "-hl",
                    "app/HashLoader.hl",
                ])
                .status();
            if let Ok(s) = status {
                if !s.success() {
                    println!("cargo:warning=haxe fallo al compilar app/LoaderPayload.hx");
                }
            }
        }

        let native_c = root.join("app").join("native").join("hashloader.c");
        let native_hdll = root.join("app").join("native").join("hashloader.hdll");
        let cl_exe = Path::new(
            r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\cl.exe",
        );
        let hl_dir = Path::new(r"C:\hashlink-1.16.0-win");
        if native_c.is_file() && cl_exe.is_file() && hl_dir.is_dir() {
            let obj_out = env::var("OUT_DIR")
                .map(|d| PathBuf::from(d).join("hashloader.obj"))
                .unwrap_or_else(|_| root.join("app").join("native").join("hashloader.obj"));
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
            let _ = fs::remove_file(root.join("app").join("native").join("hashloader.exp"));
            let _ = fs::remove_file(root.join("app").join("native").join("hashloader.lib"));
            if let Ok(s) = status {
                if !s.success() {
                    println!("cargo:warning=cl.exe fallo al compilar app/native/hashloader.c");
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    let icons_result = (|| -> Result<(), String> {
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
        let app_dir = manifest_dir.join("app");
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

        let main_icon = app_dir.join("hashloaderICON.png");
        let mod_icon = app_dir.join("hlmodformatICON.png");
        if !main_icon.exists() {
            return Err(format!("missing icon {}", main_icon.display()));
        }
        if !mod_icon.exists() {
            return Err(format!("missing icon {}", mod_icon.display()));
        }

        let exe_ico = out_dir.join("hashloader.ico");
        let mod_ico = out_dir.join("hlmod.ico");
        let logo = out_dir.join("logo.png");

        build_ico(&main_icon, &exe_ico, &[16, 24, 32, 48, 64, 128, 256])?;
        build_ico(&mod_icon, &mod_ico, &[16, 24, 32, 48, 64, 128, 256])?;
        build_logo(&main_icon, &logo, 128)?;

        // Icono del ejecutable + metadatos del producto
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

    // Payload Haxe del Modding Runtime (opcional): si existe app/HashLoader.hl
    // se embebe en el toolkit y el `cfg has_payload` activa el runner con mods de código.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let src = Path::new(&manifest_dir).join("app").join("HashLoader.hl");
        if src.is_file() {
            if let Ok(out_dir) = env::var("OUT_DIR") {
                let dst = Path::new(&out_dir).join("payload.hl");
                if fs::copy(&src, &dst).is_ok() {
                    println!("cargo:rustc-cfg=has_payload");
                    println!(
                        "cargo:warning=Payload del Modding Runtime embebido (app/HashLoader.hl)"
                    );
                }
            }
        }
    }

    // Puente nativo opcional: app/native/hashloader.hdll se embebe y el runner
    // lo despliega junto al payload para que los mods usen la API de runtime.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let src = Path::new(&manifest_dir)
            .join("app")
            .join("native")
            .join("hashloader.hdll");
        if src.is_file() {
            if let Ok(out_dir) = env::var("OUT_DIR") {
                let dst = Path::new(&out_dir).join("hashloader.hdll");
                if fs::copy(&src, &dst).is_ok() {
                    println!("cargo:rustc-cfg=has_bridge");
                    println!("cargo:warning=Puente nativo embebido (app/native/hashloader.hdll)");
                }
            }
        }
    }

    // Runtime JIT opcional embebido: sincroniza desde C:\hashlink-1.16.0-win
    // hacia app/hlvm/ (hl.exe -> hashlink.exe y libhl.dll) y lo embebe en el ejecutable.
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let hlvm = Path::new(&manifest_dir).join("app").join("hlvm");
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

        if hl_exe.is_file() && hl_dll.is_file() {
            if let Ok(out_dir) = env::var("OUT_DIR") {
                let out = Path::new(&out_dir);
                if fs::copy(&hl_exe, out.join("hashlink.exe")).is_ok()
                    && fs::copy(&hl_dll, out.join("libhl.dll")).is_ok()
                {
                    println!("cargo:rustc-cfg=has_runtime");
                    println!(
                        "cargo:warning=Runtime JIT embebido (app/hlvm/hashlink.exe + libhl.dll)"
                    );
                }
            }
        }
    }
}
