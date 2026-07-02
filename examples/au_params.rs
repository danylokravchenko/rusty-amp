//! Print an AU's parameters and their default (freshly-loaded) values.
fn main() {
    let pat = std::env::args()
        .nth(1)
        .expect("usage: dump_params <au-substring>")
        .to_lowercase();
    let found = rusty_amp::host::au::scan()
        .into_iter()
        .find(|a| a.name.to_lowercase().contains(&pat))
        .expect("no AU matches");
    let (loaded, _ins) = rusty_amp::host::au::load(&found, 48_000.0, 512).expect("load");
    println!(
        "{} — {} parameters (fresh-load defaults):",
        found.name,
        loaded.params().len()
    );
    for p in loaded.params() {
        println!("  {:<40} = {}", p.name, p.display_value());
    }
}
