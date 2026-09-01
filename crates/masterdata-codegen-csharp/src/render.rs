use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use masterdata_core::{
    BuildPlan, ErrorKind, FieldModifier, MasterdataError, PrimitiveType, ResolvedField,
    ResolvedTable, ResolvedType, Result, TypeReference, TypeSystem, csharp_property_name,
    is_csharp_reserved_keyword,
};

use crate::model::{CSharpGenerationPlan, GeneratedFile, GenerationNote};

#[derive(Debug, Clone)]
pub struct CSharpGenerator {
    namespace: String,
}

impl Default for CSharpGenerator {
    fn default() -> Self {
        Self::new("Masterdata.Generated")
    }
}

impl CSharpGenerator {
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    pub fn plan(&self, build_plan: &BuildPlan) -> Result<CSharpGenerationPlan> {
        validate_namespace(&self.namespace)?;
        let has_types = !build_plan.type_system.types.is_empty();
        if build_plan.tables.is_empty() && !has_types {
            return Err(MasterdataError::new(
                "E-CODEGEN-NO-DECLARATIONS",
                ErrorKind::NotImplemented,
                "C# generation requires at least one schema or type declaration",
            ));
        }

        let mut files =
            Vec::with_capacity(build_plan.tables.len() + build_plan.type_system.types.len());
        let mut generated_type_names = BTreeSet::new();
        let mut generated_file_names = BTreeSet::new();

        for resolved in build_plan.type_system.types.values() {
            let type_name = resolved.name().to_owned();
            insert_generated_name(
                &mut generated_type_names,
                &mut generated_file_names,
                &type_name,
            )?;
            files.push(render_type(
                &self.namespace,
                resolved,
                &build_plan.type_system,
            )?);
        }

        for table in &build_plan.tables {
            let type_name = table.csharp_name.clone();
            insert_generated_name(
                &mut generated_type_names,
                &mut generated_file_names,
                &type_name,
            )?;
            files.push(render_schema(
                &self.namespace,
                table,
                &build_plan.type_system,
            )?);
        }
        Ok(CSharpGenerationPlan {
            namespace: self.namespace.clone(),
            files,
            notes: vec![GenerationNote {
                message: "Reference integrity, cache reuse, and released binary compatibility remain outside this slice.".to_owned(),
                placeholder: true,
            }],
        })
    }

    pub fn write_to(
        &self,
        plan: &CSharpGenerationPlan,
        output_dir: &Path,
    ) -> Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(output_dir).map_err(|error| {
            MasterdataError::new(
                "E-CODEGEN-OUTPUT-CREATE",
                ErrorKind::Io,
                format!("could not create C# output directory: {error}"),
            )
            .with_source(output_dir.to_path_buf())
        })?;
        let mut written = Vec::with_capacity(plan.files.len());
        for file in &plan.files {
            let path = safe_generated_path(output_dir, &file.relative_path)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    MasterdataError::new(
                        "E-CODEGEN-OUTPUT-PARENT-CREATE",
                        ErrorKind::Io,
                        format!("could not create C# output directory: {error}"),
                    )
                    .with_source(parent.to_path_buf())
                })?;
            }
            std::fs::write(&path, &file.contents).map_err(|error| {
                MasterdataError::new(
                    "E-CODEGEN-OUTPUT-WRITE",
                    ErrorKind::Io,
                    format!("could not write generated C#: {error}"),
                )
                .with_source(path.clone())
            })?;
            written.push(path);
        }
        Ok(written)
    }
}

fn safe_generated_path(output_dir: &Path, relative_path: &Path) -> Result<std::path::PathBuf> {
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
                    | std::path::Component::ParentDir
            )
        })
    {
        return Err(MasterdataError::new(
            "E-CODEGEN-OUTPUT-PATH-ESCAPE",
            ErrorKind::Validation,
            format!(
                "generated relative path `{}` is not safely contained by the output directory",
                relative_path.display()
            ),
        )
        .with_source(output_dir.to_path_buf()));
    }
    Ok(output_dir.join(relative_path))
}

fn insert_generated_name(
    generated_type_names: &mut BTreeSet<String>,
    generated_file_names: &mut BTreeSet<String>,
    type_name: &str,
) -> Result<()> {
    if !is_csharp_identifier(type_name) || is_csharp_reserved_keyword(type_name) {
        return Err(MasterdataError::new(
            "E-CODEGEN-INVALID-TYPE-NAME",
            ErrorKind::Validation,
            format!("`{type_name}` is not a valid C# type identifier"),
        ));
    }
    if !generated_type_names.insert(type_name.to_owned()) {
        return Err(MasterdataError::new(
            "E-CODEGEN-TYPE-NAME-COLLISION",
            ErrorKind::Validation,
            format!("multiple declarations generate the C# type `{type_name}`"),
        ));
    }
    let filename_key = type_name.to_ascii_lowercase();
    if !generated_file_names.insert(filename_key) {
        return Err(MasterdataError::new(
            "E-CODEGEN-FILENAME-COLLISION",
            ErrorKind::Validation,
            format!("generated filenames collide case-insensitively with `{type_name}.g.cs`"),
        ));
    }
    Ok(())
}

fn render_type(
    namespace: &str,
    resolved: &ResolvedType,
    type_system: &TypeSystem,
) -> Result<GeneratedFile> {
    let mut document = CSharpDocument::new();
    document.header(namespace);
    match resolved {
        ResolvedType::ValueObject {
            name,
            underlying,
            conversions,
        } => {
            document.line("[MessagePack.MessagePackObject]");
            render_value_object(&mut document, name, *underlying, *conversions);
        }
        ResolvedType::Custom { name, fields } => {
            render_custom(&mut document, name, fields, type_system)?
        }
        ResolvedType::Enum {
            name,
            underlying,
            members,
        } => {
            render_enum(&mut document, name, *underlying, members, false);
        }
        ResolvedType::Flags {
            name,
            underlying,
            members,
        } => {
            render_enum(&mut document, name, *underlying, members, true);
        }
    }
    Ok(GeneratedFile {
        relative_path: std::path::PathBuf::from(format!("{}.g.cs", resolved.name())),
        contents: document.finish(),
    })
}

fn render_value_object(
    document: &mut CSharpDocument,
    name: &str,
    underlying: PrimitiveType,
    conversions: masterdata_core::ResolvedConversions,
) {
    let csharp_underlying = underlying.csharp_name();
    document.line(format!(
        "public readonly struct {name} : System.IEquatable<{name}>, System.IComparable<{name}>"
    ));
    document.line("{");
    document.line("    [MessagePack.Key(0)]");
    document.line(format!("    public {csharp_underlying} Value {{ get; }}"));
    document.line("");
    document.line(format!("    public {name}({csharp_underlying} value)"));
    document.line("    {");
    if underlying == PrimitiveType::String {
        document.line("        if (value is null)");
        document.line("        {");
        document.line("            throw new System.ArgumentNullException(nameof(value));");
        document.line("        }");
    }
    document.line("        Value = value;");
    document.line("    }");
    document.line("");
    document.line(format!(
        "    public bool Equals({name} other) => {};",
        equality_expression(underlying, "Value", "other.Value")
    ));
    document.line(format!(
        "    public override bool Equals(object? obj) => obj is {name} other && Equals(other);"
    ));
    document.line("    public override int GetHashCode() => Value.GetHashCode();");
    document.line(format!(
        "    public static bool operator ==({name} left, {name} right) => left.Equals(right);"
    ));
    document.line(format!(
        "    public static bool operator !=({name} left, {name} right) => !left.Equals(right);"
    ));
    document.line("");
    // A string Value Object must not inherit culture-sensitive comparison
    // behavior from the runtime; the approved primitive contract is ordinal
    // and culture-independent (TYPE-PRIMITIVE-008, SCHEMA-VO-013).
    let compare = comparison_expression(underlying, "Value", "other.Value");
    document.line(format!(
        "    public int CompareTo({name} other) => {compare};"
    ));
    document.line(format!(
        "    public static bool operator <({name} left, {name} right) => left.CompareTo(right) < 0;"
    ));
    document.line(format!(
        "    public static bool operator <=({name} left, {name} right) => left.CompareTo(right) <= 0;"
    ));
    document.line(format!(
        "    public static bool operator >({name} left, {name} right) => left.CompareTo(right) > 0;"
    ));
    document.line(format!(
        "    public static bool operator >=({name} left, {name} right) => left.CompareTo(right) >= 0;"
    ));
    document.line("");
    let tostring = if underlying == PrimitiveType::String {
        "Value".to_owned()
    } else {
        "Value.ToString(System.Globalization.CultureInfo.InvariantCulture)".to_owned()
    };
    document.line(format!(
        "    public override string ToString() => {tostring};"
    ));
    if conversions.from_underlying_implicit {
        document.line("");
        document.line(format!(
            "    public static implicit operator {name}({csharp_underlying} value) => new {name}(value);"
        ));
    }
    if conversions.to_underlying_implicit {
        document.line("");
        document.line(format!(
            "    public static implicit operator {csharp_underlying}({name} value) => value.Value;"
        ));
    }
    document.line("}");
}

fn render_custom(
    document: &mut CSharpDocument,
    name: &str,
    fields: &[ResolvedField],
    type_system: &TypeSystem,
) -> Result<()> {
    document.line("[MessagePack.MessagePackObject]");
    document.line(format!(
        "public readonly struct {name} : System.IEquatable<{name}>"
    ));
    document.line("{");
    for field in fields {
        // `key` is serialization metadata only and remains independent from
        // declaration order (SCHEMA-KEY-001).
        document.line(format!("    [MessagePack.Key({})]", field.key));
        document.line(format!(
            "    public {} {} {{ get; }}",
            csharp_field_type(type_system, &field.base_type, field.modifier)?,
            csharp_property_name(&field.name)
        ));
    }
    document.line("");
    let parameters = fields
        .iter()
        .map(|field| {
            Ok(format!(
                "{} {}",
                csharp_field_type(type_system, &field.base_type, field.modifier)?,
                field.name
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .join(", ");
    document.line(format!("    public {name}({parameters})"));
    document.line("    {");
    for field in fields {
        let parameter = &field.name;
        match field.modifier {
            FieldModifier::Array => {
                // The constructor checks only the direct Array state. Nested
                // type validity belongs to build validation (SCHEMA-CUSTOM-015).
                document.line(format!("        if ({parameter}.IsDefault)"));
                document.line("        {");
                document.line(format!(
                    "            throw new System.ArgumentException(\"Array value must not be default.\", nameof({parameter}));"
                ));
                document.line("        }");
            }
            FieldModifier::Required
                if matches!(
                    field.base_type,
                    TypeReference::Primitive(PrimitiveType::String)
                ) =>
            {
                document.line(format!("        if ({parameter} is null)"));
                document.line("        {");
                document.line(format!(
                    "            throw new System.ArgumentNullException(nameof({parameter}));"
                ));
                document.line("        }");
            }
            FieldModifier::Required | FieldModifier::Nullable => {}
        }
        document.line(format!(
            "        {} = {parameter};",
            csharp_property_name(&field.name)
        ));
    }
    document.line("    }");
    document.line("");
    document.line(format!("    public bool Equals({name} other)"));
    document.line("    {");
    document.line("        return ");
    for (index, field) in fields.iter().enumerate() {
        // Array equality is intentionally sequence-based rather than based on
        // ImmutableArray storage identity (TYPE-FIELD-009, SCHEMA-CUSTOM-014).
        let expression = equality_expression_for_field(field);
        let conjunction = if index + 1 == fields.len() {
            ";"
        } else {
            " &&"
        };
        document.line(format!("            {expression}{conjunction}"));
    }
    document.line("    }");
    document.line(format!(
        "    public override bool Equals(object? obj) => obj is {name} other && Equals(other);"
    ));
    document.line("    public override int GetHashCode()");
    document.line("    {");
    // Keep the generated hash implementation dependency-light. The approved
    // contract requires structural consistency, not a particular algorithm;
    // avoiding System.HashCode keeps this generated surface independent of a
    // runtime-specific hash helper; the approved contract fixes consistency,
    // not a particular hash algorithm.
    document.line("        var hash = 17;");
    for field in fields {
        let property = csharp_property_name(&field.name);
        if field.modifier == FieldModifier::Array {
            // Hash each element in order so equal Array sequences produce the
            // same structural hash (TYPE-FIELD-009, SCHEMA-CUSTOM-014).
            document.line(format!("        foreach (var item in {property})"));
            document.line("        {");
            let item_hash = if matches!(
                field.base_type,
                TypeReference::Primitive(PrimitiveType::String)
            ) {
                "(item is null ? 0 : item.GetHashCode())"
            } else {
                "item.GetHashCode()"
            };
            document.line(format!(
                "            hash = unchecked(hash * 31 + {item_hash});"
            ));
            document.line("        }");
        } else if field.modifier == FieldModifier::Nullable {
            document.line(format!(
                "        hash = unchecked(hash * 31 + ({property}?.GetHashCode() ?? 0));"
            ));
        } else {
            document.line(format!(
                "        hash = unchecked(hash * 31 + {property}.GetHashCode());"
            ));
        }
    }
    document.line("        return hash;");
    document.line("    }");
    document.line("");
    document.line(format!(
        "    public static bool operator ==({name} left, {name} right) => left.Equals(right);"
    ));
    document.line(format!(
        "    public static bool operator !=({name} left, {name} right) => !left.Equals(right);"
    ));
    document.line("}");
    Ok(())
}

fn render_enum(
    document: &mut CSharpDocument,
    name: &str,
    underlying: PrimitiveType,
    members: &[masterdata_core::ResolvedEnumMember],
    flags: bool,
) {
    if flags {
        document.line("[System.Flags]");
    }
    document.line(format!("public enum {name} : {}", underlying.csharp_name()));
    document.line("{");
    for member in members {
        document.line(format!(
            "    {} = {},",
            member.name,
            csharp_enum_value(member.value.0, underlying)
        ));
    }
    document.line("}");
}

fn csharp_enum_value(value: i128, underlying: PrimitiveType) -> String {
    match (underlying, value) {
        (PrimitiveType::Int, value) if value == i32::MIN as i128 => "int.MinValue".to_owned(),
        (PrimitiveType::Long, value) if value == i64::MIN as i128 => "long.MinValue".to_owned(),
        (PrimitiveType::UInt, value) => format!("{value}u"),
        (PrimitiveType::ULong, value) => format!("{value}UL"),
        (PrimitiveType::Long, value) => format!("{value}L"),
        (_, value) => value.to_string(),
    }
}

fn render_schema(
    namespace: &str,
    table: &ResolvedTable,
    type_system: &TypeSystem,
) -> Result<GeneratedFile> {
    let type_name = &table.csharp_name;
    let mut document = CSharpDocument::new();
    document.header(namespace);
    document.line(format!("// Source table identity: {}", table.identity));
    document.line("");
    document.line(format!(
        "[MasterMemory.MemoryTable(\"{}\"), MessagePack.MessagePackObject]",
        table.identity
    ));
    document.line(format!("public sealed partial class {type_name}"));
    document.line("{");
    let mut property_names = BTreeSet::new();
    for field in &table.fields {
        let property = csharp_property_name(&field.name);
        if !property_names.insert(property.clone()) {
            return Err(MasterdataError::new(
                "E-CODEGEN-PROPERTY-NAME-COLLISION",
                ErrorKind::Validation,
                format!("Table fields generate the same property `{property}`"),
            ));
        }
        validate_generated_member_name(&property, "E-CODEGEN-INVALID-PROPERTY-NAME")?;
        // `key` is serialization metadata only; it is not used to reorder
        // properties or derive logical field identity (SCHEMA-KEY-001).
        document.line(format!("    [MessagePack.Key({})]", field.key));
        if let Some(key_order) = table
            .primary_key
            .fields
            .iter()
            .position(|name| name == &field.name)
        {
            document.line(format!(
                "    [MasterMemory.PrimaryKey(keyOrder: {key_order})]"
            ));
        }
        for secondary in &table.secondary_keys {
            if let Some(key_order) = secondary.fields.iter().position(|name| name == &field.name) {
                if secondary.non_unique {
                    // MasterMemory associates NonUnique with the same
                    // attribute list as its SecondaryKey declaration; keeping
                    // them together is required for the source generator to
                    // lower the query as a RangeView (INDEX-UNIQUE-001).
                    document.line(format!(
                        "    [MasterMemory.SecondaryKey({}, keyOrder: {key_order}), MasterMemory.NonUnique]",
                        secondary.index_no
                    ));
                } else {
                    document.line(format!(
                        "    [MasterMemory.SecondaryKey({}, keyOrder: {key_order})]",
                        secondary.index_no
                    ));
                }
            }
        }
        document.line(format!(
            "    public {} {property} {{ get; init; }}",
            csharp_field_type(type_system, &field.base_type, field.modifier)?
        ));
    }
    document.line("}");
    Ok(GeneratedFile {
        relative_path: std::path::PathBuf::from(format!("{}.g.cs", type_name)),
        contents: document.finish(),
    })
}

fn csharp_field_type(
    _type_system: &TypeSystem,
    reference: &TypeReference,
    modifier: FieldModifier,
) -> Result<String> {
    let base = match reference {
        TypeReference::Primitive(primitive) => primitive.csharp_name().to_owned(),
        TypeReference::Named(name) => name.clone(),
    };
    Ok(match modifier {
        FieldModifier::Required => base,
        FieldModifier::Nullable => format!("{base}?"),
        FieldModifier::Array => format!("System.Collections.Immutable.ImmutableArray<{base}>"),
    })
}

fn equality_expression_for_field(field: &ResolvedField) -> String {
    let property = csharp_property_name(&field.name);
    if field.modifier == FieldModifier::Array {
        format!("System.Linq.Enumerable.SequenceEqual({property}, other.{property})")
    } else {
        equality_expression_for_type(&field.base_type, &property, &format!("other.{property}"))
    }
}

fn equality_expression(primitive: PrimitiveType, left: &str, right: &str) -> String {
    equality_expression_for_type(&TypeReference::Primitive(primitive), left, right)
}

fn equality_expression_for_type(reference: &TypeReference, left: &str, right: &str) -> String {
    match reference {
        TypeReference::Primitive(PrimitiveType::Float | PrimitiveType::Double) => {
            format!("{left} == {right}")
        }
        TypeReference::Primitive(_) | TypeReference::Named(_) => {
            format!("{left} == {right}")
        }
    }
}

fn comparison_expression(primitive: PrimitiveType, left: &str, right: &str) -> String {
    if primitive == PrimitiveType::String {
        format!("string.CompareOrdinal({left}, {right})")
    } else {
        format!("{left}.CompareTo({right})")
    }
}

fn validate_generated_member_name(name: &str, code: &str) -> Result<()> {
    if !is_csharp_identifier(name) || is_csharp_reserved_keyword(name) {
        return Err(MasterdataError::new(
            code,
            ErrorKind::Validation,
            format!("`{name}` is not a valid C# member identifier"),
        ));
    }
    Ok(())
}

fn is_csharp_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if namespace.is_empty()
        || namespace
            .split('.')
            .any(|segment| !is_csharp_identifier(segment) || is_csharp_reserved_keyword(segment))
    {
        return Err(MasterdataError::new(
            "E-CODEGEN-INVALID-NAMESPACE",
            ErrorKind::Validation,
            format!("`{namespace}` is not a valid C# namespace"),
        ));
    }
    Ok(())
}

struct CSharpDocument {
    lines: Vec<String>,
}

impl CSharpDocument {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn header(&mut self, namespace: &str) {
        self.line("// <auto-generated />");
        self.line("#nullable enable");
        self.line("");
        self.line(format!("namespace {namespace};"));
        self.line("");
    }

    fn line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    fn finish(self) -> String {
        let mut output = self.lines.join("\n");
        output.push('\n');
        output
    }
}

#[allow(dead_code)]
fn _format_note(note: &GenerationNote) -> String {
    let mut text = String::new();
    let _ = write!(&mut text, "{}", note.message);
    text
}
