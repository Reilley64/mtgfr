use std::{fs, path::Path, process::ExitCode};

use serde_json::{Map, Value, json};

const CARD_SCHEMA_PATH: &str = "crates/cards/schema/card.schema.json";
const TOKEN_SCHEMA_PATH: &str = "crates/cards/schema/token.schema.json";
const SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

fn main() -> ExitCode {
    let check = std::env::args().any(|arg| arg == "--check");
    let card = card_schema();
    let token = token_schema(&card);
    let mut drifted = false;

    drifted |= write_or_check(CARD_SCHEMA_PATH, &card, check);
    drifted |= write_or_check(TOKEN_SCHEMA_PATH, &token, check);

    if drifted {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn card_schema() -> Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(engine::toml_surface::CardToml))
        .expect("CardToml schema serializes");
    normalize_schema(&mut schema, "mtgfr card TOML");
    schema
}

fn token_schema(card: &Value) -> Value {
    let mut schema = card.clone();
    normalize_schema(&mut schema, "mtgfr token TOML");
    require_non_empty_string(&mut schema, "default_print");
    require_non_empty_string(&mut schema, "id");
    schema
}

fn normalize_schema(schema: &mut Value, title: &str) {
    convert_definitions_to_defs(schema);

    let object = schema.as_object_mut().expect("root schema object");
    object.insert(
        "$schema".to_owned(),
        Value::String(SCHEMA_DIALECT.to_owned()),
    );
    object.insert("title".to_owned(), Value::String(title.to_owned()));
}

fn convert_definitions_to_defs(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(definitions) = object.remove("definitions") {
                object.insert("$defs".to_owned(), definitions);
            }

            for value in object.values_mut() {
                convert_definitions_to_defs(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                convert_definitions_to_defs(value);
            }
        }
        Value::String(text) => {
            if text.starts_with("#/definitions/") {
                *text = text.replacen("#/definitions/", "#/$defs/", 1);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn require_non_empty_string(schema: &mut Value, field: &str) {
    let object = schema.as_object_mut().expect("root schema object");
    let properties = object
        .entry("properties")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("root properties object");

    let property = properties
        .entry(field.to_owned())
        .or_insert_with(|| json!({ "type": "string" }))
        .as_object_mut()
        .expect("string field schema object");
    property.insert("type".to_owned(), Value::String("string".to_owned()));
    property.insert("minLength".to_owned(), json!(1));
    property.remove("default");

    let required = object
        .entry("required")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .expect("root required array");

    let field_value = Value::String(field.to_owned());
    if required.contains(&field_value) {
        return;
    }

    required.insert(0, field_value);
}

fn write_or_check(path: &str, schema: &Value, check: bool) -> bool {
    let rendered = serde_json::to_string_pretty(schema).expect("schema renders") + "\n";
    let path = Path::new(path);

    if !check {
        fs::create_dir_all(path.parent().expect("schema parent")).expect("create schema dir");
        fs::write(path, rendered).expect("write schema");
        return false;
    }

    match fs::read_to_string(path) {
        Ok(existing) if existing == rendered => false,
        Ok(_) => {
            eprintln!("{} is stale; run `just cards-schema`", path.display());
            true
        }
        Err(err) => {
            eprintln!("{} is missing or unreadable: {err}", path.display());
            true
        }
    }
}
