use masterdata_core::*;
use serde_json::json;
use std::path::{Path, PathBuf};
fn request(value: serde_json::Value) -> SourceCreation {
    serde_json::from_value(value).unwrap()
}
fn table(name: &str) -> SourceCreation {
    request(
        json!({"category":"table","table":name,"fields":[{"key":0,"name":"id","type":"int"}],"primaryKey":{"fields":["id"]},"secondaryKeys":[]}),
    )
}
fn prepare(
    documents: &ProjectDocuments,
    artifact: &SourceCreation,
) -> masterdata_core::Result<SourceCreationPlan> {
    prepare_source_creation(documents, Path::new("new.yaml"), artifact)
}
#[test]
fn creation_all_artifact_categories_roundtrip_with_exact_enum_values() {
    let mut documents = ProjectDocuments::default();
    let schema = prepare(&documents, &table("item")).unwrap();
    documents.files.push(schema.document);
    let cases = [
        json!({"category":"data","table":"item"}),
        json!({"category":"value_object","name":"ItemId","underlying":"long","conversions":{"fromUnderlyingImplicit":true,"toUnderlyingImplicit":false}}),
        json!({"category":"enum","name":"Kind","underlying":"ulong","members":[{"name":"Max","value":"18446744073709551615"}]}),
        json!({"category":"flags","name":"Mode","underlying":"uint","members":[{"name":"None","value":"0"},{"name":"Read","value":"1"}]}),
        json!({"category":"custom_type","name":"Reward","fields":[{"key":0,"name":"amount","type":"int"}]}),
    ];
    for value in cases {
        let req = request(value);
        let plan = prepare(&documents, &req).unwrap();
        assert_eq!(plan.source, prepare(&documents, &req).unwrap().source);
        assert_eq!(
            parse_yaml_document(PathBuf::from("new.yaml"), &plan.source)
                .unwrap()
                .document,
            plan.document.document
        );
        if let SourceDocument::Data(data) = plan.document.document {
            assert!(data.records.is_empty());
        }
    }
}
#[test]
fn creation_rejects_unknown_table_collision_invalid_flags_and_dependencies() {
    let mut documents = ProjectDocuments::default();
    assert!(
        prepare(
            &documents,
            &request(json!({"category":"data","table":"missing"}))
        )
        .is_err()
    );
    documents
        .files
        .push(prepare(&documents, &table("item")).unwrap().document);
    assert!(prepare(&documents, &table("item")).is_err());
    assert!(prepare(&documents, &request(json!({"category":"flags","name":"Bad","underlying":"int","members":[{"name":"Both","value":"3"}]}))).is_err());
    assert!(prepare(&documents, &request(json!({"category":"custom_type","name":"Bad","fields":[{"key":0,"name":"missing","type":"Unknown"}]}))).is_err());
}
#[test]
fn unrelated_invalid_type_does_not_block_creation_but_referenced_invalid_type_does() {
    let documents = ProjectDocuments {
        files: vec![
            parse_yaml_document(
                PathBuf::from("bad.yaml"),
                "kind: type\nname: Invalid\nvalueObject:\n  underlying: bool\n",
            )
            .unwrap(),
        ],
    };
    assert!(prepare(&documents, &table("item")).is_ok());
    assert!(prepare(&documents, &request(json!({"category":"custom_type","name":"UsesInvalid","fields":[{"key":0,"name":"value","type":"Invalid"}]}))).is_err());
}

#[test]
fn table_creation_preserves_modifiers_and_composite_key_order() {
    let req = request(json!({
        "category":"table", "table":"item", "csharpName":"InventoryItem",
        "fields":[
            {"key":0,"name":"id","type":"int"},
            {"key":1,"name":"version","type":"int"},
            {"key":2,"name":"price","type":"int","nullable":true},
            {"key":3,"name":"labels","type":"string","array":true}
        ],
        "primaryKey":{"fields":["version","id"]},
        "secondaryKeys":[{"fields":["id"],"nonUnique":true}]
    }));
    let plan = prepare(&ProjectDocuments::default(), &req).unwrap();
    let SourceDocument::Schema(schema) = plan.document.document else {
        panic!("schema expected")
    };
    assert_eq!(schema.primary_key.unwrap().fields, ["version", "id"]);
    assert!(schema.secondary_keys[0].non_unique);
    assert!(schema.fields[2].nullable);
    assert!(schema.fields[3].array);
    assert_eq!(schema.csharp_name.as_deref(), Some("InventoryItem"));
}
