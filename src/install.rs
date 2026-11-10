use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Resultado de la detección de un entorno HashLink/JIT.
#[derive(Debug, Clone, Default)]
pub struct Detection {
    pub root: PathBuf,
    pub main_hl: Option<PathBuf>,
    /// bin real donde viven los hdlls y assets (donde va la cache interna).
    pub bin_dir: Option<PathBuf>,
    pub libhl: Option<PathBuf>,
    pub hashlink: Option<PathBuf>,
    pub hdll_count: usize,
}

const RUNTIME_NAMES: &[&str] = &["libhl.dll", "libhl64.dll"];

/// Comprueba si un archivo es un bytecode HashLink de juego candidato (cualquier `*.hl`
/// o `hlboot.dat`, excluyendo los propios payloads de HashLoader).
pub fn is_candidate_game_hl(p: &Path) -> bool {
    if !p.is_file() {
        return false;
    }
    let fname = p
        .file_name()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if fname == "hashloader.hl" || fname == "payload.hl" {
        return false;
    }
    if fname == "hlboot.dat" {
        return true;
    }
    p.extension()
        .map(|x| x.to_string_lossy().eq_ignore_ascii_case("hl"))
        .unwrap_or(false)
}

/// Busca el bytecode principal dentro de una lista de carpetas:
/// 1) Si existe `main.hl` o `hlboot.dat`, les da prioridad.
/// 2) Si el archivo `.hl` tiene cualquier otro nombre (ej. `game.hl`, `terex.hl`, etc.),
///    selecciona el archivo `.hl` de mayor tamaño sin exigir que se llame `main.hl`
///    ni imponer un tamaño mínimo artificial.
pub fn find_game_hl(dirs: &[PathBuf]) -> Option<PathBuf> {
    if let Some(p) = find_first(dirs, &["main.hl", "hlboot.dat"]) {
        return Some(p);
    }
    let mut best: Option<(u64, PathBuf)> = None;
    for dir in dirs {
        let Ok(it) = dir.read_dir() else { continue };
        for e in it.flatten() {
            let p = e.path();
            if !is_candidate_game_hl(&p) {
                continue;
            }
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            if size == 0 {
                continue;
            }
            let better = match &best {
                None => true,
                Some((s, _)) => size > *s,
            };
            if better {
                best = Some((size, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Detecta la estructura de un programa HashLink (modo JIT base) a partir de una
/// carpeta o archivo `.hl` elegido por el usuario. La instalación se realiza directamente
/// dentro de la carpeta elegida (build, bin o raíz) sin exigir que el bytecode se llame `main.hl`.
pub fn detect(picked: &Path) -> Detection {
    let explicit_hl = if picked.is_file() && is_candidate_game_hl(picked) {
        Some(picked.to_path_buf())
    } else {
        None
    };

    let picked_dir = if picked.is_file() {
        picked.parent().unwrap_or(picked).to_path_buf()
    } else {
        picked.to_path_buf()
    };

    // La raíz de instalación es SIEMPRE la carpeta seleccionada por el usuario
    let root = picked_dir.clone();
    let has_hl_in_root = explicit_hl.is_some()
        || find_game_hl(std::slice::from_ref(&picked_dir)).is_some();
    let bin_dir = if has_hl_in_root {
        Some(picked_dir.clone())
    } else if picked_dir.join("bin").is_dir() {
        Some(picked_dir.join("bin"))
    } else {
        Some(picked_dir.clone())
    };

    let mut det = Detection {
        root: root.clone(),
        bin_dir,
        ..Default::default()
    };

    if det.bin_dir.is_none() {
        if root.join("bin").is_dir() {
            det.bin_dir = Some(root.join("bin"));
        } else {
            det.bin_dir = Some(root.clone());
        }
    }
    let bin = det.bin_dir.clone().unwrap();

    det.main_hl = explicit_hl.or_else(|| find_game_hl(&[bin.clone(), root.clone()]));

    for name in RUNTIME_NAMES {
        if let Some(p) = find_first(&[bin.clone(), root.clone(), root.join("hlvm")], &[name]) {
            det.libhl = Some(p);
            break;
        }
    }
    det.hashlink = find_companion_exe(&[bin.clone(), root.clone(), root.join("hlvm")]);
    let scan_dirs: &[PathBuf] = if bin == root {
        std::slice::from_ref(&bin)
    } else {
        &[bin.clone(), root.clone()]
    };
    for dir in scan_dirs {
        det.hdll_count += dir
            .read_dir()
            .map(|it| it.filter_map(|e| e.ok()).filter(|e| {
                let n = e.path().extension().map(|x| x.to_string_lossy().eq_ignore_ascii_case("hdll")).unwrap_or(false);
                n
            }).count())
            .unwrap_or(0);
    }

    det
}

/// Busca el ejecutable host JIT (`hashlink.exe`, `hl.exe` o el `.exe` acompañante
/// generado por Lime/OpenFL/HaxeFlixel junto a `hlboot.dat` / `libhl.dll`).
pub fn find_companion_exe(dirs: &[PathBuf]) -> Option<PathBuf> {
    if let Some(p) = find_first(dirs, &["hashlink.exe", "hl.exe"]) {
        return Some(p);
    }
    const EXCLUDED_EXES: &[&str] = &[
        "hashloader.exe",
        "hashtoolkit.exe",
        "unins000.exe",
        "uninstall.exe",
        "crashpad_handler.exe",
        "replacevistaicon.exe",
    ];
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let has_hl_env = dir.join("hlboot.dat").is_file()
            || dir.join("libhl.dll").is_file()
            || dir.join("libhl64.dll").is_file();
        if !has_hl_env {
            continue;
        }
        let Ok(it) = dir.read_dir() else { continue };
        for e in it.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let is_exe = p
                .extension()
                .map(|x| x.to_string_lossy().eq_ignore_ascii_case("exe"))
                .unwrap_or(false);
            if !is_exe {
                continue;
            }
            let fname = p
                .file_name()
                .map(|s| s.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if EXCLUDED_EXES.contains(&fname.as_str()) {
                continue;
            }
            return Some(p);
        }
    }
    None
}

fn find_first(dirs: &[PathBuf], names: &[&str]) -> Option<PathBuf> {
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        for name in names {
            let p = dir.join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Reporta lo que quedó instalado.
#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub root: String,
    pub loader_exe: String,
    pub config: String,
    pub hlmods: String,
    pub main_hl: String,
    pub libhl: Option<String>,
    pub hashlink: Option<String>,
    pub warnings: Vec<String>,
}

fn fmt(p: &Path) -> String {
    p.display().to_string()
}

fn escape_rel(base: &Path, p: &Path) -> String {
    if base == p {
        return ".".to_string();
    }
    p.strip_prefix(base)
        .map(|rel| {
            let s = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            if s.is_empty() {
                ".".to_string()
            } else {
                s
            }
        })
        .unwrap_or_else(|_| fmt(p))
        .replace('\\', "/")
}

const HLMODS_README: &str = r#"hlmods — carpeta de mods de HashLoader
=====================================

Coloca aquí tus paquetes `.hlmod` (se pueden filtrar/compartir con otros jugadores como archivos normales).

Formato del paquete (.hlmod, zip):
  hlmod.json            -> manifiesto (campos: id, name, version, author, main...)
  code/main.hl          -> (opcional) plugin HashLink compilado con Haxe
  override/             -> (opcional) archivos que reemplazan los del juego (assets, etc.)

Manifiesto mínimo (hlmod.json):
{
  "hlmod": 1,
  "id": "mi_mod",
  "name": "Mi Mod",
  "version": "0.1.0",
  "author": "Yo"
}

Para empaquetar una carpeta de mod en .hlmod usa la opción "Empaquetar" del HashLoader Setup Toolkit.

Orden de carga: menor "priority" primero; a igualdad, orden alfabético del id.

— HashLoader (Modding Runtime sobre HashLink/JIT)
"#;

/// Aplica el HashLoader a un programa/juego HashLink (JIT).
/// Reutilizable en cualquier instancia: crea HashLoader.exe + config + hlmods.
pub fn install(det: &Detection) -> Result<InstallReport, String> {
    // clonamos: la provisión de runtime actualiza la detección antes del toml
    let mut det = det.clone();
    let root = det.root.clone();
    let bin = det.bin_dir.clone().unwrap_or_else(|| root.clone());

    let main_hl = det
        .main_hl
        .clone()
        .ok_or_else(|| {
            "no se encontró ningún archivo de bytecode (*.hl / hlboot.dat). \
             Elige la carpeta del juego o selecciona directamente el archivo .hl."
                .to_string()
        })?;

    let mut warnings = Vec::new();

    // Runtime JIT: hashlink.exe + libhl.dll deben convivir junto al bytecode.
    // Provisión en cascada: (1) ya presente en el destino, (2) hlvm portable
    // junto al ejecutable del kit, (3) runtime embebido en el kit (app/hlvm/).
    let mut hashlink = det.hashlink.clone();
    let mut libhl = det.libhl.clone();

    // (2) hlvm portable junto al HashToolkit: se copia al bin para que la
    // instancia instalada sea 100% autónoma y tenga el host JIT con soporte de plugins.
    {
        let kit_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.to_path_buf()));
        if let Some(kit) = kit_dir {
            let dst_hl = bin.join("hashlink.exe");
            if let Some(hl) = find_first(&[kit.join("hlvm"), kit.clone()], &["hashlink.exe"]) {
                if hl != dst_hl && fs::copy(&hl, &dst_hl).is_ok() {
                    hashlink = Some(dst_hl);
                } else if hashlink.is_none() {
                    hashlink = Some(hl);
                }
            }
            let dst_ll = bin.join("libhl.dll");
            if let Some(ll) = find_first(&[kit.join("hlvm"), kit.clone()], RUNTIME_NAMES) {
                if ll != dst_ll && fs::copy(&ll, &dst_ll).is_ok() {
                    libhl = Some(dst_ll);
                } else if libhl.is_none() {
                    libhl = Some(ll);
                }
            }
        }
    }

    // (3) runtime JIT embebido en HashToolkit.exe (compilado con app/hlvm/)
    #[cfg(has_runtime)]
    {
        let dst_hl = bin.join("hashlink.exe");
        if fs::write(&dst_hl, crate::EMBED_HASHLINK).is_ok() {
            hashlink = Some(dst_hl);
        }
        let dst_ll = bin.join("libhl.dll");
        if fs::write(&dst_ll, crate::EMBED_LIBHL).is_ok() {
            libhl = Some(dst_ll);
        }
    }

    // actualizar la detección: build_toml / InstallReport usan estas rutas
    det.hashlink = hashlink.clone();
    det.libhl = libhl.clone();

    if hashlink.is_none() {
        return Err(format!(
            "No se encontró un runtime HashLink JIT (hashlink.exe / libhl.dll) en:\n  - {}\n  - {}\n  - junto al HashToolkit (carpeta 'hlvm')\n  - embebido en HashToolkit.exe (app/hlvm/ al compilar)\n\n\
             Copia 'hashlink.exe' y 'libhl.dll' (del HashLink 32/64 con el que corre el juego) \
             dentro de '{}' o en una carpeta 'hlvm' junto al HashToolkit, y reintenta.",
            fmt(&bin),
            fmt(&root),
            escape_rel(&root, &bin)
        ));
    }
    if libhl.is_none() {
        warnings.push(
            "libhl.dll no encontrada: HashLink la resolverá junto al host JIT."
                .into(),
        );
    }
    if det.hdll_count == 0 {
        warnings.push(
            "no hay *.hdll en la carpeta de ejecución: el juego podría no levantar sin sus librerías."
                .into(),
        );
    }

    // — copiar el propio toolkit como HashLoader.exe (entrypoint inyectado)
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let loader_exe = root.join("HashLoader.exe");
    fs::copy(&exe, &loader_exe)
        .map_err(|e| format!("no se pudo crear HashLoader.exe: {e}"))?;

    // hashloader.toml (config del loader)
    let config = root.join("hashloader.toml");
    let toml_src = build_toml(&det, &main_hl);
    fs::write(&config, toml_src).map_err(|e| e.to_string())?;

    // hlmods/ raíz del juego (carpeta simple, sin icono propio)
    let hlmods = root.join("hlmods");
    fs::create_dir_all(&hlmods).map_err(|e| e.to_string())?;
    fs::write(hlmods.join("README.txt"), HLMODS_README).map_err(|e| e.to_string())?;

    // — cache interna del loader
    let cache = bin.join(".hashloader");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;

    Ok(InstallReport {
        root: fmt(&root),
        loader_exe: fmt(&loader_exe),
        config: fmt(&config),
        hlmods: fmt(&hlmods),
        main_hl: escape_rel(&root, &main_hl),
        libhl: libhl.as_ref().map(|p| escape_rel(&root, p)),
        hashlink: hashlink.as_ref().map(|p| escape_rel(&root, p)),
        warnings,
    })
}

fn build_toml(det: &Detection, main_hl: &Path) -> String {
    let root = &det.root;
    let bin = det.bin_dir.clone().unwrap_or_else(|| root.clone());
    let hashlink = det
        .hashlink
        .as_ref()
        .map(|p| escape_rel(root, p))
        .unwrap_or_default();
    let libhl = det
        .libhl
        .as_ref()
        .map(|p| escape_rel(root, p))
        .unwrap_or_default();

    format!(
        r#"# HashLoader {version} — configuración del loader de esta instancia.
# Editable a mano. 'hashloader.toml' debe vivir junto a HashLoader.exe.

[game]
# Raíz de ejecución (hdlls, assets, bytecode)
bin = "{bin_rel}"
main = "{main_rel}"
# Argumentos extra que se pasan al juego
args = []

[runtime]
# Ruta al host JIT hashlink.exe (déjalo vacío para auto-detectar en bin/ o raíz).
hashlink = "{hashlink}"
libhl = "{libhl}"

[mods]
# Carpeta de paquetes .hlmod; se extraen en <bin>/.hashloader/mods
folder = "hlmods"

[run]
# Payload Haxe del Modding Runtime: normalmente va embebido en HashLoader.exe;
# aquí puedes apuntar a un "HashLoader.hl" propio (opcional).
payload = ""
"#,
        version = env!("CARGO_PKG_VERSION"),
        bin_rel = escape_rel(root, &bin),
        main_rel = escape_rel(root, main_hl),
        hashlink = hashlink,
        libhl = libhl,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_rel_same_path() {
        let p = Path::new("C:/Games/FNF/bin");
        assert_eq!(escape_rel(p, p), ".");
    }

    #[test]
    fn test_escape_rel_sub_path() {
        let base = Path::new("C:/Games/FNF/bin");
        let sub = Path::new("C:/Games/FNF/bin/main.hl");
        assert_eq!(escape_rel(base, sub), "main.hl");
    }

    #[test]
    fn test_detect_preserves_picked_as_root() {
        let temp = std::env::temp_dir().join(format!("test_hl_detect_{}", std::process::id()));
        let bin = temp.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("main.hl"), b"fake_hl").unwrap();

        let det = detect(&bin);
        assert_eq!(det.root, bin, "root debe ser exactamente la carpeta elegida (bin)");
        assert_eq!(det.bin_dir, Some(bin.clone()), "bin_dir debe ser la carpeta elegida");
        assert_eq!(det.main_hl, Some(bin.join("main.hl")));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_custom_hl_name() {
        let temp = std::env::temp_dir().join(format!("test_hl_custom_{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("terex_game.hl"), b"fake_hl_bytecode").unwrap();

        let det = detect(&temp);
        assert_eq!(det.main_hl, Some(temp.join("terex_game.hl")));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_hlboot_with_companion_exe() {
        let temp = std::env::temp_dir().join(format!("test_hlboot_companion_{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("hlboot.dat"), b"fake_hlboot").unwrap();
        fs::write(temp.join("FlxBunnyMark.exe"), b"fake_exe").unwrap();
        fs::write(temp.join("libhl.dll"), b"fake_dll").unwrap();

        let det = detect(&temp);
        assert_eq!(det.main_hl, Some(temp.join("hlboot.dat")));
        assert_eq!(det.hashlink, Some(temp.join("FlxBunnyMark.exe")));
        assert_eq!(det.libhl, Some(temp.join("libhl.dll")));

        let _ = fs::remove_dir_all(&temp);
    }
}
