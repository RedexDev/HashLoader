#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;
mod assoc;
mod decompiler_gui;
mod dialog;
mod hlmod;
mod i18n;
mod inspector;
mod install;
mod runner;
mod stubs;
mod tools;
mod util;

/// Payload Haxe del Modding Runtime (HashLoader.hl) — se incluye si existe
/// app/HashLoader.hl al compilar (build.rs emite el cfg `has_payload`).
#[cfg(has_payload)]
pub const PAYLOAD_HL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.hl"));

/// Runtime JIT embebido (hashlink.exe + libhl.dll) — se incluye si existe
/// app/hlvm/ al compilar (build.rs emite el cfg `has_runtime`).
#[cfg(has_runtime)]
pub const EMBED_HASHLINK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hashlink.exe"));
#[cfg(has_runtime)]
pub const EMBED_LIBHL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libhl.dll"));

/// Puente nativo de runtime de los mods (hashloader.hdll) — se incluye si
/// existe app/native/hashloader.hdll al compilar (cfg `has_bridge`).
#[cfg(has_bridge)]
pub const HASHLOADER_HDLL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hashloader.hdll"));

use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // HashToolkit.exe --info "<mod>.hlmod"  →  Inspector gráfico del paquete
    if args.len() >= 3 && args[1] == "--info" {
        let path = std::path::PathBuf::from(&args[2]);
        std::process::exit(inspector::run(&path));
    }

    // HashToolkit.exe --pack "<carpeta-de-mod>" (clic derecho) → "<carpeta>.hlmod"
    if args.len() >= 3 && args[1] == "--pack" {
        let dir = std::path::PathBuf::from(&args[2]);
        std::process::exit(run_dir_packer(&dir));
    }

    // HashToolkit.exe --decompiler [archivo.hl] → Descompilador & Inspector gráfico
    if args.len() >= 2 && (args[1] == "--decompiler" || args[1] == "--inspector-hl") {
        let file = if args.len() >= 3 {
            Some(std::path::PathBuf::from(&args[2]))
        } else {
            None
        };
        std::process::exit(run_decompiler_gui(file));
    }

    if args.len() >= 2 && args[1] == "--version" {
        std::process::exit(0);
    }

    // HashToolkit.exe --assoc-install | --assoc-uninstall (CLI de soporte .hlmod)
    if args.len() >= 2 && args[1] == "--assoc-install" {
        std::process::exit(match assoc::install() {
            Ok(()) => {
                dialog::msgbox("HashLoader", "Soporte .hlmod instalado (usuario actual).");
                0
            }
            Err(e) => {
                dialog::msgbox("HashLoader", &format!("Fallo de registro .hlmod: {e}"));
                1
            }
        });
    }
    if args.len() >= 2 && args[1] == "--assoc-uninstall" {
        std::process::exit(match assoc::uninstall() {
            Ok(_) => 0,
            Err(e) => {
                dialog::msgbox("HashLoader", &format!("Fallo al retirar soporte: {e}"));
                1
            }
        });
    }

    // Sin argumentos:
    //  - si hay hashloader.toml junto al exe (o en cwd) => instancia instalada: runner
    //  - si no => Setup Toolkit (GUI)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let cwd = std::env::current_dir().ok();

    let runner_root = exe_dir
        .as_ref()
        .filter(|d| d.join("hashloader.toml").is_file())
        .cloned()
        .or_else(|| {
            cwd.as_ref()
                .filter(|d| d.join("hashloader.toml").is_file())
                .cloned()
        });

    if let Some(root) = runner_root {
        std::process::exit(runner::run(&root));
    }

        std::process::exit(run_gui());
}

/// Empaquetador directo para el verbo de menú (clic derecho en carpeta de mod).
fn run_dir_packer(dir: &std::path::Path) -> i32 {
    let title = "HashLoader — empaquetar mod";
    if !dir.is_dir() {
        dialog::msgbox(
            title,
            &format!("No existe la carpeta de mod:\n{}", dir.display()),
        );
        return 1;
    }
    let looks_like_mod = dir.join("hlmod.json").is_file()
        || dir.join("code").is_dir()
        || dir.join("override").is_dir();
    if !looks_like_mod {
        dialog::msgbox(
            title,
            &format!(
                "La carpeta no parece un mod de HashLoader (falta hlmod.json, code/ y override/):\n{}",
                dir.display()
            ),
        );
        return 1;
    }
    let mf = match crate::hlmod::read_manifest(dir) {
        Ok(mf) => mf,
        Err(e) => {
            dialog::msgbox(title, &format!("Manifiesto inválido:\n{e}"));
            return 1;
        }
    };
    let out = dir
        .parent()
        .unwrap_or(dir)
        .join(format!("{}.hlmod", mf.id));
    match crate::hlmod::pack(dir, &out) {
        Ok(r) => {
            let plugin = if mf.main_path().is_some() {
                "\n  incluye plugin de código"
            } else {
                ""
            };
            dialog::msgbox(
                title,
                &format!(
                    "Paquete creado:\n{}\n\n{} archivo(s) · {} bytes\nid = {} · v{}{}",
                    out.display(),
                    r.file_count,
                    r.total_bytes,
                    mf.id,
                    mf.version,
                    plugin
                ),
            );
            let _ = std::process::Command::new("explorer").arg(format!(
                "/select,\"{}\"",
                out.display()
            )).spawn();
            0
        }
        Err(e) => {
            dialog::msgbox(title, &format!("Fallo el empaquetado:\n{e}"));
            1
        }
    }
}

fn run_gui() -> i32 {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![
        format!(
            "HashLoader Setup Toolkit {} — listo",
            env!("CARGO_PKG_VERSION")
        ),
        "Kit reutilizable: instalar loader · soporte .hlmod · empaquetar mods · herramientas modding".into(),
    ]));

    let mut opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([820.0, 680.0])
            .with_min_inner_size([640.0, 520.0])
            .with_title("HashLoader Setup Toolkit"),
        ..Default::default()
    };
    if let Ok(icon) = window_icon() {
        opts.viewport.icon = Some(icon);
    }

    let result = eframe::run_native(
        "HashLoader Setup Toolkit",
        opts,
        Box::new(move |cc| Ok(Box::new(app::ToolkitApp::new(cc, log.clone())) as Box<dyn eframe::App>)),
    );

    match result {
        Ok(()) => 0,
        Err(e) => {
            dialog::msgbox("HashLoader", &format!("No se pudo abrir la interfaz: {e}"));
            1
        }
    }
}

fn run_decompiler_gui(initial_file: Option<std::path::PathBuf>) -> i32 {
    let mut opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 700.0])
            .with_min_inner_size([720.0, 500.0])
            .with_title("HashLink Bytecode Decompiler & Inspector"),
        ..Default::default()
    };
    if let Ok(icon) = window_icon() {
        opts.viewport.icon = Some(icon);
    }

    struct StandaloneDecompilerApp {
        state: decompiler_gui::DecompilerState,
    }

    impl eframe::App for StandaloneDecompilerApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(22, 24, 30))
                        .inner_margin(egui::Margin::symmetric(16, 12)),
                )
                .show(ctx, |ui| {
                    self.state.ui(ui);
                });
        }
    }

    let result = eframe::run_native(
        "HashLink Bytecode Decompiler & Inspector",
        opts,
        Box::new(move |_cc| {
            let mut state = decompiler_gui::DecompilerState::default();
            if let Some(f) = initial_file {
                state.open_file(f);
            }
            Ok(Box::new(StandaloneDecompilerApp { state }) as Box<dyn eframe::App>)
        }),
    );

    match result {
        Ok(()) => 0,
        Err(e) => {
            dialog::msgbox("Decompiler", &format!("No se pudo abrir la interfaz: {e}"));
            1
        }
    }
}

/// Icono de ventana (compartido con el inspector).
pub fn window_icon() -> Result<std::sync::Arc<egui::IconData>, String> {
    if assets::LOGO_PNG.is_empty() {
        return Err("sin logo embebido".into());
    }
    let data = eframe::icon_data::from_png_bytes(assets::LOGO_PNG)
        .map_err(|e| format!("logo: {e}"))?;
    Ok(std::sync::Arc::new(data))
}

/// Crea la textura del logo para la GUI (decodifica el PNG embebido).
pub fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    if assets::LOGO_PNG.is_empty() {
        return None;
    }
    let img = image::load_from_memory(assets::LOGO_PNG).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());
    Some(ctx.load_texture("logo", color, egui::TextureOptions::LINEAR))
}
