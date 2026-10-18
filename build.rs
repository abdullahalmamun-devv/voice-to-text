use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-changed=assets/app.rc");
    let target = env::var("TARGET").expect("Cargo target");
    if !target.contains("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output"));
    let (result, path) = if target.ends_with("gnu") {
        let path = out.join("app-resource.o");
        let command = env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into());
        let result = Command::new(command)
            .args(["-i", "assets/app.rc", "-I", "assets", "-O", "coff", "-o"])
            .arg(&path)
            .status();
        (result, path)
    } else {
        let path = out.join("app.res");
        let result = Command::new("rc.exe")
            .arg("/nologo")
            .arg("/Iassets")
            .arg(format!("/fo{}", path.display()))
            .arg("assets/app.rc")
            .status();
        (result, path)
    };
    assert!(
        result
            .expect("Windows resource compiler is required")
            .success(),
        "Resource compilation failed"
    );
    println!("cargo:rustc-link-arg={}", path.display());
}
