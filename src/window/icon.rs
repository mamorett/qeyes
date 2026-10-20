use winit::window::Icon;

pub struct DecodedIcon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_png_rgba(bytes: &[u8]) -> Option<DecodedIcon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some(DecodedIcon {
        width: info.width,
        height: info.height,
        rgba: buf,
    })
}

/// Loads the application window icon decoded from the official logo.
pub fn load_app_window_icon() -> Option<Icon> {
    let icon = decode_png_rgba(include_bytes!("../../assets/qeyes-256.png"))?;
    Icon::from_rgba(icon.rgba, icon.width, icon.height).ok()
}

/// Returns multi-resolution icons (width, height, ARGB cardinals) for X11 `_NET_WM_ICON`.
#[cfg(target_os = "linux")]
pub fn load_x11_net_wm_icons() -> Vec<u32> {
    let raw_pngs: &[&[u8]] = &[
        include_bytes!("../../assets/qeyes-16.png"),
        include_bytes!("../../assets/qeyes-32.png"),
        include_bytes!("../../assets/qeyes-48.png"),
        include_bytes!("../../assets/qeyes-64.png"),
        include_bytes!("../../assets/qeyes-128.png"),
        include_bytes!("../../assets/qeyes-256.png"),
    ];

    let mut result = Vec::new();
    for png_bytes in raw_pngs {
        if let Some(icon) = decode_png_rgba(png_bytes) {
            let w = icon.width;
            let h = icon.height;
            result.push(w);
            result.push(h);
            for chunk in icon.rgba.chunks_exact(4) {
                let r = chunk[0] as u32;
                let g = chunk[1] as u32;
                let b = chunk[2] as u32;
                let a = chunk[3] as u32;
                result.push((a << 24) | (r << 16) | (g << 8) | b);
            }
        }
    }
    result
}
