#![cfg(feature = "native-consumer")]
use masterdata_engine::{
    delivery::{self, BuildOptions, BuildPlan, SavedConfig},
    native::{Outcome, artifact::ArtifactSet, dotnet},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn copy_project(from: &Path) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::copy(
        from.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(from.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    temp
}
fn bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(root.join("sources"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.path(), fs::read(entry.path()).unwrap())
        })
        .collect()
}
#[test]
fn independent_consumers_reload_nested_values_and_sparse_keys() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let oracle = repository.join("fixtures/rewrite-oracle/v1/consumer");
    for (fixture, consumer, extra, counts) in [
        (
            oracle.join("minimal"),
            oracle.join("minimal/Consumer.cs"),
            MINIMAL_API,
            BTreeMap::from([("miniature".to_owned(), 2)]),
        ),
        (
            repository.join("fixtures/full"),
            oracle.join("Consumer.cs"),
            FULL_API,
            BTreeMap::from([("item".to_owned(), 2), ("probe".to_owned(), 1)]),
        ),
    ] {
        let project = copy_project(&fixture);
        if fixture.ends_with("full") {
            for file in ["probe-schema.yaml", "probe-data.yaml"] {
                fs::copy(oracle.join(file), project.path().join("sources").join(file)).unwrap();
            }
        }
        let before = bytes(project.path());
        let plan = BuildPlan::capture(project.path(), None).unwrap();
        assert_eq!(plan.row_counts(), counts);
        let built = dotnet::build(&plan).unwrap();
        assert!(
            built
                .native_evidence
                .contains("actual reload / every selected row")
        );
        let result =
            dotnet::verify_consumer(plan.csharp(), &built.binary, &consumer, extra).unwrap();
        for line in result.lines().filter(|line| line.starts_with("PASS ")) {
            println!("{line}");
        }
        assert!(result.contains("PASS "));
        let repeated = BuildPlan::capture(project.path(), None).unwrap();
        assert_eq!(
            plan.csharp(),
            repeated.csharp(),
            "generated public source is nondeterministic"
        );
        assert_eq!(
            built.binary,
            dotnet::build(&repeated).unwrap().binary,
            "native binary is nondeterministic"
        );
        assert_eq!(
            bytes(project.path()),
            before,
            "Build wrote canonical source"
        );
        assert!(
            !project.path().join(".masterdata/output").exists(),
            "native builder bypassed coherent artifact publication"
        );
        let dry_run = delivery::build(
            project.path(),
            &BuildOptions {
                dry_run: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(dry_run.outcome, Outcome::Success);
        assert!(
            !project.path().join(".masterdata").exists(),
            "dry run created artifact metadata or parent"
        );
        let published = delivery::build(project.path(), &BuildOptions::default()).unwrap();
        assert_eq!(published.outcome, Outcome::Success);
        let artifact = ArtifactSet::load(&SavedConfig::load(project.path()).unwrap()).unwrap();
        assert_eq!(artifact.binary, built.binary);
        assert_eq!(
            artifact.csharp,
            plan.csharp()
                .iter()
                .map(|(name, bytes)| (name.clone(), bytes.as_bytes().to_vec()))
                .collect()
        );
        dotnet::verify_consumer(plan.csharp(), &artifact.binary, &consumer, extra).unwrap();
    }
}
const MINIMAL_API: &str = r#"
var constructor=typeof(Masterdata.Generated.Reward).GetConstructors()[0];
var parameters=constructor.GetParameters();
if(parameters.Length!=2 || parameters[0].Name!="itemId" || parameters[1].Name!="amount") throw new System.Exception("public constructor declaration order changed");
if(typeof(Masterdata.Generated.Reward).GetProperty("ItemId")!.CanWrite) throw new System.Exception("Custom property is mutable");
if(typeof(Masterdata.Generated.ItemId).GetProperty("Value")!.CanWrite) throw new System.Exception("Value Object property is mutable");
var one=new Masterdata.Generated.Reward(itemId:new Masterdata.Generated.ItemId(7),amount:9);
var two=new Masterdata.Generated.Reward(itemId:new Masterdata.Generated.ItemId(7),amount:9);
if(one!=two || one.GetHashCode()!=two.GetHashCode()) throw new System.Exception("Custom equality/hash mismatch");
System.Console.WriteLine("PASS public immutable API / named arguments / equality");
"#;
const FULL_API: &str = r#"
var first = new Masterdata.Generated.Reward(values:System.Collections.Immutable.ImmutableArray.Create(1L,2L), itemId:new Masterdata.Generated.ItemId(7), note:null, amount:9);
var second = new Masterdata.Generated.Reward(values:System.Collections.Immutable.ImmutableArray.Create(1L,2L), itemId:new Masterdata.Generated.ItemId(7), note:null, amount:9);
var reversed = new Masterdata.Generated.Reward(values:System.Collections.Immutable.ImmutableArray.Create(2L,1L), itemId:new Masterdata.Generated.ItemId(7), note:null, amount:9);
if(first!=second || first.GetHashCode()!=second.GetHashCode() || first==reversed) throw new System.Exception("Array equality/hash/order corrupted");
try { _=new Masterdata.Generated.Reward(values:default, itemId:new Masterdata.Generated.ItemId(7), note:null, amount:9);throw new System.Exception("default Array accepted"); } catch(System.ArgumentException) {}
try { _=new Masterdata.Generated.ItemCode(null!);throw new System.Exception("null required string accepted"); } catch(System.ArgumentNullException) {}
var oldCulture=System.Globalization.CultureInfo.CurrentCulture;
try {
    System.Globalization.CultureInfo.CurrentCulture=System.Globalization.CultureInfo.GetCultureInfo("tr-TR");
    if(new Masterdata.Generated.ItemCode("I").CompareTo(new Masterdata.Generated.ItemCode("i"))>=0) throw new System.Exception("string Value Object comparison is not Ordinal");
} finally {System.Globalization.CultureInfo.CurrentCulture=oldCulture;}
System.Console.WriteLine("PASS ImmutableArray / constructor guards / Ordinal comparison");
"#;

#[test]
fn authored_messagepack_keys_primary_secondary_and_reference_helpers_reload_in_native_consumer() {
    use masterdata_engine::{
        table_declaration::{Change, Command, ReferenceInput},
        workspace::Workspace,
    };
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let oracle = repository.join("fixtures/rewrite-oracle/v1/consumer");
    let project = copy_project(&repository.join("fixtures/full"));
    for file in ["probe-schema.yaml", "probe-data.yaml"] {
        fs::copy(oracle.join(file), project.path().join("sources").join(file)).unwrap();
    }
    let before = bytes(project.path());
    let mut workspace = Workspace::open(project.path()).unwrap();
    let changes = [
        Change::SetFieldKey {
            occurrence: 0,
            key: "25".into(),
        },
        Change::AddSecondaryKey {
            fields: vec!["name".into()],
            non_unique: true,
        },
        Change::AddReference {
            declaration: ReferenceInput {
                name: "peers".into(),
                fields: vec!["code".into()],
                target_table: "item".into(),
                target_fields: vec!["code".into()],
                csharp_name: Some("FindPeers".into()),
            },
        },
    ];
    let apply = |workspace: &mut Workspace, change| {
        let detail = workspace
            .table_declaration_detail("sources/catalog-schema.yaml", "item")
            .unwrap()
            .detail;
        let review = workspace
            .prepare_table_declaration(Command {
                table: detail.table,
                source: detail.source,
                identity: detail.identity,
                change,
            })
            .unwrap();
        assert_eq!(
            workspace
                .apply_migration(&review.plan.token, false)
                .unwrap()
                .outcome,
            Outcome::Success
        );
    };
    for change in changes {
        apply(&mut workspace, change);
    }
    let plan = BuildPlan::capture(project.path(), None).unwrap();
    let built = dotnet::build(&plan).unwrap();
    let extra = r#"
var db=new Masterdata.Generated.MemoryDatabase(System.IO.File.ReadAllBytes("masterdata.bytes"));
var row=db.ItemMasterTable.FindById(1001);
var matches=0;foreach(var peer in row.FindPeers(db)){if(peer.Code.Value!="sword")throw new System.Exception("wrong relation");matches++;}
if(matches!=2)throw new System.Exception("authored nonunique Reference lost matches");
matches=0;foreach(var named in db.ItemMasterTable.FindByName("Sword")){if(named.Id!=1001)throw new System.Exception("wrong authored key");matches++;}
if(matches!=1)throw new System.Exception("authored secondary lookup failed");
System.Console.WriteLine("PASS authored sparse MessagePack key / nonunique SK / exact helper override / relation");
"#;
    let result = dotnet::verify_consumer(
        plan.csharp(),
        &built.binary,
        &oracle.join("Consumer.cs"),
        extra,
    )
    .unwrap();
    assert!(result.contains("PASS authored sparse MessagePack key"));
    // Removing the old PK would invalidate the probe's incoming Reference.
    // Remove that relationship explicitly; preserve every record byte.
    let detail = workspace
        .table_declaration_detail("sources/probe-schema.yaml", "probe")
        .unwrap()
        .detail;
    let removal = workspace
        .prepare_table_declaration(Command {
            table: detail.table,
            source: detail.source,
            identity: detail.identity,
            change: Change::RemoveReference { occurrence: 0 },
        })
        .unwrap();
    assert_eq!(
        workspace
            .apply_migration(&removal.plan.token, true)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    apply(
        &mut workspace,
        Change::SetPrimaryKey {
            fields: vec!["intValue".into()],
        },
    );
    apply(
        &mut workspace,
        Change::EditSecondaryKey {
            occurrence: 2,
            fields: vec!["rarity".into(), "code".into()],
            non_unique: false,
        },
    );
    let plan = BuildPlan::capture(project.path(), None).unwrap();
    let built = dotnet::build(&plan).unwrap();
    let consumer = project.path().join("AuthoredConsumer.cs");
    fs::write(&consumer, r#"
using System;
using MasterMemory;
using Masterdata.Generated;
[assembly: MasterMemoryGeneratorOptions(Namespace="Masterdata.Generated")]
namespace RewriteOracle { public static class Consumer { public static void Check(byte[] bytes) {
var db=new MemoryDatabase(bytes);
var first=db.ItemMasterTable.FindByIntValue(int.MaxValue);var second=db.ItemMasterTable.FindByIntValue(int.MinValue);
if(first.Id!=1001||second.Id!=1002||first.Reward.ItemId.Value!=2001||first.LongValue!=long.MinValue||first.UlongValue!=ulong.MaxValue)throw new Exception("authored PK or preserved values corrupted");
if(db.ItemMasterTable.FindByRarityAndCode((Rarity.Common,new ItemCode("sword"))).Id!=1001)throw new Exception("authored composite order corrupted");
if(db.ProbeTable.FindById(1).ParentId!=1001||typeof(Probe).GetMethod("GetParent")!=null)throw new Exception("explicit relationship removal changed records or retained helper");
Console.WriteLine("PASS authored PK / ordered composite SK / nested Value Object / lossless values / explicit Reference removal");
} } }
"#).unwrap();
    let result = dotnet::verify_consumer(plan.csharp(), &built.binary, &consumer, "").unwrap();
    assert!(result.contains("PASS authored PK / ordered composite SK"));
    for (path, original) in before {
        if !["catalog-schema.yaml", "probe-schema.yaml"]
            .iter()
            .any(|name| path.file_name().unwrap() == *name)
        {
            assert_eq!(
                fs::read(path).unwrap(),
                original,
                "declaration authoring changed non-target bytes"
            );
        }
    }
    println!("PASS authored declarations / independent oracle / actual MasterMemory reload");
}

#[test]
fn empty_table_preserves_generated_api_and_profiles_use_the_same_canonical_dataset() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let minimal = copy_project(&repository.join("fixtures/rewrite-oracle/v1/consumer/minimal"));
    fs::remove_file(minimal.path().join("sources/data.yaml")).unwrap();
    let plan = BuildPlan::capture(minimal.path(), None).unwrap();
    assert_eq!(plan.row_counts(), BTreeMap::from([("miniature".into(), 0)]));
    let built = dotnet::build(&plan).unwrap();
    let consumer = minimal.path().join("EmptyConsumer.cs");
    fs::write(&consumer,"using MasterMemory;\n[assembly: MasterMemoryGeneratorOptions(Namespace=\"Masterdata.Generated\")]\nnamespace RewriteOracle {public static class Consumer {public static void Check(byte[] bytes) {var db=new Masterdata.Generated.MemoryDatabase(bytes);if(db.MiniatureTable.Count!=0)throw new System.Exception(\"empty Table omitted\");System.Console.WriteLine(\"PASS empty Table API / actual reload\");}}}\n").unwrap();
    dotnet::verify_consumer(plan.csharp(), &built.binary, &consumer, "").unwrap();
    let full = copy_project(&repository.join("fixtures/full"));
    let config = full.path().join("masterdata.toml");
    let mut bytes = fs::read_to_string(&config).unwrap();
    bytes.push_str("\n[build.profiles.production]\ninclude_tags=[\"production\"]\n[build.profiles.production-copy]\ninclude_tags=[\"production\"]\n");
    fs::write(config, bytes).unwrap();
    let production = BuildPlan::capture(full.path(), Some("production")).unwrap();
    let same = BuildPlan::capture(full.path(), Some("production-copy")).unwrap();
    assert_eq!(
        production.row_counts(),
        BTreeMap::from([("item".into(), 1)])
    );
    assert_eq!(
        dotnet::build(&production).unwrap().binary,
        dotnet::build(&same).unwrap().binary,
        "Profile names leaked into binary identity"
    );
    assert_eq!(production.csharp(), same.csharp());
    assert!(BuildPlan::capture(full.path(), Some("missing")).is_err());
    println!("PASS empty Table / profile capture / canonical dataset determinism");
}

#[test]
fn distinct_case_identifiers_compile_without_generated_filename_aliases() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let project = copy_project(&repository.join("fixtures/rewrite-oracle/v1/consumer/minimal"));
    fs::write(
        project.path().join("sources/upper-id.yaml"),
        "kind: type\nname: ITEMID\nvalueObject:\n  underlying: int\n",
    )
    .unwrap();
    let plan = BuildPlan::capture(project.path(), None).unwrap();
    assert!(
        plan.csharp()
            .values()
            .any(|source| source.contains("struct ITEMID"))
    );
    assert!(
        plan.csharp()
            .values()
            .any(|source| source.contains("struct ItemId"))
    );
    let staged = tempfile::tempdir().unwrap();
    for name in plan.csharp().keys() {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(staged.path().join(name))
            .unwrap();
    }
    let binary = dotnet::build(&plan).unwrap().binary;
    dotnet::verify_consumer(
        plan.csharp(),
        &binary,
        &repository.join("fixtures/rewrite-oracle/v1/consumer/minimal/Consumer.cs"),
        "",
    )
    .unwrap();
    fs::write(
        project.path().join("sources/attribute-name.yaml"),
        "kind: type\nname: PrimaryKey\nvalueObject:\n  underlying: int\n",
    )
    .unwrap();
    assert_eq!(
        BuildPlan::capture(project.path(), None).err().unwrap().code,
        "E-CODEGEN"
    );
    println!("PASS exact identifiers / portable distinct file slots / generated-scope collision");
}
