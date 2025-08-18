// main.rs

// ----- default config written on `init` -----
static CONFIG_TEXT: &str = "# Number of notes to show when running `codemap show`
default_note_count = 3

# Timestamp format for logs (unused for now)
timestamp_format = \"iso8601\"

# Optional: Project tag for filtering later (future feature)
project_name = \"my-project\"

# LLM (optional)
llm_provider = \"openai\"        # openai | cmd (cmd not used in MVP)
llm_model = \"gpt-4o-mini\"
llm_api_key = \"\"               # or set OPENAI_API_KEY in env
max_prompt_chars = 4000
";

use anyhow::{anyhow, Result};
use clap::Parser;
use regex::Regex;
use rprompt::prompt_reply;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;
use std::time::Instant;

// ---------- Metric Measuring ---------- 
fn diff_function_counts(old: &ProjectContext, new: &ProjectContext) -> (usize, usize, usize, usize) {
    // returns (total, added, modified, unchanged)
    let mut total = 0usize;
    let mut added = 0usize;
    let mut modified = 0usize;
    let mut unchanged = 0usize;

    for (path, newf) in &new.files {
        total += newf.functions.len();
        match old.files.get(path) {
            None => {
                // all functions in new file are "added"
                added += newf.functions.len();
            }
            Some(oldf) => {
                let mut old_by_name: HashMap<&str, &FunctionInfo> = HashMap::new();
                for of in &oldf.functions {
                    old_by_name.insert(of.name.as_str(), of);
                }
                for nf in &newf.functions {
                    match old_by_name.get(nf.name.as_str()) {
                        None => added += 1,
                        Some(of) => {
                            if of.hash == nf.hash { unchanged += 1; } else { modified += 1; }
                        }
                    }
                }
            }
        }
    }

    (total, added, modified, unchanged)
}

fn write_metrics(m: &UpdateMetrics) -> Result<()> {
    let path = Path::new(".codemap/").join("metrics.json");
    let prev: Vec<UpdateMetrics> = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut all = prev;
    all.push(m.clone());
    fs::write(path, serde_json::to_string_pretty(&all)?)?;
    Ok(())
}


// ---------- Data Models ----------

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct FileDiffMetrics {
    added: usize,
    modified: usize,
    removed: usize,
    unchanged: usize,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct UpdateMetrics {
    total_files: usize,
    total_functions: usize,
    added_functions: usize,
    modified_functions: usize,
    removed_functions: usize,
    unchanged_functions: usize,
    reuse_ratio: f32,  // unchanged / (unchanged + modified)
    duration_ms: u128, // wall clock
}

#[derive(Serialize, Deserialize, Clone)]
struct EntryPoint {
    path: String,
    rank: u8,
    reason: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ProjectContext {
    folders: HashMap<String, FolderNode>,
    files: HashMap<String, FileContext>,
    #[serde(default)]
    entry_points: Vec<EntryPoint>,
    #[serde(default)]
    project_brief: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FolderNode {
    children: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FileContext {
    language: String,
    functions: Vec<FunctionInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FunctionInfo {
    name: String,
    line: usize,
    summary: Option<String>,
    hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    default_note_count: Option<usize>,
    timestamp_format: Option<String>,
    project_name: Option<String>,

    // LLM
    llm_provider: Option<String>,   // "openai"
    llm_model: Option<String>,      // "gpt-4o-mini"
    llm_api_key: Option<String>,    // or env
    max_prompt_chars: Option<usize> // 4000 default
}

impl Config {
    fn load() -> Result<Config> {
        let path = Path::new(".codemap/").join("config.toml");
        let content = fs::read_to_string(&path)
            .map_err(|_| anyhow!(".codemap/config.toml is missing (run `codemap init`)"))?;
        Ok(toml::from_str(&content)?)
    }
}

fn save_config(cfg: &Config) -> Result<()> {
    let path = Path::new(".codemap/").join("config.toml");
    fs::write(&path, toml::to_string_pretty(cfg)?)?;
    Ok(())
}

fn get_api_key(cfg: &Config) -> Option<String> {
    std::env::var("OPENAI_API_KEY")
        .ok()
        .or_else(|| cfg.llm_api_key.clone())
}

// ---------- CLI ----------

#[derive(Parser)]
#[command(name = "codemap", about = "A CLI for understanding codebases")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
#[command(rename_all = "lowercase")]
enum Commands {
    Init,
    Update,
    Show { num: Option<i32> },
    Summary,
    Auth { key: Option<String> }, // set OpenAI key
    Edit,
    Delete,
}

// ---------- Core Ops ----------

fn init() -> Result<()> {
    let path = Path::new(".codemap/");
    let log_path = path.join("log.json");
    let config_path = path.join("config.toml");

    if log_path.exists() && config_path.exists() {
        println!("codemap already initialized.");
        return Ok(());
    }

    fs::create_dir_all(path)?;
    println!("Created ./.codemap");

    File::create(&log_path)?;
    println!("Wrote .codemap/log.json");

    let mut file = File::create(&config_path)?;
    file.write_all(CONFIG_TEXT.as_bytes())?;
    println!("Wrote .codemap/config.toml");

    // initial context
    let context_path = path.join("context.json");
    let context_data = build_context()?;
    let json = serde_json::to_string_pretty(&context_data)?;
    fs::write(context_path, json)?;
    println!("Wrote .codemap/context.json");

    Ok(())
}

fn build_context() -> Result<ProjectContext> {
    let mut folders: HashMap<String, FolderNode> = HashMap::new();
    let mut files: HashMap<String, FileContext> = HashMap::new();

    let ignored_dirs = ["target", ".git", "node_modules", ".venv", "__pycache__"];

    for entry in WalkDir::new(".") {
        let entry = entry?;
        let path = entry.path();

        // ignore caches
        if entry
            .path()
            .components()
            .any(|c| ignored_dirs.contains(&c.as_os_str().to_string_lossy().as_ref()))
        {
            continue;
        }

        // ignore hidden except .codemap
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with('.')
            && !entry.file_name().to_string_lossy().starts_with(".codemap")
        {
            continue;
        }

        if path.is_dir() {
            let mut children = vec![];
            for child in fs::read_dir(path)? {
                let child = child?;
                children.push(child.file_name().to_string_lossy().to_string());
            }
            let relative_path = path.strip_prefix(".")?.display().to_string();
            folders.insert(relative_path, FolderNode { children });
        }

        // inside build_context(), in the `if path.is_file()` block, replace the read_to_string usage with:
if path.is_file() {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if let Some(lang) = detect_language(ext) {
            // Read raw bytes (handles binary/invalid-UTF8 safely)
            match fs::read(path) {
                Ok(bytes) => {
                    let contents = String::from_utf8_lossy(&bytes); // contents: Cow<str>
                    let functions = extract_functions(&contents, &lang);
                    let rel = path.strip_prefix(".")?.display().to_string();
                    files.insert(
                        rel,
                        FileContext {
                            language: lang,
                            functions,
                        },
                    );
                }
                Err(_) => {
                    // unreadable file -> skip
                }
            }
        }
    }
}

    }

    Ok(ProjectContext {
        folders,
        files,
        entry_points: Vec::new(),
        project_brief: None,
    })
}

fn detect_language(extension: &str) -> Option<String> {
    match extension.to_ascii_lowercase().as_str() {
        "rs" => Some("rust".into()),
        "py" => Some("python".into()),
        "js" => Some("javascript".into()),
        "ts" => Some("typescript".into()),
        "java" => Some("java".into()),
        "go" => Some("go".into()),
        "cpp" | "cc" | "cxx" | "c++" => Some("cpp".into()),
        "c" => Some("c".into()),
        _ => None,
    }
}

fn extract_functions(source: &str, lang: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    if lang == "rust" {
        let re = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap();
        let lines: Vec<&str> = source.lines().collect();

        for i in 0..lines.len() {
            if let Some(caps) = re.captures(lines[i]) {
                let name = caps.get(3).unwrap().as_str().to_string();

                // capture body to hash
                let mut body = String::new();
                let mut open = 0;
                let mut found = false;

                for j in i..lines.len() {
                    let line = lines[j];
                    body.push_str(line);
                    body.push('\n');

                    for c in line.chars() {
                        if c == '{' {
                            open += 1;
                            found = true;
                        } else if c == '}' {
                            open -= 1;
                        }
                    }
                    if found && open == 0 {
                        break;
                    }
                }

                let hash = hash_string(&body);

                functions.push(FunctionInfo {
                    name,
                    line: i + 1,
                    summary: None,
                    hash,
                });
            }
        }
    }

    functions
}

fn hash_string(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn load_previous_context() -> Result<ProjectContext> {
    let base = Path::new(".codemap/");
    if !base.is_dir() {
        return Err(anyhow!("Codemap not initialized. Run `codemap init`."));
    }
    let context_path = base.join("context.json");
    let json = fs::read_to_string(context_path)?;
    let context: ProjectContext = serde_json::from_str(&json)?;
    Ok(context)
}

// ---------- Merge ----------

/// merge functions within a file: add new, keep unchanged (preserve summary), reset changed, drop deleted
fn merge_file_contexts(old: &FileContext, newf: &FileContext) -> FileContext {
    let mut old_map: HashMap<&str, &FunctionInfo> = HashMap::new();
    for f in &old.functions {
        old_map.insert(f.name.as_str(), f);
    }

    let mut out = FileContext {
        language: newf.language.clone(),
        functions: Vec::new(),
    };

    for nf in &newf.functions {
        match old_map.get(nf.name.as_str()) {
            None => {
                // new function
                out.functions.push(nf.clone());
            }
            Some(of) => {
                if of.hash == nf.hash {
                    // unchanged: keep old summary
                    let mut kept = nf.clone();
                    kept.summary = of.summary.clone();
                    out.functions.push(kept);
                } else {
                    // changed: clear summary
                    let mut changed = nf.clone();
                    changed.summary = None;
                    out.functions.push(changed);
                }
            }
        }
    }
    // deletions are omitted implicitly
    out
}

/// merge projects: add new files, merge existing, drop deleted; carry over entry_points/brief from old (will be overwritten if LLM runs)
fn merge_contexts(old_ctx: &ProjectContext, new_ctx: &ProjectContext) -> ProjectContext {
    let mut merged_files = HashMap::new();

    for (file_path, new_file) in &new_ctx.files {
        match old_ctx.files.get(file_path) {
            None => {
                merged_files.insert(file_path.clone(), new_file.clone());
            }
            Some(old_file) => {
                let merged = merge_file_contexts(old_file, new_file);
                merged_files.insert(file_path.clone(), merged);
            }
        }
    }

    // note: files present in old but not in new are considered deleted (not inserted)

    ProjectContext {
        folders: new_ctx.folders.clone(),
        files: merged_files,
        entry_points: old_ctx.entry_points.clone(), // keep until LLM refreshes
        project_brief: old_ctx.project_brief.clone(),
    }
}

// ---------- Onboarding (Tier-1) ----------

fn truncate_to_bytes(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

fn push_file(
    ctx: &ProjectContext,
    path: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<(String, String)>,
    max_bytes_per_file: usize,
) {
    if seen.contains(path) || !ctx.files.contains_key(path) {
        return;
    }
    if let Ok(text) = fs::read_to_string(path) {
        let snippet = truncate_to_bytes(&text, max_bytes_per_file);
        out.push((path.to_string(), snippet));
        seen.insert(path.to_string());
    }
}


fn heuristic_reason(p: &str) -> String {
    let l = p.to_lowercase();
    if l.ends_with("src/main.rs") { return "Binary entrypoint".into(); }
    if l.ends_with("src/lib.rs")  { return "Library root".into(); }
    if l.starts_with("src/bin/")  { return "CLI subcommand entrypoint".into(); }
    if l.contains("router") || l.contains("route") { return "Routing hub".into(); }
    if l.contains("handler") { return "Request handler".into(); }
    if l.contains("server") { return "Server bootstrap".into(); }
    if l.ends_with("readme") || l.ends_with("readme.md") { return "Project docs".into(); }
    "Likely important module".into()
}


/// Pick likely onboarding files and return (path, whole-file snippet)
fn build_entry_candidates(
    ctx: &ProjectContext,
    max_files: usize,
    max_bytes_per_file: usize,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // 1) README (case-insensitive)
    if let Some(readme) = ctx.files.keys().find(|k| {
        let k = k.to_lowercase();
        k.ends_with("readme.md") || k.ends_with("readme")
    }).cloned() {
        push_file(ctx, &readme, &mut seen, &mut out, max_bytes_per_file);
        if out.len() >= max_files { return out; }
    }

    // 2) main/lib
    for p in ["src/main.rs", "src/lib.rs"] {
        push_file(ctx, p, &mut seen, &mut out, max_bytes_per_file);
        if out.len() >= max_files { return out; }
    }

    // 3) bin/*
    let mut bin_paths: Vec<String> = ctx.files.keys()
        .filter(|k| k.starts_with("src/bin/"))
        .cloned()
        .collect();
    bin_paths.sort();
    for k in bin_paths {
        push_file(ctx, &k, &mut seen, &mut out, max_bytes_per_file);
        if out.len() >= max_files { return out; }
    }

    // 4) routing/handler/server-ish files
    let pats = ["route", "router", "handler", "server", "controller", "cli", "command"];
    let mut heuristic_paths: Vec<String> = ctx.files.keys()
        .filter(|k| {
            let lk = k.to_lowercase();
            pats.iter().any(|p| lk.contains(p))
        })
        .cloned()
        .collect();
    heuristic_paths.sort();
    for k in heuristic_paths {
        push_file(ctx, &k, &mut seen, &mut out, max_bytes_per_file);
        if out.len() >= max_files { return out; }
    }

    // 5) fill remaining by function count (descending)
    let mut remaining: Vec<(&String, &FileContext)> =
        ctx.files.iter().filter(|(k, _)| !seen.contains(*k)).collect();
    remaining.sort_by_key(|(_, fc)| usize::MAX - fc.functions.len());
    for (k, _) in remaining {
        push_file(ctx, k, &mut seen, &mut out, max_bytes_per_file);
        if out.len() >= max_files { break; }
    }

    out
}

fn render_onboarding_prompt(cands: &[(String, String)], max_chars: usize) -> String {
    let mut s = String::from(
        "You are onboarding a developer to this repository.
        Return STRICT JSON with <=7 UNIQUE entries by path.
        Format: {\"entries\":[{\"path\":\"...\",\"rank\":1-10,\"reason\":\"...\"}],\"project_brief\":\"...\"}

        Rules:
        - Do not repeat a path.
        - Use higher rank for more important files.
        - Keep the brief to 3–5 sentences.

        "

    );
    for (p, txt) in cands {
        if s.len() >= max_chars {
            break;
        }
        s.push_str(p);
        s.push_str("\n---\n");
        let remaining = max_chars.saturating_sub(s.len());
        s.push_str(&truncate_to_bytes(txt, remaining));
        s.push_str("\n\n");
    }
    s
}

#[derive(serde::Deserialize)]
struct LlmOut {
    entries: Vec<EntryPoint>,
    project_brief: String,
}

fn openai_rank(prompt: &str, model: &str, api_key: &str) -> Result<LlmOut> {
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role":"system","content":"You help developers quickly onboard to codebases."},
            {"role":"user","content": prompt}
        ],
        "response_format": { "type": "json_object" }
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()?;

    let status = resp.status();
    if !status.is_success() {
        // `text()` consumes resp; that’s fine on the error path
        let txt = resp.text().unwrap_or_default();
        return Err(anyhow!("OpenAI HTTP {}: {}", status, txt));
    }

    // Success path: parse JSON (we didn't consume `resp`)
    let v: serde_json::Value = resp.json()?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("missing content in OpenAI response: {}", v))?;
    let out: LlmOut = serde_json::from_str(content)
        .map_err(|e| anyhow!("JSON parse error: {e}\ncontent: {}", content))?;
    Ok(out)
}


// ---------- Commands ----------

// === In update_context(), wrap work with timing and fill UpdateMetrics ===
fn update_context() -> Result<()> {
    let t0 = Instant::now();

    let prev_context: ProjectContext = load_previous_context()?;
    let fresh_context: ProjectContext = build_context()?;

    // merge
    let mut merged = merge_contexts(&prev_context, &fresh_context);

    // LLM onboarding (optional)
    let cfg = Config::load().unwrap_or_else(|_| toml::from_str(CONFIG_TEXT).unwrap());
    let max_chars = cfg.max_prompt_chars.unwrap_or(4000);

    // Candidate gathering
    let cands = build_entry_candidates(&merged, 15, 40_000);
    eprintln!("LLM: {} candidate files", cands.len());
    for (p, _) in cands.iter().take(10) { eprintln!("  - {}", p); }

    // Track LLM timing + outcome
    let mut llm_called = false;
    let mut llm_ok = false;
    let mut llm_ms = 0u128;
    if !cands.is_empty() {
        if let Some(api_key) = get_api_key(&cfg) {
            let model = cfg.llm_model.as_deref().unwrap_or("gpt-4o-mini");
            eprintln!("LLM: calling model={}", model);
            let prompt = render_onboarding_prompt(&cands, max_chars);
            llm_called = true;
            let t_llm = Instant::now();
            match openai_rank(&prompt, model, &api_key) {
                Ok(out) => {
                    llm_ms = t_llm.elapsed().as_millis();
                    eprintln!("LLM: success ({} entries)", out.entries.len());
                    merged.entry_points = out.entries;
                    merged.project_brief = Some(out.project_brief);
                    llm_ok = true;
                }
                Err(e) => {
                    llm_ms = t_llm.elapsed().as_millis();
                    eprintln!("LLM: error -> {e}");
                }
            }
        } else {
            eprintln!("LLM: no API key — skipping.");
        }
    }

    // Fallback heuristics if needed (unchanged)
    if merged.entry_points.is_empty() {
        let cands = build_entry_candidates(&merged, 10, 40_000);
        merged.entry_points = cands
            .iter().take(7).enumerate()
            .map(|(i, (p, _))| EntryPoint {
                path: p.clone(),
                rank: (10 - i as u8).max(1),
                reason: heuristic_reason(p),
            })
            .collect();
        if merged.project_brief.is_none() {
            merged.project_brief = Some(
                "No LLM brief available. Showing heuristic entry points to start reading the codebase.".to_string()
            );
        }
    }

    // --- Metrics aggregation ---
    let duration_ms = t0.elapsed().as_millis();
    let total_files = merged.files.len();
    let (total_functions, added_functions, modified_functions, unchanged_functions) =
        diff_function_counts(&prev_context, &merged);

    // reuse ratio = unchanged / (unchanged + modified)  (guard 0-div)
    let denom = (unchanged_functions + modified_functions) as f32;
    let reuse_ratio = if denom > 0.0 {
        (unchanged_functions as f32) / denom
    } else { 1.0 };

    eprintln!(
        "codemap: files={}, functions={}, added={}, modified={}, unchanged={}, reuse={:.2}, entry_points={}",
        total_files, total_functions, added_functions, modified_functions, unchanged_functions, reuse_ratio, merged.entry_points.len()
    );

    // Write merged context
    let path = Path::new(".codemap/").join("context.json");
    fs::write(&path, serde_json::to_string_pretty(&merged)?)?;
    println!("Updated .codemap/context.json");

    // Save metrics snapshot
    let snapshot = UpdateMetrics {
        total_files,
        total_functions,
        added_functions,
        modified_functions,
        removed_functions: 0, // optional: compute by walking old->new inverse
        unchanged_functions,
        reuse_ratio,
        duration_ms,
    };
    write_metrics(&snapshot)?; // appends to metrics.json history

    // Short human line (easy to paste into README / logs)
    println!(
        "metrics: files={}, funcs={}, +{} ~{} ={} reuse={:.0}% time={}ms llm_called={} llm_ok={} llm_time={}ms",
        total_files, total_functions, added_functions, modified_functions, unchanged_functions,
        reuse_ratio * 100.0, duration_ms, llm_called, llm_ok, llm_ms
    );

    Ok(())
}

fn summary() {
    let path = Path::new(".codemap/").join("context.json");
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(ctx) = serde_json::from_str::<ProjectContext>(&text) {
            if let Some(brief) = ctx.project_brief.as_ref() {
                println!("== Project Brief ==\n{}\n", brief);
            } else {
                println!("(no project brief yet)\n");
            }
            if ctx.entry_points.is_empty() {
                println!("(no entry points yet)");
            } else {
                println!("== Top Entry Points ==");
                for ep in &ctx.entry_points {
                    println!("[{}] {} — {}", ep.rank, ep.path, ep.reason);
                }
            }
            return;
        }
    }
    eprintln!("No context found. Run `codemap init` then `codemap update`.");
}

#[derive(Serialize, Deserialize, Debug)]
struct LogEntry {
    timestamp: String,
    note: String,
}

fn show(num: i32) {
    if num <= 0 {
        eprintln!("Please provide a positive number of entries to show.");
        return;
    }
    let path = Path::new(".codemap/").join("log.json");
    let content = fs::read_to_string(&path).unwrap_or_default();
    let log_json: Vec<LogEntry> = serde_json::from_str(&content).unwrap_or_default();

    let count = num.min(log_json.len() as i32) as usize;
    for i in 0..count {
        println!("{:?}: {:?}", &log_json[i].timestamp, &log_json[i].note);
    }
}

fn auth_set(key_opt: Option<String>) -> Result<()> {
    fs::create_dir_all(".codemap")?;
    let mut cfg = Config::load().unwrap_or_else(|_| toml::from_str(CONFIG_TEXT).unwrap());
    let key = match key_opt {
        Some(k) => k,
        None => prompt_reply("OpenAI API key (sk-...): ")?,
    };
    cfg.llm_api_key = Some(key.trim().to_string());
    save_config(&cfg)?;
    println!("API key saved. (Env var OPENAI_API_KEY overrides config.)");
    Ok(())
}

// ---------- Main ----------

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init()?,
        Commands::Update => update_context()?,
        Commands::Show { num } => show(num.unwrap_or(3)),
        Commands::Summary => summary(),
        Commands::Auth { key } => auth_set(key)?,
        // placeholders
        Commands::Edit => println!("edit (not implemented)"),
        Commands::Delete => println!("delete (not implemented)"),
    }

    Ok(())
}
