fn main() {
    slint_build::compile("ui/appwindow.slint").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/SvagView_Logo_Icon.ico");
    println!("cargo:rerun-if-changed=resources/app.rc");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let res_path = format!("{}/app.res", out_dir);
        let rc_path = "resources/app.rc";

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let ico_abs = format!(
            "{}/resources/SvagView_Logo_Icon.ico",
            manifest_dir.replace('\\', "/")
        );
        let rc_content = format!("1 ICON \"{}\"\n", ico_abs);
        std::fs::write(rc_path, rc_content).unwrap();

        let rc_sdk = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe";
        let rc_exe = if std::path::Path::new(rc_sdk).exists() {
            rc_sdk
        } else {
            "rc.exe"
        };

        let status = std::process::Command::new(rc_exe)
            .arg("/fo")
            .arg(&res_path)
            .arg(rc_path)
            .status();

        if let Ok(st) = status {
            if st.success() {
                println!("cargo:rustc-link-arg={}", res_path);
            } else {
                eprintln!("Warning: rc.exe failed with status {:?}", st);
            }
        } else if let Err(e) = status {
            eprintln!("Warning: Failed to execute rc.exe: {}", e);
        }
    }
}
