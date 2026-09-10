//! Entry point binario mission control.
use deepsky_app::app::App;

fn main() {
    let app = App::new();
    println!("DeepskyEyes {} — connected={}", app.version(), app.state.connected);
}
