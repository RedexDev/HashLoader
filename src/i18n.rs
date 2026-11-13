#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
    Russian,
    Portuguese,
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

pub struct Texts {
    pub app_title: &'static str,
    pub app_subtitle: &'static str,

    // Card 1: Instalar
    pub card_install_title: &'static str,
    pub card_install_desc: &'static str,
    pub btn_install_folder: &'static str,
    pub btn_select_hl_file: &'static str,
    pub select_hl_dialog_title: &'static str,

    // Card 2: Asociación .hlmod
    pub card_assoc_title: &'static str,
    pub card_assoc_desc: &'static str,
    pub assoc_status_installed: &'static str,
    pub assoc_status_foreign: &'static str,
    pub assoc_status_not_installed: &'static str,
    pub btn_assoc_install: &'static str,
    pub btn_assoc_uninstall: &'static str,

    // Card 3: Empaquetar
    pub card_pack_title: &'static str,
    pub card_pack_desc: &'static str,
    pub btn_pack_folder: &'static str,

    // Card 4: Herramientas
    pub card_tools_title: &'static str,
    pub card_tools_desc: &'static str,
    pub tools_status_label: &'static str,
    pub tools_status_not_installed: &'static str,
    pub tools_status_installing: &'static str,
    pub tools_status_installed: &'static str,
    pub tools_status_paths: &'static str,
    pub btn_install_tools: &'static str,
    pub tools_available_heading: &'static str,
    pub btn_open_decompiler: &'static str,
    pub btn_gen_stubs: &'static str,
    pub btn_open_tools_dir: &'static str,
    pub btn_uninstall_tools: &'static str,

    // Log panel
    pub log_header: &'static str,
    pub log_no_activity: &'static str,
    pub btn_clear_log: &'static str,

    // Footer y zona de idiomas
    pub lang_label: &'static str,
    pub visit_github: &'static str,
    pub footer_engine: &'static str,

    // Descompilador
    pub btn_back_to_toolkit: &'static str,
    pub decompiler_heading: &'static str,
    pub btn_open_hl: &'static str,
    pub no_file_open: &'static str,
    pub btn_export_all_stubs: &'static str,
    pub decompiler_welcome_heading: &'static str,
    pub decompiler_welcome_desc: &'static str,
    pub stat_types: &'static str,
    pub stat_functions: &'static str,
    pub stat_strings: &'static str,
    pub search_class_label: &'static str,
    pub matching_classes: &'static str,
    pub tab_methods: &'static str,
    pub tab_fields: &'static str,
    pub tab_stubs: &'static str,
    pub tab_strings: &'static str,
    pub search_string_label: &'static str,
    pub btn_copy_haxe_def: &'static str,
    pub pick_stubs_out_dir: &'static str,
    pub stubs_exported_msg: &'static str,
}

pub const EN_TEXTS: Texts = Texts {
    app_title: "HashLoader — Setup Toolkit",
    app_subtitle: "HashLink/JIT Modding Runtime",

    card_install_title: "1 · Install HashLoader (main)",
    card_install_desc: "Applies the loader to any HashLink game/program in base JIT mode (any *.hl or hlboot.dat file, regardless of name). Creates HashLoader.exe + hashloader.toml + hlmods/ inside the build folder.",
    btn_install_folder: "Install HashLoader in game folder…",
    btn_select_hl_file: "Select .hl / hlboot.dat file directly…",
    select_hl_dialog_title: "Select HashLink bytecode (*.hl / hlboot.dat)",

    card_assoc_title: "2 · System .hlmod support (optional)",
    card_assoc_desc: "Format support: {}. When installed, Windows recognizes .hlmod packages with their icon and displays their info when opened.",
    assoc_status_installed: "installed",
    assoc_status_foreign: "registered by another application",
    assoc_status_not_installed: "not installed",
    btn_assoc_install: "Install .hlmod support",
    btn_assoc_uninstall: "Remove support",

    card_pack_title: "3 · Package mod (developers)",
    card_pack_desc: "Mod folder (hlmod.json + code/ + override/) → .hlmod package ready for hlmods/.",
    btn_pack_folder: "Package folder → .hlmod",

    card_tools_title: "4 · Modding & Reverse Engineering Tools (advanced)",
    card_tools_desc: "Integrated suite to build mods and inject code into HashLink games and programs without source code: Hooks SDK (HashHook.hx), Haxe stubs generator (.hx), mod templates (.hlmod), and bytecode visual inspector (.hl).",
    tools_status_label: "Status:",
    tools_status_not_installed: "Not installed",
    tools_status_installing: "Installing components…",
    tools_status_installed: "✔ Installed",
    tools_status_paths: "(tools/sdk, tools/stubs, tools/templates)",
    btn_install_tools: "Install Modding & RE tools…",
    tools_available_heading: "Available tools with graphical interface:",
    btn_open_decompiler: "🔍 Open HL Bytecode Decompiler / Inspector",
    btn_gen_stubs: "⚡ Generate Haxe Stubs (.hx)…",
    btn_open_tools_dir: "📁 Open tools folder (disk)",
    btn_uninstall_tools: "🗑️ Uninstall Modding & RE tools",

    log_header: "Log",
    log_no_activity: "(no activity)",
    btn_clear_log: "Clear log",

    lang_label: "Language:",
    visit_github: "Visit the Hash Loader creator's GitHub!",
    footer_engine: "Engine: HashLink JIT + hl.Api.loadPlugin (plugin modules) · asset override · priority sorting",

    btn_back_to_toolkit: "⬅ Back to Toolkit",
    decompiler_heading: "HL Bytecode Decompiler & Inspector",
    btn_open_hl: "📂 Open .hl file…",
    no_file_open: "No file opened",
    btn_export_all_stubs: "⚡ Export ALL Stubs (.hx)",
    decompiler_welcome_heading: "HashLink Bytecode Decompiler & Inspector",
    decompiler_welcome_desc: "Open a main.hl or .dat file from any game to inspect its classes, methods, and generate stubs.",
    stat_types: "Types",
    stat_functions: "Functions",
    stat_strings: "Strings",
    search_class_label: "Search class:",
    matching_classes: "matching classes",
    tab_methods: "Methods",
    tab_fields: "Fields",
    tab_stubs: "Haxe Stubs (.hx)",
    tab_strings: "Strings",
    search_string_label: "Search string:",
    btn_copy_haxe_def: "📋 Copy Haxe definition",
    pick_stubs_out_dir: "Choose output folder for stubs",
    stubs_exported_msg: "Exported {} .hx files to {}!",
};

pub const ES_TEXTS: Texts = Texts {
    app_title: "HashLoader — Setup Toolkit",
    app_subtitle: "Modding Runtime para HashLink/JIT",

    card_install_title: "1 · Instalar HashLoader (principal)",
    card_install_desc: "Aplica el loader a cualquier juego/programa HashLink en modo JIT base (cualquier archivo *.hl o hlboot.dat, sin importar su nombre). Crea HashLoader.exe + hashloader.toml + hlmods/ dentro de la carpeta de la build.",
    btn_install_folder: "Instalar HashLoader en carpeta del juego…",
    btn_select_hl_file: "Seleccionar archivo .hl / hlboot.dat directamente…",
    select_hl_dialog_title: "Seleccionar bytecode HashLink (*.hl / hlboot.dat)",

    card_assoc_title: "2 · Soporte .hlmod en el sistema (opcional)",
    card_assoc_desc: "Soporte del formato: {}. Al instalarlo, Windows reconoce los paquetes .hlmod con su icono y muestra su información al abrirlos.",
    assoc_status_installed: "instalado",
    assoc_status_foreign: "registrado por otra aplicación",
    assoc_status_not_installed: "no instalado",
    btn_assoc_install: "Instalar soporte .hlmod",
    btn_assoc_uninstall: "Quitar soporte",

    card_pack_title: "3 · Empaquetar mod (desarrolladores)",
    card_pack_desc: "Carpeta de mod (hlmod.json + code/ + override/) → paquete .hlmod listo para hlmods/.",
    btn_pack_folder: "Empaquetar carpeta → .hlmod",

    card_tools_title: "4 · Herramientas de Modding & Reverse Engineering (avanzado)",
    card_tools_desc: "Suite integrada para crear mods e inyectar código en juegos y programas HashLink sin disponer de su código fuente: SDK de Hooks (HashHook.hx), generador de stubs Haxe (.hx), plantillas de mod (.hlmod) e inspector gráfico de bytecode (.hl).",
    tools_status_label: "Estado:",
    tools_status_not_installed: "No instalado",
    tools_status_installing: "Instalando componentes…",
    tools_status_installed: "✔ Instalado",
    tools_status_paths: "(tools/sdk, tools/stubs, tools/templates)",
    btn_install_tools: "Instalar herramientas de Modding & RE…",
    tools_available_heading: "Herramientas disponibles con interfaz gráfica:",
    btn_open_decompiler: "🔍 Abrir Descompilador / Inspector HL",
    btn_gen_stubs: "⚡ Generar Stubs Haxe (.hx)…",
    btn_open_tools_dir: "📁 Abrir carpeta de herramientas (disco)",
    btn_uninstall_tools: "🗑️ Desinstalar herramientas de Modding & RE",

    log_header: "Registro",
    log_no_activity: "(sin actividad)",
    btn_clear_log: "Limpiar registro",

    lang_label: "Idioma:",
    visit_github: "¡Visita el GitHub del creador de Hash Loader!",
    footer_engine: "Motor: HashLink JIT + hl.Api.loadPlugin (módulos plugin) · override de assets · orden por priority",

    btn_back_to_toolkit: "⬅ Volver al Toolkit",
    decompiler_heading: "Descompilador & Inspector de Bytecode HL",
    btn_open_hl: "📂 Abrir archivo .hl…",
    no_file_open: "Ningún archivo abierto",
    btn_export_all_stubs: "⚡ Exportar TODOS los Stubs (.hx)",
    decompiler_welcome_heading: "Descompilador e Inspector de Bytecode HashLink",
    decompiler_welcome_desc: "Abre un archivo main.hl o .dat de cualquier juego para inspeccionar sus clases, métodos y generar stubs.",
    stat_types: "Tipos",
    stat_functions: "Funciones",
    stat_strings: "Strings",
    search_class_label: "Buscar clase:",
    matching_classes: "clases coincidentes",
    tab_methods: "Métodos",
    tab_fields: "Campos",
    tab_stubs: "Stubs Haxe (.hx)",
    tab_strings: "Strings",
    search_string_label: "Buscar string:",
    btn_copy_haxe_def: "📋 Copiar definición Haxe",
    pick_stubs_out_dir: "Elegir carpeta destino para stubs",
    stubs_exported_msg: "¡Exportados {} archivos .hx en {}!",
};

pub const RU_TEXTS: Texts = Texts {
    app_title: "HashLoader — Setup Toolkit",
    app_subtitle: "Среда моддинга для HashLink/JIT",

    card_install_title: "1 · Установить HashLoader (основное)",
    card_install_desc: "Внедряет загрузчик в любую игру или программу на HashLink в режиме JIT (любой файл *.hl или hlboot.dat независимо от имени). Создаёт HashLoader.exe + hashloader.toml + hlmods/ внутри папки сборки.",
    btn_install_folder: "Установить HashLoader в папку игры…",
    btn_select_hl_file: "Выбрать файл .hl / hlboot.dat напрямую…",
    select_hl_dialog_title: "Выбрать байт-код HashLink (*.hl / hlboot.dat)",

    card_assoc_title: "2 · Поддержка .hlmod в системе (необязательно)",
    card_assoc_desc: "Статус формата: {}. При установке Windows свяжет файлы .hlmod со своим значком и покажет информацию при открытии.",
    assoc_status_installed: "установлено",
    assoc_status_foreign: "зарегистрировано другим приложением",
    assoc_status_not_installed: "не установлено",
    btn_assoc_install: "Установить поддержку .hlmod",
    btn_assoc_uninstall: "Удалить поддержку",

    card_pack_title: "3 · Упаковать мод (для разработчиков)",
    card_pack_desc: "Папка мода (hlmod.json + code/ + override/) → пакет .hlmod, готовый для папки hlmods/.",
    btn_pack_folder: "Упаковать папку → .hlmod",

    card_tools_title: "4 · Инструменты моддинга и реверс-инжиниринга (продвинутое)",
    card_tools_desc: "Встроенный набор для создания модов и внедрения кода в игры и программы HashLink без исходников: SDK хуков (HashHook.hx), генератор заглушек Haxe (.hx), шаблоны модов (.hlmod) и визуальный инспектор байт-кода (.hl).",
    tools_status_label: "Статус:",
    tools_status_not_installed: "Не установлено",
    tools_status_installing: "Установка компонентов…",
    tools_status_installed: "✔ Установлено",
    tools_status_paths: "(tools/sdk, tools/stubs, tools/templates)",
    btn_install_tools: "Установить инструменты моддинга и RE…",
    tools_available_heading: "Доступные графические инструменты:",
    btn_open_decompiler: "🔍 Открыть декомпилятор / инспектор HL",
    btn_gen_stubs: "⚡ Создать заглушки Haxe (.hx)…",
    btn_open_tools_dir: "📁 Открыть папку инструментов (на диске)",
    btn_uninstall_tools: "🗑️ Удалить инструменты моддинга и RE",

    log_header: "Журнал",
    log_no_activity: "(нет активности)",
    btn_clear_log: "Очистить журнал",

    lang_label: "Язык:",
    visit_github: "Посетите GitHub создателя Hash Loader!",
    footer_engine: "Движок: HashLink JIT + hl.Api.loadPlugin (модули плагинов) · замена ресурсов · сортировка по приоритету",

    btn_back_to_toolkit: "⬅ Назад в Toolkit",
    decompiler_heading: "Декомпилятор и инспектор байт-кода HL",
    btn_open_hl: "📂 Открыть файл .hl…",
    no_file_open: "Файл не открыт",
    btn_export_all_stubs: "⚡ Экспортировать ВСЕ заглушки (.hx)",
    decompiler_welcome_heading: "Декомпилятор и инспектор байт-кода HashLink",
    decompiler_welcome_desc: "Откройте файл main.hl или .dat любой игры для просмотра классов, методов и создания заглушек.",
    stat_types: "Типы",
    stat_functions: "Функции",
    stat_strings: "Строки",
    search_class_label: "Поиск класса:",
    matching_classes: "найдено классов",
    tab_methods: "Методы",
    tab_fields: "Поля",
    tab_stubs: "Заглушки Haxe (.hx)",
    tab_strings: "Строки",
    search_string_label: "Поиск строки:",
    btn_copy_haxe_def: "📋 Скопировать определение Haxe",
    pick_stubs_out_dir: "Выберите папку назначения для заглушек",
    stubs_exported_msg: "Экспортировано {} файлов .hx в {}!",
};

pub const PT_TEXTS: Texts = Texts {
    app_title: "HashLoader — Setup Toolkit",
    app_subtitle: "Ambiente de Modding para HashLink/JIT",

    card_install_title: "1 · Instalar HashLoader (principal)",
    card_install_desc: "Aplica o loader a qualquer jogo/programa HashLink em modo JIT base (qualquer arquivo *.hl ou hlboot.dat, independentemente do nome). Cria HashLoader.exe + hashloader.toml + hlmods/ dentro da pasta da build.",
    btn_install_folder: "Instalar HashLoader na pasta do jogo…",
    btn_select_hl_file: "Selecionar arquivo .hl / hlboot.dat diretamente…",
    select_hl_dialog_title: "Selecionar bytecode HashLink (*.hl / hlboot.dat)",

    card_assoc_title: "2 · Suporte a .hlmod no sistema (opcional)",
    card_assoc_desc: "Suporte ao formato: {}. Ao instalar, o Windows reconhece os pacotes .hlmod com seu ícone e exibe suas informações ao abri-los.",
    assoc_status_installed: "instalado",
    assoc_status_foreign: "registrado por outro aplicativo",
    assoc_status_not_installed: "não instalado",
    btn_assoc_install: "Instalar suporte a .hlmod",
    btn_assoc_uninstall: "Remover suporte",

    card_pack_title: "3 · Empacotar mod (desenvolvedores)",
    card_pack_desc: "Pasta do mod (hlmod.json + code/ + override/) → pacote .hlmod pronto para hlmods/.",
    btn_pack_folder: "Empacotar pasta → .hlmod",

    card_tools_title: "4 · Ferramentas de Modding & Engenharia Reversa (avançado)",
    card_tools_desc: "Suíte integrada para criar mods e injetar código em jogos e programas HashLink sem dispor do código-fonte: SDK de Hooks (HashHook.hx), gerador de stubs Haxe (.hx), modelos de mod (.hlmod) e inspetor visual de bytecode (.hl).",
    tools_status_label: "Estado:",
    tools_status_not_installed: "Não instalado",
    tools_status_installing: "Instalando componentes…",
    tools_status_installed: "✔ Instalado",
    tools_status_paths: "(tools/sdk, tools/stubs, tools/templates)",
    btn_install_tools: "Instalar ferramentas de Modding & RE…",
    tools_available_heading: "Ferramentas disponíveis com interface gráfica:",
    btn_open_decompiler: "🔍 Abrir Descompilador / Inspetor HL",
    btn_gen_stubs: "⚡ Gerar Stubs Haxe (.hx)…",
    btn_open_tools_dir: "📁 Abrir pasta de ferramentas (disco)",
    btn_uninstall_tools: "🗑️ Desinstalar ferramentas de Modding & RE",

    log_header: "Registro",
    log_no_activity: "(sem atividade)",
    btn_clear_log: "Limpar registro",

    lang_label: "Idioma:",
    visit_github: "Visite o GitHub do criador do Hash Loader!",
    footer_engine: "Motor: HashLink JIT + hl.Api.loadPlugin (módulos de plugin) · substituição de assets · ordenação por prioridade",

    btn_back_to_toolkit: "⬅ Voltar ao Toolkit",
    decompiler_heading: "Descompilador & Inspetor de Bytecode HL",
    btn_open_hl: "📂 Abrir arquivo .hl…",
    no_file_open: "Nenhum arquivo aberto",
    btn_export_all_stubs: "⚡ Exportar TODOS os Stubs (.hx)",
    decompiler_welcome_heading: "Descompilador e Inspetor de Bytecode HashLink",
    decompiler_welcome_desc: "Abra um arquivo main.hl ou .dat de qualquer jogo para inspecionar suas classes, métodos e gerar stubs.",
    stat_types: "Tipos",
    stat_functions: "Funções",
    stat_strings: "Strings",
    search_class_label: "Buscar classe:",
    matching_classes: "classes encontradas",
    tab_methods: "Métodos",
    tab_fields: "Campos",
    tab_stubs: "Stubs Haxe (.hx)",
    tab_strings: "Strings",
    search_string_label: "Buscar string:",
    btn_copy_haxe_def: "📋 Copiar definição Haxe",
    pick_stubs_out_dir: "Escolher pasta de destino para os stubs",
    stubs_exported_msg: "Exportados {} arquivos .hx para {}!",
};

impl Language {
    pub fn texts(&self) -> &'static Texts {
        match self {
            Self::English => &EN_TEXTS,
            Self::Spanish => &ES_TEXTS,
            Self::Russian => &RU_TEXTS,
            Self::Portuguese => &PT_TEXTS,
        }
    }
}

