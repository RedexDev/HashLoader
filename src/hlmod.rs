use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Manifest root entry expected inside every .hlmod package.
pub const MANIFEST_ENTRY: &str = "hlmod.json";
pub const DEFAULT_MAIN: &str = "code/main.hl";

/// `hlmod.json` — manifiesto de un .hlmod.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ModManifest {
    /// Número de formato del paquete (1).
    #[serde(default, rename = "hlmod")]
    pub format: u32,
    /// Identificador único corto (sin espacios, login-style).
    #[serde(default)]
    pub id: String,
    /// Nombre visible del mod.
    #[serde(default)]
    pub name: String,
    /// Versión semántica del mod.
    #[serde(default = "default_version")]
    pub version: String,
    /// Autor.
    #[serde(default)]
    pub author: String,
    /// Descripción corta (opcional).
    #[serde(default)]
    pub description: String,
    /// Plugin HashLink principal (opcional), compilado con Haxe (`main.hl`).
    #[serde(default)]
    pub main: Option<String>,
    /// Orden de carga (menor = más temprano).
    #[serde(default)]
    pub priority: i32,
    /// Dependencias exigidas: ["id>=1.0"] (informativo en v0).
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub website: Option<String>,
}

fn default_version() -> String {
    "0.1.0".into()
}

impl ModManifest {
    pub fn main_path(&self) -> Option<String> {
        self.main.clone().filter(|s| !s.trim().is_empty())
    }

    pub fn slug_from_path(p: &Path) -> String {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "mod".into());
        slugify(&name)
    }

    /// Genera un manifiesto básico para carpetas sin hlmod.json.
    pub fn scaffold_for(source: &Path) -> ModManifest {
        let id = Self::slug_from_path(source);
        let fallback_name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Mod".into());
        let main = if source.is_dir() {
            detect_mod_entry(source).or_else(|| Some(DEFAULT_MAIN.into()))
        } else {
            Some(DEFAULT_MAIN.into())
        };
        ModManifest {
            format: HLMOD_FORMAT,
            id: id.clone(),
            name: fallback_name,
            main,
            ..Default::default()
        }
    }
}

/// Detecta el archivo `.hl` de un mod dentro de `code/` o en la raíz de `mod_dir`,
/// sin obligar a que se llame `code/main.hl`.
pub fn detect_mod_entry(mod_dir: &Path) -> Option<String> {
    if mod_dir.join(DEFAULT_MAIN).is_file() {
        return Some(DEFAULT_MAIN.into());
    }
    for sub in [mod_dir.join("code"), mod_dir.to_path_buf()] {
        let Ok(rd) = fs::read_dir(&sub) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let is_hl = p
                .extension()
                .map(|x| x.to_string_lossy().eq_ignore_ascii_case("hl"))
                .unwrap_or(false);
            if is_hl {
                if let Ok(rel) = p.strip_prefix(mod_dir) {
                    let s = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join("/");
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

pub const HLMOD_FORMAT: u32 = 1;

pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

pub fn normalize_id(raw: &str) -> String {
    let id = slugify(raw);
    if id.is_empty() {
        "mod".into()
    } else {
        id
    }
}

/// Bloquea rutas tipo zip-slip (`../`, absolutas, unidades, etc. al descomprimir).
pub fn safe_rel_path(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Lee el manifiesto de un .hlmod (o carpeta de mod sin empaquetar).
pub fn read_manifest(path: &Path) -> Result<ModManifest, String> {
    if path.is_dir() {
        let f = path.join(MANIFEST_ENTRY);
        if f.exists() {
            let raw = fs::read(f).map_err(|e| e.to_string())?;
            return parse_manifest(&raw);
        }
        return Ok(ModManifest::scaffold_for(path));
    }
    let file = fs::File::open(path).map_err(|e| format!("no se pudo abrir: {e}"))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| format!("paquete inválido: {e}"))?;

    // 1) buscar el manifiesto en la raíz
    let mut m: Option<String> = archive
        .by_name(MANIFEST_ENTRY)
        .ok()
        .and_then(|mut e| {
            let mut buf = String::new();
            e.read_to_string(&mut buf).ok()?;
            Some(buf)
        });

    // 2) fallback: cualquier .json raíz
    if m.is_none() {
        for i in 0..archive.len() {
            let mut e = archive
                .by_index(i)
                .map_err(|e| format!("paquete zip ilegible: {e}"))?;
            let n = e.name().to_string();
            if n.starts_with('/') || n.contains("..") || n.contains(':') {
                continue;
            }
            if n.ends_with(".json") && Path::new(&n).components().count() == 1 {
                let mut buf = String::new();
                let _ = e.read_to_string(&mut buf);
                m = Some(buf);
                break;
            }
        }
    }

    let Some(m) = m else {
        return Ok(ModManifest::scaffold_for(path));
    };
    if m.trim().is_empty() {
        return Ok(ModManifest::scaffold_for(path));
    }
    parse_manifest(m.as_bytes())
}

pub fn parse_manifest(raw: &[u8]) -> Result<ModManifest, String> {
    let mut mf: ModManifest = serde_json::from_slice(raw)
        .map_err(|e| format!("hlmod.json inválido: {e}"))?;
    mf.format = if mf.format == 0 { HLMOD_FORMAT } else { mf.format };
    if mf.id.is_empty() {
        mf.id = "mod".into();
    }
    mf.id = normalize_id(&mf.id);
    if mf.name.is_empty() {
        mf.name = mf.id.clone();
    }
    Ok(mf)
}

/// Lista recursiva de archivos dentro de `dir` (sin descender en `.hashloader`).
pub fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if p.is_dir() {
            if name == ".hashloader" || name.starts_with('.') {
                continue;
            }
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// Empaqueta una carpeta de mod (con hlmod.json o autogenerado) hacia `out.hlmod`.
pub struct PackResult {
    pub file_count: usize,
    pub total_bytes: u64,
}

pub fn pack(source: &Path, out_path: &Path) -> Result<PackResult, String> {
    if !source.is_dir() {
        return Err(format!("la carpeta de origen no existe: {}", source.display()));
    }
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut files = Vec::new();
    collect_files(source, &mut files);

    // contenido del manifiesto
    let manifest_text = match (source.join(MANIFEST_ENTRY).exists(), files.iter().any(|f| {
        f.file_name().map(|n| n.to_string_lossy() == MANIFEST_ENTRY).unwrap_or(false)
    })) {
        (true, _) => {
            let raw = fs::read(source.join(MANIFEST_ENTRY)).map_err(|e| e.to_string())?;
            // valida pero preserva el original tal cual
            let _ = parse_manifest(&raw)?;
            raw
        }
        (false, false) => {
            let mf = ModManifest::scaffold_for(source);
            serde_json::to_vec_pretty(&mf).map_err(|e| e.to_string())?
        }
        (false, true) => {
            return Err(format!("{MANIFEST_ENTRY} debe estar en la raíz de la carpeta"));
        }
    };

    // el plugin debe existir si se declara
    if let Ok(mf) = parse_manifest(&manifest_text) {
        if let Some(main) = mf.main_path() {
            if !source.join(&main).exists() {
                return Err(format!(
                    "hlmod.json declara main = '{main}' pero no existe en la carpeta"
                ));
            }
        }
    }

    let file = fs::File::create(out_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // manifiesto SIEMPRE primero
    zip.start_file(MANIFEST_ENTRY, opts)
        .map_err(|e| e.to_string())?;
    zip.write_all(&manifest_text).map_err(|e| e.to_string())?;

    let mut file_count = 0usize;
    let mut total_bytes: u64 = 0;
    for f in files {
        if f.file_name().map(|n| n.to_string_lossy() == MANIFEST_ENTRY).unwrap_or(false) {
            continue;
        }
        let rel = f.strip_prefix(source).map_err(|e| e.to_string())?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        zip.start_file(&name, opts).map_err(|e| e.to_string())?;
        let data = fs::read(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
        file_count += 1;
        total_bytes += data.len() as u64;
    }
    zip.finish().map_err(|e| e.to_string())?;

    Ok(PackResult {
        file_count,
        total_bytes,
    })
}

/// Extrae todo un .hlmod a `dest`.
pub fn extract_package(path: &Path, dest: &Path) -> Result<usize, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| format!("paquete inválido: {e}"))?;
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.is_empty() || name.ends_with('/') {
            if let Some(rel) = safe_rel_path(name.trim_end_matches('/')) {
                let _ = fs::create_dir_all(dest.join(rel));
            }
            continue;
        }
        let Some(rel) = safe_rel_path(&name) else {
            return Err(format!("entrada peligrosa bloqueada: {name}"));
        };
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}
