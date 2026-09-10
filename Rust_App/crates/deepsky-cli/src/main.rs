//! deepsky-eyes CLI — primo client di integrazione (README §86).
//! Comandi: discover, capabilities, connect, camera list/info, set, capture, sequence.

mod commands;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");
    match cmd {
        "discover" => commands::discover::run(),
        "capabilities" => commands::capabilities::run(),
        "camera" => commands::camera::run(&args[2..]),
        "capture" => commands::capture::run(),
        "sequence" => commands::sequence::run(),
        _ => print_help(),
    }
}

fn print_help() {
    println!("deepsky-eyes <discover|capabilities|camera|capture|sequence>");
    println!("  camera list | camera info <id>");
}
