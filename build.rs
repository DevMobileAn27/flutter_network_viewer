fn main() {
    println!("cargo:rerun-if-changed=native/updater_macos.m");
    println!("cargo:rerun-if-changed=vendor/SHA256SUMS");
    println!("cargo:rerun-if-changed=vendor/sparkle");
    println!("cargo:rerun-if-changed=vendor/winsparkle");
    let python = if cfg!(windows) { "python" } else { "python3" };
    assert!(
        std::process::Command::new(python)
            .args(["-B", "tools/verify_updater_vendor.py"])
            .status()
            .unwrap()
            .success(),
        "updater vendor integrity verification failed"
    );
    println!("cargo:rerun-if-changed=native/windows.rc");
    println!("cargo:rerun-if-changed=assets/icons/AppIcon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("HOST").unwrap().contains("windows")
    {
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let resource = output.join("viewer.res");
        assert!(
            std::process::Command::new("rc.exe")
                .args(["/nologo", "/c65001", "/fo"])
                .arg(&resource)
                .arg("native/windows.rc")
                .status()
                .expect("Windows SDK resource compiler rc.exe required")
                .success()
        );
        println!(
            "cargo:rustc-link-arg-bin=flutter-network-viewer={}",
            resource.display()
        );
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let object = output.join("updater.o");
    let target = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let arch = if target == "aarch64" {
        "arm64"
    } else {
        "x86_64"
    };
    assert!(
        std::process::Command::new("clang")
            .args([
                "-fobjc-arc",
                "-mmacosx-version-min=12.0",
                "-arch",
                arch,
                "-Fvendor/sparkle",
                "-c",
                "native/updater_macos.m",
                "-o"
            ])
            .arg(&object)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new("ar")
            .arg("crs")
            .arg(output.join("libfnv_updater.a"))
            .arg(object)
            .status()
            .unwrap()
            .success()
    );
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=fnv_updater");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
}
