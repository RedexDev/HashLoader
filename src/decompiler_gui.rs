use std::path::PathBuf;
use std::sync::Arc;
use egui::{Color32, RichText, ScrollArea, Ui};
use hlbc::types::Type;
use hlbc::Bytecode;

pub struct DecompilerState {
    pub file_path: Option<PathBuf>,
    pub bytecode: Option<Arc<Bytecode>>,
    pub search_class: String,
    pub selected_class_idx: usize,
    pub selected_method_idx: usize,
    pub active_tab: usize, // 0: Métodos, 1: Campos, 2: Stubs Haxe, 3: Strings
    pub search_string: String,
    pub status_msg: Option<String>,
    pub lang: crate::i18n::Language,
}

impl Default for DecompilerState {
    fn default() -> Self {
        Self {
            file_path: None,
            bytecode: None,
            search_class: String::new(),
            selected_class_idx: 0,
            selected_method_idx: 0,
            active_tab: 0,
            search_string: String::new(),
            status_msg: None,
            lang: crate::i18n::Language::English,
        }
    }
}

impl DecompilerState {
    pub fn open_file(&mut self, path: PathBuf) {
        match Bytecode::from_file(&path) {
            Ok(bc) => {
                self.file_path = Some(path);
                self.bytecode = Some(Arc::new(bc));
                self.selected_class_idx = 0;
                self.selected_method_idx = 0;
                self.status_msg = Some(match self.lang {
                    crate::i18n::Language::English => "Bytecode loaded successfully.".into(),
                    crate::i18n::Language::Spanish => "Bytecode cargado con éxito.".into(),
                    crate::i18n::Language::Russian => "Байт-код успешно загружен.".into(),
                    crate::i18n::Language::Portuguese => "Bytecode carregado com sucesso.".into(),
                });
            }
            Err(e) => {
                self.status_msg = Some(format!("Error: {e}"));
            }
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let t = self.lang.texts();
        ui.horizontal(|ui| {
            if ui.button(RichText::new(t.btn_open_hl).strong()).clicked() {
                if let Some(f) = rfd::FileDialog::new()
                    .set_title(t.select_hl_dialog_title)
                    .add_filter("HashLink Bytecode (*.hl, *.dat)", &["hl", "dat"])
                    .pick_file()
                {
                    self.open_file(f);
                }
            }

            if let Some(path) = &self.file_path {
                ui.label(RichText::new(path.display().to_string()).strong());
            } else {
                ui.label(RichText::new(t.no_file_open).weak());
            }

            if let Some(_bc) = &self.bytecode {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(RichText::new(t.btn_export_all_stubs).color(Color32::from_rgb(100, 220, 100)).strong()).clicked() {
                        if let Some(out) = rfd::FileDialog::new().set_title(t.pick_stubs_out_dir).pick_folder() {
                            if let Some(src) = &self.file_path {
                                match crate::stubs::generate_all_stubs(src, &out) {
                                    Ok(rep) => {
                                        self.status_msg = Some(t.stubs_exported_msg.replace("{}", &rep.files_written.to_string()) + &format!(" ({})", rep.out_dir.display()));
                                        crate::util::open_folder(&rep.out_dir);
                                    }
                                    Err(e) => self.status_msg = Some(format!("Error: {e}")),
                                }
                            }
                        }
                    }
                });
            }
        });

        ui.separator();

        let Some(bc) = self.bytecode.clone() else {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading(RichText::new(t.decompiler_welcome_heading).size(20.0).weak());
                ui.label(t.decompiler_welcome_desc);
                ui.add_space(20.0);
            });
            return;
        };

        // Barra informativa de estadísticas
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("HL v{}", bc.version)).strong().color(Color32::from_rgb(160, 200, 255)));
            ui.label("·");
            ui.label(format!("{}: {}", t.stat_types, bc.types.len()));
            ui.label("·");
            ui.label(format!("{}: {}", t.stat_functions, bc.functions.len()));
            ui.label("·");
            ui.label(format!("{}: {}", t.stat_strings, bc.strings.len()));
            if let Some(msg) = &self.status_msg {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(msg).color(Color32::LIGHT_GREEN).size(12.0));
                });
            }
        });

        ui.separator();

        // Lista de clases filtrada
        let filter = self.search_class.to_lowercase();
        let mut class_types: Vec<(usize, &hlbc::types::TypeObj)> = Vec::new();
        for (idx, t_item) in bc.types.iter().enumerate() {
            if let Type::Obj(obj) = t_item {
                let name = &bc[obj.name];
                if filter.is_empty() || name.to_lowercase().contains(&filter) {
                    class_types.push((idx, obj));
                }
            }
        }

        // Layout de dos columnas
        ui.columns(2, |cols| {
            // Columna 1: Lista de Clases y Búsqueda
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(t.search_class_label);
                    ui.text_edit_singleline(&mut self.search_class);
                    if !self.search_class.is_empty() && ui.button("❌").clicked() {
                        self.search_class.clear();
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new(format!("{} {}", class_types.len(), t.matching_classes)).weak().size(11.0));

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(420.0)
                    .show(ui, |ui| {
                        for (i, (_t_idx, obj)) in class_types.iter().enumerate() {
                            let name = &bc[obj.name];
                            let is_selected = self.selected_class_idx == i;
                            let label = format!("{} ({}f, {}m)", name.as_str(), obj.fields.len(), obj.protos.len());
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_class_idx = i;
                                self.selected_method_idx = 0;
                            }
                        }
                    });
            });

            // Columna 2: Detalles de la clase seleccionada
            cols[1].vertical(|ui| {
                let selected_obj = class_types.get(self.selected_class_idx).map(|(_, o)| *o);
                if let Some(obj) = selected_obj {
                    let class_name = &bc[obj.name];
                    ui.heading(RichText::new(class_name.as_str()).size(18.0).color(Color32::from_rgb(250, 204, 21)));

                    // Tabs
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.active_tab == 0, format!("{} ({})", t.tab_methods, obj.protos.len())).clicked() {
                            self.active_tab = 0;
                        }
                        if ui.selectable_label(self.active_tab == 1, format!("{} ({})", t.tab_fields, obj.fields.len())).clicked() {
                            self.active_tab = 1;
                        }
                        if ui.selectable_label(self.active_tab == 2, t.tab_stubs).clicked() {
                            self.active_tab = 2;
                        }
                        if ui.selectable_label(self.active_tab == 3, format!("{} ({})", t.tab_strings, bc.strings.len())).clicked() {
                            self.active_tab = 3;
                        }
                    });
                    ui.separator();

                    match self.active_tab {
                        0 => self.render_methods_tab(ui, &bc, obj),
                        1 => self.render_fields_tab(ui, &bc, obj),
                        2 => self.render_stubs_tab(ui, &bc, obj),
                        3 => self.render_strings_tab(ui, &bc),
                        _ => {}
                    }
                } else {
                    let hint = match self.lang {
                        crate::i18n::Language::English => "Select a class on the left to view its members.",
                        crate::i18n::Language::Spanish => "Selecciona una clase a la izquierda para ver sus miembros.",
                        crate::i18n::Language::Russian => "Выберите класс слева для просмотра его членов.",
                        crate::i18n::Language::Portuguese => "Selecione uma classe à esquerda para ver seus membros.",
                    };
                    ui.label(hint);
                }
            });
        });
    }

    fn render_methods_tab(&mut self, ui: &mut Ui, bc: &Bytecode, obj: &hlbc::types::TypeObj) {
        if obj.protos.is_empty() {
            let hint = match self.lang {
                crate::i18n::Language::English => "(This class has no methods of its own)",
                crate::i18n::Language::Spanish => "(Esta clase no tiene métodos propios)",
                crate::i18n::Language::Russian => "(У этого класса нет собственных методов)",
                crate::i18n::Language::Portuguese => "(Esta classe não possui métodos próprios)",
            };
            ui.label(RichText::new(hint).weak());
            return;
        }

        ui.horizontal(|ui| {
            // Lista de métodos
            ui.allocate_ui_with_layout(egui::vec2(180.0, 360.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ScrollArea::vertical().auto_shrink([false, false]).max_height(350.0).show(ui, |ui| {
                    for (m_idx, proto) in obj.protos.iter().enumerate() {
                        let mname = &bc[proto.name];
                        let is_sel = self.selected_method_idx == m_idx;
                        if ui.selectable_label(is_sel, mname.as_str()).clicked() {
                            self.selected_method_idx = m_idx;
                        }
                    }
                });
            });

            ui.separator();

            // Vista de descompilación / opcodes del método seleccionado
            ui.vertical(|ui| {
                if let Some(proto) = obj.protos.get(self.selected_method_idx) {
                    let mname = &bc[proto.name];
                    ui.label(RichText::new(format!("function {}", mname.as_str())).strong());

                    if let Some(func) = bc.functions.get(proto.findex.0) {
                        ScrollArea::vertical().auto_shrink([false, false]).max_height(320.0).show(ui, |ui| {
                            // Descompilación del método
                            let dec = hlbc_decompiler::decompile_function(bc, func);
                            let dec_lbl = match self.lang {
                                crate::i18n::Language::English => "Decompiled structure:",
                                crate::i18n::Language::Spanish => "Estructura descompilada:",
                                crate::i18n::Language::Russian => "Декомпилированная структура:",
                                crate::i18n::Language::Portuguese => "Estrutura descompilada:",
                            };
                            ui.label(RichText::new(dec_lbl).weak().size(11.0));
                            ui.add(
                                egui::TextEdit::multiline(&mut format!("{:#?}", dec))
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10)
                            );

                            ui.add_space(6.0);
                            let op_lbl = match self.lang {
                                crate::i18n::Language::English => "Bytecode opcodes",
                                crate::i18n::Language::Spanish => "Opcodes de bytecode",
                                crate::i18n::Language::Russian => "Опкоды байт-кода",
                                crate::i18n::Language::Portuguese => "Opcodes de bytecode",
                            };
                            ui.collapsing(format!("{} ({})", op_lbl, func.ops.len()), |ui| {
                                for (op_idx, op) in func.ops.iter().enumerate() {
                                    ui.monospace(format!("{:04}: {:?}", op_idx, op));
                                }
                            });
                        });
                    }
                }
            });
        });
    }

    fn render_fields_tab(&mut self, ui: &mut Ui, bc: &Bytecode, obj: &hlbc::types::TypeObj) {
        if obj.fields.is_empty() {
            let hint = match self.lang {
                crate::i18n::Language::English => "(This class has no data fields)",
                crate::i18n::Language::Spanish => "(Esta clase no tiene campos de datos)",
                crate::i18n::Language::Russian => "(У этого класса нет полей данных)",
                crate::i18n::Language::Portuguese => "(Esta classe não possui campos de dados)",
            };
            ui.label(RichText::new(hint).weak());
            return;
        }

        ScrollArea::vertical().auto_shrink([false, false]).max_height(350.0).show(ui, |ui| {
            for (f_idx, field) in obj.fields.iter().enumerate() {
                let fname = &bc[field.name];
                let ftype = crate::stubs::hl_type_to_haxe(bc, &bc[field.t]);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}.", f_idx)).weak());
                    ui.label(RichText::new(fname.as_str()).strong());
                    ui.label(":");
                    ui.monospace(ftype);
                });
            }
        });
    }

    fn render_stubs_tab(&mut self, ui: &mut Ui, bc: &Bytecode, obj: &hlbc::types::TypeObj) {
        let t = self.lang.texts();
        if let Some((_, _, code_txt)) = crate::stubs::generate_class_stub(bc, obj) {
            ui.horizontal(|ui| {
                if ui.button(t.btn_copy_haxe_def).clicked() {
                    ui.ctx().copy_text(code_txt.clone());
                    self.status_msg = Some(match self.lang {
                        crate::i18n::Language::English => "Definition copied to clipboard.".into(),
                        crate::i18n::Language::Spanish => "Definición copiada al portapapeles.".into(),
                        crate::i18n::Language::Russian => "Определение скопировано в буфер обмена.".into(),
                        crate::i18n::Language::Portuguese => "Definição copiada para a área de transferência.".into(),
                    });
                }
            });
            ui.add_space(4.0);
            ScrollArea::vertical().auto_shrink([false, false]).max_height(330.0).show(ui, |ui| {
                let mut display_text = code_txt.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut display_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                );
            });
        } else {
            let hint = match self.lang {
                crate::i18n::Language::English => "This class is an internal special type and does not generate a public stub.",
                crate::i18n::Language::Spanish => "Esta clase es un tipo interno especial y no genera stub público.",
                crate::i18n::Language::Russian => "Этот класс является специальным внутренним типом и не создает публичных заглушек.",
                crate::i18n::Language::Portuguese => "Esta classe é um tipo interno especial e não gera stub público.",
            };
            ui.label(hint);
        }
    }

    fn render_strings_tab(&mut self, ui: &mut Ui, bc: &Bytecode) {
        let t = self.lang.texts();
        ui.horizontal(|ui| {
            ui.label(t.search_string_label);
            ui.text_edit_singleline(&mut self.search_string);
            if !self.search_string.is_empty() && ui.button("❌").clicked() {
                self.search_string.clear();
            }
        });

        let sfilter = self.search_string.to_lowercase();
        ScrollArea::vertical().auto_shrink([false, false]).max_height(320.0).show(ui, |ui| {
            for (idx, s) in bc.strings.iter().enumerate() {
                if sfilter.is_empty() || s.to_lowercase().contains(&sfilter) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("[{}]", idx)).weak());
                        ui.label(s.as_str());
                    });
                }
            }
        });
    }
}
