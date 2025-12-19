//! Image handling utilities for attachments
//!
//! Provides functions for loading images from files and clipboard,
//! and converting them to formats suitable for the LLM API.

use crate::messages::ImageAttachment;
use std::path::PathBuf;
use std::sync::Arc;

/// Load an image from a file path and convert to PNG bytes
pub async fn load_image_from_path(path: &PathBuf) -> Result<ImageAttachment, String> {
    use image::GenericImageView;
    use std::io::Cursor;

    // Read the file
    let data = tokio::fs::read(path).await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Load and decode the image
    let img = image::load_from_memory(&data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let (width, height) = img.dimensions();

    // Convert to PNG format for consistent handling
    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode as PNG: {}", e))?;

    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    Ok(ImageAttachment {
        data: Arc::new(png_data),
        width,
        height,
        filename,
    })
}

/// Paste an image from the system clipboard
pub async fn paste_image_from_clipboard() -> Result<ImageAttachment, String> {
    // Run clipboard access in blocking task since arboard is not async
    tokio::task::spawn_blocking(|| {
        use arboard::Clipboard;

        let mut clipboard = Clipboard::new()
            .map_err(|e| format!("Failed to access clipboard: {}", e))?;

        let img_data = clipboard.get_image()
            .map_err(|e| format!("No image in clipboard: {}", e))?;

        // Convert RGBA pixels to PNG
        let width = img_data.width as u32;
        let height = img_data.height as u32;

        // Create image buffer from raw RGBA data
        let img_buffer: image::RgbaImage = image::ImageBuffer::from_raw(
            width,
            height,
            img_data.bytes.into_owned(),
        ).ok_or_else(|| "Failed to create image buffer".to_string())?;

        // Encode as PNG
        let mut png_data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_data);
        img_buffer.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode as PNG: {}", e))?;

        Ok(ImageAttachment {
            data: Arc::new(png_data),
            width,
            height,
            filename: Some("clipboard.png".to_string()),
        })
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))?
}

/// Check if a file path is a supported image format
pub fn is_image_file(path: &PathBuf) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("bmp"))
}
