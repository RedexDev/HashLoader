/// MessageBox nativa de Windows (no necesita ventana padre; GUI-less).
#[cfg(windows)]
pub fn msgbox(title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_ICONWARNING};
    let t = crate::util::wide_str(title);
    let m = crate::util::wide_str(text);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            m.as_ptr(),
            t.as_ptr(),
            MB_ICONWARNING | MB_OK,
        );
    }
    let _ = MB_ICONINFORMATION;
}

#[cfg(not(windows))]
pub fn msgbox(_title: &str, text: &str) {
    println!("{title}: {text}", title = _title);
    eprintln!("{text}");
}
