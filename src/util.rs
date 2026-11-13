use std::path::{Path, PathBuf};
use std::process::Command;

/// Nombre del binario del loader según el sistema operativo.
#[cfg(windows)]
pub const LOADER_EXE_NAME: &str = "HashLoader.exe";
#[cfg(not(windows))]
pub const LOADER_EXE_NAME: &str = "HashLoader";

/// Nombre del ejecutable JIT principal según el sistema operativo.
#[cfg(windows)]
pub const HOST_EXE_NAME: &str = "hashlink.exe";
#[cfg(not(windows))]
pub const HOST_EXE_NAME: &str = "hashlink";

/// Nombre de la librería compartida principal de HashLink según el sistema operativo.
#[cfg(windows)]
pub const LIBHL_NAME: &str = "libhl.dll";
#[cfg(not(windows))]
pub const LIBHL_NAME: &str = "libhl.so";

/// Cadena UTF-16 terminada en nulo. El `Vec` devuelto vive mientras el
/// llamador lo sostenga (los punteros crudos temporales no duran la llamada).
#[cfg(windows)]
pub fn wide_str(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let mut v: Vec<u16> = OsStr::new(s).encode_wide().collect();
    v.push(0);
    v
}

/// Directorio persistente de datos del usuario para HashLoader.
/// En Windows usa `%LOCALAPPDATA%\HashLoader` y en Unix `$XDG_DATA_HOME/HashLoader`
/// (`$HOME/.local/share/HashLoader` por defecto).
pub fn app_data_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|p| PathBuf::from(p).join("AppData").join("Local"))
                .unwrap_or_else(std::env::temp_dir)
        });

    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|p| PathBuf::from(p).join(".local").join("share"))
                .unwrap_or_else(std::env::temp_dir)
        });

    base.join("HashLoader")
}

/// Abre una carpeta en el explorador de archivos del sistema.
pub fn open_folder(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// Resalta un archivo en el explorador (Windows) o abre su carpeta contenedora (Unix).
pub fn reveal_file(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("explorer")
            .arg(format!("/select,\"{}\"", path.display()))
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        open_folder(target);
    }
}

/// Abre un archivo de texto (logs, etc.) con el visor del sistema.
pub fn open_text_file(path: &Path) {
    #[cfg(windows)]
    {
        let _ = Command::new("notepad").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// Asegura permisos de ejecución (0o755) en Unix al escribir binarios o bibliotecas compartidas.
pub fn mark_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o755);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}
