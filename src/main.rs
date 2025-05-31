extern crate clap;
extern crate libc;

use clap::Command;
use inotify::{Inotify, WatchMask};
use libc::close;
use libc::open;
use std::env;

use crate::igrepper::igrepper;
use file_reading::{SourceInput, SourceProducer};
mod file_reading;
mod igrepper;

const PARAMETER_ERROR: &str = "Data can only be passed by STDIN if no file parameter is specified";
const DEFAULT_EDITOR_COMMAND: [&str; 3] = ["vim", "-R", "-"];

fn main() {
    let matches = Command::new("igrepper")
        .version("1.3.6")
        .about("The interactive grepper")
        .arg(clap::arg!(-e --regex <REGEX> "Regular expression to preload"))
        .arg(clap::arg!(-c --context <CONTEXT> "Print CONTEXT num of output context"))
        .arg(clap::arg!(-w --word "Preload the regular expression '\\S+'").conflicts_with("regex"))
        .arg(
            clap::arg!(-f --follow "Reload the file as it changes. Requires [file] to be set.")
                .requires("FILE"),
        )
        .arg(
            clap::arg!(<FILE> "Sets the input file to use. If not set, reads from stdin.")
                .required(false),
        )
        .get_matches();

    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) } != 0;
    let mut file_path: Option<&str> = None;
    let source_producer: SourceProducer = if is_tty {
        let path = matches.get_one::<String>("FILE").unwrap_or_else(|| {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        });
        file_path = Some(path);
        SourceProducer {
            input: SourceInput::FilePath(path.to_string()),
        }
    } else {
        if matches.get_one::<String>("FILE").is_some() {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        }
        let source = file_reading::read_source_from_stdin();
        reopen_stdin();
        SourceProducer {
            input: SourceInput::FullInput(source),
        }
    };

    let context: u32 = match matches.get_one::<String>("context") {
        None => 0,
        Some(context_string) => context_string.parse::<u32>().unwrap(),
    };

    let initial_regex = if matches.get_flag("word") {
        Some("\\S+")
    } else {
        matches.get_one::<String>("regex").map(|s| s.as_str())
    };

    let inotify = if matches.get_flag("follow") {
        let inotify = Inotify::init()
            .expect("Failed to monitor file changes, error while initializing inotify instance");

        inotify
            .watches()
            .add(
                file_path.unwrap(),
                WatchMask::MODIFY | WatchMask::CLOSE_WRITE,
            )
            .expect("Failed to add file watch");
        Some(inotify)
    } else {
        None
    };

    let external_editor: Vec<String> = get_external_editor();

    igrepper(
        source_producer,
        context,
        initial_regex,
        inotify,
        external_editor,
    )
    .unwrap();
}

fn get_external_editor() -> Vec<String> {
    if let Ok(a) = env::var("IGREPPER_EDITOR") {
        let editor_command: Vec<String> =
            a.split_ascii_whitespace().map(|s| s.to_string()).collect();
        if !editor_command.is_empty() {
            return editor_command;
        }
    }
    DEFAULT_EDITOR_COMMAND
        .iter()
        .map(|s| s.to_string())
        .collect()
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
