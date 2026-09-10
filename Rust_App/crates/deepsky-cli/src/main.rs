//! deepsky-eyes CLI — real integration client over the shared controller.
//! Every command talks to a real backend; failures exit nonzero with the cause.
mod commands;

use deepsky_app::source::Source;

fn main() {
    let code = match run(std::env::args().skip(1).collect()) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("deepsky-eyes: error: {message}");
            1
        }
    };
    std::process::exit(code);
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (source, rest) = parse_source(&args)?;
    let (command, rest) = rest.split_first().map(|(c, r)| (c.as_str(), r)).unwrap_or(("help", &[][..]));
    match command {
        "discover" => commands::discover::run(&source),
        "capabilities" => commands::capabilities::run(&source, rest),
        "camera" => commands::camera::run(&source, rest),
        "capture" => commands::acquire::run_capture(&source, rest),
        "sequence" => commands::acquire::run_sequence(&source, rest),
        "serve" => commands::serve::run(rest),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command '{other}'; see 'help'")),
    }
}

fn parse_source(args: &[String]) -> Result<(Source, Vec<String>), String> {
    let mut kind = "sim".to_string();
    let mut addr: Option<String> = None;
    let mut serial: Option<String> = None;
    let mut realtime = false;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source" => {
                i += 1;
                kind = args.get(i).cloned().ok_or("--source needs sim|tcp|adb")?;
            }
            "--addr" => {
                i += 1;
                addr = Some(args.get(i).cloned().ok_or("--addr needs IP:port")?);
            }
            "--serial" => {
                i += 1;
                serial = Some(args.get(i).cloned().ok_or("--serial needs a value")?);
            }
            "--realtime" => realtime = true,
            other => rest.push(other.to_string()),
        }
        i += 1;
    }
    let source = match kind.as_str() {
        "sim" => Source::Simulator { realtime },
        "tcp" => Source::Tcp(addr.ok_or("--source tcp needs --addr IP:port")?),
        "adb" => Source::Adb(serial),
        other => return Err(format!("unknown --source '{other}': sim|tcp|adb")),
    };
    Ok((source, rest))
}

pub(crate) fn flag(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == format!("--{name}") {
            return args.get(i + 1).cloned();
        }
        if let Some(value) = args[i].strip_prefix(&format!("--{name}=")) {
            return Some(value.to_string());
        }
        i += 1;
    }
    None
}

pub(crate) fn flag_or<T>(args: &[String], name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match flag(args, name) {
        Some(value) => value.parse().map_err(|e| format!("--{name}: {e}")),
        None => Ok(default),
    }
}

fn print_help() {
    println!("deepsky-eyes [--source sim|tcp|adb] [--addr IP:port] [--serial S] [--realtime] <command>");
    println!("  discover                        list announced cameras");
    println!("  capabilities --camera ID        dump announced capabilities as JSON");
    println!("  camera list | camera info ID    list or inspect cameras");
    println!("  capture --out DIR --project NAME [--kind light|dark|flat|bias|test]");
    println!("            [--exposure-ns N] [--sensitivity N] [--focus-mdiopt N] [--wb-kelvin N]");
    println!("  sequence --out DIR --project NAME --frames N [same options] [--delay-ns N]");
    println!("  serve [--port N]                serve the SYNTHETIC simulator on TCP loopback");
}
