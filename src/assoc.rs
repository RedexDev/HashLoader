use std::path::PathBuf;

use winreg::enums::*;
use winreg::RegKey;

pub const PROG_ID: &str = "HashLoader.hlmodPackage";
pub const DOT_HLMOD: &str = ".hlmod";
/// HKCU\Software\Classes\Directory\shell\HashLoaderPack — verbo de carpeta *(dev)*
pub const PACK_VERB_KEY: &str = r"Directory\shell\HashLoaderPack";

/// %LOCALAPPDATA%\HashLoader
pub fn app_data_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    local.join("HashLoader")
}

const ICON_FILE: &str = "hlmod.ico";

fn icon_path() -> PathBuf {
    app_data_dir().join(ICON_FILE)
}

fn exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssocStatus {
    NotInstalled,
    Installed,
    InstalledForeign,
}

/// HKCU\Software\Classes — base correcta para asociaciones por usuario.
fn classes_key() -> Result<RegKey, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey(r"Software\Classes")
        .map_err(|e| e.to_string())?;
    Ok(key)
}

pub fn status() -> AssocStatus {
    let Ok(classes) = classes_key() else {
        return AssocStatus::NotInstalled;
    };
    let Ok(key) = classes.open_subkey_with_flags(DOT_HLMOD, KEY_READ) else {
        return AssocStatus::NotInstalled;
    };
    let Ok(read) = key.get_value::<String, _>("") else {
        return AssocStatus::NotInstalled;
    };
    if read == PROG_ID {
        AssocStatus::Installed
    } else {
        AssocStatus::InstalledForeign
    }
}

/// Garantiza los iconos y datos de soporte en %LOCALAPPDATA%\HashLoader.
pub fn ensure_app_data() -> Result<(), String> {
    let dir = app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(ICON_FILE), crate::assets::HLMOD_ICO).map_err(|e| e.to_string())?;
    // marca de app actual (por si el toolkit se mueve)
    if let Some(exe) = exe_path() {
        let _ = std::fs::write(dir.join("toolkit.path"), exe.display().to_string());
    }
    Ok(())
}

fn exe_quoted() -> Option<String> {
    exe_path().map(|p| format!("\"{}\"", p.display()))
}

fn exe_quoted_fallback() -> String {
    // Si el exe actual no existe (caso raro), usa el path guardado.
    let stored = app_data_dir().join("toolkit.path");
    if stored.exists() {
        if let Ok(text) = std::fs::read_to_string(&stored) {
            let p = PathBuf::from(text.trim());
            if p.exists() {
                return format!("\"{}\"", p.display());
            }
        }
    }
    let exe = exe_path().unwrap_or_else(|| "HashToolkit.exe".into());
    format!("\"{}\"", exe.display())
}

// ---------------------------------------------------------------------------
// Gestores de archivos (WinRAR / 7-Zip) y percepción de "comprimido"
// ---------------------------------------------------------------------------

fn find_winrar() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(k) = hklm.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\App Paths\WinRAR.exe") {
        if let Ok(v) = k.get_value::<String, _>("") {
            let p = PathBuf::from(v.trim_matches('"'));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
    let pfx86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| format!("{pf} (x86)"));
    for cand in [format!(r"{pf}\WinRAR\WinRAR.exe"), format!(r"{pfx86}\WinRAR\WinRAR.exe")] {
        let p = PathBuf::from(&cand);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn find_7zip() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(k) = hklm.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\App Paths\7zFM.exe") {
        if let Ok(v) = k.get_value::<String, _>("") {
            let p = PathBuf::from(v.trim_matches('"'));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
    for cand in [
        format!(r"{pf}\7-Zip\7zFM.exe"),
        format!(r"{pf} (x86)\7-Zip\7zFM.exe"),
    ] {
        let p = PathBuf::from(&cand);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// ¿Existe un ProgId instalado en el sistema (HKCU u HKLM)?
fn progid_exists(name: &str) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    hkcu.open_subkey(format!(r"Software\Classes\{name}")).is_ok()
        || hklm.open_subkey(format!(r"Software\Classes\{name}")).is_ok()
}

/// Añade un verbo de menú contextual que abre el paquete con una app externa.
fn add_archive_verb(
    prog: &RegKey,
    verb: &str,
    title: &str,
    app_exe: &str,
) -> Result<(), String> {
    let (v, _) = prog
        .create_subkey(format!("shell\\{verb}"))
        .map_err(|e| e.to_string())?;
    v.set_value("", &title.to_string())
        .map_err(|e| e.to_string())?;
    v.set_value("MUIVerb", &title.to_string())
        .map_err(|e| e.to_string())?;
    v.set_value("Icon", &format!("\"{app_exe}\",0"))
        .map_err(|e| e.to_string())?;
    let (c, _) = v
        .create_subkey("command")
        .map_err(|e| e.to_string())?;
    c.set_value("", &format!("\"{}\" \"%1\"", app_exe))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Registra la extensión .hlmod en HKCU\Software\Classes (soporte portátil, sin admin).
pub fn install() -> Result<(), String> {
    ensure_app_data()?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // limpieza de versiones antiguas que escribían en la raíz de HKCU (bug)
    if hkcu.open_subkey(DOT_HLMOD).is_ok() {
        let _ = hkcu.delete_subkey_all(DOT_HLMOD);
    }
    if hkcu.open_subkey(PROG_ID).is_ok() {
        let _ = hkcu.delete_subkey_all(PROG_ID);
    }

    let classes = classes_key()?;

    // .hlmod -> ProgId (+ percepción de archivo comprimido en Explorer)
    let (dotkey, _) = classes
        .create_subkey(DOT_HLMOD)
        .map_err(|e| e.to_string())?;
    dotkey
        .set_value("", &PROG_ID)
        .map_err(|e| e.to_string())?;
    let _ = dotkey.set_value("PerceivedType", &"compressed");
    let _ = dotkey.set_value("ContentType", &"application/zip");

    // ProgId completo
    let (key, _) = classes.create_subkey(PROG_ID).map_err(|e| e.to_string())?;
    key.set_value(
        "",
        &"HashLoader Mod Pack (.hlmod) — Modding Runtime",
    )
    .map_err(|e| e.to_string())?;

    // icono del formato (ruta absoluta ya expandida)
    let (icon, _) = key
        .create_subkey("DefaultIcon")
        .map_err(|e| e.to_string())?;
    icon.set_value("", &format!("\"{}\",0", icon_path().display()))
        .map_err(|e| e.to_string())?;

    // verbo abrir -> visor del toolkit
    let (open, _) = key
        .create_subkey("shell\\open")
        .map_err(|e| e.to_string())?;
    open.set_value("", &"Abrir inspector de HashLoader")
        .map_err(|e| e.to_string())?;
    let (cmd, _) = open
        .create_subkey("command")
        .map_err(|e| e.to_string())?;
    let exe_str = exe_quoted().unwrap_or_else(exe_quoted_fallback);
    cmd.set_value("", &format!("{} --info \"%1\"", exe_str))
        .map_err(|e| e.to_string())?;

    // verbos para explorar el paquete con gestores de archivos instalados
    if let Some(exe) = find_winrar() {
        add_archive_verb(&key, "WinRAR", "Abrir con WinRAR", &exe.to_string_lossy().as_ref())?;
    }
    if let Some(exe) = find_7zip() {
        add_archive_verb(&key, "7-Zip", "Abrir con 7-Zip", &exe.to_string_lossy().as_ref())?;
    }

    // verbo de desarrollador: clic derecho en carpeta de mod → empaquetar
    if let Some(exe_str) = exe_quoted() {
        let (p, _) = classes
            .create_subkey(PACK_VERB_KEY)
            .map_err(|e| e.to_string())?;
        p.set_value("", &"Empaquetar en .hlmod")
            .map_err(|e| e.to_string())?;
        let _ = p.set_value("MUIVerb", &"Empaquetar en .hlmod && HashLoader");
        let _ = p.set_value("Icon", &format!("{exe_str},0"));
        let (c, _) = p
            .create_subkey("command")
            .map_err(|e| e.to_string())?;
        c.set_value("", &format!("{} --pack \"%1\"", exe_str))
            .map_err(|e| e.to_string())?;
    }

    // Explorer FileExts: abrir siempre con este ProgId + limpiar UserChoice ajena
    let fe_path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.hlmod";
    if let Ok((fe, _)) = hkcu.create_subkey(fe_path) {
        if let Ok((op, _)) = fe.create_subkey("OpenWithProgIds") {
            // nuestra app + el visor de carpetas comprimidas nativo de Windows
            op.set_value(PROG_ID, &"".to_string())
                .map_err(|e| e.to_string())?;
            op.set_value("CompressedFolder", &"".to_string())
                .map_err(|e| e.to_string())?;
            // y los ProgId de WinRAR/7-Zip que existan (auto-degradan si no están)
            for pid in ["WinRAR", "WinRAR.ZIP"] {
                if progid_exists(pid) {
                    op.set_value(pid, &"".to_string())
                        .map_err(|e| e.to_string())?;
                    break;
                }
            }
            for pid in ["7-Zip.7z", "7-Zip.zip"] {
                if progid_exists(pid) {
                    op.set_value(pid, &"".to_string())
                        .map_err(|e| e.to_string())?;
                    break;
                }
            }
        }
        // si había una UserChoice que no es la nuestra, el garabato de
        // protección impide reescribirla: se borra para que Explorer vuelva a
        // evaluar la asociación de Software\Classes
        let own_uc = fe
            .open_subkey("UserChoice")
            .ok()
            .and_then(|uc| uc.get_value::<String, _>("ProgId").ok())
            .map(|v| v == PROG_ID)
            .unwrap_or(false);
        if own_uc {
            // la elección del usuario ya es la nuestra; nada que hacer
        } else if uc_exists(&fe) {
            let _ = hkcu.delete_subkey_all(format!(
                "{fe_path}\\UserChoice"
            ));
        }
    }

    notify_shell();
    Ok(())
}

fn uc_exists(fe: &RegKey) -> bool {
    fe.enum_keys()
        .filter_map(|k| k.ok())
        .any(|k| k.eq_ignore_ascii_case("UserChoice"))
}

/// Quita la asociación hecha por HashLoader si es la suya.
pub fn uninstall() -> Result<bool, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = classes_key()?;
    let mut had_ours = false;
    if let Ok(dotkey) = classes.open_subkey_with_flags(DOT_HLMOD, KEY_QUERY_VALUE) {
        if dotkey.get_value::<String, _>("").ok().as_deref() == Some(PROG_ID) {
            let _ = classes.delete_subkey_all(DOT_HLMOD);
            had_ours = true;
        }
    }
    if let Ok(key) = classes.open_subkey_with_flags(PROG_ID, KEY_QUERY_VALUE) {
        if key.get_value::<String, _>("").ok().is_some() {
            let _ = classes.delete_subkey_all(PROG_ID);
            had_ours = true;
        }
    }
    if had_ours {
        let fe_path = r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.hlmod";
        if let Ok(fe) = hkcu.open_subkey_with_flags(fe_path, KEY_ALL_ACCESS) {
            if let Ok(op) = fe.open_subkey("OpenWithProgIds") {
                let _ = op.delete_value(PROG_ID);
            }
        }
    }
    // verbo de carpeta propio; lo retiramos siempre que exista
    let _ = classes.delete_subkey_all(PACK_VERB_KEY);
    if had_ours {
        notify_shell();
    }
    Ok(had_ours)
}

fn notify_shell() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::Shell::SHChangeNotify(
            windows_sys::Win32::UI::Shell::SHCNE_ASSOCCHANGED as i32,
            windows_sys::Win32::UI::Shell::SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}
