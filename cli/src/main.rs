//! `lxwengctl` entry
//!
//! The cli program to communicate with lxwengd

mod cli;

use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use clap::Parser;

fn main() {
    // let _cli = cli::Cli::parse();
    let mut conn = UnixStream::connect("/run/user/1000/lxwengd.sock").unwrap();
    conn.write_all(b"status").unwrap();
    let mut str = String::new();
    // conn.read_to_string(&mut str).unwrap();
    println!("{str}");
}
