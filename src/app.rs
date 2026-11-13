use std::sync::{Arc, Mutex};

use egui::{Color32, RichText};

use crate::assoc::AssocStatus;
use crate::i18n::Language;

pub struct ToolkitApp {
    log: Arc<Mutex<Vec<String>>>,
    logo: Option<egui::TextureHandle>,
    assoc_status: AssocStatus,
    tools_progress: Arc<Mutex<Option<(f32, String)>>>,
    decompiler: crate::decompiler_gui::DecompilerState,
    show_decompiler: bool,
    pub language: Language,
}

impl ToolkitApp {
    pub fn new(cc: &eframe::CreationContext<'_>, log: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            logo: crate::load_logo(&cc.egui_ctx),
            log,
            assoc_status: crate::assoc::status(),
            tools_progress: Arc::new(Mutex::new(None)),
            decompiler: crate::decompiler_gui::DecompilerState::default(),
            show_decompiler: false,
            language: Language::English,
        }
    }

    fn log_str(&self, s: &str) {
        if let Ok(mut v) = self.log.lock() {
            for line in s.lines() {
                v.push(line.to_string());
            }
        }
    }
}

impl eframe::App for ToolkitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Barra inferior fija (docked footer) - siempre visible en la ventana
        egui::TopBottomPanel::bottom("toolkit_footer_panel")
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(18, 20, 26))
                    .inner_margin(egui::Margin::symmetric(16, 9))
                    .stroke(egui::Stroke::new(1.0f32, Color32::from_rgb(38, 42, 54))),
            )
            .show(ctx, |ui| {
                self.footer(ui);
            });

        if self.show_decompiler {
            let t = self.language.texts();
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(Color32::from_rgb(22, 24, 30))
                        .inner_margin(egui::Margin::symmetric(16, 12)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new(t.btn_back_to_toolkit).strong().size(14.0))
                            .clicked()
                        {
                            self.show_decompiler = false;
                        }
                        ui.add_space(10.0);
                        ui.heading(
                            RichText::new(t.decompiler_heading)
                                .size(18.0)
                                .strong(),
                        );
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    self.decompiler.ui(ui);
                });
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(Color32::from_rgb(22, 24, 30))
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ctx, |ui| {
                self.header(ui);
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        card_frame(ui).show(ui, |ui| self.card_install(ui));
                        ui.add_space(10.0);
                        card_frame(ui).show(ui, |ui| self.card_assoc(ui));
                        ui.add_space(10.0);
                        card_frame(ui).show(ui, |ui| self.card_pack(ui));
                        ui.add_space(10.0);
                        card_frame(ui).show(ui, |ui| self.card_tools(ui));
                        ui.add_space(10.0);
                        self.log_panel(ui);
                        ui.add_space(10.0);
                    });
            });
    }
}

fn card_frame(ui: &mut egui::Ui) -> egui::Frame {
    egui::Frame::default()
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .fill(ui.visuals().panel_fill)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .corner_radius(6)
}

impl ToolkitApp {
    fn header(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        ui.horizontal(|ui| {
            if let Some(tex) = &self.logo {
                ui.add(
                    egui::Image::new(tex).fit_to_exact_size(egui::vec2(52.0, 52.0)),
                );
            }
            ui.vertical(|ui| {
                ui.heading(RichText::new(t.app_title).size(24.0).strong());
                ui.label(
                    RichText::new(format!(
                        "{} · v1.0",
                        t.app_subtitle
                    ))
                    .weak()
                    .size(12.0),
                );
            });

            // Botones de selección de idioma también en la cabecera superior derecha
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let langs = [
                    (Language::Portuguese, "Português"),
                    (Language::Russian, "Русский"),
                    (Language::Spanish, "Español"),
                    (Language::English, "English"),
                ];

                for (l, name) in langs {
                    let is_sel = self.language == l;
                    let btn = egui::Button::new(
                        RichText::new(name)
                            .size(11.5)
                            .strong()
                            .color(if is_sel {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(175, 175, 185)
                            }),
                    )
                    .fill(if is_sel {
                        Color32::from_rgb(0x23, 0x6e, 0xaa)
                    } else {
                        Color32::from_rgb(0x28, 0x2b, 0x33)
                    });

                    if ui.add(btn).clicked() {
                        self.language = l;
                        self.decompiler.lang = l;
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
    }

    fn card_install(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        ui.label(RichText::new(t.card_install_title).size(16.0).strong());
        ui.label(RichText::new(t.card_install_desc).weak());
        ui.add_space(8.0);
        let btn = egui::Button::new(
            RichText::new(t.btn_install_folder).size(20.0).strong(),
        )
        .min_size(egui::vec2(ui.available_width(), 52.0))
        .fill(Color32::from_rgb(0x1f, 0x8a, 0x3d));
        if ui.add(btn).clicked() {
            if let Some(dir) = pick_folder() {
                self.action_install_target(&dir);
            }
        }
        ui.add_space(4.0);
        let btn_file = egui::Button::new(
            RichText::new(t.btn_select_hl_file).size(14.0),
        )
        .min_size(egui::vec2(ui.available_width(), 34.0));
        if ui.add(btn_file).clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .set_title(t.select_hl_dialog_title)
                .add_filter("HashLink Bytecode (*.hl, *.dat)", &["hl", "dat"])
                .pick_file()
            {
                self.action_install_target(&file);
            }
        }
    }

    fn action_install_target(&mut self, target: &std::path::Path) {
        self.log_str(&format!("instalar HashLoader en: {}", target.display()));
        let det = crate::install::detect(target);

        // rechazo temprano solo si no hay rastro de un programa HashLink
        if det.main_hl.is_none() && det.hdll_count == 0 {
            let report = format!(
                "Localizado en el destino:\n  bytecodes .hl : ninguno\n  *.hdll        : 0\n  bin           : {}\n\nElige la carpeta del juego (o selecciona directamente el archivo *.hl / hlboot.dat).",
                pb_str(&det.bin_dir),
            );
            self.log_str("destino NO listo: ni .hl ni .hdll");
            crate::dialog::msgbox("No parece un destino HL/JIT", &report);
            return;
        }

        self.log_str(&format!(
            "destino: bytecode:{} libhl:{} hashlink:{} hdlls:{}",
            pb_str(&det.main_hl),
            pb_str(&det.libhl),
            pb_str(&det.hashlink),
            det.hdll_count
        ));
        match crate::install::install(&det) {
            Ok(rep) => {
                self.log_str(&format!(
                    "INSTALADO:\n  {} : {}\n  config         : {}\n  hlmods         : {}\n  bytecode (.hl) : {}",
                    crate::util::LOADER_EXE_NAME, rep.loader_exe, rep.config, rep.hlmods, rep.main_hl
                ));
                for w in &rep.warnings {
                    self.log_str(&format!("AVISO: {w}"));
                }
                crate::util::open_folder(std::path::Path::new(&rep.hlmods));
            }
            Err(e) => {
                self.log_str(&format!("ERROR: {e}"));
                crate::dialog::msgbox("Fallo de instalación del loader", &e);
            }
        }
    }

    // ------------------------------------------------------------------
    // 2) Soporte .hlmod en el sistema (opcional)
    // ------------------------------------------------------------------
    fn card_assoc(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        let estado = match self.assoc_status {
            AssocStatus::Installed => t.assoc_status_installed,
            AssocStatus::InstalledForeign => t.assoc_status_foreign,
            AssocStatus::NotInstalled => t.assoc_status_not_installed,
        };
        ui.label(RichText::new(t.card_assoc_title).size(16.0).strong());
        ui.label(RichText::new(t.card_assoc_desc.replace("{}", estado)).weak());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(t.btn_assoc_install)).clicked() {
                match crate::assoc::install() {
                    Ok(()) => {
                        self.assoc_status = crate::assoc::status();
                        self.log_str("Soporte .hlmod registrado.");
                    }
                    Err(e) => {
                        self.log_str(&format!("ERROR: {e}"));
                        crate::dialog::msgbox("Fallo de registro .hlmod", &e);
                    }
                }
            }
            if ui.add(egui::Button::new(t.btn_assoc_uninstall)).clicked() {
                match crate::assoc::uninstall() {
                    Ok(had) => {
                        self.assoc_status = crate::assoc::status();
                        self.log_str(if had {
                            "Soporte .hlmod desinstalado."
                        } else {
                            "No había soporte de HashLoader."
                        });
                    }
                    Err(e) => crate::dialog::msgbox("Fallo al retirar soporte", &e),
                }
            }
        });
    }

    // ------------------------------------------------------------------
    // 3) Empaquetar carpetas en .hlmod (desarrolladores)
    // ------------------------------------------------------------------
    fn card_pack(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        ui.label(RichText::new(t.card_pack_title).size(16.0).strong());
        ui.label(RichText::new(t.card_pack_desc).weak());
        ui.add_space(6.0);
        if ui.add(egui::Button::new(t.btn_pack_folder)).clicked() {
            self.action_pack();
        }
    }

    fn action_pack(&mut self) {
        self.log_str("empaquetando carpeta de mod…");
        let Some(src) = pick_folder() else { return };
        let mf = match crate::hlmod::read_manifest(&src) {
            Ok(mf) => mf,
            Err(e) => {
                self.log_str(&format!("carpeta de mod inválida: {e}"));
                crate::dialog::msgbox("hlmod.json", &e);
                return;
            }
        };
        let default_name = format!("{}.hlmod", mf.id);
        let Some(out) = save_file(&default_name) else { return };
        match crate::hlmod::pack(&src, &out) {
            Ok(r) => {
                self.log_str(&format!(
                    "paquete creado: {} ({} archivos, {} bytes) · id={} version={} plugin={}",
                    out.display(),
                    r.file_count,
                    r.total_bytes,
                    mf.id,
                    mf.version,
                    mf.main_path().unwrap_or_else(|| "(solo assets)".into()),
                ));
                crate::util::reveal_file(&out);
            }
            Err(e) => {
                self.log_str(&format!("ERROR empaquetando: {e}"));
                crate::dialog::msgbox("Fallo de empaquetado", &e);
            }
        }
    }

    // ------------------------------------------------------------------
    // 4) Herramientas de Modding & Reverse Engineering (SDK, Stubs, GUI)
    // ------------------------------------------------------------------
    fn card_tools(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        let current_prog = self.tools_progress.lock().ok().and_then(|g| g.clone());
        let st = crate::tools::status(&current_prog);
        let tdir = crate::tools::tools_dir();

        ui.label(RichText::new(t.card_tools_title).size(16.0).strong());
        ui.label(RichText::new(t.card_tools_desc).weak());
        ui.add_space(4.0);

        // Ruta de herramientas en disco con botón de acceso rápido
        ui.horizontal(|ui| {
            ui.label(RichText::new("📁").size(12.0));
            ui.label(RichText::new(tdir.display().to_string()).weak().size(11.5));
            if ui.small_button(t.btn_open_tools_dir).clicked() {
                let _ = std::fs::create_dir_all(&tdir);
                crate::util::open_folder(&tdir);
            }
        });
        ui.add_space(8.0);

        match st {
            crate::tools::ToolsStatus::NotInstalled => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.tools_status_label).weak());
                    ui.label(
                        RichText::new(t.tools_status_not_installed)
                            .color(Color32::from_rgb(220, 160, 60))
                            .strong(),
                    );
                });
                ui.add_space(6.0);
                let btn = egui::Button::new(
                    RichText::new(t.btn_install_tools)
                        .size(17.0)
                        .strong(),
                )
                .min_size(egui::vec2(ui.available_width(), 46.0))
                .fill(Color32::from_rgb(0x1a, 0x66, 0x99));

                if ui.add(btn).clicked() {
                    self.log_str("Iniciando instalación de Herramientas de Modding...");
                    if let Ok(mut slot) = self.tools_progress.lock() {
                        *slot = Some((0.05, "Iniciando instalación...".into()));
                    }
                    crate::tools::install_tools_background(
                        Arc::clone(&self.log),
                        Arc::clone(&self.tools_progress),
                    );
                }
            }
            crate::tools::ToolsStatus::Installing { progress, msg } => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.tools_status_label).weak());
                    ui.label(
                        RichText::new(t.tools_status_installing)
                            .color(Color32::from_rgb(100, 180, 255))
                            .strong(),
                    );
                });
                ui.add_space(6.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(msg)
                        .animate(true),
                );
                ui.ctx().request_repaint();
            }
            crate::tools::ToolsStatus::Installed => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.tools_status_label).weak());
                    ui.label(
                        RichText::new(t.tools_status_installed)
                            .color(Color32::from_rgb(100, 220, 100))
                            .strong(),
                    );
                    ui.label(
                        RichText::new(t.tools_status_paths)
                            .weak()
                            .size(11.0),
                    );
                });
                ui.add_space(8.0);

                ui.label(RichText::new(t.tools_available_heading).size(13.0).strong());
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(t.btn_open_decompiler)
                                    .size(14.0)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(0x24, 0x54, 0x7a)),
                        )
                        .clicked()
                    {
                        self.show_decompiler = true;
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(t.btn_gen_stubs)
                                    .size(14.0)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(0x22, 0x6e, 0x42)),
                        )
                        .clicked()
                    {
                        self.action_generate_stubs();
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(t.btn_open_tools_dir)
                                    .size(14.0),
                            ),
                        )
                        .clicked()
                    {
                        let tdir = crate::tools::tools_dir();
                        let _ = std::fs::create_dir_all(&tdir);
                        crate::util::open_folder(&tdir);
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                let btn_uninstall = egui::Button::new(
                    RichText::new(t.btn_uninstall_tools)
                        .size(14.0)
                        .strong()
                        .color(Color32::from_rgb(255, 200, 200)),
                )
                .min_size(egui::vec2(ui.available_width(), 38.0))
                .fill(Color32::from_rgb(0x6e, 0x22, 0x22));

                if ui.add(btn_uninstall).clicked() {
                    self.log_str("Desinstalando suite de herramientas...");
                    match crate::tools::uninstall_tools(Arc::clone(&self.log)) {
                        Ok(()) => {
                            self.log_str("[SDK] Desinstalación completada con éxito.");
                        }
                        Err(e) => {
                            self.log_str(&format!("[SDK ERROR] {e}"));
                            crate::dialog::msgbox("Fallo de desinstalación", &e);
                        }
                    }
                }
            }
        }
    }

    fn action_generate_stubs(&mut self) {
        let t = self.language.texts();
        self.log_str("[STUBS] Iniciando generador de Stubs Haxe (.hx)...");
        let Some(hl_file) = rfd::FileDialog::new()
            .set_title(t.select_hl_dialog_title)
            .add_filter("HashLink Bytecode (*.hl, *.dat)", &["hl", "dat"])
            .pick_file()
        else {
            self.log_str("[STUBS] Operación cancelada: no se eligió archivo .hl.");
            return;
        };

        let file_stem = hl_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("game");

        let default_dir = crate::tools::tools_dir().join("stubs").join(file_stem);

        let out_dir = match rfd::FileDialog::new()
            .set_title(t.pick_stubs_out_dir)
            .set_directory(&crate::tools::tools_dir().join("stubs"))
            .pick_folder()
        {
            Some(d) => d,
            None => default_dir,
        };

        self.log_str(&format!(
            "[STUBS] Analizando {} y generando stubs en {}...",
            hl_file.display(),
            out_dir.display()
        ));

        match crate::stubs::generate_all_stubs(&hl_file, &out_dir) {
            Ok(rep) => {
                let msg = format!(
                    "¡Stubs Haxe generados con éxito!\n\nBytecode: {}\nClases: {}\nArchivos .hx: {}\nCarpeta: {}",
                    hl_file.display(),
                    rep.classes_count,
                    rep.files_written,
                    rep.out_dir.display()
                );
                self.log_str(&format!(
                    "[STUBS] Completado: {} clases, {} archivos .hx escritos en {}",
                    rep.classes_count,
                    rep.files_written,
                    rep.out_dir.display()
                ));
                crate::dialog::msgbox("Stubs Haxe", &msg);
                crate::util::open_folder(&rep.out_dir);
            }
            Err(e) => {
                self.log_str(&format!("[STUBS ERROR] {e}"));
                crate::dialog::msgbox("Error", &e);
            }
        }
    }

    fn log_panel(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();
        egui::CollapsingHeader::new(RichText::new(t.log_header).weak())
            .default_open(true)
            .show(ui, |ui| {
                let lines: Vec<String> = self
                    .log
                    .lock()
                    .map(|v| v.clone())
                    .unwrap_or_default();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(170.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &lines {
                            ui.monospace(line.clone());
                        }
                        if lines.is_empty() {
                            ui.label(RichText::new(t.log_no_activity).weak());
                        }
                    });
            });
        if ui.small_button(t.btn_clear_log).clicked() {
            if let Ok(mut v) = self.log.lock() {
                v.clear();
            }
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        let t = self.language.texts();

        // 1) Zona de selección de idioma
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.lang_label).strong().size(13.0));

            let langs = [
                (Language::English, "English"),
                (Language::Spanish, "Español"),
                (Language::Russian, "Русский"),
                (Language::Portuguese, "Português"),
            ];

            for (l, name) in langs {
                let is_sel = self.language == l;
                let btn = egui::Button::new(
                    RichText::new(name)
                        .size(12.0)
                        .strong()
                        .color(if is_sel {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(180, 180, 190)
                        }),
                )
                .fill(if is_sel {
                    Color32::from_rgb(0x23, 0x6e, 0xaa)
                } else {
                    Color32::from_rgb(0x28, 0x2b, 0x33)
                });

                if ui.add(btn).clicked() {
                    self.language = l;
                    self.decompiler.lang = l;
                }
            }
        });

        ui.add_space(3.0);

        // 2) Texto interactivo hacia el GitHub del creador (https://github.com/RedexDev)
        ui.horizontal(|ui| {
            let link_text = RichText::new(format!("⭐ {}", t.visit_github))
                .size(13.5)
                .strong()
                .underline()
                .color(Color32::from_rgb(115, 195, 255));
            let resp = ui
                .hyperlink_to(link_text, "https://github.com/RedexDev")
                .on_hover_text("https://github.com/RedexDev");
            if resp.clicked() {
                ui.ctx().open_url(egui::OpenUrl {
                    url: "https://github.com/RedexDev".to_string(),
                    new_tab: true,
                });
            }
        });

        ui.add_space(2.0);

        // 3) Nota de especificación del motor
        ui.label(
            RichText::new(t.footer_engine)
                .weak()
                .size(11.0),
        );
    }
}

fn pb_str(p: &Option<std::path::PathBuf>) -> String {
    p.as_ref()
        .map(|x| x.display().to_string())
        .unwrap_or_else(|| "-".into())
}

fn pick_folder() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Seleccionar carpeta")
        .pick_folder()
}

fn save_file(name: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Guardar paquete .hlmod como…")
        .set_file_name(name)
        .add_filter("HashLoader mod (*.hlmod)", &["hlmod"])
        .save_file()
}
