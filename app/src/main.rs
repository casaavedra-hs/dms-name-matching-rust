mod db;
mod engine;
mod fuzzy;
#[cfg(feature = "gpu")]
mod gpu;
mod model;
mod normalize;
mod report;
mod resources;
mod web_ui;

fn main() {
    if let Err(e) = web_ui::run() {
        eprintln!("DMS Name Matching failed to start: {e:#}");
        std::process::exit(1);
    }
}
