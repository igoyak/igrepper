extern crate clap;
extern crate libc;
use crate::igrepper::igrepper;
use clap::{App, Arg};
use libc::close;
use libc::open;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;

mod igrepper;

const PARAMETER_ERROR: &str = "Data can only be passed by STDIN if no file parameter is specified";

fn main() {
    let matches = App::new("igrepper")
        .version("1.1.1")
        .about("The interactive grepper")
        .arg(
            Arg::with_name("regex")
                .short("e")
                .long("regex")
                .value_name("REGEX")
                .help("Regular expression to preload")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("context")
                .short("c")
                .long("context")
                .value_name("CONTEXT")
                .help("Print CONTEXT num of output context")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("file")
                .help("Sets the input file to use. If not set, reads from stdin.")
                .index(1),
        )
        .arg(
            Arg::with_name("word")
                .short("w")
                .long("word")
                .conflicts_with("regex")
                .help("Preload the regular expression '\\S+'"),
        )
        .get_matches();
    let file_option = matches.value_of("file");
    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO as i32) } != 0;
    let source = if is_tty {
        let file_path = file_option.unwrap_or_else(|| {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        });
        read_source_from_file(file_path).unwrap_or_else(|error| {
            eprintln!("Failed to open file '{}': {}", file_path, error);
            std::process::exit(1);
        })
    } else {
        if file_option != None {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        }
        let source = read_source_from_stdin();
        reopen_stdin();
        source
    };
    let context: u32 = match matches.value_of("context") {
        None => 0,
        Some(context_string) => context_string.parse::<u32>().unwrap(),
    };

    let initial_regex = if matches.is_present("word") {
        Some("\\S+")
    } else {
        matches.value_of("regex")
    };

    igrepper(source, context, initial_regex);
}

/// Close STDIN and open TTY as file descriptor 0.
/// Used when all file input has been read, and STDIN should
/// now come from the terminal.
fn reopen_stdin() {
    unsafe {
        let close_returncode = close(0);
        assert_eq!(close_returncode, 0, "Failed to close stdin");
        let ptr = "/dev/tty\0".as_ptr() as *const i8;
        let open_returncode = open(ptr, 0);
        assert_eq!(open_returncode, 0, "Failed to open /dev/tty");
    }
}

fn read_source_from_file(file_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let source: Vec<String> = reader.lines().map(|res| res.unwrap()).collect();
    Ok(source)
}

fn read_source_from_stdin() -> Vec<String> {
    let stdin = io::stdin();
    stdin.lock().lines().map(|res| res.unwrap()).collect()
}
