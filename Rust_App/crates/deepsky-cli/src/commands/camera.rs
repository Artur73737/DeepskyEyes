//! CLI camera list/info over announced evidence.
use deepsky_app::source::Source;

pub fn run(source: &Source, args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("list") => super::discover::run(source),
        Some("info") => {
            let id = args.get(1).cloned().ok_or("usage: camera info <id>")?;
            super::capabilities::run(source, &[format!("--camera={id}")])
        }
        _ => Err("usage: camera <list|info <id>>".into()),
    }
}
