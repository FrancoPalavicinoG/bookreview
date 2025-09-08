use rocket::form::{Form, FromForm};
use rocket::{post, routes, Route};
use rocket::response::{Flash, Redirect};
use rocket::fs::TempFile;
use std::env;
use std::fs;
use std::path::Path;
use uuid::Uuid;

// Function to detect image type from magic bytes
fn detect_image_type(content: &[u8]) -> Option<&'static str> {
    if content.len() < 8 {
        return None;
    }
    
    // Check magic bytes for different image formats
    if content.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if content.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if content.starts_with(b"GIF87a") || content.starts_with(b"GIF89a") {
        Some("gif")
    } else if content.starts_with(b"RIFF") && content.len() > 12 && &content[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

#[derive(FromForm)]
pub struct FileUpload<'r> {
    pub file: TempFile<'r>,
    pub upload_type: String, // "book_cover" or "author_image"
    pub entity_id: String,   // book_id or author_id
}

#[post("/", data = "<upload>")]
pub async fn upload_file(upload: Form<FileUpload<'_>>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let uploads_dir = env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    
    // Create uploads directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&uploads_dir) {
        return Err(Flash::error(Redirect::to("/upload"), format!("Failed to create upload directory: {}", e)));
    }
    
    // Get the original filename if available
    let original_filename = upload.file.name().unwrap_or("unknown");
    
    // Try to detect file type from magic bytes instead of just filename
    let temp_path = upload.file.path();
    let file_content = if let Some(path) = temp_path {
        std::fs::read(path).unwrap_or_default()
    } else {
        Vec::new()
    };
    
    // Detect file type from magic bytes
    let extension = detect_image_type(&file_content).unwrap_or_else(|| {
        // Fallback to filename extension if magic bytes detection fails
        Path::new(original_filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
    });
    
    if extension.is_empty() || !["jpg", "jpeg", "png", "gif", "webp"].contains(&extension.to_lowercase().as_str()) {
        return Err(Flash::error(Redirect::to("/upload"), 
            format!("Invalid file type. Detected: '{}'. Only JPG, PNG, GIF, and WebP images are allowed.", extension)));
    }

    // Generate unique filename
    let unique_filename = format!("{}_{}.{}", 
        upload.upload_type, 
        Uuid::new_v4(), 
        extension
    );

    let file_path = Path::new(&uploads_dir).join(&unique_filename);

    // Save the file by copying content instead of moving (avoids cross-device link error)
    if let Some(temp_path) = upload.file.path() {
        if let Err(e) = std::fs::copy(temp_path, &file_path) {
            return Err(Flash::error(Redirect::to("/upload"), format!("Failed to save file: {}", e)));
        }
    } else {
        return Err(Flash::error(Redirect::to("/upload"), "No temporary file path available"));
    }

    // Return success with the file path
    let relative_path = format!("static/{}", unique_filename);
    Ok(Flash::success(
        Redirect::to("/upload"),
        format!("File uploaded successfully. Access at: /{}", relative_path)
    ))
}

pub fn get_upload_routes() -> Vec<Route> {
    routes![upload_file]
}

pub fn get_uploads_dir() -> String {
    env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string())
}

pub fn create_uploads_directory() -> Result<(), std::io::Error> {
    let uploads_dir = get_uploads_dir();
    fs::create_dir_all(&uploads_dir)?;
    
    // Create subdirectories for organization
    fs::create_dir_all(format!("{}/books", uploads_dir))?;
    fs::create_dir_all(format!("{}/authors", uploads_dir))?;
    
    Ok(())
}
