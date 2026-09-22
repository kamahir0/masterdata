//! GUI transport values remain exact text until parsed by the shared service.
use crate::{
    TableAuthoringSession, TableFileDiff,
    authoring::project_relative_string,
    table_authoring::{MigrationCompatibilityView, Prepared, migration_compatibility},
};
use masterdata_core::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeOperationInput {
    Conversions {
        target: String,
        #[serde(rename = "fromUnderlyingImplicit")]
        from: bool,
        #[serde(rename = "toUnderlyingImplicit")]
        to: bool,
    },
    AddEnum {
        target: String,
        name: String,
        value: String,
    },
    RenameEnum {
        target: String,
        member: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    DropEnum {
        target: String,
        member: String,
    },
    AddCustom {
        target: String,
        field: TypeFieldDefinition,
        initializer: Option<String>,
    },
    RenameCustom {
        target: String,
        field: String,
        #[serde(rename = "newName")]
        new_name: String,
    },
    DropCustom {
        target: String,
        field: String,
    },
}
impl TypeOperationInput {
    fn command(self) -> Result<TypeMigrationCommand> {
        use TypeMigrationOperation as Op;
        let (target, operation) = match self {
            Self::Conversions { target, from, to } => (
                target,
                Op::SetValueObjectConversions(ConversionDefinition {
                    from_underlying_implicit: from,
                    to_underlying_implicit: to,
                }),
            ),
            Self::AddEnum {
                target,
                name,
                value,
            } => {
                // Parsing integer text avoids IEEE 754 coercion, including ulong::MAX.
                let value: IntegerLiteral = serde_yaml::from_value(parse_constant(&value)?)
                    .map_err(|e| error(format!("explicit integer value required: {e}")))?;
                (target, Op::AddEnumMember(EnumMember { name, value }))
            }
            Self::RenameEnum {
                target,
                member,
                new_name,
            } => (target, Op::RenameEnumMember { member, new_name }),
            Self::DropEnum { target, member } => (target, Op::DropEnumMember { member }),
            Self::AddCustom {
                target,
                field,
                initializer,
            } => {
                let initializer = initializer.map(|text| parse_constant(&text)).transpose()?;
                (target, Op::AddCustomField { field, initializer })
            }
            Self::RenameCustom {
                target,
                field,
                new_name,
            } => (target, Op::RenameCustomField { field, new_name }),
            Self::DropCustom { target, field } => (target, Op::DropCustomField { field }),
        };
        Ok(TypeMigrationCommand { target, operation })
    }
}
struct UniqueObject(std::collections::BTreeMap<String, Box<serde_json::value::RawValue>>);
impl<'de> Deserialize<'de> for UniqueObject {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueObject;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an object with unique keys")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut values = std::collections::BTreeMap::new();
                while let Some((key, value)) =
                    map.next_entry::<String, Box<serde_json::value::RawValue>>()?
                {
                    if values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate constant mapping key"));
                    }
                }
                Ok(UniqueObject(values))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

// WHY: serde_json::Value may classify an overflowing integer as f64 and round
// it before the Type System sees it. RawValue retains scalar category and exact
// integer digits, including -0 and integers nested inside objects/arrays.
// EVIDENCE: GUI-TYPE-INT-009; TYPE-PRIMITIVE-003.
pub(crate) fn parse_constant(text: &str) -> Result<serde_yaml::Value> {
    use serde_yaml::Value;
    let raw: Box<serde_json::value::RawValue> =
        serde_json::from_str(text).map_err(|e| error(format!("constant must be JSON: {e}")))?;
    let text = raw.get();
    match text.as_bytes()[0] {
        b'{' => {
            let values: UniqueObject =
                serde_json::from_str(text).map_err(|e| error(e.to_string()))?;
            values
                .0
                .into_iter()
                .map(|(key, value)| Ok((Value::String(key), parse_constant(value.get())?)))
                .collect::<Result<serde_yaml::Mapping>>()
                .map(Value::Mapping)
        }
        b'[' => {
            let values: Vec<Box<serde_json::value::RawValue>> =
                serde_json::from_str(text).map_err(|e| error(e.to_string()))?;
            values
                .into_iter()
                .map(|value| parse_constant(value.get()))
                .collect::<Result<Vec<_>>>()
                .map(Value::Sequence)
        }
        b'-' | b'0'..=b'9' if !text.contains(['.', 'e', 'E']) => {
            if let Ok(value) = text.parse::<i64>() {
                Ok(Value::from(value))
            } else if let Ok(value) = text.parse::<u64>() {
                Ok(Value::from(value))
            } else {
                Err(error(
                    "integer constant is outside the supported exact integer range",
                ))
            }
        }
        _ => {
            let value: serde_json::Value =
                serde_json::from_str(text).map_err(|e| error(e.to_string()))?;
            serde_yaml::to_value(value).map_err(|e| error(e.to_string()))
        }
    }
}
fn error(message: impl Into<String>) -> MasterdataError {
    MasterdataError::new("E-TYPE-EDITOR-INPUT", ErrorKind::Validation, message)
}
pub use masterdata_core::{TypeMemberView, TypeSnapshot};
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypePlanView {
    pub token: String,
    pub target: String,
    pub operation: String,
    pub selector: String,
    pub destructive: bool,
    pub affected_occurrence_count: usize,
    pub files: Vec<TableFileDiff>,
    pub diagnostics: Vec<Diagnostic>,
    pub compatibility: MigrationCompatibilityView,
}
impl TableAuthoringSession {
    pub fn open_type(&self, root: &Path, path: &str) -> Result<TypeSnapshot> {
        let project = Project::discover(Some(root), root)?;
        let docs = project.load_documents()?;
        type_snapshot(&docs, &project.root().join(path), path)
    }
    pub fn plan_type(&mut self, root: &Path, input: TypeOperationInput) -> Result<TypePlanView> {
        self.ensure_mutation_allowed(root)?;
        let project = Project::discover(Some(root), root)?;
        let before = project.load_documents()?;
        let command = input.command()?;
        let dry_run = dry_run_type_migration(&before, &command)?;
        self.sequence += 1;
        let token = self.sequence.to_string();
        let files = dry_run
            .candidate
            .affected_files
            .iter()
            .map(|file| TableFileDiff {
                path: project_relative_string(project.root(), &file.path),
                before: before
                    .files
                    .iter()
                    .find(|f| f.path == file.path)
                    .expect("base")
                    .source
                    .clone(),
                after: dry_run
                    .candidate
                    .transformed_documents
                    .files
                    .iter()
                    .find(|f| f.path == file.path)
                    .expect("candidate")
                    .source
                    .clone(),
            })
            .collect();
        let compatibility =
            migration_compatibility(&project, &before, &dry_run.candidate.transformed_documents);
        let view = TypePlanView {
            token: token.clone(),
            target: command.target,
            operation: command.operation.name().into(),
            selector: command.operation.selector().into(),
            destructive: command.operation.destructive(),
            affected_occurrence_count: dry_run.affected_occurrence_count,
            files,
            diagnostics: vec![],
            compatibility,
        };
        self.plans
            .retain(|_, plan| plan.project.root() != project.root());
        self.plans.insert(
            token,
            Prepared {
                project,
                before,
                dry_run: dry_run.candidate,
            },
        );
        Ok(view)
    }
}
