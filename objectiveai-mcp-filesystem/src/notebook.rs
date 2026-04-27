use std::path::Path;

/// A content block from a notebook -- either text or an embedded image.
pub enum NotebookBlock {
    Text(String),
    Image { base64: String, media_type: String },
}

#[derive(serde::Deserialize)]
struct Notebook {
    cells: Vec<Cell>,
    #[serde(default)]
    metadata: NotebookMetadata,
}

#[derive(serde::Deserialize, Default)]
struct NotebookMetadata {
    #[serde(default)]
    language_info: Option<LanguageInfo>,
    #[serde(default)]
    kernelspec: Option<KernelSpec>,
}

#[derive(serde::Deserialize)]
struct LanguageInfo {
    name: Option<String>,
}

#[derive(serde::Deserialize)]
struct KernelSpec {
    language: Option<String>,
}

#[derive(serde::Deserialize)]
struct Cell {
    cell_type: String,
    source: CellSource,
    #[serde(default)]
    execution_count: Option<serde_json::Value>,
    #[serde(default)]
    outputs: Vec<CellOutput>,
}

/// Cell source can be a single string or array of strings.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum CellSource {
    String(String),
    Lines(Vec<String>),
}

impl Default for CellSource {
    fn default() -> Self {
        CellSource::String(String::new())
    }
}

impl CellSource {
    fn as_str(&self) -> String {
        match self {
            CellSource::String(s) => s.clone(),
            CellSource::Lines(lines) => lines.join(""),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "output_type")]
enum CellOutput {
    #[serde(rename = "stream")]
    Stream {
        #[serde(default)]
        text: CellSource,
    },
    #[serde(rename = "execute_result")]
    ExecuteResult {
        #[serde(default)]
        data: OutputData,
    },
    #[serde(rename = "display_data")]
    DisplayData {
        #[serde(default)]
        data: OutputData,
    },
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        ename: String,
        #[serde(default)]
        evalue: String,
        #[serde(default)]
        traceback: Vec<String>,
    },
}

#[derive(serde::Deserialize, Default)]
struct OutputData {
    #[serde(rename = "text/plain")]
    text_plain: Option<CellSource>,
    #[serde(rename = "image/png")]
    image_png: Option<String>,
    #[serde(rename = "image/jpeg")]
    image_jpeg: Option<String>,
}

impl NotebookMetadata {
    fn language(&self) -> &str {
        self.language_info
            .as_ref()
            .and_then(|li| li.name.as_deref())
            .or_else(|| self.kernelspec.as_ref().and_then(|ks| ks.language.as_deref()))
            .unwrap_or("python")
    }
}

/// Read and parse a Jupyter notebook, returning content blocks.
pub async fn read_notebook(path: &Path) -> Result<Vec<NotebookBlock>, String> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read notebook: {e}"))?;
    let notebook: Notebook = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse notebook JSON: {e}"))?;

    let language = notebook.metadata.language().to_string();
    let mut blocks: Vec<NotebookBlock> = Vec::new();
    let mut text_buf = String::new();

    for (i, cell) in notebook.cells.iter().enumerate() {
        let source = cell.source.as_str();

        // Cell header
        if i > 0 {
            text_buf.push('\n');
        }

        match cell.cell_type.as_str() {
            "code" => {
                // Execution count label
                if let Some(count) = &cell.execution_count {
                    if let Some(n) = count.as_u64() {
                        text_buf.push_str(&format!("In [{n}]:\n"));
                    } else {
                        text_buf.push_str("In [ ]:\n");
                    }
                }
                text_buf.push_str(&format!("```{language}\n{source}\n```\n"));

                // Process outputs
                for output in &cell.outputs {
                    process_output(output, &mut blocks, &mut text_buf);
                }
            }
            "markdown" => {
                text_buf.push_str(&source);
                text_buf.push('\n');
            }
            "raw" => {
                text_buf.push_str(&source);
                text_buf.push('\n');
            }
            _ => {
                text_buf.push_str(&source);
                text_buf.push('\n');
            }
        }
    }

    // Flush remaining text
    if !text_buf.is_empty() {
        blocks.push(NotebookBlock::Text(text_buf));
    }

    Ok(blocks)
}

fn process_output(output: &CellOutput, blocks: &mut Vec<NotebookBlock>, text_buf: &mut String) {
    match output {
        CellOutput::Stream { text } => {
            text_buf.push_str(&text.as_str());
        }
        CellOutput::ExecuteResult { data } | CellOutput::DisplayData { data } => {
            // Check for embedded images first
            if let Some(png_data) = &data.image_png {
                // Flush text before image
                if !text_buf.is_empty() {
                    blocks.push(NotebookBlock::Text(std::mem::take(text_buf)));
                }
                blocks.push(NotebookBlock::Image {
                    base64: png_data.trim().to_string(),
                    media_type: "image/png".to_string(),
                });
            } else if let Some(jpeg_data) = &data.image_jpeg {
                if !text_buf.is_empty() {
                    blocks.push(NotebookBlock::Text(std::mem::take(text_buf)));
                }
                blocks.push(NotebookBlock::Image {
                    base64: jpeg_data.trim().to_string(),
                    media_type: "image/jpeg".to_string(),
                });
            } else if let Some(text_data) = &data.text_plain {
                text_buf.push_str("Out:\n");
                text_buf.push_str(&text_data.as_str());
                text_buf.push('\n');
            }
        }
        CellOutput::Error { ename, evalue, traceback } => {
            text_buf.push_str(&format!("Error: {ename}: {evalue}\n"));
            for line in traceback {
                text_buf.push_str(line);
                text_buf.push('\n');
            }
        }
    }
}
