use super::*;
use std::path::PathBuf;

impl Workspace {
    pub fn invalidate_environment(&mut self, error: Error) {
        if self
            .environment_error
            .as_ref()
            .is_some_and(|old| old.code == error.code && old.message == error.message)
        {
            return;
        }
        self.environment_error = Some(error);
        self.generation += 1;
        self.external_version += 1;
        self.diagnostics_pending = true;
    }

    /// Filesystem events are hints. Actual source identity is always read again;
    /// neither an event nor this derivative inventory authorizes a write.
    pub fn refresh_paths(&mut self, paths: &[PathBuf]) -> Vec<String> {
        if let Err(error) = self.check_config() {
            self.invalidate_environment(error);
            return vec![];
        }
        let before = self.generation;
        let observed_version = self.external_version;
        let mut targets = BTreeSet::new();
        let mut folders = BTreeSet::new();
        let mut remove_folders = BTreeSet::new();
        for path in paths {
            let Ok(relative) = path.strip_prefix(&self.read.root) else {
                continue;
            };
            let logical = relative.to_string_lossy().replace('\\', "/");
            if !self.read.roots.iter().any(|root| path.starts_with(root)) {
                continue;
            }
            // A removed/renamed directory can invalidate known children. Its
            // cached inventory is only a bounded target list, never file identity.
            for known in self.read.sources.keys() {
                if self.read.root.join(known).starts_with(path) {
                    targets.insert(known.clone());
                }
            }
            if path.is_dir() {
                let mut entries = BTreeSet::new();
                let mut children = BTreeSet::new();
                if native::checked_path(&self.read.root, &self.read.roots, &logical).is_ok()
                    && project::enumerate(path, &mut entries, &mut children).is_ok()
                {
                    targets.extend(entries.into_iter().filter_map(|p| {
                        p.strip_prefix(&self.read.root)
                            .ok()
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                    }));
                    folders.extend(children.into_iter().filter_map(|p| {
                        p.strip_prefix(&self.read.root)
                            .ok()
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                    }));
                }
            } else if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| matches!(s, "yaml" | "yml"))
            {
                targets.insert(logical.clone());
            }
            if !path.is_dir() {
                remove_folders.insert(logical);
            }
        }
        let old_folders = self.read.folders.clone();
        let p = Arc::make_mut(&mut self.read);
        p.folders.retain(|folder| {
            !remove_folders
                .iter()
                .any(|removed| folder == removed || folder.starts_with(&format!("{removed}/")))
        });
        p.folders.extend(folders);
        let folder_changed = old_folders != p.folders;
        let mut changed = vec![];
        for path in targets {
            let generation = self.generation;
            if self.read.sources.contains_key(&path) {
                let _ = self.refresh_source(&path);
            } else if let Ok(snapshot) = native::capture(&self.read.root, &self.read.roots, &path) {
                match Document::parse(snapshot.bytes.clone()) {
                    Ok(doc) => self.replace_read(&path, &snapshot, Some(Arc::new(doc)), None),
                    Err(error) => {
                        self.replace_read(&path, &snapshot, None, Some(error.to_string()))
                    }
                }
                self.snapshots.insert(path.clone(), snapshot);
            }
            if generation != self.generation {
                changed.push(path);
            }
        }
        if (self.generation != before && self.external_version == observed_version)
            || folder_changed
        {
            self.external_version += 1;
        }
        changed
    }
}
