use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/ticca-desktop.svg");
    println!("cargo:rerun-if-changed=assets/linux/ticca-desktop.desktop");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" {
        return;
    }

    let Some(profile_dir) = profile_dir_from_out_dir() else {
        println!("cargo:warning=Unable to determine target profile directory from OUT_DIR; skipping Linux bundle generation");
        return;
    };

    let bundle_root = profile_dir.join("bundle").join("linux");
    if let Err(error) = generate_linux_bundle(&bundle_root) {
        println!("cargo:warning=Failed to generate Linux desktop bundle: {error}");
    }
}

fn profile_dir_from_out_dir() -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").ok()?);
    let mut current: &Path = &out_dir;

    loop {
        if current.file_name().and_then(|s| s.to_str()) == Some("build") {
            return current.parent().map(|p| p.to_path_buf());
        }

        current = current.parent()?;
    }
}

fn generate_linux_bundle(bundle_root: &Path) -> Result<(), String> {
    fs::create_dir_all(bundle_root).map_err(|e| format!("create bundle dir: {e}"))?;

    let desktop_src = Path::new("assets/linux/ticca-desktop.desktop");
    let desktop_dst = bundle_root.join("ticca-desktop.desktop");
    fs::copy(desktop_src, &desktop_dst).map_err(|e| format!("copy desktop file: {e}"))?;

    let svg_src = Path::new("assets/icons/ticca-desktop.svg");
    let scalable_dir = bundle_root
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps");
    fs::create_dir_all(&scalable_dir).map_err(|e| format!("create scalable dir: {e}"))?;
    fs::copy(svg_src, scalable_dir.join("ticca-desktop.svg"))
        .map_err(|e| format!("copy svg icon: {e}"))?;

    let svg_data = fs::read(svg_src).map_err(|e| format!("read svg icon: {e}"))?;
    let icon_sizes = [16u32, 32, 48, 64, 128, 256];
    for size in icon_sizes {
        let png_path = bundle_root
            .join("icons")
            .join("hicolor")
            .join(format!("{size}x{size}"))
            .join("apps")
            .join("ticca-desktop.png");
        fs::create_dir_all(png_path.parent().unwrap())
            .map_err(|e| format!("create png dir ({size}): {e}"))?;

        render_svg_to_png_file(&svg_data, size, &png_path)
            .map_err(|e| format!("render png ({size}): {e}"))?;
    }

    Ok(())
}

fn render_svg_to_png_file(svg_data: &[u8], size: u32, output_path: &Path) -> Result<(), String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_data, &options)
        .map_err(|e| format!("parse svg: {e}"))?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| "create pixmap failed".to_string())?;

    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let x = (size as f32 - svg_size.width() * scale) / 2.0;
    let y = (size as f32 - svg_size.height() * scale) / 2.0;

    let transform = resvg::tiny_skia::Transform::from_translate(x, y).post_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut rgba = pixmap.data().to_vec();
    unpremultiply_alpha_in_place(&mut rgba);

    let img = image::RgbaImage::from_raw(size, size, rgba)
        .ok_or_else(|| "convert pixmap to image failed".to_string())?;

    img.save_with_format(output_path, image::ImageFormat::Png)
        .map_err(|e| format!("write png: {e}"))?;

    Ok(())
}

fn unpremultiply_alpha_in_place(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a == 0 || a == 255 {
            continue;
        }
        px[0] = ((px[0] as u32 * 255) / a).min(255) as u8;
        px[1] = ((px[1] as u32 * 255) / a).min(255) as u8;
        px[2] = ((px[2] as u32 * 255) / a).min(255) as u8;
    }
}
