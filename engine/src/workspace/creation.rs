use super::*;
use crate::creation::{self, Artifact, Request};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(super) struct PendingCreation {
    request: Request,
    bytes: Option<Arc<str>>,
}

impl Workspace {
    fn creation_root(&self, request: &Request) -> Result<PathBuf> {
        self.check_config()?;
        if self.recovery_required {
            return Err(Error::new(
                "E-RECOVERY-REQUIRED",
                "source creation is gated",
            ));
        }
        let index = self
            .read
            .config
            .sources
            .roots
            .iter()
            .position(|r| r == &request.root)
            .ok_or_else(|| {
                Error::new(
                    "E-PATH-SCOPE",
                    "exactly one configured source root required",
                )
            })?;
        let root = self.read.roots[index].clone();
        let path = native::creation_path(&self.read.root, &root, &request.path)?;
        if !matches!(request.artifact, Artifact::Folder)
            && !path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| matches!(e, "yaml" | "yml"))
        {
            return Err(Error::new(
                "E-CREATE-EXTENSION",
                "source file extension must be .yaml or .yml",
            ));
        }
        Ok(root)
    }
    pub fn creation_choices(&self) -> serde_json::Value {
        let mut value = creation::choices(&self.read);
        value["roots"] = serde_json::json!(self.read.config.sources.roots.iter().zip(&self.read.roots).map(|(name,path)|serde_json::json!({"root":name,"path":path.strip_prefix(&self.read.root).unwrap().to_string_lossy().replace('\\',"/")})).collect::<Vec<_>>());
        value
    }
    pub fn creation_defaults(
        &self,
        category: &str,
        path: &str,
        table: Option<&str>,
    ) -> Result<Artifact> {
        creation::defaults(&self.read, category, path, table)
    }
    pub fn creation_preview(&self, request: &Request) -> serde_json::Value {
        let checked = self
            .creation_root(request)
            .and_then(|_| creation::candidate(&self.validation_snapshot(), request));
        serde_json::json!({"valid":checked.is_ok(),"identity":request.artifact.identity(),"category":request.artifact.category(),"diagnostics":checked.err().map(|e|serde_json::json!({"code":e.code,"message":e.message})).into_iter().collect::<Vec<_>>()})
    }
    fn fresh_creation_project(&self) -> Result<Project> {
        self.check_config()?;
        let mut project = Project::open(&self.read.root)?;
        // Unsaved declarations participate only while their actual physical base
        // is unchanged. An external replacement must not be masked by an overlay.
        for (path, draft) in &self.drafts {
            if native::capture(&project.root, &project.roots, path)
                .is_ok_and(|actual| actual.matches(&draft.base))
                && let Some(source) = project.sources.get(path)
            {
                let mut source = (**source).clone();
                source.bytes = draft.document.bytes.clone();
                source.document = Some(draft.document.clone());
                source.identity = draft.document.identity.clone();
                project.sources.insert(path.clone(), Arc::new(source));
            }
        }
        project.rebuild_declarations();
        Ok(project)
    }
    pub fn create_source(&mut self, request: &Request) -> Result<WriteResult> {
        self.create_source_with_fault(request, Fault::None)
    }
    pub fn create_source_with_fault(
        &mut self,
        request: &Request,
        fault: Fault,
    ) -> Result<WriteResult> {
        if self.creations.contains_key(&request.path) {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "Recheck destination before another creation attempt",
            ));
        }
        let root = self.creation_root(request)?;
        let target = native::creation_path(&self.read.root, &root, &request.path)?;
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                return Ok(WriteResult::new(
                    &request.path,
                    Outcome::Conflict,
                    "E-CREATE-CONFLICT: destination already exists",
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(project::io_error(e)),
        }
        let candidate = creation::candidate(&self.fresh_creation_project()?, request)?;
        let bytes = candidate.as_ref().map(|d| d.bytes.clone());
        let mut result = native::create_exclusive(
            &self.read.root,
            &root,
            &request.path,
            bytes.as_deref(),
            fault,
            || {
                self.creation_root(request)?;
                creation::candidate(&self.fresh_creation_project()?, request)?;
                Ok(())
            },
        );
        if result.outcome == Outcome::Success
            && let Err(error) = self.accept_creation(request)
        {
            result = WriteResult::new(&request.path, Outcome::OutcomeUnknown, error.to_string());
        }
        if result.outcome == Outcome::OutcomeUnknown {
            self.creations.insert(
                request.path.clone(),
                PendingCreation {
                    request: request.clone(),
                    bytes,
                },
            );
        }
        Ok(result)
    }
    fn accept_creation(&mut self, request: &Request) -> Result<()> {
        if matches!(request.artifact, Artifact::Folder) {
            Arc::make_mut(&mut self.read)
                .folders
                .insert(request.path.clone());
            self.external_version += 1;
        } else {
            let snapshot = native::capture(&self.read.root, &self.read.roots, &request.path)?;
            let document = Document::parse(snapshot.bytes.clone())?;
            self.replace_read(&request.path, &snapshot, Some(Arc::new(document)), None);
            self.snapshots.insert(request.path.clone(), snapshot);
            self.external_version += 1;
        }
        Ok(())
    }
    pub fn recheck_creation(&mut self, path: &str) -> Result<WriteResult> {
        let pending = self.creations.get(path).cloned().ok_or_else(|| {
            Error::new(
                "E-CREATE-NOT-PENDING",
                "no uncertain creation at this destination",
            )
        })?;
        let root = self.creation_root(&pending.request)?;
        let target = native::creation_path(&self.read.root, &root, path)?;
        let result = match fs::symlink_metadata(&target) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => WriteResult::new(
                path,
                Outcome::Failure,
                "Recheck confirmed no destination; explicit retry is available",
            ),
            Err(e) => return Err(project::io_error(e)),
            Ok(metadata) => {
                let complete = if let Some(bytes) = &pending.bytes {
                    native::capture(&self.read.root, &[root], path).is_ok_and(|s| s.bytes == *bytes)
                } else {
                    metadata.is_dir()
                        && native::checked_path(&self.read.root, &[root], path)
                            .is_ok_and(|p| p.is_dir())
                };
                if complete {
                    self.accept_creation(&pending.request)?;
                    WriteResult::new(
                        path,
                        Outcome::Success,
                        "Recheck confirmed complete destination",
                    )
                } else {
                    WriteResult::new(
                        path,
                        Outcome::Conflict,
                        "Recheck found a different destination; existing entry preserved",
                    )
                }
            }
        };
        self.creations.remove(path);
        Ok(result)
    }
}
