use std::path::{Path, PathBuf};

use glob::Pattern;
use regex::RegexBuilder;
use tokio::fs;
use walkdir::WalkDir;

use crate::util;

#[derive(Debug, serde::Serialize)]
pub struct GrepSearchOutput {
    pub mode: Option<String>,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub filenames: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(rename = "numLines", skip_serializing_if = "Option::is_none")]
    pub num_lines: Option<usize>,
    #[serde(rename = "numMatches", skip_serializing_if = "Option::is_none")]
    pub num_matches: Option<usize>,
    #[serde(rename = "appliedLimit", skip_serializing_if = "Option::is_none")]
    pub applied_limit: Option<usize>,
    #[serde(rename = "appliedOffset", skip_serializing_if = "Option::is_none")]
    pub applied_offset: Option<usize>,
}

pub struct GrepSearchInput {
    pub pattern: String,
    pub path: Option<String>,
    pub glob: Option<String>,
    pub output_mode: Option<String>,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub context_short: Option<usize>,
    pub context: Option<usize>,
    pub line_numbers: Option<bool>,
    pub case_insensitive: Option<bool>,
    pub file_type: Option<String>,
    pub head_limit: Option<usize>,
    pub offset: Option<usize>,
    pub multiline: Option<bool>,
}

pub async fn grep_search(input: &GrepSearchInput) -> Result<String, String> {
    let base_path = match &input.path {
        Some(p) => util::normalize_path(p).await.map_err(|e| format!("Invalid path: {e}"))?,
        None => std::env::current_dir().map_err(|e| format!("Failed to get CWD: {e}"))?,
    };

    let regex = RegexBuilder::new(&input.pattern)
        .case_insensitive(input.case_insensitive.unwrap_or(false))
        .dot_matches_new_line(input.multiline.unwrap_or(false))
        .build()
        .map_err(|e| format!("Invalid regex pattern: {e}"))?;

    // Split glob on whitespace/commas (preserving brace patterns), matching ripgrep CLI behavior
    let glob_filters: Vec<Pattern> = match &input.glob {
        Some(glob) => {
            let mut patterns = Vec::new();
            for raw in glob.split_whitespace() {
                if raw.contains('{') && raw.contains('}') {
                    patterns.push(Pattern::new(raw).map_err(|e| format!("Invalid glob filter: {e}"))?);
                } else {
                    for part in raw.split(',').filter(|s| !s.is_empty()) {
                        patterns.push(Pattern::new(part).map_err(|e| format!("Invalid glob filter: {e}"))?);
                    }
                }
            }
            patterns
        }
        None => Vec::new(),
    };

    let file_type = input.file_type.as_deref();
    let output_mode = input
        .output_mode
        .clone()
        .unwrap_or_else(|| "files_with_matches".into());
    let (ctx_before, ctx_after) = if let Some(c) = input.context.or(input.context_short) {
        (c, c) // -C overrides both -B and -A
    } else {
        (input.before.unwrap_or(0), input.after.unwrap_or(0))
    };

    let to_relative = |p: &Path| -> String {
        pathdiff::diff_paths(p, &base_path)
            .unwrap_or_else(|| p.to_path_buf())
            .to_string_lossy()
            .into_owned()
    };

    let mut filenames = Vec::new();
    let mut content_lines = Vec::new();
    let mut total_matches = 0usize;
    let mut count_lines: Vec<String> = Vec::new();
    let mut count_file_count = 0usize;

    // For files_with_matches mode, collect (path, mtime) pairs for sorting
    let mut file_mtimes: Vec<(String, Option<std::time::SystemTime>)> = Vec::new();

    for file_path in collect_search_files(&base_path).await.map_err(|e| format!("Search failed: {e}"))? {
        if !matches_filters(&file_path, &glob_filters, file_type) {
            continue;
        }

        let Ok(file_contents) = fs::read_to_string(&file_path).await else {
            continue; // Skip files that can't be read as text
        };

        // Skip binary files (null byte detection, matching ripgrep's heuristic)
        if file_contents.as_bytes().contains(&0) {
            continue;
        }

        let rel_path = to_relative(&file_path);

        if output_mode == "count" {
            let count = regex.find_iter(&file_contents).count();
            if count > 0 {
                count_lines.push(format!("{rel_path}:{count}"));
                count_file_count += 1;
                total_matches += count;
            }
            continue;
        }

        let lines: Vec<&str> = file_contents.lines().collect();
        let mut matched_lines = Vec::new();
        if input.multiline.unwrap_or(false) {
            // Match against full content, find all match positions
            for mat in regex.find_iter(&file_contents) {
                let start_byte = mat.start();
                // Count newlines before match start to get line number
                let line_num = file_contents[..start_byte].matches('\n').count();
                if !matched_lines.contains(&line_num) {
                    matched_lines.push(line_num);
                }
                total_matches += 1;
            }
        } else {
            for (index, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    total_matches += 1;
                    matched_lines.push(index);
                }
            }
        }

        if matched_lines.is_empty() {
            continue;
        }

        if output_mode == "files_with_matches" {
            let mtime = match fs::metadata(&file_path).await {
                Ok(m) => m.modified().ok(),
                Err(_) => None,
            };
            file_mtimes.push((rel_path, mtime));
            continue;
        }
        filenames.push(rel_path.clone());

        if output_mode == "content" {
            // Track which lines have been emitted to avoid duplicates from overlapping context
            let mut emitted_lines: std::collections::HashSet<usize> = std::collections::HashSet::new();
            for index in matched_lines {
                let start = index.saturating_sub(ctx_before);
                let end = (index + ctx_after + 1).min(lines.len());
                for (current, line) in lines.iter().enumerate().take(end).skip(start) {
                    if !emitted_lines.insert(current) {
                        continue; // Already emitted this line
                    }
                    let prefix = if input.line_numbers.unwrap_or(true) {
                        format!("{rel_path}:{}:", current + 1)
                    } else {
                        format!("{rel_path}:")
                    };
                    let truncated_line = if line.len() > 500 { &line[..500] } else { line };
                    content_lines.push(format!("{prefix}{truncated_line}"));
                }
            }
        }
    }

    if output_mode == "content" {
        let (lines, limit, offset) = apply_limit(content_lines, input.head_limit, input.offset);
        let output = GrepSearchOutput {
            mode: Some(output_mode),
            num_files: 0,
            filenames: Vec::new(),
            num_lines: Some(lines.len()),
            content: Some(lines.join("\n")),
            num_matches: None,
            applied_limit: limit,
            applied_offset: offset,
        };
        return serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {e}"));
    }

    if output_mode == "count" {
        let (limited_count_lines, applied_limit, applied_offset) =
            apply_limit(count_lines, input.head_limit, input.offset);

        // Recompute file count and matches from LIMITED data
        let limited_file_count = limited_count_lines.len();
        let limited_matches: usize = limited_count_lines
            .iter()
            .filter_map(|line| line.rsplit(':').next()?.parse::<usize>().ok())
            .sum();

        let output = GrepSearchOutput {
            mode: Some("count".into()),
            num_files: limited_file_count,
            filenames: Vec::new(),
            num_lines: None,
            content: Some(limited_count_lines.join("\n")),
            num_matches: Some(limited_matches),
            applied_limit,
            applied_offset,
        };
        return serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize output: {e}"));
    }

    // For files_with_matches, sort by mtime (newest first) with alphabetical tiebreaker
    if output_mode == "files_with_matches" {
        file_mtimes.sort_by(|(a_name, a_time), (b_name, b_time)| {
            match (b_time, a_time) {
                (Some(b_t), Some(a_t)) => {
                    let time_cmp = b_t.cmp(a_t); // newest first
                    if time_cmp == std::cmp::Ordering::Equal {
                        a_name.cmp(b_name) // alphabetical tiebreaker
                    } else {
                        time_cmp
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Greater, // b has mtime, a doesn't → a (no mtime) sorts to end
                (None, Some(_)) => std::cmp::Ordering::Less,    // a has mtime, b doesn't → b (no mtime) sorts to end
                (None, None) => a_name.cmp(b_name), // alphabetical when both None
            }
        });
        filenames = file_mtimes.into_iter().map(|(name, _)| name).collect();
    }

    let (filenames, applied_limit, applied_offset) =
        apply_limit(filenames, input.head_limit, input.offset);

    let output = GrepSearchOutput {
        mode: Some(output_mode.clone()),
        num_files: filenames.len(),
        filenames,
        content: None,
        num_lines: None,
        num_matches: None,
        applied_limit,
        applied_offset,
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| format!("Failed to serialize output: {e}"))
}

const VCS_DIRS: &[&str] = &[".git", ".svn", ".hg", ".bzr", ".jj", ".sl"];

fn is_vcs_dir(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_dir() {
        let name = entry.file_name().to_string_lossy();
        return VCS_DIRS.iter().any(|&vcs| name == vcs);
    }
    false
}

async fn collect_search_files(base_path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let base_metadata = fs::metadata(base_path).await?;
    if base_metadata.is_file() {
        return Ok(vec![base_path.to_path_buf()]);
    }

    // walkdir's iterator is synchronous; run it on a blocking thread so we
    // don't tie up the runtime while traversing very large trees.
    let base_path = base_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        let walker = WalkDir::new(&base_path).into_iter().filter_entry(|e| !is_vcs_dir(e));
        for entry in walker {
            let entry = entry.map_err(|e| std::io::Error::other(e.to_string()))?;
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }
        Ok(files)
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}

fn type_to_extensions(file_type: &str) -> Option<&'static [&'static str]> {
    match file_type {
        "rust" | "rs" => Some(&["rs"]),
        "python" | "py" => Some(&["py", "pyi", "pyw"]),
        "javascript" | "js" => Some(&["js", "mjs", "cjs", "jsx"]),
        "typescript" | "ts" => Some(&["ts", "mts", "cts", "tsx"]),
        "java" => Some(&["java"]),
        "c" => Some(&["c", "h"]),
        "cpp" => Some(&["cpp", "cxx", "cc", "c++", "hpp", "hxx", "hh", "h++"]),
        "go" => Some(&["go"]),
        "ruby" | "rb" => Some(&["rb", "rbw"]),
        "php" => Some(&["php", "php3", "php4", "php5", "phtml"]),
        "swift" => Some(&["swift"]),
        "kotlin" | "kt" => Some(&["kt", "kts"]),
        "scala" => Some(&["scala", "sc"]),
        "r" => Some(&["r", "R", "Rmd"]),
        "shell" | "sh" | "bash" => Some(&["sh", "bash", "zsh", "fish"]),
        "html" => Some(&["html", "htm", "xhtml"]),
        "css" => Some(&["css", "scss", "sass", "less"]),
        "json" => Some(&["json", "jsonl", "geojson"]),
        "yaml" | "yml" => Some(&["yaml", "yml"]),
        "toml" => Some(&["toml"]),
        "xml" => Some(&["xml", "xsl", "xslt", "svg"]),
        "markdown" | "md" => Some(&["md", "markdown", "mkd"]),
        "sql" => Some(&["sql"]),
        "lua" => Some(&["lua"]),
        "perl" | "pl" => Some(&["pl", "pm", "t"]),
        "haskell" | "hs" => Some(&["hs", "lhs"]),
        "elixir" | "ex" => Some(&["ex", "exs"]),
        "erlang" | "erl" => Some(&["erl", "hrl"]),
        "clojure" | "clj" => Some(&["clj", "cljs", "cljc", "edn"]),
        "dart" => Some(&["dart"]),
        "zig" => Some(&["zig"]),
        "nim" => Some(&["nim"]),
        "protobuf" | "proto" => Some(&["proto"]),
        "graphql" | "gql" => Some(&["graphql", "gql"]),
        "dockerfile" => Some(&["Dockerfile"]),
        "make" => Some(&["mk", "mak"]),
        "cmake" => Some(&["cmake"]),
        "tf" | "terraform" => Some(&["tf", "tfvars"]),
        "csharp" | "cs" => Some(&["cs"]),
        _ => None, // Fall back to treating it as a literal extension
    }
}

fn matches_filters(path: &Path, glob_filters: &[Pattern], file_type: Option<&str>) -> bool {
    if !glob_filters.is_empty() && !glob_filters.iter().any(|p| p.matches_path(path)) {
        return false;
    }

    if let Some(file_type) = file_type {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if let Some(extensions) = type_to_extensions(file_type) {
            if !extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension)) {
                return false;
            }
        } else {
            // Treat as literal extension
            if !extension.eq_ignore_ascii_case(file_type) {
                return false;
            }
        }
    }

    true
}

fn apply_limit<T>(
    items: Vec<T>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> (Vec<T>, Option<usize>, Option<usize>) {
    let offset_value = offset.unwrap_or(0);
    let mut items: Vec<T> = items.into_iter().skip(offset_value).collect();
    let explicit_limit = limit.unwrap_or(250);
    if explicit_limit == 0 {
        return (items, None, (offset_value > 0).then_some(offset_value));
    }

    let truncated = items.len() > explicit_limit;
    items.truncate(explicit_limit);
    (
        items,
        truncated.then_some(explicit_limit),
        (offset_value > 0).then_some(offset_value),
    )
}
