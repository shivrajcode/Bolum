fn main() {
    // ONNX Runtime's prebuilt macOS archive includes Objective-C availability
    // checks. Fat LTO needs the Clang runtime that supplies their symbols.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output = std::process::Command::new("clang")
        .arg("--print-runtime-dir")
        .output()
        .expect("clang is required to link ONNX Runtime on macOS");
    assert!(output.status.success(), "clang runtime lookup failed");
    let dir = String::from_utf8(output.stdout).expect("clang runtime path must be UTF-8");
    println!("cargo:rustc-link-search=native={}", dir.trim());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}
