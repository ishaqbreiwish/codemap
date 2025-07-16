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

use anyhow::{Result};
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::io::Write; // for flushing stdout
use std::fs::File;
use rprompt::prompt_reply;
use chrono::Utc;
use toml;


/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
#[command(name = "codemap", about = "A CLI for understanding codebases")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct LogEntry {
    timestamp: String,
    note: String,
}

#[derive(Debug)]
struct Config {
    default_note_count: Option<usize>,
    timestamp_format: Option<String>,
    enable_summaries: Option<bool>,
    summary_command: Option<String>,
    project_name: Option<String>,
}

impl Config {
    fn load() -> Result<Config> => {
        let path = Path::new(".codemap/");
        let config_path = path.join("config.toml");

        let content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => eprintln!("Could not read file `{}`", &config_path);
        }

    }
}


#[derive(clap::Subcommand)]
#[command(rename_all = "lowercase")]
enum Commands {
    Init, // initializes remind me project
    Note {note: Option<String>,}, // adds a new note
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


    Ok(())
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

fn summary() {
    println!("Showing summary...");
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init()?,
        Commands::Note { note } => {
            let final_note = match note {
                Some(n) if !n.trim().is_empty() => n,
                _ => prompt_reply("Write a note: ").unwrap(),
            };
            add_note(&final_note);
        }
        Commands::Show { num } => {
            let final_num = num.unwrap_or(3);
            show(final_num)
        }
        Commands::Summary => summary(),
    }
    

    Ok(())
}
