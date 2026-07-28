use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn check_mode_reports_reference_drift_without_writing_file() {
    let root = workspace_root();
    let reference = root.join(".agents/skills/card-dsl/DSL_REFERENCE.md");
    let original = fs::read_to_string(&reference).ok();
    let bin = std::env::var("CARGO_BIN_EXE_gen_dsl_reference").expect("gen_dsl_reference bin path");

    fs::write(&reference, "# stale\n").expect("write stale DSL reference");

    let output = Command::new(bin)
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("run gen_dsl_reference --check");

    match original {
        Some(contents) => fs::write(&reference, contents).expect("restore DSL reference"),
        None => fs::remove_file(&reference).expect("remove temporary DSL reference"),
    }

    assert!(!output.status.success(), "check unexpectedly passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DSL_REFERENCE.md"), "{stderr}");
    assert!(stderr.contains("cards-dsl-ref"), "{stderr}");
}
