use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    game: Option<GameSection>,
    #[serde(default)]
    runtime: Option<RuntimeSection>,
    #[serde(default)]
    mods: Option<ModsSection>,
    #[serde(default)]
    run: Option<RunSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GameSection {
    #[serde(default)]
    bin: Option<String>,
    #[serde(default)]
    main: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuntimeSection {
    #[serde(default)]
    hashlink: Option<String>,
    #[serde(default)]
    libhl: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModsSection {
    #[serde(default)]
    folder: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RunSection {
    #[serde(default)]
    payload: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub path: String,
    pub priority: i32,
}

#[derive(Debug, Serialize)]
pub struct LoaderJson {
    pub version: u32,
    pub bin: String,
    pub main_hl: String,
    pub payload: Option<String>,
    pub mods: Vec<LoaderMod>,
}

pub fn run(root: &Path) -> i32 {
    let cfg_path = root.join("hashloader.toml");
    if !cfg_path.is_file() {
        crate::dialog::msgbox(
            "HashLoader",
            &format!(
                "hashloader.toml no encontrado en\n{}\n\nEjecuta el Setup Toolkit para instalar el loader.",
                root.display()
            ),
        );
        return 2;
    }

    let cfg_txt = match fs::read_to_string(&cfg_path) {
        Ok(t) => t,
        Err(e) => {
            crate::dialog::msgbox("HashLoader — error", &format!("hashloader.toml: {e}"));
            return 2;
        }
    };
    let cfg = match toml::from_str::<Config>(&cfg_txt) {
        Ok(t) => t,
        Err(e) => {
            crate::dialog::msgbox("hashloader.toml", &format!("Configuración inválida:\n{e}"));
            return 2;
        }
    };

    try_run(root, &cfg).unwrap_or_else(|e| {
        crate::dialog::msgbox("HashLoader — error", &e);
        1
    })
}

fn resolve(root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." {
        root.to_path_buf()
    } else {
        let p = PathBuf::from(trimmed);
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    }
}

fn find_runtime(root: &Path, bin: &Path, runtime: &RuntimeSection) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(h) = &runtime.hashlink {
        if !h.trim().is_empty() {
            candidates.push(resolve(root, h));
        }
    }
    #[cfg(windows)]
    {
        candidates.push(bin.join("hashlink.exe"));
        candidates.push(root.join("hashlink.exe"));
        candidates.push(bin.join("hl.exe"));
        candidates.push(root.join("hl.exe"));
    }
    #[cfg(not(windows))]
    {
        candidates.push(bin.join("hashlink"));
        candidates.push(root.join("hashlink"));
        candidates.push(bin.join("hl"));
        candidates.push(root.join("hl"));
    }
    if let Some(hl) = candidates.iter().find(|c| c.is_file()) {
        return Ok(hl.clone());
    }
    if let Some(companion) =
        crate::install::find_companion_exe(&[bin.to_path_buf(), root.to_path_buf()])
    {
        return Ok(companion);
    }
    #[cfg(not(windows))]
    {
        for sys_hl in ["/usr/local/bin/hl", "/usr/bin/hl"] {
            let p = PathBuf::from(sys_hl);
            if p.is_file() {
                return Ok(p);
            }
        }
        if let Ok(path_env) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_env) {
                let p = dir.join("hl");
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }
    let searched = candidates
        .iter()
        .map(|c| format!(" {}", c.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "No se encontró el host JIT '{}'. Búsquedas:\n{searched}\n\n\
         Ajusta [runtime] hashlink = \"...\" en hashloader.toml",
        crate::util::HOST_EXE_NAME
    ))
}

fn try_run(root: &Path, cfg: &Config) -> Result<i32, String> {
    let game = cfg.game.clone().unwrap_or_default();
    let runtime = cfg.runtime.clone().unwrap_or_default();
    let mods_conf = cfg.mods.clone().unwrap_or_default();
    let run_conf = cfg.run.clone().unwrap_or_default();

    // ---- bin de ejecución ----
    let bin = match &game.bin {
        Some(b) if !b.trim().is_empty() => resolve(root, b),
        _ => {
            let sub = root.join("bin");
            if sub.is_dir() {
                sub
            } else {
                root.to_path_buf()
            }
        }
    };
    let bin = if bin.is_dir() { bin } else { root.to_path_buf() };

    let cache = bin.join(".hashloader");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let log_path = cache.join("run.log");
    let mut log = String::new();

    log.push_str(&format!("== HashLoader {} ==\n", env!("CARGO_PKG_VERSION")));
    log.push_str(format!("bin    : {}\n", bin.display()).as_str());

    // Puente nativo de runtime: los mods lo cargan por nombre desde el PATH.
    write_embedded_bridge(&cache, &mut log);

    // ---- listar y extraer mods ----
    let hlmods = resolve(root, mods_conf.folder.as_deref().unwrap_or("hlmods"));
    log.push_str(format!("hlmods : {}\n", hlmods.display()).as_str());

    let mut pkgs: Vec<(crate::hlmod::ModManifest, PathBuf)> = Vec::new();
    if hlmods.is_dir() {
        let mut files: Vec<PathBuf> = fs::read_dir(&hlmods)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .map(|e| e.eq_ignore_ascii_case("hlmod"))
                        .unwrap_or(false)
            })
            .collect();
        files.sort();
        for pkg_path in files {
            let name = pkg_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            log.push_str(&format!("\n== {name} ==\n"));
            match extract_mod(&pkg_path, &cache, &mut log) {
                Ok((mf, dir)) => pkgs.push((mf, dir)),
                Err(e) => log.push_str(&format!("  omitido ({e})\n")),
            }
        }
    } else {
        log.push_str("(no hay carpeta de mods)\n");
    }

    // orden de carga: prioridad (menor primero) y luego id
    pkgs.sort_by(|a, b| a.0.priority.cmp(&b.0.priority).then(a.0.id.cmp(&b.0.id)));

    // ---- overrides (assets que reemplazan los del juego) ----
    log.push_str("\n== overrides ==\n");
    for (mf, dir) in &pkgs {
        let src = dir.join("override");
        if !src.is_dir() {
            continue;
        }
        match apply_override(&src, &bin, dir) {
            Ok(n) => log.push_str(&format!("  {} -> {n} archivo(s) aplicado(s)\n", mf.id)),
            Err(e) => log.push_str(&format!("  {} ERROR: {e}\n", mf.id)),
        }
    }

    // ---- módulo principal del juego (JIT) ----
    let main_hl = match &game.main {
        Some(m) if !m.trim().is_empty() => {
            let candidate = resolve(root, m);
            if candidate.is_file() {
                candidate
            } else {
                let detected = pick_boot_target(root, &bin)?;
                log.push_str(&format!(
                    "\nAVISO: [game] main = '{}' no existe; se auto-detectó '{}'\n",
                    m,
                    detected.display()
                ));
                detected
            }
        }
        _ => pick_boot_target(root, &bin)?,
    };

    // ---- payload del Modding Runtime ----
    let payload: Option<PathBuf> = match run_conf.payload.as_deref() {
        Some(p) if !p.trim().is_empty() => {
            let p = resolve(root, p);
            if p.is_file() {
                log.push_str("\npayload configurado manualmente\n");
                Some(p)
            } else {
                log.push_str(&format!(
                    "\nAVISO: payload configurado '{}' no existe; se omite.\n",
                    p.display()
                ));
                None
            }
        }
        _ => {
            let sideload = root.join("HashLoader.hl");
            if sideload.is_file() {
                log.push_str("\npayload = HashLoader.hl (sibling)\n");
                Some(sideload)
            } else if let Some(p) = write_embedded_payload(&cache, &mut log) {
                Some(p)
            } else {
                None
            }
        }
    };

    // ---- loader.json (contrato para el payload Haxe) ----
    let mut code_mods_count = 0usize;
    let list: Vec<LoaderMod> = pkgs
        .iter()
        .filter_map(|(mf, dir)| {
            let entry_path = match mf.main_path() {
                Some(rel_main) => {
                    let entry = dir.join(&rel_main);
                    if !entry.is_file() {
                        log.push_str(&format!(
                            "aviso     : '{}' declara '{}{}' inexistente\n",
                            mf.id,
                            dir.file_name()
                                .map(|d| d.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            &format!("{}{}", std::path::MAIN_SEPARATOR, rel_main)
                        ));
                        return None;
                    }
                    code_mods_count += 1;
                    entry.display().to_string()
                }
                None => String::new(),
            };
            Some(LoaderMod {
                id: mf.id.clone(),
                name: mf.name.clone(),
                version: mf.version.clone(),
                author: mf.author.clone(),
                path: entry_path,
                priority: mf.priority,
            })
        })
        .collect();

    let lj = LoaderJson {
        version: 1,
        bin: bin.display().to_string(),
        main_hl: main_hl.display().to_string(),
        payload: payload.as_ref().map(|p| p.display().to_string()),
        mods: list,
    };
    let lj_txt = serde_json::to_string_pretty(&lj).map_err(|e| e.to_string())?;
    let lj_path = cache.join("loader.json");
    fs::write(&lj_path, &lj_txt).map_err(|e| e.to_string())?;
    log.push_str(&format!(
        "\nmods      : {} total ({} plugin(s) de código)\n",
        lj.mods.len(),
        code_mods_count
    ));

    // ---- host JIT ----
    let hashlink = find_runtime(root, &bin, &runtime)?;
    let boot = payload
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| main_hl.display().to_string());
    log.push_str(format!("host     : {}\n", hashlink.display()).as_str());
    log.push_str(format!("boot     : {boot}\n").as_str());

    fs::write(&log_path, &log).map_err(|e| e.to_string())?;

    // ---- spawn JIT ----
    let mut cmd = Command::new(&hashlink);
    cmd.current_dir(&bin);
    if let Some(p) = &payload {
        cmd.arg(p);
    }
    cmd.arg(&main_hl);
    if let Some(args) = &game.args {
        for a in args {
            let a = a.trim();
            if !a.is_empty() {
                cmd.arg(a);
            }
        }
    }
    let selfdir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| root.to_path_buf());
    let mut path_dirs = vec![bin.clone(), selfdir];
    // .hashloader contiene hashloader.hdll y el payload embebido.
    path_dirs.push(cache.clone());
    if let Some(p) = hashlink.parent() {
        path_dirs.push(p.to_path_buf());
    }
    // [runtime] libhl: ubicación explícita de libhl.dll (su carpeta entra al PATH)
    if let Some(l) = &runtime.libhl {
        if !l.trim().is_empty() {
            let lp = resolve(root, l);
            if let Some(par) = lp.parent() {
                path_dirs.push(par.to_path_buf());
            }
        }
    }
    add_path(&mut cmd, &path_dirs);
    cmd.env("HASHLOADER_CONFIG", lj_path.display().to_string());
    cmd.env("HASHLOADER_ROOT", root.display().to_string());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "No se pudo iniciar el host JIT:\n  {}\n({e})",
            hashlink.display()
        )
    })?;
    let loggers = spawn_loggers(&mut child, &log_path);

    let status = child.wait().map_err(|e| e.to_string())?;
    for h in loggers {
        let _ = h.join();
    }
    let code = status.code().unwrap_or(1);
    if code != 0 {
        let err_path = log_path.with_extension("stderr");
        let err_tail = read_tail(&err_path);
        let log_tail = read_tail(&log_path);
        let detail = if !err_tail.trim().is_empty() {
            format!("{log_tail}\n--- stderr ---\n{err_tail}")
        } else {
            log_tail
        };
        crate::dialog::msgbox(
            "HashLoader",
            &format!(
                "El juego terminó con código {code}.\n\nRevisa:\n{}\n\n{detail}",
                log_path.display()
            ),
        );
    }
    Ok(code)
}

fn write_if_changed(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Ok(existing) = fs::read(target) {
        if existing.as_slice() == bytes {
            return Ok(());
        }
    }
    fs::write(target, bytes)
}

fn write_embedded_bridge(cache: &Path, log: &mut String) {
    #[cfg(has_bridge)]
    {
        let target = cache.join("hashloader.hdll");
        match write_if_changed(&target, crate::HASHLOADER_HDLL) {
            Ok(()) => {
                crate::util::mark_executable(&target);
                log.push_str(&format!("bridge : {}\n", target.display()));
            }
            Err(e) => log.push_str(&format!("aviso  : no se pudo escribir hashloader.hdll ({e})\n")),
        }
    }
    #[cfg(not(has_bridge))]
    {
        let _ = (cache, log);
    }
}

fn write_embedded_payload(cache: &Path, log: &mut String) -> Option<PathBuf> {
    #[cfg(has_payload)]
    {
        let target = cache.join("HashLoader.hl");
        if write_if_changed(&target, crate::PAYLOAD_HL).is_ok() {
            log.push_str("\npayload embebido del Modding Runtime -> ok\n");
            return Some(target);
        }
        None
    }
    #[cfg(not(has_payload))]
    {
        let _ = cache;
        log.push_str(
            "\n(sin payload embebido: el juego arrancará sin código de mods.\n \
             Coloca 'HashLoader.hl' junto a HashLoader.exe para activar el Modding Runtime.)\n",
        );
        None
    }
}

fn pick_boot_target(root: &Path, bin: &Path) -> Result<PathBuf, String> {
    if let Some(found) = crate::install::find_game_hl(&[bin.to_path_buf(), root.to_path_buf()]) {
        return Ok(found);
    }
    Err(format!(
        "no se encontró ningún archivo de bytecode del juego (*.hl / hlboot.dat) en:\n {}\n {}\nAjusta [game] main = \"...\" en hashloader.toml",
        bin.display(),
        root.display(),
    ))
}

/// Extrae (o reutiliza cache de) un .hlmod; devuelve manifiesto + dir.
fn extract_mod(
    pkg_path: &Path,
    cache: &Path,
    log: &mut String,
) -> Result<(crate::hlmod::ModManifest, PathBuf), String> {
    let meta = fs::metadata(pkg_path).map_err(|e| e.to_string())?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.len();
    let mut mf = crate::hlmod::read_manifest(pkg_path)?;

    let dir = cache.join(format!("mods/{}-{}", mf.id, mf.version));
    let key = dir.join(".hlmodkey");
    let key_txt = format!("{modified}:{size}");
    let up_to_date =
        key.is_file() && fs::read_to_string(&key).ok().as_deref() == Some(key_txt.as_str());

    if up_to_date {
        log.push_str(&format!("  {} v{} (cache)\n", mf.id, mf.version));
    } else {
        let _ = fs::remove_dir_all(&dir);
        let n = crate::hlmod::extract_package(pkg_path, &dir)?;
        fs::write(&key, &key_txt).map_err(|e| e.to_string())?;
        log.push_str(&format!("  {} v{} -> {n} archivos\n", mf.id, mf.version));
    }

    if let Some(main) = mf.main_path() {
        if !dir.join(&main).is_file() {
            if let Some(detected_rel) = crate::hlmod::detect_mod_entry(&dir) {
                log.push_str(&format!(
                    "  aviso: '{main}' no existe; usando '{detected_rel}'\n"
                ));
                mf.main = Some(detected_rel);
            } else {
                return Err(format!("declara main={main} pero no está en el paquete"));
            }
        }
    } else if let Some(detected_rel) = crate::hlmod::detect_mod_entry(&dir) {
        mf.main = Some(detected_rel);
    }
    Ok((mf, dir))
}

fn apply_override(src: &Path, bin: &Path, mod_dir: &Path) -> Result<usize, String> {
    let mut files = Vec::new();
    collect(&src, &mut files);
    let mut count = 0;
    for f in files {
        let rel = f.strip_prefix(src).map_err(|e| e.to_string())?;
        let target = bin.join(rel);
        if target.is_file() && same_bytes(&f, &target)? {
            continue;
        }
        // backup del original dentro del mod para poder quitar el override luego
        let backup = mod_dir.join("originals").join(rel);
        if target.is_file() && !backup.is_file() {
            if let Some(parent) = backup.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&target, &backup).map_err(|e| e.to_string())?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(&f, &target).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}

fn same_bytes(a: &Path, b: &Path) -> Result<bool, String> {
    let (mut fa, mut fb) = match (fs::File::open(a), fs::File::open(b)) {
        (Ok(fa), Ok(fb)) => (fa, fb),
        _ => return Ok(false),
    };
    let mut da = Vec::new();
    let mut db = Vec::new();
    let _ = fa.read_to_end(&mut da);
    let _ = fb.read_to_end(&mut db);
    Ok(da == db)
}

fn add_path(cmd: &mut Command, dirs: &[PathBuf]) {
    // PATH (Windows y Unix, usando el separador nativo mediante std::env::join_paths)
    let mut path_list: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for d in dirs {
        if !path_list.contains(d) {
            path_list.push(d.clone());
        }
    }
    if let Ok(joined) = std::env::join_paths(&path_list) {
        cmd.env("PATH", joined);
    }

    // En Unix/Linux, HashLink resuelve libhl.so y los *.hdll mediante dlopen() / LD_LIBRARY_PATH.
    // Colocamos primero las rutas locales (.hashloader, bin, etc.) para priorizar el puente y libhl local.
    #[cfg(unix)]
    {
        let mut ld_list: Vec<PathBuf> = Vec::new();
        for d in dirs {
            if !ld_list.contains(d) {
                ld_list.push(d.clone());
            }
        }
        if let Some(cur) = std::env::var_os("LD_LIBRARY_PATH") {
            for d in std::env::split_paths(&cur) {
                if !ld_list.contains(&d) {
                    ld_list.push(d);
                }
            }
        }
        if let Ok(joined) = std::env::join_paths(&ld_list) {
            cmd.env("LD_LIBRARY_PATH", joined);
        }
    }
}

/// Redirige stdout/stderr del hijo hacia archivos de log sin bloquearse.
fn spawn_loggers(
    child: &mut std::process::Child,
    log_path: &Path,
) -> Vec<std::thread::JoinHandle<()>> {
    let out_path = log_path.with_extension("stdout");
    let err_path = log_path.with_extension("stderr");
    let mut handles = Vec::new();
    if let Some(st) = child.stdout.take() {
        handles.push(std::thread::spawn(move || drain_to(st, &out_path)));
    }
    if let Some(st) = child.stderr.take() {
        handles.push(std::thread::spawn(move || drain_to(st, &err_path)));
    }
    handles
}

fn drain_to(st: impl Read, path: &Path) {
    let Ok(f) = fs::File::create(path) else {
        return;
    };
    let mut w = BufWriter::new(f);
    let mut buf = [0u8; 8192];
    let mut reader = st;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if w.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = w.flush();
}

fn read_tail(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(4000);
            chars[start..].iter().collect()
        }
        Err(_) => String::new(),
    }
}
