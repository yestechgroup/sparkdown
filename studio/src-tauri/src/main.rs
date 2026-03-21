#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// On Ubuntu, snap packages can leak older glibc libraries into LD_LIBRARY_PATH,
/// causing "undefined symbol: __libc_pthread_init" when WebKitGTK loads the wrong
/// libpthread. If snap paths are detected, re-exec with a sanitized LD_LIBRARY_PATH.
#[cfg(target_os = "linux")]
fn sanitize_ld_library_path() {
    use std::env;

    const MARKER: &str = "__SPARKDOWN_LD_SANITIZED";
    if env::var_os(MARKER).is_some() {
        return;
    }

    if let Ok(ld_path) = env::var("LD_LIBRARY_PATH") {
        if ld_path.contains("/snap/") {
            use std::os::unix::process::CommandExt;

            let cleaned: String = ld_path
                .split(':')
                .filter(|p| !p.contains("/snap/"))
                .collect::<Vec<_>>()
                .join(":");

            let exe = env::current_exe().expect("failed to get current exe path");
            let err = std::process::Command::new(exe)
                .args(&env::args().collect::<Vec<_>>()[1..])
                .env(MARKER, "1")
                .env("LD_LIBRARY_PATH", &cleaned)
                .exec();
            // exec() replaces the process; it only returns on error
            eprintln!("sparkdown-studio: re-exec failed: {err}");
            std::process::exit(1);
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    sanitize_ld_library_path();

    sparkdown_studio_lib::run();
}
