/// Recursos embebidos generados por build.rs a partir de app/*.png.
#[cfg(windows)]
pub const HLMOD_ICO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hlmod.ico"));

#[cfg(not(windows))]
pub const HLMOD_PNG_128: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hlmod_128.png"));
#[cfg(not(windows))]
pub const HLMOD_PNG_256: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hlmod_256.png"));

pub const LOGO_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.png"));
