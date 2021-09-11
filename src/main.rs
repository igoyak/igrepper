extern crate clap;
extern crate libc;

use clap::{App, Arg};
use inotify::{Inotify, WatchMask};
use libc::close;
use libc::open;

use crate::igrepper::igrepper;
use file_reading::{SourceInput, SourceProducer};
mod file_reading;
mod igrepper;

const PARAMETER_ERROR: &str = "Data can only be passed by STDIN if no file parameter is specified";

fn main() {
    let matches = App::new("igrepper")
        .version("1.2.1")
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
        .arg(
            Arg::with_name("follow")
                .short("f")
                .long("follow")
                .requires("file")
                .help("Reload the file as it changes. Requires [file] to be set."),
        )
        .get_matches();
    let file_option = matches.value_of("file");
    let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO as i32) } != 0;
    let mut file_path: Option<&str> = None;
    let source_producer: SourceProducer = if is_tty {
        let path = file_option.unwrap_or_else(|| {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        });
        file_path = Some(path);
        SourceProducer {
            input: SourceInput::FilePath(path.to_string()),
        }
    } else {
        if file_option != None {
            eprintln!("{}", PARAMETER_ERROR);
            std::process::exit(1);
        }
        let source = file_reading::read_source_from_stdin();
        reopen_stdin();
        SourceProducer {
            input: SourceInput::FullInput(source),
        }
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

    let inotify = if matches.is_present("follow") {
        let mut inotify = Inotify::init()
            .expect("Failed to monitor file changes, error while initializing inotify instance");

        inotify
            .add_watch(
                file_path.unwrap(),
                WatchMask::MODIFY | WatchMask::CLOSE_WRITE,
            )
            .expect("Failed to add file watch");
        Some(inotify)
    } else {
        None
    };

    igrepper(source_producer, context, initial_regex, inotify);
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
