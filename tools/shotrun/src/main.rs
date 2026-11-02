//! shotrun <script> <trace windows path> [mod dir...]: plays a replay on the
//! original code with the given loose-file mod folders.
fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let script = std::fs::read_to_string(args.next().expect("script"))?;
    let trace = args.next().expect("trace");
    let mut options = ffx_harness::Options { virtual_clock: true, ..ffx_harness::Options::default() };
    if std::env::var_os("SHOTRUN_ORIGINAL").is_some() { options.hook_disable = Some("replacements".into()); }
    // seconds to wait for the first frame (default 90; more on a busy machine)
    if let Some(s) = std::env::var("SHOTRUN_STARTUP").ok().and_then(|s| s.parse().ok()) { options.startup = std::time::Duration::from_secs(s); }
    // Graphics at maximum (tools/sandbox/max.ini) unless SHOTRUN_SETTINGS
    // names other settings (a name in ffx-rust's tools/sandbox or a path).
    let max = concat!(env!("CARGO_MANIFEST_DIR"), "/../sandbox/max.ini");
    options.settings = Some(std::env::var("SHOTRUN_SETTINGS").unwrap_or_else(|_| max.into()));
    options.mods = args.map(std::path::PathBuf::from).collect();
    let mut game = ffx_harness::Game::launch(&options)?;
    eprintln!("slot {}, port {}", game.instance().unwrap_or(0), game.port());
    let played = ffx_harness::replay::play(&mut game, &script, &trace);
    let log = game.new_log();
    let quit = game.quit();
    for l in log.lines().filter(|l| l.contains("FAULT") || l.contains("PANIC") || l.contains("mod")).take(20) { eprintln!("log: {l}"); }
    // SHOTRUN_LOG_GREP=<text>: every log line with it (FFX_HOOK_LOG_OPENS's file opens, say)
    if let Ok(pat) = std::env::var("SHOTRUN_LOG_GREP") {
        for l in log.lines().filter(|l| l.contains(&pat)) { eprintln!("log: {l}"); }
    }
    for line in played? { println!("{line}"); }
    quit
}
