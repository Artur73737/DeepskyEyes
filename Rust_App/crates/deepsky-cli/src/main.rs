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
    validate_args(command, rest)?;
    match command {
        "discover" => commands::discover::run(&source),
        "capabilities" => commands::capabilities::run(&source, rest),
        "camera" => commands::camera::run(&source, rest),
        "capture" => commands::acquire::run_capture(&source, rest),
        "sequence" => commands::acquire::run_sequence(&source, rest),
        "serve" => commands::serve::run(rest),
        "preview" => commands::preview::run(&source, rest),
        "execute" => commands::execute::run(&source,rest),
        "session-inspect" => commands::execute::inspect(rest),
        "request-template" => commands::execute::template(&source,rest),
        "status" | "ping" | "thermal" | "autofocus" | "capability_dump" => commands::diagnostics::run(&source,command,rest),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command '{other}'; see 'help'")),
    }
}

fn parse_source(args: &[String]) -> Result<(Source, Vec<String>), String> {
    let mut kind = "adb".to_string();
    let mut addr: Option<String> = None;
    let mut serial: Option<String> = None;
    let mut realtime = false;
    let mut rest = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < args.len() {
        if ["--source","--addr","--serial","--realtime"].contains(&args[i].as_str()) && !seen.insert(args[i].as_str()) {
            return Err(format!("duplicate global option {}",args[i]));
        }
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
    if realtime && kind != "sim" { return Err("--realtime applies only to explicit --source sim".into()); }
    if seen.contains("--addr") && kind != "tcp" { return Err("--addr applies only to --source tcp".into()); }
    if seen.contains("--serial") && kind != "adb" { return Err("--serial applies only to --source adb".into()); }
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
    println!("Default source: ADB hardware. Simulator requires explicit --source sim.");
    println!("deepsky-eyes [--source sim|tcp|adb] [--addr IP:port] [--serial S] [--realtime] <command>");
    println!("  discover                        list announced cameras");
    println!("  capabilities --camera ID        dump announced capabilities as JSON");
    println!("  camera list | camera info ID    list or inspect cameras");
    println!("  capture --out DIR --project NAME [--camera ID] [--kind light|dark|flat|bias|test]");
    println!("            [--exposure-ns N] [--sensitivity N] [--focus-mdiopt N] [--wb-preset NAME] [--wb-kelvin N]");
    println!("            [--zoom RATIO | --zoom-x1000 N] [--stream WxH:FORMAT[:MODE]]");
    println!("  sequence --out DIR --project NAME --frames N [same options] [--delay-ns N]");
    println!("            [--strict-results] stop after saving a frame whose observed controls differ");
    println!("  serve [--port N]                serve the SYNTHETIC simulator on TCP loopback");
    println!("  preview [--camera ID] [--out FILE] [--exposure-ns N] [--sensitivity N]");
    println!("            [--frames N] [--autofocus] [same camera controls as capture]");
    println!("  Additional controls: --crop x,y,width,height --frame-duration-ns N --ois MODE --eis MODE");
    println!("            --processing edge=off,noise_reduction=off (announced keys/modes only)");
    println!("  --wb-kelvin is explicit: unsupported cameras reject it; no preset substitution.");
    println!("  status | ping | thermal | capability_dump [--out FILE]   JSON diagnostics");
    println!("  autofocus [--camera ID] [--out FILE]   center AF, measured distance as JSON");
    println!("  request-template [--camera ID] [--stream WxH:FORMAT[:MODE]] [--out FILE]");
    println!("  execute --request FILE --out FILE   full JSON controls; RAW/DNG/JPEG + device metadata");
    println!("  session-inspect --path DIR   latest immutable manifest and integrity scan (no camera needed)");
}

fn validate_args(command: &str, args: &[String]) -> Result<(),String> {
    let controls = ["camera","out","exposure-ns","sensitivity","focus-mdiopt","wb-preset","wb-kelvin","zoom","zoom-x1000","stream","crop","ois","eis","processing","frame-duration-ns"];
    let mut allowed = Vec::new();
    match command {
        "capture" | "sequence" | "preview" => {
            allowed.extend(controls);
            if command == "preview" { allowed.extend(["frames","autofocus"]); }
            else { allowed.extend(["project","kind","delay-ns","strict-results"]); if command == "sequence" { allowed.push("frames"); } }
        },
        "capabilities" => allowed.push("camera"),
        "request-template" => allowed.extend(["camera","stream","out"]),
        "execute" => allowed.extend(["request","out"]),
        "session-inspect" => allowed.push("path"),
        "autofocus" => allowed.extend(["camera","out"]),
        "status" | "ping" | "thermal" | "capability_dump" => allowed.push("out"),
        "serve" => allowed.push("port"),
        "camera" => return match args {
            [cmd] if cmd == "list" => Ok(()),
            [cmd,id] if cmd == "info" && !id.starts_with('-') => Ok(()),
            _ => Err("usage: camera list | camera info ID".into()),
        },
        _ => {}
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].strip_prefix("--").ok_or_else(||format!("unexpected argument '{}'",args[i]))?;
        let (name, inline) = flag.split_once('=').map(|(n,v)|(n,Some(v))).unwrap_or((flag,None));
        if !allowed.contains(&name) { return Err(format!("unknown option --{name} for {command}")); }
        if !seen.insert(name) { return Err(format!("duplicate option --{name}")); }
        if name == "autofocus" || name == "strict-results" {
            if inline.is_some() { return Err(format!("--{name} takes no value")); }
        } else {
            let value = match inline { Some(v)=>v, None=>{i+=1;args.get(i).map(String::as_str).ok_or_else(||format!("--{name} needs a value"))?} };
            if value.is_empty() || value.starts_with("--") { return Err(format!("--{name} needs a value")); }
        }
        i+=1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test] fn rejects_typos_missing_and_duplicate_options() {
        for args in [vec!["--sensitivty","100"],vec!["--ois"],vec!["--ois","--eis","off"],vec!["--ois","on","--ois=off"]] {
            assert!(super::validate_args("capture", &args.iter().map(|s|s.to_string()).collect::<Vec<_>>()).is_err());
        }
    }
    #[test] fn defaults_to_hardware() { assert!(matches!(super::parse_source(&[]).unwrap().0,deepsky_app::source::Source::Adb(None))); }
}
