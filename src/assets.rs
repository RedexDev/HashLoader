/// Recursos embebidos generados por build.rs a partir de app/*.png.
#[cfg(windows)]
pub const HLMOD_ICO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hlmod.ico"));

#[cfg(not(windows))]
pub const HLMOD_ICO: &[u8] = &[];

#[cfg(windows)]
pub const LOGO_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.png"));

#[cfg(not(windows))]
pub const LOGO_PNG: &[u8] = &[];
