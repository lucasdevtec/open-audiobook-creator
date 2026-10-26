use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_path = PathBuf::from(&out_dir);
            let empty_archive = b"!<arch>\n";

            let _ = fs::write(out_path.join("WebView2Loader.dll.lib"), empty_archive);
            let _ = fs::write(out_path.join("libWebView2Loader.dll.a"), empty_archive);
            println!("cargo:rustc-link-search=native={}", out_path.display());

            // Substitui a import library dinâmica WebView2Loader.dll.lib do webview2-com-sys
            // por um arquivo estático vazio para que os símbolos sejam resolvidos pelo loader embutido no .exe.
            if let Some(build_root) = out_path.parent().and_then(|p| p.parent()) {
                if let Ok(entries) = fs::read_dir(build_root) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with("webview2-com-sys-") {
                            for arch in ["x64", "x86", "arm64"] {
                                let dir = entry.path().join("out").join(arch);
                                if dir.exists() {
                                    let _ = fs::write(
                                        dir.join("WebView2Loader.dll.lib"),
                                        empty_archive,
                                    );
                                    let _ = fs::write(
                                        dir.join("libWebView2Loader.dll.a"),
                                        empty_archive,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

