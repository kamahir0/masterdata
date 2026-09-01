use std::collections::BTreeMap;
use std::path::PathBuf;

use masterdata_codegen_csharp::CSharpGenerationPlan;
use masterdata_core::{
    BuildPlan, ErrorKind, MasterdataError, NormalizedValue, Result, csharp_property_name,
};
use serde::{Deserialize, Serialize};

/// Internal handshake version for the repository-owned Rust/.NET builder.
/// This is intentionally not a released compatibility protocol.
pub const BUILD_PROTOCOL_VERSION: u32 = 1;
pub const MASTERMEMORY_VERSION: &str = "3.0.4";
pub const MESSAGEPACK_VERSION: &str = "3.1.3";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MasterMemoryBuildRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    pub namespace: String,
    #[serde(rename = "outputPath")]
    pub output_path: PathBuf,
    pub tables: Vec<NormalizedTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedTable {
    pub identity: String,
    #[serde(rename = "typeName")]
    pub type_name: String,
    pub fields: Vec<NormalizedField>,
    pub records: Vec<NormalizedRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedField {
    pub key: u32,
    pub name: String,
    #[serde(rename = "propertyName")]
    pub property_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedRecord {
    pub fields: BTreeMap<String, NormalizedValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MasterMemoryTableReport {
    pub identity: String,
    #[serde(rename = "recordCount")]
    pub record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MasterMemoryBuildReport {
    pub status: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    #[serde(rename = "masterMemoryVersion")]
    pub master_memory_version: String,
    #[serde(rename = "messagePackVersion")]
    pub message_pack_version: String,
    #[serde(rename = "binaryPath")]
    pub binary_path: PathBuf,
    #[serde(rename = "binarySize")]
    pub binary_size: u64,
    #[serde(rename = "tableCount")]
    pub table_count: usize,
    #[serde(rename = "recordCount")]
    pub record_count: usize,
    pub tables: Vec<MasterMemoryTableReport>,
}

impl MasterMemoryBuildRequest {
    /// Build the internal request from the validated/canonical model. The
    /// Rust Type System performs all semantic normalization; this layer only
    /// packages that result for the mechanical .NET adapter.
    pub fn from_plan(
        plan: &BuildPlan,
        generation: &CSharpGenerationPlan,
        output_path: PathBuf,
    ) -> Result<Self> {
        if !output_path.is_absolute() {
            return Err(MasterdataError::new(
                "E-DOTNET-BUILDER-PREPARE",
                ErrorKind::Config,
                "staged MasterMemory output path must be absolute",
            )
            .with_source(output_path));
        }

        let mut tables = Vec::with_capacity(plan.tables.len());
        for table in &plan.tables {
            let fields = table
                .fields
                .iter()
                .map(|field| NormalizedField {
                    key: field.key,
                    name: field.name.clone(),
                    property_name: csharp_property_name(&field.name),
                })
                .collect::<Vec<_>>();
            let mut records = Vec::with_capacity(table.records.len());
            for record in &table.records {
                let mut normalized_fields = BTreeMap::new();
                for field in &table.fields {
                    let value = record.fields.get(&field.name).ok_or_else(|| {
                        MasterdataError::new(
                            "E-DOTNET-BUILDER-NORMALIZE",
                            ErrorKind::Validation,
                            format!(
                                "resolved record for table `{}` is missing field `{}`",
                                table.identity, field.name
                            ),
                        )
                        .with_related_requirement("SCHEMA-TABLE-006")
                    })?;
                    let normalized = plan
                        .type_system
                        .normalize_field_value(field, value)
                        .map_err(|error| {
                            MasterdataError::new(
                                "E-DOTNET-BUILDER-NORMALIZE",
                                ErrorKind::Validation,
                                format!(
                                    "could not normalize table `{}` field `{}`: {}",
                                    table.identity,
                                    field.name,
                                    error.diagnostic().message
                                ),
                            )
                            .with_related_requirement("SCHEMA-TABLE-006")
                        })?;
                    normalized_fields.insert(field.name.clone(), normalized);
                }
                records.push(NormalizedRecord {
                    fields: normalized_fields,
                });
            }
            tables.push(NormalizedTable {
                identity: table.identity.clone(),
                type_name: table.csharp_name.clone(),
                fields,
                records,
            });
        }

        Ok(Self {
            protocol_version: BUILD_PROTOCOL_VERSION,
            namespace: generation.namespace.clone(),
            output_path,
            tables,
        })
    }

    pub fn record_count(&self) -> usize {
        self.tables.iter().map(|table| table.records.len()).sum()
    }
}
