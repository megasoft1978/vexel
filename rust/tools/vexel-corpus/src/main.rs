#![forbid(unsafe_code)]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(vexel_corpus::cli::main_with(&args));
}
