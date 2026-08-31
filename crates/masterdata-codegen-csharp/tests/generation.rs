use std::fs;

use masterdata_codegen_csharp::CSharpGenerator;
use masterdata_core::ProjectService;
use tempfile::tempdir;

#[test]
fn renders_an_immutable_scaffold_from_a_schema() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.test\"\nname = \"Codegen\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(
        directory.path().join("sources").join("schema.yaml"),
        "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\n",
    )
    .expect("schema");
    fs::write(
        directory.path().join("sources").join("data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 1001\n    name: Potion\n",
    )
    .expect("data");

    let plan = ProjectService::new()
        .prepare_build(Some(directory.path()), directory.path())
        .expect("build plan");
    let generated = CSharpGenerator::default()
        .plan(&plan)
        .expect("generation plan");
    assert_eq!(generated.files.len(), 1);
    assert!(
        generated.files[0]
            .contents
            .contains("public sealed partial class ItemMaster")
    );
    assert!(
        generated.files[0]
            .contents
            .contains("// Source table identity: item")
    );
    assert!(generated.notes.iter().any(|note| note.placeholder));
}

#[test]
fn rejects_reserved_csharp_type_names() {
    let directory = fixture_project("kind: schema\ntable: item\ncsharpName: class\nfields: []\n");
    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default().plan(&plan).expect_err("keyword");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-INVALID-TYPE-NAME");
}

#[test]
fn rejects_invalid_table_field_names_without_normalization() {
    let directory = fixture_project(
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: foo-bar\n    type: int\n  - key: 1\n    name: foo_bar\n    type: int\n",
    );
    let error = ProjectService::new()
        .prepare_build(Some(directory.path()), directory.path())
        .expect_err("invalid field names");
    assert_eq!(error.diagnostic().code, "E-BUILD-VALIDATION-FAILED");
}

#[test]
fn rejects_invalid_namespace_before_rendering() {
    let directory = fixture_project("kind: schema\ntable: item\nfields: []\n");
    let plan = build_plan(directory.path());
    let error = CSharpGenerator::new("Masterdata..Generated")
        .plan(&plan)
        .expect_err("namespace");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-INVALID-NAMESPACE");
}

#[test]
fn rejects_type_name_collisions_across_tables() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.collision\"\nname = \"Codegen Collision\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (name, table) in [("first.yaml", "foo-bar"), ("second.yaml", "foo_bar")] {
        fs::write(
            directory.path().join("sources").join(name),
            format!("kind: schema\ntable: {table}\nfields: []\n"),
        )
        .expect("schema");
    }

    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default()
        .plan(&plan)
        .expect_err("type collision");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-TYPE-NAME-COLLISION");
}

#[test]
fn rejects_case_insensitive_generated_filename_collisions() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.filename\"\nname = \"Codegen Filename\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (name, csharp_name) in [("first.yaml", "Item"), ("second.yaml", "item")] {
        fs::write(
            directory.path().join("sources").join(name),
            format!("kind: schema\ntable: {name}\ncsharpName: {csharp_name}\nfields: []\n"),
        )
        .expect("schema");
    }

    let plan = build_plan(directory.path());
    let error = CSharpGenerator::default()
        .plan(&plan)
        .expect_err("filename collision");
    assert_eq!(error.diagnostic().code, "E-CODEGEN-FILENAME-COLLISION");
}

#[test]
fn renders_type_system_api_in_declaration_order() {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.types\"\nname = \"Codegen Types\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    let sources = [
        (
            "item-id.yaml",
            "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n  conversions:\n    fromUnderlyingImplicit: true\n    toUnderlyingImplicit: true\n",
        ),
        (
            "reward.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 9\n      name: tags\n      type: string\n      array: true\n    - key: 1\n      name: itemId\n      type: ItemId\n    - key: 4\n      name: note\n      type: string\n      nullable: true\n",
        ),
        (
            "rarity.yaml",
            "kind: type\nname: ItemRarity\nenum:\n  underlying: long\n  members:\n    - name: Legendary\n      value: 100\n    - name: Common\n      value: 1\n",
        ),
        (
            "feature.yaml",
            "kind: type\nname: Feature\nflags:\n  underlying: uint\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n    - name: Ice\n      value: 2\n",
        ),
        (
            "item-schema.yaml",
            "kind: schema\ntable: item\ncsharpName: ItemRow\nfields:\n  - key: 1\n    name: rarity\n    type: ItemRarity\n  - key: 0\n    name: itemId\n    type: ItemId\n  - key: 2\n    name: tags\n    type: Feature\n    array: true\n",
        ),
    ];
    for (file, source) in sources {
        fs::write(directory.path().join("sources").join(file), source).expect("source");
    }

    let plan = ProjectService::new()
        .prepare_build(Some(directory.path()), directory.path())
        .expect("build plan");
    let generated = CSharpGenerator::default()
        .plan(&plan)
        .expect("generation plan");

    let contents = |name: &str| {
        let relative_path = std::path::PathBuf::from(format!("{name}.g.cs"));
        generated
            .files
            .iter()
            .find(|file| file.relative_path == relative_path)
            .unwrap_or_else(|| panic!("missing generated file {name}"))
            .contents
            .clone()
    };

    let item_id = contents("ItemId");
    assert!(item_id.contains("public readonly struct ItemId"));
    assert!(item_id.contains("System.IComparable<ItemId>"));
    assert!(item_id.contains("public int CompareTo(ItemId other)"));
    assert!(item_id.contains("operator <(ItemId left, ItemId right)"));
    assert!(item_id.contains("implicit operator ItemId(int value)"));
    assert!(item_id.contains("implicit operator int(ItemId value)"));

    let reward = contents("Reward");
    let tags = reward
        .find("ImmutableArray<string> Tags")
        .expect("Tags property");
    let item = reward.find("ItemId ItemId").expect("ItemId property");
    let note = reward.find("string? Note").expect("Note property");
    assert!(
        tags < item && item < note,
        "properties follow declaration order"
    );
    assert!(reward.contains(
        "public Reward(System.Collections.Immutable.ImmutableArray<string> tags, ItemId itemId, string? note)"
    ));
    assert!(reward.contains("[MessagePack.Key(9)]"));
    assert!(reward.contains("[MessagePack.Key(1)]"));
    assert!(reward.contains("if (tags.IsDefault)"));
    assert!(reward.contains("public bool Equals(Reward other)"));
    assert!(reward.contains("SequenceEqual(Tags, other.Tags)"));

    let rarity = contents("ItemRarity");
    assert!(rarity.contains("public enum ItemRarity : long"));
    assert!(
        rarity.find("Legendary = 100").expect("Legendary")
            < rarity.find("Common = 1").expect("Common")
    );

    let feature = contents("Feature");
    assert!(feature.contains("[System.Flags]"));
    assert!(feature.contains("public enum Feature : uint"));
    assert!(feature.contains("None = 0,"));

    let table = contents("ItemRow");
    assert!(table.contains("[MessagePack.Key(1)]"));
    assert!(table.contains("public ItemRarity Rarity { get; init; }"));
    assert!(table.contains("public ItemId ItemId { get; init; }"));
    assert!(table.contains(
        "public System.Collections.Immutable.ImmutableArray<Feature> Tags { get; init; }"
    ));
}

#[test]
fn generated_type_system_csharp_compiles() {
    if std::process::Command::new("dotnet")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("dotnet is unavailable; skipping generated C# compilation check");
        return;
    }

    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.compile\"\nname = \"Codegen Compile\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    for (file, source) in [
        (
            "item-id.yaml",
            "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
        ),
        (
            "user-code.yaml",
            "kind: type\nname: UserCode\nvalueObject:\n  underlying: string\n",
        ),
        (
            "reward.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: itemId\n      type: ItemId\n    - key: 1\n      name: note\n      type: string\n      nullable: true\n    - key: 2\n      name: tags\n      type: string\n      array: true\n",
        ),
        (
            "rarity.yaml",
            "kind: type\nname: ItemRarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n",
        ),
        (
            "feature.yaml",
            "kind: type\nname: Feature\nflags:\n  underlying: uint\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n",
        ),
        (
            "long-feature.yaml",
            "kind: type\nname: LongFeature\nflags:\n  underlying: long\n  members:\n    - name: None\n      value: 0\n    - name: Highest\n      value: -9223372036854775808\n",
        ),
        (
            "item-schema.yaml",
            "kind: schema\ntable: item\ncsharpName: ItemRow\nfields:\n  - key: 0\n    name: itemId\n    type: ItemId\n  - key: 1\n    name: rarity\n    type: ItemRarity\n  - key: 2\n    name: featureValues\n    type: Feature\n    array: true\n",
        ),
    ] {
        fs::write(directory.path().join("sources").join(file), source).expect("source");
    }
    let plan = ProjectService::new()
        .prepare_build(Some(directory.path()), directory.path())
        .expect("build plan");
    let generated = CSharpGenerator::default()
        .plan(&plan)
        .expect("generation plan");
    let generated_dir = directory.path().join("Generated");
    fs::create_dir(&generated_dir).expect("Generated");
    for file in &generated.files {
        fs::write(generated_dir.join(&file.relative_path), &file.contents).expect("generated C#");
    }
    fs::write(
        directory.path().join("MessagePackStub.cs"),
        "using System;\nnamespace MessagePack;\n[AttributeUsage(AttributeTargets.Property)]\npublic sealed class KeyAttribute : Attribute\n{\n    public KeyAttribute(int key) { }\n}\n",
    )
    .expect("MessagePack stub");
    fs::write(
        directory.path().join("compile.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <OutputType>Exe</OutputType>\n    <TargetFramework>net8.0</TargetFramework>\n    <Nullable>enable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n  </PropertyGroup>\n  <ItemGroup>\n    <Compile Include=\"Generated/*.g.cs\" />\n    <Compile Include=\"MessagePackStub.cs\" />\n    <Compile Include=\"Program.cs\" />\n  </ItemGroup>\n</Project>\n",
    )
    .expect("project");
    fs::write(
        directory.path().join("Program.cs"),
        "using System.Collections.Immutable;\nusing System.Globalization;\nusing Masterdata.Generated;\n\nif (new ItemId(1).CompareTo(new ItemId(2)) >= 0) return 1;\nif (!(new ItemId(1) < new ItemId(2))) return 2;\nif (!new ItemId(1).Equals(new ItemId(1))) return 3;\nif (new UserCode(\"A\").CompareTo(new UserCode(\"a\")) == 0) return 4;\nCultureInfo.CurrentCulture = new CultureInfo(\"tr-TR\");\nif (new UserCode(\"A\").CompareTo(new UserCode(\"B\")) >= 0) return 5;\nvar first = new Reward(new ItemId(1), \"note\", ImmutableArray.Create(\"a\", \"b\"));\nvar second = new Reward(new ItemId(1), \"note\", ImmutableArray.Create(\"a\", \"b\"));\nif (!first.Equals(second) || first.GetHashCode() != second.GetHashCode()) return 6;\nif (Feature.Fire != (Feature)1) return 7;\nreturn 0;\n",
    )
    .expect("program");

    let output = std::process::Command::new("dotnet")
        .args([
            "build",
            "compile.csproj",
            "--nologo",
            "--configuration",
            "Release",
        ])
        .current_dir(directory.path())
        .output()
        .expect("dotnet build");
    assert!(
        output.status.success(),
        "generated C# did not compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new("dotnet")
        .args([
            "run",
            "--project",
            "compile.csproj",
            "--no-build",
            "--configuration",
            "Release",
        ])
        .current_dir(directory.path())
        .output()
        .expect("dotnet run");
    assert!(
        output.status.success(),
        "generated C# runtime smoke test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture_project(schema: &str) -> tempfile::TempDir {
    let directory = tempdir().expect("temp directory");
    fs::create_dir(directory.path().join("sources")).expect("sources");
    fs::write(
        directory.path().join("masterdata.toml"),
        "[project]\nid = \"codegen.test\"\nname = \"Codegen\"\nversion = \"0.1.0\"\n",
    )
    .expect("config");
    fs::write(directory.path().join("sources").join("schema.yaml"), schema).expect("schema");
    directory
}

fn build_plan(root: &std::path::Path) -> masterdata_core::BuildPlan {
    ProjectService::new()
        .prepare_build(Some(root), root)
        .expect("build plan")
}
