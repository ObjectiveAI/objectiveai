use crate::state::{FileStateCache, FileStateEntry};
use crate::util;
use std::path::Path;
use tokio::fs;

const MAX_READ_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_IMAGE_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20 MB
const PDF_MAX_PAGES_PER_READ: usize = 20;
const PDF_MAX_EXTRACT_SIZE: u64 = 100 * 1024 * 1024; // 100MB

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

// Binary extensions -- does NOT include image extensions (handled separately)
const BINARY_EXTENSIONS: &[&str] = &[
    // Images that we DON'T support reading (non-web formats)
    "bmp", "ico", "tiff", "tif", "avif", "heic", "heif",
    // Video
    "mp4", "avi", "mov", "wmv", "flv", "mkv", "webm", "m4v", "mpg", "mpeg",
    // Audio
    "mp3", "wav", "flac", "aac", "ogg", "wma", "m4a", "opus",
    // Archives
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "zst", "lz4",
    // Executables/Libraries
    "exe", "dll", "so", "dylib", "o", "a", "lib", "obj", "class", "pyc", "pyo",
    // Documents (binary)
    "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp",
    // Databases
    "db", "sqlite", "sqlite3", "mdb",
    // Fonts
    "ttf", "otf", "woff", "woff2", "eot",
    // Other binary
    "bin", "dat", "iso", "img", "dmg", "wasm", "deb", "rpm",
];

fn has_extension_in(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| extensions.iter().any(|&b| b.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// Detect image format from magic bytes.
fn detect_image_format(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 4 && bytes[..4] == [0x89, 0x50, 0x4E, 0x47] {
        "image/png"
    } else if bytes.len() >= 3 && bytes[..3] == [0xFF, 0xD8, 0xFF] {
        "image/jpeg"
    } else if bytes.len() >= 4 && &bytes[..4] == b"GIF8" {
        "image/gif"
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png" // default fallback
    }
}

/// Output variants from read_file.
pub enum ReadOutput {
    /// Text file content (JSON serialized)
    Text(String),
    /// Image file (base64 data + media type)
    Image { base64: String, media_type: String },
    /// Notebook cells (text + embedded images), also used for PDF page images
    Notebook(Vec<crate::notebook::NotebookBlock>),
    /// File unchanged since last read (dedup stub)
    FileUnchanged(String),
}

#[derive(Debug, serde::Serialize)]
struct TextFilePayload {
    #[serde(rename = "filePath")]
    file_path: String,
    content: String,
    #[serde(rename = "numLines")]
    num_lines: usize,
    #[serde(rename = "startLine")]
    start_line: usize,
    #[serde(rename = "totalLines")]
    total_lines: usize,
}

#[derive(Debug, serde::Serialize)]
struct ReadFileJsonOutput {
    #[serde(rename = "type")]
    kind: String,
    file: TextFilePayload,
}

pub async fn read_file(
    file_state: &FileStateCache,
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    pages: Option<&str>,
) -> Result<ReadOutput, String> {
    // UNC path security check
    if util::is_unc_path(path) {
        return Err("Cannot read files on UNC paths.".into());
    }

    // Check for blocked devices
    if util::is_blocked_device(path) {
        return Err(format!("Cannot read '{path}': this device file would block or produce infinite output."));
    }

    let absolute_path = util::normalize_path_allow_missing(path)
        .await
        .map_err(|e| format!("Failed to resolve path: {e}"))?;
    let absolute_path_str = absolute_path.to_string_lossy().to_string();

    // File-not-found with suggestions
    if !fs::try_exists(&absolute_path).await.unwrap_or(false) {
        let mut msg = format!(
            "File does not exist. Note: your current working directory is {}.",
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        );
        if let Some(similar) = util::find_similar_file(&absolute_path).await {
            msg.push_str(&format!("\nDid you mean: {similar}"));
        }
        if let Some(suggested) = util::suggest_path_under_cwd(path).await {
            msg.push_str(&format!("\nSuggested path: {suggested}"));
        }
        return Err(msg);
    }

    // file_unchanged dedup: if same path, same offset/limit, same mtime -> return stub
    if let Some(cached) = file_state.get(&absolute_path_str).await {
        // Only dedup entries that came from a prior Read (offset is Some), not from Edit/Write
        if cached.offset.is_some() && cached.offset == offset && cached.limit == limit {
            if let Ok(current_mtime) = util::get_file_mtime_ms(&absolute_path).await {
                if current_mtime == cached.timestamp {
                    return Ok(ReadOutput::FileUnchanged(
                        "File unchanged since last read. The content from the earlier Read tool_result in this conversation is still current \u{2014} refer to that instead of re-reading.".into()
                    ));
                }
            }
        }
    }

    let ext = absolute_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Image files -- return as Content::image
    if has_extension_in(&absolute_path, IMAGE_EXTENSIONS) {
        let metadata = fs::metadata(&absolute_path)
            .await
            .map_err(|e| format!("Failed to read file metadata: {e}"))?;
        if metadata.len() > MAX_IMAGE_FILE_SIZE {
            return Err(format!(
                "Image file is too large ({} bytes, max 20MB).",
                metadata.len()
            ));
        }
        let bytes = fs::read(&absolute_path)
            .await
            .map_err(|e| format!("Failed to read image file: {e}"))?;
        let media_type = detect_image_format(&bytes).to_string();
        let base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        // Update file state (store placeholder content, not the base64)
        let mtime_ms = util::get_file_mtime_ms(&absolute_path)
            .await
            .map_err(|e| format!("Failed to get file mtime: {e}"))?;
        file_state.set(absolute_path_str, FileStateEntry {
            content: format!("[image: {} bytes]", bytes.len()),
            timestamp: mtime_ms,
            offset,
            limit,
            is_partial_view: false,
        }).await;

        return Ok(ReadOutput::Image { base64, media_type });
    }

    // Notebook files (.ipynb)
    if ext.eq_ignore_ascii_case("ipynb") {
        let metadata = fs::metadata(&absolute_path)
            .await
            .map_err(|e| format!("Failed to read file metadata: {e}"))?;
        if metadata.len() > MAX_READ_SIZE_BYTES {
            return Err(format!(
                "Notebook file is too large ({} bytes, max 10MB).",
                metadata.len()
            ));
        }
        let blocks = crate::notebook::read_notebook(&absolute_path).await?;

        // Update file state
        let raw = fs::read_to_string(&absolute_path).await.unwrap_or_default();
        let mtime_ms = util::get_file_mtime_ms(&absolute_path)
            .await
            .map_err(|e| format!("Failed to get file mtime: {e}"))?;
        file_state.set(absolute_path_str, FileStateEntry {
            content: util::normalize_line_endings(&raw),
            timestamp: mtime_ms,
            offset,
            limit,
            is_partial_view: false,
        }).await;

        return Ok(ReadOutput::Notebook(blocks));
    }

    // PDF files
    if ext.eq_ignore_ascii_case("pdf") {
        return read_pdf(&absolute_path, pages).await;
    }

    // Binary file rejection
    if has_extension_in(&absolute_path, BINARY_EXTENSIONS) {
        return Err(format!(
            "This tool cannot read binary files. The file appears to be a binary .{ext} file. \
             Please use appropriate tools for binary file analysis."
        ));
    }

    // Check file size before reading text
    let metadata = fs::metadata(&absolute_path)
        .await
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if metadata.len() > MAX_READ_SIZE_BYTES {
        return Err(format!(
            "File is too large to read ({} bytes, max 10MB). \
             Consider reading specific line ranges with offset and limit.",
            metadata.len()
        ));
    }

    let raw_content = fs::read_to_string(&absolute_path)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let content = util::normalize_line_endings(&raw_content);
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let start_index = match offset {
        Some(0) | None => 0,
        Some(n) => (n.saturating_sub(1)).min(total_lines),
    };

    let end_index = match limit {
        Some(l) => start_index.saturating_add(l).min(total_lines),
        None => total_lines,
    };

    let selected = lines[start_index..end_index].join("\n");
    let num_lines = end_index.saturating_sub(start_index);
    let start_line = start_index.saturating_add(1);

    let mtime_ms = util::get_file_mtime_ms(&absolute_path)
        .await
        .map_err(|e| format!("Failed to get file mtime: {e}"))?;

    file_state.set(absolute_path_str.clone(), FileStateEntry {
        content: content.clone(),
        timestamp: mtime_ms,
        offset,
        limit,
        is_partial_view: false,
    }).await;

    let output = ReadFileJsonOutput {
        kind: "text".into(),
        file: TextFilePayload {
            file_path: absolute_path_str,
            content: selected,
            num_lines,
            start_line,
            total_lines,
        },
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output: {e}"))?;
    Ok(ReadOutput::Text(json))
}

/// Parse a PDF page range string like "1-5", "3", "10-20", "3-".
/// Returns (first_page, last_page) where last_page may be usize::MAX for open-ended ranges.
fn parse_pdf_page_range(pages: &str) -> Result<(usize, usize), String> {
    let pages = pages.trim();
    if let Some((first, last)) = pages.split_once('-') {
        let first: usize = first.trim().parse()
            .map_err(|_| format!("Invalid pages parameter: \"{pages}\". Use formats like \"1-5\", \"3\", or \"10-20\". Pages are 1-indexed."))?;
        if first == 0 {
            return Err("Pages are 1-indexed. Use 1 for the first page.".into());
        }
        let last_str = last.trim();
        if last_str.is_empty() {
            return Ok((first, usize::MAX)); // open-ended
        }
        let last: usize = last_str.parse()
            .map_err(|_| format!("Invalid pages parameter: \"{pages}\". Use formats like \"1-5\", \"3\", or \"10-20\"."))?;
        if last < first {
            return Err(format!("Invalid page range: last page ({last}) is before first page ({first})."));
        }
        Ok((first, last))
    } else {
        let page: usize = pages.parse()
            .map_err(|_| format!("Invalid pages parameter: \"{pages}\". Use formats like \"1-5\", \"3\", or \"10-20\". Pages are 1-indexed."))?;
        if page == 0 {
            return Err("Pages are 1-indexed. Use 1 for the first page.".into());
        }
        Ok((page, page))
    }
}

/// Get PDF page count using pdfinfo.
async fn get_pdf_page_count(path: &std::path::Path) -> Result<usize, String> {
    let output = tokio::process::Command::new("pdfinfo")
        .arg(path.to_string_lossy().as_ref())
        .output()
        .await
        .map_err(|_| "pdfinfo not available. Install poppler-utils.".to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(count_str) = line.strip_prefix("Pages:") {
            if let Ok(count) = count_str.trim().parse::<usize>() {
                return Ok(count);
            }
        }
    }

    // Fallback: assume it's small enough
    Ok(1)
}

/// Extract PDF pages as JPEG images using pdftoppm.
async fn read_pdf(path: &std::path::Path, pages: Option<&str>) -> Result<ReadOutput, String> {
    let file_size = fs::metadata(path)
        .await
        .map_err(|e| format!("Failed to read PDF metadata: {e}"))?
        .len();
    if file_size > PDF_MAX_EXTRACT_SIZE {
        return Err(format!("PDF file is too large ({file_size} bytes, max 100MB)."));
    }

    // Check if pdftoppm is available
    let pdftoppm_check = tokio::process::Command::new("pdftoppm")
        .arg("-v")
        .output()
        .await;
    if pdftoppm_check.is_err() {
        return Err("PDF reading requires pdftoppm (from poppler-utils). Install it with: apt install poppler-utils (Linux), brew install poppler (macOS), or pacman -S poppler (MSYS2).".into());
    }

    // Parse pages parameter
    let (first_page, last_page) = if let Some(pages_str) = pages {
        parse_pdf_page_range(pages_str)?
    } else {
        // Get page count first using pdfinfo
        let page_count = get_pdf_page_count(path).await?;
        if page_count > 10 {
            return Err(format!(
                "PDF has {page_count} pages. Please specify a page range using the 'pages' parameter (max {PDF_MAX_PAGES_PER_READ} pages per request). Example: pages=\"1-10\""
            ));
        }
        (1, page_count)
    };

    // Enforce max pages
    let effective_last = if last_page == usize::MAX {
        first_page + PDF_MAX_PAGES_PER_READ - 1
    } else {
        last_page.min(first_page + PDF_MAX_PAGES_PER_READ - 1)
    };
    if last_page != usize::MAX && (last_page - first_page + 1) > PDF_MAX_PAGES_PER_READ {
        return Err(format!(
            "Page range exceeds maximum of {PDF_MAX_PAGES_PER_READ} pages per request. Please use a smaller range."
        ));
    }

    // Create temp dir for output images
    let tmp_dir = std::env::temp_dir().join(format!(
        "objectiveai-mcp-pdf-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    ));
    fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| format!("Failed to create temp dir for PDF: {e}"))?;

    // Run pdftoppm to extract pages as JPEG
    let mut cmd = tokio::process::Command::new("pdftoppm");
    cmd.arg("-jpeg")
        .arg("-r").arg("150")  // 150 DPI
        .arg("-f").arg(first_page.to_string())
        .arg("-l").arg(effective_last.to_string())
        .arg(path.to_string_lossy().as_ref())
        .arg(tmp_dir.join("page").to_string_lossy().as_ref());

    let output = cmd.output()
        .await
        .map_err(|e| format!("Failed to run pdftoppm: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_dir_all(&tmp_dir).await;
        return Err(format!("pdftoppm failed: {stderr}"));
    }

    // Collect the generated JPEG files
    let mut image_files: Vec<std::path::PathBuf> = Vec::new();
    let mut entries = fs::read_dir(&tmp_dir)
        .await
        .map_err(|e| format!("Failed to read temp dir: {e}"))?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_path = entry.path();
        let is_jpg = entry_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "jpg")
            .unwrap_or(false);
        if is_jpg {
            image_files.push(entry_path);
        }
    }

    // Sort by filename to get pages in order
    image_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    if image_files.is_empty() {
        let _ = fs::remove_dir_all(&tmp_dir).await;
        return Err("PDF extraction produced no pages. The PDF may be empty or corrupted.".into());
    }

    // Read each image and build notebook-style blocks (reuse NotebookBlock)
    use crate::notebook::NotebookBlock;
    let mut blocks = Vec::new();

    for (i, entry_path) in image_files.iter().enumerate() {
        let img_bytes = fs::read(entry_path)
            .await
            .map_err(|e| format!("Failed to read extracted page: {e}"))?;
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &img_bytes,
        );
        // Add a text label before each page
        blocks.push(NotebookBlock::Text(format!("Page {}:", first_page + i)));
        blocks.push(NotebookBlock::Image {
            base64: b64,
            media_type: "image/jpeg".to_string(),
        });
    }

    // Clean up temp dir
    let _ = fs::remove_dir_all(&tmp_dir).await;

    Ok(ReadOutput::Notebook(blocks))
}
