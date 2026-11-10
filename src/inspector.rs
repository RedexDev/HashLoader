use std::path::{Path, PathBuf};
use std::time::Duration;

use egui::{Color32, RichText, ScrollArea};

/// Selector de pestañas del inspector.
const TAB_CONTENIDO: usize = 0;
const TAB_LOG: usize = 1;

pub struct InspectorApp {
    pkg: PathBuf,
    logo: Option<egui::TextureHandle>,
    manifest: crate::hlmod::ModManifest,
    entries: Vec<(String, u64)>,
    instance: Option<PathBuf>,
    log_state: Option<LogState>,
    log_lines: Vec<String>,
    log_cached: Option<(u64, u64)>, // (len, mtime)
    tab: usize,
    filter: String,
    preview: Option<(String, String)>,
    last_line_count: usize,
}

struct LogState {
    path: Option<PathBuf>,
}

impl InspectorApp {
    fn new(cc: &eframe::CreationContext<'_>, pkg: &Path) -> Self {
        let manifest = crate::hlmod::read_manifest(pkg).unwrap_or_default();
        let entries = read_entries(pkg);
        let instance = find_instance(pkg);
        let mut log_lines = Vec::new();
        log_lines.push(
            "(esperando: el registro del mod aparecera aqui cuando el juego corra con este paquete)"
                .into(),
        );
        Self {
            logo: crate::load_logo(&cc.egui_ctx),
            pkg: pkg.to_path_buf(),
            manifest,
            entries,
            tab: if instance.is_some() { TAB_LOG } else { TAB_CONTENIDO },
            instance: instance.clone(),
            log_state: Some(LogState { path: None }),
            log_lines,
            log_cached: None,
            filter: String::new(),
            preview: None,
            last_line_count: usize::MAX,
        }
    }

    fn refresh_log(&mut self) {
        let Some(instance) = self.instance.clone() else { return };
        // localizar run.log la primera vez
        let cached_path = self.log_state.as_ref().and_then(|s| s.path.clone());
        let path = match cached_path {
            Some(p) => Some(p),
            None => find_run_log(&instance),
        };
        let Some(path) = path else {
            let msg = "(esta instancia todavia no ha corrido: no hay run.log)";
            if self.log_lines.len() != 1 || self.log_lines[0] != msg {
                self.log_lines = vec![msg.to_string()];
            }
            return;
        };
        self.log_state = Some(LogState {
            path: Some(path.clone()),
        });
        let Ok(md) = std::fs::metadata(&path) else { return };
        let key = (md.len(), mtime(&md));
        if self.log_cached.as_ref() == Some(&key) {
            return; // no cambio; la GUI no pierde tiempo
        }
        self.log_cached = Some(key);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let want = self
                .pkg
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let sections = extract_sections(&text, &want);
            self.log_lines = if sections.is_empty() {
                vec!["(el log del juego no menciona todavia este mod)".into()]
            } else {
                sections
            };
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(tex) = &self.logo {
                ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(40.0, 40.0)));
            }
            ui.vertical(|ui| {
                ui.heading(RichText::new(&self.manifest.name).strong());
                ui.label(
                    RichText::new(format!(
                        "id: {} · v{} · {}{} · formato .hlmod v{}",
                        self.manifest.id,
                        self.manifest.version,
                        if self.manifest.author.is_empty() {
                            "autor desconocido".to_string()
                        } else {
                            format!("por {}", self.manifest.author)
                        },
                        if self.manifest.main_path().is_some() {
                            " · plugin de codigo"
                        } else {
                            " · solo assets"
                        },
                        self.manifest.format,
                    ))
                    .weak()
                    .size(12.0),
                );
            });
        });
        ui.add_space(4.0);
        ui.label(RichText::new(&self.pkg.display().to_string()).weak().size(11.0));
        match &self.instance {
            Some(root) => {
                ui.label(
                    RichText::new(format!(
                        "instalado en la instancia: {} (hlmods)",
                        root.display()
                    ))
                    .color(Color32::from_rgb(0x37, 0xc2, 0x6a)),
                );
            }
            None => {
                ui.label(
                    RichText::new(
                        "no esta instalado en ninguna instancia local (copialo a hlmods/ para ver su log)",
                    )
                    .weak(),
                );
            }
        }
        ui.add_space(6.0);
        ui.separator();
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Copiar id").clicked() {
                ui.ctx().copy_text(self.manifest.id.clone());
            }
            if self.instance.is_some() {
                if let Some(p) = &self.log_state.as_ref().and_then(|s| s.path.clone()) {
                    if ui.button("Ver run.log completo").clicked() {
                        let _ = std::process::Command::new("notepad").arg(p).spawn();
                    }
                }
                if let Some(root) = &self.instance.clone() {
                    let hlmods = root.join("hlmods");
                    if ui.button("Abrir hlmods").clicked() {
                        let _ = std::process::Command::new("explorer").arg(&hlmods).spawn();
                    }
                }
            }
        });
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_log();
        ctx.request_repaint_after(Duration::from_millis(500));

        egui::CentralPanel::default().show(ctx, |ui| {
            self.header(ui);
            self.buttons(ui);
            ui.add_space(4.0);

            // pestañas
            let names: [(&str, usize); 2] = [
                ("Contenido del paquete", TAB_CONTENIDO),
                ("Registro del mod (en vivo)", TAB_LOG),
            ];
            ui.horizontal(|ui| {
                for (text, id) in names {
                    if ui.selectable_label(self.tab == id, text).clicked() {
                        self.tab = id;
                        self.preview = None;
                    }
                    ui.add_space(8.0);
                }
            });
            ui.separator();

            if self.tab == TAB_CONTENIDO {
                self.show_contents(ui);
            } else {
                self.show_log(ui);
            }
        });
    }
}

impl InspectorApp {
    fn show_contents(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text("filtrar por nombre de archivo…")
                .desired_width(320.0),
        );
        ui.add_space(2.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let filter = self.filter.to_lowercase();
                let count = self.entries.len();
                for (name, size) in self.entries.clone() {
                    if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                        continue;
                    }
                    let label = format!("{name}  ({})", human_size(size));
                    let is_manifest = name.eq_ignore_ascii_case("hlmod.json");
                    let text_like = text_like(&name);
                    let tag = if is_manifest {
                        "manifest"
                    } else if text_like {
                        "texto"
                    } else {
                        "binario"
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("[{tag}]")).weak().small());
                        if text_like || is_manifest {
                            if ui
                                .selectable_label(
                                    self.preview.as_ref().map(|(n, _)| n == &name).unwrap_or(false),
                                    label,
                                )
                                .clicked()
                            {
                                match read_text_entry(&self.pkg, &name) {
                                    Some(text) => {
                                        let shown = if text.chars().count() > 120_000 {
                                            format!("(archivo muy grande, primera parte)\n\n{}",
                                                text.chars().take(100_000).collect::<String>())
                                        } else {
                                            text
                                        };
                                        self.preview = Some((name, shown));
                                    }
                                    None => self.preview = Some((name, "(no se pudo leer como texto)".into())),
                                }
                            }
                        } else {
                            ui.label(RichText::new(label).weak());
                        }
                        if size == 0 {
                            ui.label(RichText::new("(carpeta o vacio)").weak().small());
                        }
                    });
                }
                if count == 0 {
                    ui.label(RichText::new("(paquete vacio)").weak());
                }
            });

        if let Some((name, text)) = &self.preview {
            ui.add_space(6.0);
            ui.separator();
            ui.label(RichText::new(format!("vista previa: {name}")).strong());
            ScrollArea::vertical()
                .id_salt("preview_area")
                .auto_shrink([false, false])
                .min_scrolled_height(120.0)
                .max_height(200.0)
                .show(ui, |ui| {
                    let mut text = text.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Monospace)
                            .interactive(false),
                    );
                });
        }
    }

    fn show_log(&mut self, ui: &mut egui::Ui) {
        if self.instance.is_none() {
            ui.label(
                RichText::new(
                    "Este paquete no esta dentro de hlmods/ de ninguna instancia instalada.\n\
                     El registro aparece cuando el mod vive en el juego y este ha corrido.",
                )
                .weak(),
            );
            return;
        }
        ui.label(
            RichText::new(format!(
                "seccion de run.log para '{}' (se actualiza en vivo mientras el juego funciona)",
                self.pkg
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            ))
            .weak()
            .size(11.0),
        );
        ui.add_space(4.0);

        ScrollArea::vertical()
            .id_salt("log_area")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.log_lines.len() != self.last_line_count {
                    self.last_line_count = self.log_lines.len();
                }
                for line in &self.log_lines {
                    let lower = line.to_lowercase();
                    let color = if lower.starts_with("aviso") {
                        Color32::from_rgb(0xe8, 0xb8, 0x35)
                    } else if lower.contains("error") {
                        Color32::from_rgb(0xff, 0x6a, 0x5e)
                    } else if lower.contains("plugin") || lower.contains("-> ") || lower.contains("ok") {
                        Color32::from_rgb(0x37, 0xc2, 0x6a)
                    } else {
                        ui.style().visuals.text_color()
                    };
                    ui.monospace(RichText::new(line).color(color).size(12.0));
                }
            });
    }
}

// ---------------------------------------------------------------------------
// utilidades
// ---------------------------------------------------------------------------

fn mtime(md: &std::fs::Metadata) -> u64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn human_size(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

fn text_like(name: &str) -> bool {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "json" | "txt" | "md" | "xml" | "csv" | "toml" | "ini" | "cfg" | "hxml" | "hx" | "yaml" | "yml")
}

/// Lee textos de una entrada del paquete (None si binaria o inexistente).
fn read_text_entry(pkg: &Path, name: &str) -> Option<String> {
    let file = std::fs::File::open(pkg).ok()?;
    let mut z = zip::ZipArchive::new(std::io::BufReader::new(file)).ok()?;
    for i in 0..z.len() {
        let mut e = z.by_index(i).ok()?;
        if e.name().replace('\\', "/") == name {
            let limit = e.size().min(262_144) as usize;
            let mut raw = vec![0u8; limit];
            std::io::Read::read_exact(&mut e, &mut raw).ok()?;
            if raw.iter().any(|&c| c == 0) {
                return None;
            }
            return String::from_utf8(raw).ok();
        }
    }
    None
}

/// Lista completa nombre+tamano del paquete.
fn read_entries(pkg: &Path) -> Vec<(String, u64)> {
    let Ok(file) = std::fs::File::open(pkg) else {
        return Vec::new();
    };
    let Ok(mut z) = zip::ZipArchive::new(std::io::BufReader::new(file)) else {
        return Vec::new();
    };
    let mut out: Vec<(String, u64)> = Vec::new();
    for i in 0..z.len() {
        if let Ok(e) = z.by_index(i) {
            out.push((e.name().replace('\\', "/"), e.size()));
        }
    }
    // hlmod.json primero, resto alfabetico
    out.sort_by(|a, b| {
        let a_m = a.0.eq_ignore_ascii_case("hlmod.json");
        let b_m = b.0.eq_ignore_ascii_case("hlmod.json");
        b_m.cmp(&a_m).then_with(|| a.0.cmp(&b.0))
    });
    out
}

/// Busca la instancia: un ancestro que sea carpeta hlmods con hashloader.toml.
fn find_instance(pkg: &Path) -> Option<PathBuf> {
    for anc in pkg.ancestors().skip(1) {
        if anc
            .file_name()
            .map(|d| d.eq_ignore_ascii_case("hlmods"))
            .unwrap_or(false)
        {
            if let Some(root) = anc.parent() {
                if root.join("hashloader.toml").is_file() || root.join("HashLoader.exe").is_file()
                {
                    return Some(root.to_path_buf());
                }
            }
        }
    }
    None
}

/// ubicación de run.log de una instancia (bin/.hashloader/run.log)
fn find_run_log(root: &Path) -> Option<PathBuf> {
    let cands = [
        root.join("bin").join(".hashloader").join("run.log"),
        root.join(".hashloader").join("run.log"),
    ];
    for c in cands {
        if c.is_file() {
            return Some(c);
        }
    }
    None
}

/// corta las secciones "== <paquete> ==" del run.log
fn extract_sections(text: &str, want: &str) -> Vec<String> {
    let head = format!("== {want} ==");
    let mut out: Vec<String> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("== ") && t.ends_with(" ==") {
            // cabecera de seccion (de este mod u otra): cambia de estado
            inside = t == head;
            continue;
        }
        if inside && !line.trim().is_empty() {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        vec!["(aun no hay actividad registrada de este mod)".to_string()]
    } else {
        out
    }
}

/// Abre la ventana del inspector (el doble clic del .hlmod termina aqui).
pub fn run(pkg: &Path) -> i32 {
    if !pkg.is_file() {
        crate::dialog::msgbox("HashLoader", &format!("El paquete no existe:\n{}", pkg.display()));
        return 1;
    }
    let title = format!(
        "Inspector de HashLoader — {}",
        pkg.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    let mut opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([820.0, 560.0])
            .with_min_inner_size([620.0, 420.0]),
        ..Default::default()
    };
    if let Ok(icon) = crate::window_icon() {
        opts.viewport.icon = Some(icon);
    }
    let pkg = pkg.to_path_buf();
    let result = eframe::run_native(
        &title,
        opts,
        Box::new(move |cc| Ok(Box::new(InspectorApp::new(cc, &pkg)) as Box<dyn eframe::App>)),
    );
    match result {
        Ok(()) => 0,
        Err(e) => {
            crate::dialog::msgbox("HashLoader", &format!("No se pudo abrir el inspector: {e}"));
            1
        }
    }
}
