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
fn check_mode_reports_schema_drift_without_writing_files() {
    let root = workspace_root();
    let card_schema = root.join("crates/cards/schema/card.schema.json");
    let token_schema = root.join("crates/cards/schema/token.schema.json");
    let original_card = fs::read_to_string(&card_schema).ok();
    let original_token = fs::read_to_string(&token_schema).ok();
    let bin = std::env::var("CARGO_BIN_EXE_gen_card_schema").expect("gen_card_schema bin path");

    fs::write(&card_schema, "{}\n").expect("write stale card schema");
    fs::write(&token_schema, "{}\n").expect("write stale token schema");

    let output = Command::new(bin)
        .arg("--check")
        .current_dir(&root)
        .output()
        .expect("run gen_card_schema --check");

    match original_card {
        Some(contents) => fs::write(&card_schema, contents).expect("restore card schema"),
        None => fs::remove_file(&card_schema).expect("remove temporary card schema"),
    }
    match original_token {
        Some(contents) => fs::write(&token_schema, contents).expect("restore token schema"),
        None => fs::remove_file(&token_schema).expect("remove temporary token schema"),
    }

    assert!(!output.status.success(), "check unexpectedly passed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("card.schema.json"), "{stderr}");
    assert!(stderr.contains("token.schema.json"), "{stderr}");
}
