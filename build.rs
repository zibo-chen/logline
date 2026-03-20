fn main() {
    let build_version = std::env::var("LOGLINE_BUILD_VERSION")
        .ok()
        .or_else(github_ref_version)
        .or_else(git_describe_version)
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into()));
    println!("cargo:rustc-env=LOGLINE_BUILD_VERSION={build_version}");

    #[cfg(target_os = "windows")]
    {
        let ico_path = std::path::Path::new("res/icon.ico");
        let png_path = std::path::Path::new("res/icon.png");

        // Auto-generate .ico from .png if it doesn't exist (or is older than the PNG)
        if !ico_path.exists() && png_path.exists() {
            if let Err(e) = generate_ico_from_png(png_path, ico_path) {
                println!("cargo:warning=Failed to generate icon.ico from icon.png: {e}");
            }
        }

        if ico_path.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("res/icon.ico");
            if let Err(e) = res.compile() {
                println!("cargo:warning=Failed to embed Windows icon: {e}");
            }
        } else {
            println!("cargo:warning=res/icon.ico not found and could not be generated. Windows exe will use default icon.");
        }
    }
}

/// Generate a minimal ICO file wrapping the PNG data.
/// ICO format supports embedding PNG images directly (Vista+ icon format).
#[cfg(target_os = "windows")]
fn generate_ico_from_png(
    png_path: &std::path::Path,
    ico_path: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;

    let png_data = std::fs::read(png_path)?;

    // Read PNG dimensions from IHDR chunk (bytes 16-23)
    if png_data.len() < 24 || &png_data[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Not a valid PNG file",
        ));
    }
    let width = u32::from_be_bytes([png_data[16], png_data[17], png_data[18], png_data[19]]);
    let height = u32::from_be_bytes([png_data[20], png_data[21], png_data[22], png_data[23]]);

    // ICO dimensions: 0 means 256 in ICO format
    let ico_w = if width >= 256 { 0u8 } else { width as u8 };
    let ico_h = if height >= 256 { 0u8 } else { height as u8 };

    let data_offset: u32 = 6 + 16; // header(6) + one entry(16)
    let data_size = png_data.len() as u32;

    let mut ico = Vec::with_capacity(data_offset as usize + png_data.len());

    // ICONDIR header
    ico.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Type: 1 = ICO
    ico.extend_from_slice(&1u16.to_le_bytes()); // Count: 1 image

    // ICONDIRENTRY
    ico.push(ico_w); // Width
    ico.push(ico_h); // Height
    ico.push(0); // Color palette count
    ico.push(0); // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Color planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
    ico.extend_from_slice(&data_size.to_le_bytes()); // Image data size
    ico.extend_from_slice(&data_offset.to_le_bytes()); // Image data offset

    // PNG data
    ico.extend_from_slice(&png_data);

    let mut file = std::fs::File::create(ico_path)?;
    file.write_all(&ico)?;

    println!("cargo:warning=Generated res/icon.ico from res/icon.png ({width}x{height})");
    Ok(())
}

fn github_ref_version() -> Option<String> {
    let ref_name = std::env::var("GITHUB_REF_NAME").ok()?;
    let version = ref_name.trim_start_matches('v').trim().to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

fn git_describe_version() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8(output.stdout).ok()?;
    let version = version.trim().trim_start_matches('v').to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}
