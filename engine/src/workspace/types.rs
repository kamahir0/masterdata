use super::*;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeProjection {
    pub kind: &'static str,
    pub clicked: String,
    pub source: String,
    pub name: String,
    pub identity: String,
    pub declaration: semantic::Type,
    pub protected_members: Vec<String>,
    pub generation: u64,
    pub measurement: Measurement,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum SelectionProjection {
    Table(Projection),
    Type(TypeProjection),
}
impl Workspace {
    pub(super) fn type_projection_current(
        &mut self,
        clicked: &str,
        freshness: instrument::Span,
    ) -> Result<TypeProjection> {
        let document = self.current_doc(clicked)?;
        let (name, _) = semantic::parse_type(&document)?;
        for dependency in self.read.type_dependencies(&name) {
            if dependency != clicked {
                self.refresh_source(&dependency)?;
            }
        }
        let document = self.current_doc(clicked)?;
        let (name, declaration) = semantic::parse_type(&document)?;
        // Invalid/deleted/renamed dependencies remove current editing authority.
        // A read-generation projection never authorizes the later Plan or write.
        crate::migration::field_resolution(
            &self.read,
            &Field {
                key: 0,
                name: "value".into(),
                type_name: name.clone(),
                nullable: false,
                array: false,
            },
        )?;
        drop(freshness);
        let _projection = instrument::span("projection");
        Ok(TypeProjection {
            kind: "type",
            clicked: clicked.into(),
            source: clicked.into(),
            name,
            identity: document.identity.clone(),
            protected_members: if matches!(&declaration, semantic::Type::Enum { flags: true, .. }) {
                vec!["None".into()]
            } else {
                vec![]
            },
            declaration,
            generation: self.generation,
            measurement: instrument::measure(|| ()).1,
        })
    }
    pub fn prepare_type_migration(
        &mut self,
        source: &str,
        identity: &str,
        command: crate::type_migration::Command,
    ) -> Result<MigrationReview> {
        self.prepare_type_migration_input(source, identity, command, None)
    }
    pub fn type_initializer_shape(
        &mut self,
        source: &str,
        identity: &str,
        declaration: &crate::creation::Declaration,
    ) -> Result<Shape> {
        let project = self.type_input_project(source, identity)?;
        let field = Field {
            key: 0,
            name: "value".into(),
            type_name: declaration.type_name.clone(),
            nullable: declaration.nullable,
            array: declaration.array,
        };
        crate::migration::field_resolution(&project, &field)?;
        semantic::shape(&field, &project.types)
    }
    fn type_input_project(&self, source: &str, identity: &str) -> Result<Project> {
        let project = Project::open(&self.read.root)?;
        let target = project
            .sources
            .get(source)
            .and_then(|source| source.document.as_ref())
            .ok_or_else(|| {
                Error::new(
                    "E-TYPE-MIGRATION-TARGET",
                    "current type source is unavailable",
                )
            })?;
        if target.identity != identity || semantic::parse_type(target).is_err() {
            return Err(Error::new(
                "E-MIGRATION-STALE",
                "type source changed; confirm the current declaration before requesting a Plan",
            ));
        }
        Ok(project)
    }
    pub fn prepare_type_migration_input(
        &mut self,
        source: &str,
        identity: &str,
        mut command: crate::type_migration::Command,
        input: Option<crate::initializer::Input>,
    ) -> Result<MigrationReview> {
        self.migration_gate(std::iter::empty())?;
        let project = self.type_input_project(source, identity)?;
        let target = project
            .sources
            .get(source)
            .and_then(|source| source.document.as_ref())
            .ok_or_else(|| {
                Error::new(
                    "E-TYPE-MIGRATION-TARGET",
                    "current type source is unavailable",
                )
            })?;
        if target.identity != identity
            || project
                .type_sources
                .get(command.target())
                .is_none_or(|path| path != source)
        {
            return Err(Error::new(
                "E-MIGRATION-STALE",
                "type source changed; confirm the current declaration before requesting a Plan",
            ));
        }
        if let Some(input) = input {
            let crate::type_migration::Command::AddCustomField {
                declaration,
                initializer,
                ..
            } = &mut command
            else {
                return Err(Error::new(
                    "E-INITIALIZER",
                    "initializer is only available for Add Custom Field",
                ));
            };
            if initializer.is_some() {
                return Err(Error::new(
                    "E-INITIALIZER",
                    "one initializer input is required",
                ));
            }
            let field = crate::migration::field_declaration(declaration, &project.types)?;
            *initializer = Some(crate::initializer::resolve(&field, &input, &project.types)?);
        }
        let plan = crate::type_migration::derive(&project, command)?;
        // The saved-source Plan remains reviewable when an affected data draft
        // exists. The shared Apply gate owns the prohibition on committing it.
        let plan = native::SourceSetPlan::prepare(&project, plan)?;
        Ok(self.register_migration(plan))
    }
}
