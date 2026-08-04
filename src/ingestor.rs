use crate::model::{CodeSnippet, Solution};
use crate::storage::Database;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_and_ingest(db: &Database) -> Result<usize> {
    let search_paths = get_log_directories();
    let mut ingested_count = 0;

    for dir in search_paths {
        if dir.exists() && dir.is_dir() {
            if let Ok(count) = scan_directory(&dir, db) {
                ingested_count += count;
            }
        }
    }

    Ok(ingested_count)
}

fn get_log_directories() -> Vec<PathBuf> {
    let mut dirs_list = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // GitHub Copilot CLI standard locations
        dirs_list.push(home.join(".config").join("github-copilot"));
        dirs_list.push(home.join(".local").join("share").join("github-copilot"));

        // agy CLI (Google Antigravity CLI) locations
        dirs_list.push(home.join(".gemini").join("antigravity-cli").join("brain"));
        dirs_list.push(home.join(".config").join("antigravity-cli"));
        dirs_list.push(home.join(".antigravity"));

        // Windows AppData locations
        if cfg!(target_os = "windows") {
            if let Ok(appdata) = std::env::var("APPDATA") {
                dirs_list.push(PathBuf::from(appdata).join("github-copilot"));
                dirs_list.push(PathBuf::from(appdata).join("antigravity-cli"));
            }
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                dirs_list.push(PathBuf::from(localappdata).join("github-copilot"));
                dirs_list.push(PathBuf::from(localappdata).join("antigravity-cli"));
            }
        }
    }
    dirs_list
}

fn scan_directory(dir: &Path, db: &Database) -> Result<usize> {
    let mut count = 0;
    let entries = fs::read_dir(dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(sub_count) = scan_directory(&path, db) {
                count += sub_count;
            }
        } else if is_transcript_file(&path) {
            if let Ok(solution) = parse_transcript_file(&path) {
                if db.insert_solution(&solution).is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

fn is_transcript_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy();
        ext_str == "json" || ext_str == "jsonl" || ext_str == "log"
    } else {
        false
    }
}

fn parse_transcript_file(path: &Path) -> Result<Solution> {
    let content = fs::read_to_string(path)?;
    let mut prompt_summary = String::new();
    let mut commands = Vec::new();
    let mut code_snippets = Vec::new();

    // Parse JSON Lines format (supports both Copilot CLI and agy CLI transcripts)
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

            // Extract user input / prompt (agy CLI: USER_INPUT)
            if msg_type == "USER_INPUT" || prompt_summary.is_empty() {
                if let Some(prompt) = val.get("content").and_then(|c| c.as_str()) {
                    if prompt_summary.is_empty() && prompt.len() > 2 {
                        let clean_prompt = prompt.lines().next().unwrap_or(prompt);
                        if !clean_prompt.starts_with('<') {
                            prompt_summary = clean_prompt.to_string();
                        }
                    }
                }
            }

            // Extract tool calls / commands (agy CLI: run_command)
            if let Some(tool_calls) = val.get("tool_calls").and_then(|t| t.as_array()) {
                for tool in tool_calls {
                    if let Some(cmd) = tool.get("CommandLine").and_then(|c| c.as_str()) {
                        if !commands.contains(&cmd.to_string()) {
                            commands.push(cmd.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback markdown code block extraction
    extract_code_blocks(&content, &mut code_snippets, &mut commands);

    if prompt_summary.is_empty() {
        prompt_summary = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Copilot Session".to_string());
    }

    let timestamp = fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
        .unwrap_or_else(|_| chrono::Utc::now().timestamp());

    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    Ok(Solution {
        id: 0,
        prompt_summary,
        commands,
        code_snippets,
        project_path,
        git_repo: None,
        is_pinned: false,
        timestamp,
    })
}

fn extract_code_blocks(content: &str, snippets: &mut Vec<CodeSnippet>, commands: &mut Vec<String>) {
    let mut in_block = false;
    let mut lang = String::new();
    let mut current_block = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if in_block {
                in_block = false;
                let trimmed = current_block.trim();
                if !trimmed.is_empty() {
                    if lang == "bash" || lang == "sh" || lang == "powershell" || lang == "cmd" {
                        commands.push(trimmed.to_string());
                    } else {
                        snippets.push(CodeSnippet {
                            language: if lang.is_empty() { "text".to_string() } else { lang.clone() },
                            code: trimmed.to_string(),
                        });
                    }
                }
                current_block.clear();
            } else {
                in_block = true;
                lang = line.trim_start_matches("```").trim().to_string();
            }
        } else if in_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }
}
