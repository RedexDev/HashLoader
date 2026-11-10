use std::ffi::OsStr;

/// Cadena UTF-16 terminada en nulo. El `Vec` devuelto vive mientras el
/// llamador lo sostenga (los punteros crudos temporales no duran la llamada).
#[cfg(windows)]
pub fn wide_str(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut v: Vec<u16> = OsStr::new(s).encode_wide().collect();
    v.push(0);
    v
}

#[cfg(not(windows))]
pub fn wide_str(_s: &str) -> Vec<u16> {
    Vec::new()
}
