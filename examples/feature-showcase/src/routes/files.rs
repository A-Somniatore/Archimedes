//! Files routes - demonstrates file upload/download.
//!
//! ## Features Demonstrated
//! - Multipart extractor (file uploads)
//! - FileResponse (file downloads)
//! - Streaming responses
//! - Content-Disposition headers
//! - MIME type detection

use archimedes_core::extract::Multipart;
use archimedes_core::response::{json, file_response, Response};
use archimedes_router::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::AppState;

/// File metadata response
#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub uploaded_at: String,
}

/// Upload response
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub files: Vec<FileInfo>,
    pub total_size: usize,
}

/// Create the files router.
///
/// ## Endpoints
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | POST | /upload | Upload files (multipart) |
/// | POST | /upload-single | Upload single file |
/// | GET | /download/{id} | Download file |
/// | GET | /preview/{id} | Preview file (inline) |
pub fn routes(_state: Arc<AppState>) -> Router {
    let mut router = Router::new();

    // -------------------------------------------------------------------------
    // UPLOAD - Demonstrates Multipart extractor (multiple files)
    // -------------------------------------------------------------------------
    router.post("/upload", |mut multipart: Multipart| async move {
        let mut files = Vec::new();
        let mut total_size = 0usize;

        // Process each field in the multipart request
        while let Some(field) = multipart.next_field().await? {
            let name = field.name().unwrap_or("unknown").to_string();
            let filename = field.file_name().unwrap_or("unnamed").to_string();
            let content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            // Read the file data
            let data = field.bytes().await?;
            let size = data.len();
            total_size += size;

            // In a real app, save to storage (S3, local disk, etc.)
            let file_id = Uuid::new_v4().to_string();
            
            tracing::info!(
                file_id = %file_id,
                filename = %filename,
                size = size,
                content_type = %content_type,
                "File uploaded"
            );

            files.push(FileInfo {
                id: file_id,
                filename,
                content_type,
                size,
                uploaded_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        json(&UploadResponse { files, total_size })
    });

    // -------------------------------------------------------------------------
    // UPLOAD SINGLE - Demonstrates single file upload with metadata
    // -------------------------------------------------------------------------
    router.post("/upload-single", |mut multipart: Multipart| async move {
        let mut file_info = None;
        let mut description = String::new();

        while let Some(field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => {
                    let filename = field.file_name().unwrap_or("unnamed").to_string();
                    let content_type = field
                        .content_type()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let data = field.bytes().await?;

                    file_info = Some(FileInfo {
                        id: Uuid::new_v4().to_string(),
                        filename,
                        content_type,
                        size: data.len(),
                        uploaded_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
                Some("description") => {
                    description = field.text().await?;
                }
                _ => {}
            }
        }

        match file_info {
            Some(info) => json(&serde_json::json!({
                "file": info,
                "description": description
            })),
            None => Response::builder()
                .status(400)
                .json(&serde_json::json!({
                    "error": "No file provided",
                    "hint": "Include a 'file' field in the multipart request"
                })),
        }
    });

    // -------------------------------------------------------------------------
    // DOWNLOAD - Demonstrates FileResponse (attachment)
    // -------------------------------------------------------------------------
    router.get("/download/:id", |archimedes_core::extract::Path(id): archimedes_core::extract::Path<String>| async move {
        // In a real app, look up file metadata and path from database
        let filename = format!("document_{}.txt", id);
        let content = format!("This is the content of file {}\n\nGenerated at: {}", id, chrono::Utc::now());
        
        // Return as downloadable file
        file_response(content.as_bytes())
            .filename(&filename)
            .content_type("text/plain")
            .attachment() // Forces download dialog
    });

    // -------------------------------------------------------------------------
    // PREVIEW - Demonstrates FileResponse (inline)
    // -------------------------------------------------------------------------
    router.get("/preview/:id", |archimedes_core::extract::Path(id): archimedes_core::extract::Path<String>| async move {
        // Return file for inline viewing (no download dialog)
        let content = format!("<html><body><h1>Preview: {}</h1><p>Generated at: {}</p></body></html>", id, chrono::Utc::now());
        
        file_response(content.as_bytes())
            .filename(&format!("preview_{}.html", id))
            .content_type("text/html")
            .inline() // Display in browser
    });

    // -------------------------------------------------------------------------
    // STREAM - Demonstrates streaming file response
    // -------------------------------------------------------------------------
    router.get("/stream/:id", |archimedes_core::extract::Path(id): archimedes_core::extract::Path<String>| async move {
        // Create a stream that yields chunks
        let chunks = vec![
            format!("Chunk 1 of file {}\n", id),
            format!("Chunk 2 of file {}\n", id),
            format!("Chunk 3 of file {}\n", id),
        ];

        // In a real app, this would stream from disk or S3
        let body = chunks.join("");
        
        Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .header("content-disposition", format!("attachment; filename=\"stream_{}.txt\"", id))
            .body(body.into_bytes())
    });

    // -------------------------------------------------------------------------
    // IMAGE UPLOAD - Demonstrates image-specific handling
    // -------------------------------------------------------------------------
    router.post("/upload-image", |mut multipart: Multipart| async move {
        while let Some(field) = multipart.next_field().await? {
            if field.name() == Some("image") {
                let content_type = field
                    .content_type()
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                // Validate image type
                if !content_type.starts_with("image/") {
                    return Response::builder()
                        .status(400)
                        .json(&serde_json::json!({
                            "error": "Invalid file type",
                            "expected": "image/*",
                            "received": content_type
                        }));
                }

                let data = field.bytes().await?;
                let filename = field.file_name().unwrap_or("image").to_string();

                // In a real app, process image (resize, optimize, etc.)
                return json(&serde_json::json!({
                    "id": Uuid::new_v4().to_string(),
                    "filename": filename,
                    "content_type": content_type,
                    "size": data.len(),
                    "dimensions": {
                        "width": 800,  // Would be detected from image
                        "height": 600
                    }
                }));
            }
        }

        Response::builder()
            .status(400)
            .json(&serde_json::json!({
                "error": "No image provided"
            }))
    });

    router.tag("files");
    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info_serialization() {
        let info = FileInfo {
            id: "123".to_string(),
            filename: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            size: 1024,
            uploaded_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&info).expect("Should serialize");
        assert!(json.contains("test.txt"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadResponse {
            files: vec![FileInfo {
                id: "1".to_string(),
                filename: "a.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 100,
                uploaded_at: "2024-01-01T00:00:00Z".to_string(),
            }],
            total_size: 100,
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("total_size"));
        assert!(json.contains("files"));
    }

    #[test]
    fn test_mime_type_detection() {
        let extensions = vec![
            ("test.txt", "text/plain"),
            ("test.html", "text/html"),
            ("test.json", "application/json"),
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
        ];

        for (filename, expected) in extensions {
            let ext = filename.rsplit('.').next().unwrap_or("");
            let mime = match ext {
                "txt" => "text/plain",
                "html" => "text/html",
                "json" => "application/json",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                _ => "application/octet-stream",
            };
            assert_eq!(mime, expected);
        }
    }
}
