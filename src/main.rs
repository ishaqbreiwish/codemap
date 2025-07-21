// this is the standard config file that shows up
static CONFIG_TEXT: &str = "# Number of notes to show when running `codemap show`\n\
default_note_count = 3\n\n\
# Whether to timestamp notes in a human-readable format\n\
timestamp_format = \"iso8601\"  # e.g. \"2025-07-11T20:15:00Z\"\n\n\
# Enable or disable LLM summaries (can be toggled per-project)\n\
enable_summaries = true\n\n\
# Optional: Path to custom summary command (for local LLMs)\n\
summary_command = \"llama-cli summarize\"\n\n\
# Optional: Project tag for filtering later (future feature)\n\
project_name = \"my-project\"";

use anyhow::{Result, anyhow};
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::io::Write; // for flushing stdout
use std::fs::File;
use std::process::Command;
use rprompt::prompt_reply;
use chrono::Utc;
use toml;
use std::collections::HashMap;
use walkdir::WalkDir;
use regex::Regex;




/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
#[command(name = "codemap", about = "A CLI for understanding codebases")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}


// Log entry struct
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct LogEntry {
    timestamp: String,
    note: String,
}

// full tree like project context
#[derive(Serialize, Deserialize)]
struct ProjectContext {
    folders: HashMap<String, FolderNode>,
    files: HashMap<String, FileContext>,
}

// individual folder nodes 
#[derive(Serialize, Deserialize)]
struct FolderNode {
    children: Vec<String>, // just names of subfolders and files
}

// individual file nodes
#[derive(Serialize, Deserialize)]
struct FileContext {
    language: String,
    functions: Vec<FunctionInfo>,
}

// struct for each function definition
#[derive(Serialize, Deserialize)]
struct FunctionInfo {
    name: String,
    line: usize,
    summary: Option<String>,
    hash: String,
}


#[derive(Debug)]
#[derive(Deserialize)]
struct Config {
    default_note_count: Option<usize>,
    timestamp_format: Option<String>,
    enable_summaries: Option<bool>,
    summary_command: Option<String>,
    project_name: Option<String>,
}

impl Config {
    fn load() -> Result<Config> {
        let path = Path::new(".codemap/");
        let config_path = path.join("config.toml");

        let content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return Err(anyhow!(".codemap/config.toml is missing"))
        };

        Ok(toml::from_str(&content)?)
    }
}


#[derive(clap::Subcommand)]
#[command(rename_all = "lowercase")]
enum Commands {
    Init, // initializes remind me project
    Add {
        note: Option<String>,
        #[arg(long)]
        auto: bool,
    },
    Show {num: Option<i32>},
    Summary,
}

fn init() -> Result<()> {
    // Convert the path string into a Path object
    let path = Path::new(".codemap/");
    let log_path = path.join("log.json");
    let config_path = path.join("config.toml");

    // Check if it already exists
    if log_path.exists() && config_path.exists() {
        println!("Folder already exists.");
        return Ok(());
    }

    // Create the directory (and any missing parent dirs)
    // .codemap folder
    fs::create_dir_all(path)?;
    println!("Folder created.");

    // create empty log file
    File::create(log_path)?;
     println!("successfully wrote log file");

    // create filled out config file
    let mut file = File::create(&config_path)?;
    file.write_all(CONFIG_TEXT.as_bytes())?;
    println!("Successfully wrote config file.");

    // --- generate .codemap/context.json ---
    let context_path = path.join("context.json");
    let context_data = build_context()?; 
    let json = serde_json::to_string_pretty(&context_data)?;
    fs::write(context_path, json)?;

    Ok(())
}

// helper fucntion for building the context file
fn build_context() -> Result<ProjectContext> {
    // initialize maps for folders and files
    let mut folders = HashMap::new();
    let mut files = HashMap::new();

    // remove all cache directories from being considered
    let ignored_dirs = ["target", ".git", "node_modules", ".venv", "__pycache__"];
    // recursively walk through every file and folder
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap(); // Handle errors properly 
        let path = entry.path();

        // remove cache directories from consideration
        if entry.path().components().any(|c| ignored_dirs.contains(&c.as_os_str().to_string_lossy().as_ref())) {
            continue;
        }

        // remove hidden directories from consideration
        if entry.file_name().to_string_lossy().starts_with('.') &&
        !entry.file_name().to_string_lossy().starts_with(".codemap") {
            continue;
        }

        if path.is_dir() {
            let mut children = vec![];

            for child in fs::read_dir(path)? {
                let child = child?;
                let name = child.file_name().to_string_lossy().to_string();
                children.push(name);
            }
            let relative_path = path.strip_prefix(".")?.display().to_string();  
            folders.insert(relative_path, FolderNode { children });
        }

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if let Some(lang) = detect_language(ext) {
                    let contents = fs::read_to_string(path)?;
                    let functions = extract_functions(&contents, &lang);

                    let file_context = FileContext {
                        language: lang,
                        functions,
                    };

                    let rel = path.strip_prefix(".")?.display().to_string();
                    files.insert(rel, file_context);
                }
            }
        }
    }


    Ok(ProjectContext { folders, files })
}

// helper function for detecting language
fn detect_language(extension: &str) -> Option<String> {
    match extension.to_ascii_lowercase().as_str() {
        "rs" => Some("rust".to_string()),
        "py" => Some("python".to_string()),
        "js" => Some("javascript".to_string()),
        "ts" => Some("typescript".to_string()),
        "java" => Some("java".to_string()),
        "go" => Some("go".to_string()),
        "cpp" | "cc" | "cxx" | "c++" => Some("cpp".to_string()),
        "c" => Some("c".to_string()),
        _ => None,
    }
}

// helper function for extraction functions
fn extract_functions(source: &str, lang: &str) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();

    if lang == "rust" {
        let re = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap(); // // Compile a regex to match Rust function defs
        let lines: Vec<&str> = source.lines().collect(); // Split the file source into a list of lines for line-by-line analysis

        for i in 0..lines.len() { // Loop through each line and try to detect function definitions
            if let Some(caps) = re.captures(lines[i]) {    // If the current line matches the function regex, capture it
                let name = caps.get(3).unwrap().as_str().to_string(); // this gets the actual name of the function

                // Function body extraction
                let mut body = String::new(); // new empty string for the body
                let mut open_brackets = 0; // open brcket counter to know when we hit the end
                let mut found_brace = false; // indicates if we hit a brace

                for j in i..lines.len() { // iterate through the whole page
                    let line = lines[j]; // set line equal to current lines
                    body.push_str(line); // adds each new strign to the body 
                    body.push('\n'); // for readability

                    for c in line.chars() {  // iterates through eveyr character and checks for brace
                        if c == '{' {
                            open_brackets += 1;
                            found_brace = true;
                        } else if c == '}' {
                            open_brackets -= 1;
                        }
                    }

                    if found_brace && open_brackets == 0 {
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
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}


fn add_note(note: &str) {
    let path = Path::new(".codemap/");
    let log_path = path.join("log.json");
    
    // check if codemap is there
    if !path.exists() || !path.is_dir() {
        println!("Error: This project is not initialized with `codemap init`.");
        return;
    }

    let final_note = note.to_string();

    // new entry
    let timestamp = Utc::now().format("%m/%d/%Y").to_string();
    let entry = LogEntry {
        timestamp,
        note: final_note,
    };

    // Read existing log as string
    let content = fs::read_to_string(&log_path).unwrap_or_else(|_| String::new());

    // Parse as JSON, or default to empty vec
    let mut log_json: Vec<LogEntry> = serde_json::from_str(&content).unwrap_or_else(|_| vec![]);

    // Push new entry
    log_json.push(entry);

    // Write full log back
    let json = serde_json::to_string_pretty(&log_json).expect("Failed to serialize log.");
    fs::write(&log_path, json).expect("Failed to write log.");

    println!("Note added successfully.");
}

fn add_auto_note() -> Result<()> {
    // load in the config file
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("⚠️ Failed to load config: {e}");
            return Err(e); 
        }
    };

    // Run git diff HEAD
    let output = Command::new("git")
        .args([
            "diff", "--unified=5",
            "--ignore-blank-lines", "--ignore-space-at-eol",
            "HEAD", "--", "*.rs", "*.toml", ":!Cargo.lock"
        ])
        .output()?;

    // pass the diff output into the stdin of the summary command
    let diff = String::from_utf8_lossy(&output.stdout);
    add_note(&diff);

    Ok(())
}

fn summary() {
    println!("Showing summary...");
}


fn show(num: i32) {
    if num <= 0 {
        eprintln!("Please provide a positive number of entries to show.");
        return;
    }

    let path = Path::new(".codemap/");
    let log_path = path.join("log.json");

     // Read existing log as string
    let content = fs::read_to_string(&log_path).unwrap_or_else(|_| String::new());

    // Parse as JSON, or default to empty vec
    let log_json: Vec<LogEntry> = serde_json::from_str(&content).unwrap_or_else(|_| vec![]);
    
    let count = num.min(log_json.len() as i32) as usize;

    // Print top num entries
    for i in 0..count {
        println!("{:?}: {:?}", &log_json[i].timestamp, &log_json[i].note);
    }
}


fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init()?,
        Commands::Add { note, auto } => {
            if auto {
                add_auto_note()?;
            } else {
                let final_note = match note {
                    Some(n) if !n.trim().is_empty() => n,
                    _ => prompt_reply("Write a note: ").unwrap(),
                };
                add_note(&final_note);
            }
        }        
        Commands::Show { num } => {
            let final_num = num.unwrap_or(3);
            show(final_num)
        }
        Commands::Summary => summary(),
    }
    

    Ok(())
}
