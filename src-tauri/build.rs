fn main() {
    tauri_build::build();

    // Windows test binaries (from `cargo test`) do not receive the app
    // manifest: tauri-winres only embeds resources into the app binary, so
    // test executables link against the classic comctl32 v5 and crash at
    // startup with STATUS_ENTRYPOINT_NOT_FOUND (TaskDialogIndirect, which
    // only exists in comctl32 v6). Embed the same common-controls v6
    // manifest into every test target of this package.
    #[cfg(target_os = "windows")]
    {
        // rc.exe resolves relative paths against its working directory, so
        // generate an rc that references the manifest by absolute path.
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("windows")
            .join("snipdeck-test-manifest.xml");
        let rc_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
            .join("snipdeck-test-manifest.rc");
        std::fs::write(
            &rc_path,
            // resource type 24 (RT_MANIFEST); `RT_MANIFEST` itself is not an
            // rc keyword and would silently compile to an empty resource
            format!("1 24 \"{}\"", manifest.display()),
        )
        .expect("failed to write test manifest rc");
        let _ = embed_resource::compile_for_tests(&rc_path, embed_resource::NONE);
    }
}
