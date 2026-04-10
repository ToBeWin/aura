use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    #[cfg(target_os = "macos")]
    compile_macos_speech_helper();

    tauri_build::build()
}

#[cfg(target_os = "macos")]
fn compile_macos_speech_helper() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let source_path = manifest_dir.join("src").join("asr").join("transcribe_audio.swift");
    let plist_path = manifest_dir.join("src").join("asr").join("SpeechHelper-Info.plist");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing out dir"));
    let helper_path = out_dir.join("aura-speech-helper");

    println!("cargo:rerun-if-changed={}", source_path.display());
    println!("cargo:rerun-if-changed={}", plist_path.display());

    let status = Command::new("swiftc")
        .env("SWIFT_MODULECACHE_PATH", "/tmp/swift-module-cache")
        .env("CLANG_MODULE_CACHE_PATH", "/tmp/clang-module-cache")
        .arg(&source_path)
        .arg("-o")
        .arg(&helper_path)
        .arg("-Xlinker")
        .arg("-sectcreate")
        .arg("-Xlinker")
        .arg("__TEXT")
        .arg("-Xlinker")
        .arg("__info_plist")
        .arg("-Xlinker")
        .arg(&plist_path)
        .status()
        .unwrap_or_else(|error| panic!("failed to compile macOS speech helper: {}", error));

    if !status.success() {
        panic!(
            "failed to compile macOS speech helper from {}",
            display_path(&source_path)
        );
    }

    println!(
        "cargo:rustc-env=AURA_SPEECH_HELPER_PATH={}",
        helper_path.display()
    );
}

#[cfg(target_os = "macos")]
fn display_path(path: &Path) -> String {
    path.display().to_string()
}
