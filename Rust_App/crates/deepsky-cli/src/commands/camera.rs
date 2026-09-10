//! CLI camera list/info.
pub fn run(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("list") => println!("camera list: stub"),
        Some("info") => println!("camera info: stub"),
        _ => println!("usage: camera <list|info <id>>"),
    }
}
