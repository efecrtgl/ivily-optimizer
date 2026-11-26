// Sürüm derlemelerinde ek konsol penceresi açılmasını engeller.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Ivily Ekosistem Protokolü: `--iep describe` / `--iep request` pencere
    // açmadan yanıt verip çıkar (bkz. docs/IEP.md). Tek örnek kilidi ve
    // arayüz bu durumda hiç başlatılmaz.
    if let Some(code) = ivily_lib::ecosystem::run_cli(
        std::env::args().skip(1),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    ) {
        std::process::exit(code);
    }
    ivily_lib::run()
}
