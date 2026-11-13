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

#[cfg(windows)]
const RUNTIME_NAMES: &[&str] = &["libhl.dll", "libhl64.dll"];
#[cfg(not(windows))]
const RUNTIME_NAMES: &[&str] = &["libhl.so", "libhl.so.1", "libhl64.so"];

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

fn count_hdlls(dir: &Path) -> usize {
    dir.read_dir()
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_file()
                        && p.extension()
                            .map(|x| x.to_string_lossy().eq_ignore_ascii_case("hdll"))
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn should_skip_subdir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    matches!(
        name.to_ascii_lowercase().as_str(),
        "hlmods"
            | "originals"
            | "obj"
            | "haxe"
            | "src"
            | "source"
            | "target"
            | "node_modules"
            | "code"
            | "override"
    )
}

/// Busca recursivamente (hasta `max_depth` niveles) la subcarpeta donde vive realmente
/// la build ejecutable de HashLink (`bin/`, `build/`, `windows/`, etc.).
fn find_build_subdir(base: &Path, max_depth: usize) -> Option<PathBuf> {
    use std::collections::VecDeque;

    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((base.to_path_buf(), 0));

    let mut best: Option<(i64, PathBuf)> = None;

    while let Some((dir, depth)) = queue.pop_front() {
        if depth > 0 {
            if let Some(hl) = find_game_hl(std::slice::from_ref(&dir)) {
                let mut score: i64 = 1000 - (depth as i64 * 150);

                let hl_name = hl
                    .file_name()
                    .map(|s| s.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if hl_name == "hlboot.dat" || hl_name == "main.hl" {
                    score += 25;
                }
                if dir
                    .file_name()
                    .map(|s| s.to_string_lossy().eq_ignore_ascii_case("bin"))
                    .unwrap_or(false)
                {
                    score += 50;
                }
                if RUNTIME_NAMES.iter().any(|n| dir.join(n).is_file()) {
                    score += 1000;
                }
                if find_companion_exe(std::slice::from_ref(&dir)).is_some() {
                    score += 1000;
                }
                if count_hdlls(&dir) > 0 {
                    score += 500;
                }

                let is_better = match &best {
                    None => true,
                    Some((best_score, _)) => score > *best_score,
                };
                if is_better {
                    best = Some((score, dir.clone()));
                }
            }
        }

        if depth < max_depth {
            let Ok(it) = dir.read_dir() else { continue };
            let mut subdirs: Vec<PathBuf> = it
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .map(|n| !should_skip_subdir(&n.to_string_lossy()))
                            .unwrap_or(false)
                })
                .collect();
            subdirs.sort();
            for sub in subdirs {
                queue.push_back((sub, depth + 1));
            }
        }
    }

    best.map(|(_, p)| p)
}

/// Detecta la estructura de un programa HashLink (modo JIT base) a partir de una
/// carpeta o archivo `.hl` elegido por el usuario. La instalación (`HashLoader.exe`,
/// `hashloader.toml`, `hlmods/` y `.hashloader/`) se coloca siempre **dentro** de la
/// carpeta de la build ejecutable (ya se llame `bin` o cualquier otro nombre),
/// sin exigir que el bytecode se llame `main.hl`.
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

    // Si la carpeta elegida contiene directamente el bytecode (o el usuario eligió un archivo),
    // instalamos allí mismo. Si eligió una carpeta contenedora de proyecto/build, localizamos
    // la subcarpeta de la build ejecutable (sea `bin/` o cualquier otro nombre) para que
    // `HashLoader.exe`, `hashloader.toml` y `hlmods/` queden todos dentro de esa build.
    let build_dir = if explicit_hl.is_some()
        || find_game_hl(std::slice::from_ref(&picked_dir)).is_some()
    {
        picked_dir.clone()
    } else if let Some(sub) = find_build_subdir(&picked_dir, 3) {
        sub
    } else if picked_dir.join("bin").is_dir() {
        picked_dir.join("bin")
    } else {
        picked_dir.clone()
    };

    let root = build_dir.clone();
    let bin = build_dir.clone();

    let mut det = Detection {
        root,
        bin_dir: Some(bin.clone()),
        ..Default::default()
    };

    det.main_hl = explicit_hl.or_else(|| find_game_hl(std::slice::from_ref(&bin)));

    let mut runtime_dirs = vec![bin.clone()];
    if picked_dir != bin {
        runtime_dirs.push(picked_dir.clone());
        runtime_dirs.push(picked_dir.join("hlvm"));
    }
    runtime_dirs.push(bin.join("hlvm"));

    for name in RUNTIME_NAMES {
        if let Some(p) = find_first(&runtime_dirs, &[name]) {
            det.libhl = Some(p);
            break;
        }
    }
    det.hashlink = find_companion_exe(&runtime_dirs);
    det.hdll_count = count_hdlls(&bin);
    if det.hdll_count == 0 && picked_dir != bin {
        det.hdll_count = count_hdlls(&picked_dir);
    }

    det
}

/// Busca el ejecutable host JIT (`hashlink.exe`/`hl.exe` en Windows, `hashlink`/`hl`
/// en Unix, o el binario acompañante generado por Lime/OpenFL/HaxeFlixel junto a
/// `hlboot.dat` / `libhl`).
pub fn find_companion_exe(dirs: &[PathBuf]) -> Option<PathBuf> {
    #[cfg(windows)]
    let host_names = ["hashlink.exe", "hl.exe"];
    #[cfg(not(windows))]
    let host_names = ["hashlink", "hl"];

    if let Some(p) = find_first(dirs, &host_names) {
        return Some(p);
    }

    #[cfg(windows)]
    {
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
                || RUNTIME_NAMES.iter().any(|n| dir.join(n).is_file());
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

    #[cfg(not(windows))]
    {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;

        const EXCLUDED_BINS: &[&str] = &["hashloader", "hashtoolkit", "uninstall", "unins000"];
        for dir in dirs {
            if !dir.is_dir() {
                continue;
            }
            let has_hl_env = dir.join("hlboot.dat").is_file()
                || RUNTIME_NAMES.iter().any(|n| dir.join(n).is_file());
            if !has_hl_env {
                continue;
            }
            let Ok(it) = dir.read_dir() else { continue };
            for e in it.flatten() {
                let p = e.path();
                if !p.is_file() {
                    continue;
                }
                let fname = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if EXCLUDED_BINS.contains(&fname.as_str()) {
                    continue;
                }
                if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
                    let low = ext.to_ascii_lowercase();
                    if matches!(
                        low.as_str(),
                        "hdll"
                            | "so"
                            | "hl"
                            | "dat"
                            | "txt"
                            | "md"
                            | "json"
                            | "toml"
                            | "png"
                            | "jpg"
                            | "ogg"
                            | "wav"
                            | "ttf"
                            | "xml"
                    ) {
                        continue;
                    }
                }
                let is_exec = e
                    .metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                let is_elf = fs::File::open(&p)
                    .ok()
                    .and_then(|mut f| {
                        let mut magic = [0u8; 4];
                        f.read_exact(&mut magic).ok().map(|_| magic == *b"\x7fELF")
                    })
                    .unwrap_or(false);
                if is_exec && is_elf {
                    return Some(p);
                }
            }
        }
        None
    }
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
    #[cfg(windows)]
    let kit_host_names: &[&str] = &[crate::util::HOST_EXE_NAME];
    #[cfg(not(windows))]
    let kit_host_names: &[&str] = &[crate::util::HOST_EXE_NAME, "hl"];

    {
        let kit_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.to_path_buf()));
        if let Some(kit) = kit_dir {
            let kit_search = [kit.join("hlvm").join("linux"), kit.join("hlvm"), kit.clone()];
            let dst_hl = bin.join(crate::util::HOST_EXE_NAME);
            if let Some(hl) = find_first(&kit_search, kit_host_names) {
                if hl != dst_hl && fs::copy(&hl, &dst_hl).is_ok() {
                    crate::util::mark_executable(&dst_hl);
                    hashlink = Some(dst_hl);
                } else if hashlink.is_none() {
                    hashlink = Some(hl);
                }
            }
            let dst_ll = bin.join(crate::util::LIBHL_NAME);
            if let Some(ll) = find_first(&kit_search, RUNTIME_NAMES) {
                if ll != dst_ll && fs::copy(&ll, &dst_ll).is_ok() {
                    crate::util::mark_executable(&dst_ll);
                    libhl = Some(dst_ll);
                } else if libhl.is_none() {
                    libhl = Some(ll);
                }
            }
        }
    }

    // (3) runtime JIT embebido en HashToolkit (compilado con app/hlvm/ o sistema)
    #[cfg(has_runtime)]
    {
        let dst_hl = bin.join(crate::util::HOST_EXE_NAME);
        if fs::write(&dst_hl, crate::EMBED_HASHLINK).is_ok() {
            crate::util::mark_executable(&dst_hl);
            hashlink = Some(dst_hl);
        }
        let dst_ll = bin.join(crate::util::LIBHL_NAME);
        if fs::write(&dst_ll, crate::EMBED_LIBHL).is_ok() {
            crate::util::mark_executable(&dst_ll);
            libhl = Some(dst_ll);
        }
    }

    // (4) En Unix, fallback a HashLink instalado en el sistema (/usr/local/bin/hl, /usr/bin/hl)
    #[cfg(not(windows))]
    {
        if hashlink.is_none() {
            for cand in ["/usr/local/bin/hl", "/usr/bin/hl"] {
                let p = PathBuf::from(cand);
                if p.is_file() {
                    let dst_hl = bin.join(crate::util::HOST_EXE_NAME);
                    if fs::copy(&p, &dst_hl).is_ok() {
                        crate::util::mark_executable(&dst_hl);
                        hashlink = Some(dst_hl);
                    } else {
                        hashlink = Some(p);
                    }
                    break;
                }
            }
        }
        if libhl.is_none() {
            for cand in [
                "/usr/local/lib/libhl.so",
                "/usr/lib/libhl.so",
                "/usr/lib/x86_64-linux-gnu/libhl.so",
            ] {
                let p = PathBuf::from(cand);
                if p.is_file() {
                    let dst_ll = bin.join(crate::util::LIBHL_NAME);
                    if fs::copy(&p, &dst_ll).is_ok() {
                        crate::util::mark_executable(&dst_ll);
                        libhl = Some(dst_ll);
                    } else {
                        libhl = Some(p);
                    }
                    break;
                }
            }
        }
    }

    // actualizar la detección: build_toml / InstallReport usan estas rutas
    det.hashlink = hashlink.clone();
    det.libhl = libhl.clone();

    if hashlink.is_none() {
        return Err(format!(
            "No se encontró un runtime HashLink JIT ({host} / {lib}) en:\n  - {}\n  - {}\n  - junto al HashToolkit (carpeta 'hlvm')\n  - embebido en HashToolkit\n\n\
             Copia '{host}' y '{lib}' (del HashLink con el que corre el juego) \
             dentro de '{}' o en una carpeta 'hlvm' junto al HashToolkit, y reintenta.",
            fmt(&bin),
            fmt(&root),
            escape_rel(&root, &bin),
            host = crate::util::HOST_EXE_NAME,
            lib = crate::util::LIBHL_NAME,
        ));
    }
    if libhl.is_none() {
        warnings.push(format!(
            "{} no encontrada: HashLink la resolverá junto al host JIT.",
            crate::util::LIBHL_NAME
        ));
    }
    if det.hdll_count == 0 {
        warnings.push(
            "no hay *.hdll en la carpeta de ejecución: el juego podría no levantar sin sus librerías."
                .into(),
        );
    }

    // — copiar el propio toolkit como HashLoader(.exe) (entrypoint inyectado)
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let loader_exe = root.join(crate::util::LOADER_EXE_NAME);
    fs::copy(&exe, &loader_exe)
        .map_err(|e| format!("no se pudo crear {}: {e}", crate::util::LOADER_EXE_NAME))?;
    crate::util::mark_executable(&loader_exe);

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
# Editable a mano. 'hashloader.toml' debe vivir junto a {loader}.

[game]
# Raíz de ejecución (hdlls, assets, bytecode)
bin = "{bin_rel}"
main = "{main_rel}"
# Argumentos extra que se pasan al juego
args = []

[runtime]
# Ruta al host JIT {host} (déjalo vacío para auto-detectar en bin/ o raíz).
hashlink = "{hashlink}"
libhl = "{libhl}"

[mods]
# Carpeta de paquetes .hlmod; se extraen en <bin>/.hashloader/mods
folder = "hlmods"

[run]
# Payload Haxe del Modding Runtime: normalmente va embebido en {loader};
# aquí puedes apuntar a un "HashLoader.hl" propio (opcional).
payload = ""
"#,
        version = env!("CARGO_PKG_VERSION"),
        loader = crate::util::LOADER_EXE_NAME,
        host = crate::util::HOST_EXE_NAME,
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

    fn fake_libhl_name() -> &'static str {
        crate::util::LIBHL_NAME
    }

    fn create_fake_companion(dir: &Path, stem: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let p = dir.join(format!("{stem}.exe"));
            fs::write(&p, b"fake_exe").unwrap();
            p
        }
        #[cfg(not(windows))]
        {
            let p = dir.join(stem);
            fs::write(&p, b"\x7fELF_fake_bin").unwrap();
            crate::util::mark_executable(&p);
            p
        }
    }

    #[test]
    fn test_detect_hlboot_with_companion_exe() {
        let temp = std::env::temp_dir().join(format!("test_hlboot_companion_{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("hlboot.dat"), b"fake_hlboot").unwrap();
        let companion = create_fake_companion(&temp, "FlxBunnyMark");
        fs::write(temp.join(fake_libhl_name()), b"fake_lib").unwrap();

        let det = detect(&temp);
        assert_eq!(det.main_hl, Some(temp.join("hlboot.dat")));
        assert_eq!(det.hashlink, Some(companion));
        assert_eq!(det.libhl, Some(temp.join(fake_libhl_name())));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_enters_bin_subdir_as_root() {
        let temp = std::env::temp_dir().join(format!("test_hl_parent_bin_{}", std::process::id()));
        let bin = temp.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("main.hl"), b"fake_hl").unwrap();
        fs::write(bin.join(fake_libhl_name()), b"fake_lib").unwrap();

        let det = detect(&temp);
        assert_eq!(det.root, bin, "root debe entrar en bin/ para que exe y toml queden dentro");
        assert_eq!(det.bin_dir, Some(bin.clone()));
        assert_eq!(det.main_hl, Some(bin.join("main.hl")));

        let toml_out = build_toml(&det, &bin.join("main.hl"));
        assert!(toml_out.contains("bin = \".\""));
        assert!(toml_out.contains("main = \"main.hl\""));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_enters_custom_build_subdir_as_root() {
        let temp = std::env::temp_dir().join(format!("test_hl_custom_build_{}", std::process::id()));
        let build_dir = temp.join("win64_release");
        fs::create_dir_all(&build_dir).unwrap();
        fs::write(build_dir.join("hlboot.dat"), b"fake_hlboot").unwrap();
        let companion = create_fake_companion(&build_dir, "MyGame");
        fs::write(build_dir.join(fake_libhl_name()), b"fake_lib").unwrap();
        fs::write(build_dir.join("sdl.hdll"), b"fake_hdll").unwrap();

        let det = detect(&temp);
        assert_eq!(det.root, build_dir, "root debe resolver la carpeta real de la build sin importar su nombre");
        assert_eq!(det.bin_dir, Some(build_dir.clone()));
        assert_eq!(det.main_hl, Some(build_dir.join("hlboot.dat")));
        assert_eq!(det.hashlink, Some(companion));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_nested_export_prefers_runtime_build_over_mod_code() {
        let temp = std::env::temp_dir().join(format!("test_hl_nested_build_{}", std::process::id()));
        let mod_code = temp.join("example").join("code");
        let real_build = temp.join("export").join("hl").join("bin");
        fs::create_dir_all(&mod_code).unwrap();
        fs::create_dir_all(&real_build).unwrap();
        fs::write(mod_code.join("main.hl"), b"small_mod").unwrap();
        fs::write(real_build.join("hlboot.dat"), b"real_game").unwrap();
        let _companion = create_fake_companion(&real_build, "Game");
        fs::write(real_build.join(fake_libhl_name()), b"fake_lib").unwrap();

        let det = detect(&temp);
        assert_eq!(det.root, real_build);
        assert_eq!(det.bin_dir, Some(real_build.clone()));
        assert_eq!(det.main_hl, Some(real_build.join("hlboot.dat")));

        let _ = fs::remove_dir_all(&temp);
    }
}
