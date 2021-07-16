use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone)]
pub enum SourceInput {
    FullInput(Vec<String>),
    FilePath(String),
}

pub struct SourceProducer {
    pub input: SourceInput,
}

impl SourceProducer {
    pub fn get_source(&self) -> Vec<String> {
        match &self.input {
            SourceInput::FilePath(path) => {
                read_source_from_file(path.as_str()).unwrap_or_else(|error| {
                    eprintln!("Failed to open file '{}': {}", path, error);
                    std::process::exit(1);
                })
            }
            SourceInput::FullInput(full_input) => full_input.clone(),
        }
    }
}

fn read_source_from_file(file_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let source: Vec<String> = reader.lines().map(|res| res.unwrap()).collect();
    Ok(source)
}

pub fn read_source_from_stdin() -> Vec<String> {
    let stdin = io::stdin();
    stdin.lock().lines().map(|res| res.unwrap()).collect()
}
