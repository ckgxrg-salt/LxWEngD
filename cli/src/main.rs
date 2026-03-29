//! `lxwengctl`
//!
//! Tiny wrapper to send messages to the socket

use std::fmt::Display;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    version = "1.1.0",
    about="CLI tool for manipulating lxwengd.",
    long_about = None
)]
pub struct Cli {
    #[arg(short = 'm', long = "monitor")]
    monitor: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Load playlists")]
    Playlist {
        #[arg(short = 'p', long = "paused")]
        paused: bool,

        #[arg(short = 'r', long = "resume")]
        resume: ResumeMode,

        path: PathBuf,
    },

    #[command(about = "Stop and unload a playlist")]
    Stop {
        #[arg(short = 'c', long = "no-resume")]
        no_resume: bool,
    },

    #[command(about = "Resume/Play a playlist")]
    Play,

    #[command(about = "Pause a playlist")]
    Pause {
        #[arg(short = 'c', long = "clear")]
        clear: bool,
    },

    #[command(about = "Toggle play/pause for a playlist")]
    Toggle {
        #[arg(short = 'c', long = "clear")]
        clear: bool,
    },

    #[command(about = "Show LxWEngd status")]
    Status,

    #[command(about = "Quit LxWEngd")]
    Quit,
}

#[derive(Clone, ValueEnum)]
enum ResumeMode {
    Ignore,
    Delete,
    True,
}

impl Display for ResumeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ignore => write!(f, "ignore"),
            Self::Delete => write!(f, "delete"),
            Self::True => write!(f, "true"),
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let monitor = cli.monitor.unwrap_or(String::from("NOMONITOR"));
    let mut conn =
        UnixStream::connect("/run/user/1000/lxwengd.sock").expect("Unable to connect to LxWEngd");

    let msg = match cli.command {
        Command::Playlist {
            paused,
            resume,
            path,
        } => format!(
            "load {} {} {} {}\n",
            path.to_string_lossy(),
            monitor,
            paused,
            resume
        ),
        Command::Stop { no_resume } => format!("unload {no_resume} {monitor}\n"),

        Command::Play => format!("play {monitor}\n"),
        Command::Pause { clear } => format!("pause {clear} {monitor}\n"),
        Command::Toggle { clear } => format!("toggle {clear} {monitor}\n"),

        Command::Status => String::from("status\n"),
        Command::Quit => String::from("quit\n"),
    };
    conn.write_all(msg.as_bytes()).unwrap();

    let mut response = String::new();
    conn.read_to_string(&mut response).unwrap();
    println!("{response}");
}
