use std::{fs, process::Command};

#[test]
fn check_mode_reports_reference_drift_without_writing_file() {
    // The binary renders the reference from its own compiled schema and only ever touches
    // `.agents/skills/card-dsl/DSL_REFERENCE.md` *relative to its working directory*, so a temp dir
    // holding nothing but a stale copy is a complete stand-in for the repo. Do not point this at
    // the real workspace root: it would have to overwrite the checked-in file and restore it
    // afterwards, and two concurrent runs then race — the second captures the first's `# stale` as
    // its "original" and restores that, destroying the real file.
    let root = std::env::temp_dir().join(format!("gen_dsl_reference_check_{}", std::process::id()));
    let reference = root.join(".agents/skills/card-dsl/DSL_REFERENCE.md");
    fs::create_dir_all(reference.parent().expect("reference has a parent"))
        .expect("create temp reference dir");
    fs::write(&reference, "# stale\n").expect("write stale DSL reference");

    let bin = std::env::var("CARGO_BIN_EXE_gen_dsl_reference").expect("gen_dsl_reference bin path");
    let output = Command::new(bin)
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("run gen_dsl_reference --check");

    let stale_after = fs::read_to_string(&reference).expect("reference still readable");
    fs::remove_dir_all(&root).expect("clean up temp reference dir");

    assert_eq!(stale_after, "# stale\n", "--check must not write the file");
    assert!(!output.status.success(), "check unexpectedly passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DSL_REFERENCE.md"), "{stderr}");
    assert!(stderr.contains("cards-dsl-ref"), "{stderr}");
}
