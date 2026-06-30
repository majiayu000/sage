//! Extension package lifecycle coordinator.

use super::package_error::{PackageError, PackageResult};
use super::package_registry_bridge::{PackageRegistryBridge, RegisteredPackageAssets};
use super::package_store::{
    DiscoveredExtensionPackage, ExtensionPackageStore, InstalledPackageRecord,
    InstalledPackageState, validate_manifest_assets_under_root,
};
use crate::commands::CommandRegistry;
use crate::hooks::HookRegistry;
use crate::skills::SkillRegistry;
use std::path::{Path, PathBuf};

/// Coordinates package store state and runtime registry mutations.
#[derive(Debug)]
pub struct ExtensionPackageManager {
    store: ExtensionPackageStore,
    bridge: PackageRegistryBridge,
}

impl ExtensionPackageManager {
    /// Create a package manager from a store.
    pub fn new(store: ExtensionPackageStore) -> Self {
        Self {
            store,
            bridge: PackageRegistryBridge::new(),
        }
    }

    /// Create a manager using the default user extension package store.
    pub fn default_user_manager() -> PackageResult<Self> {
        Ok(Self::new(ExtensionPackageStore::default_user_store()?))
    }

    /// Access the package store.
    pub fn store(&self) -> &ExtensionPackageStore {
        &self.store
    }

    /// Access package-sourced runtime declarations.
    pub fn bridge(&self) -> &PackageRegistryBridge {
        &self.bridge
    }

    /// Discover available packages from package roots.
    pub fn discover(&self, roots: &[PathBuf]) -> PackageResult<Vec<DiscoveredExtensionPackage>> {
        self.store.discover(roots)
    }

    /// List installed packages.
    pub fn list(&self) -> PackageResult<Vec<InstalledPackageRecord>> {
        self.store.list()
    }

    /// Read one installed package.
    pub fn read(&self, package_id: &str) -> PackageResult<InstalledPackageRecord> {
        self.store.read(package_id)
    }

    /// Install a package into the managed store without enabling it.
    pub fn install(&self, source_root: &Path) -> PackageResult<InstalledPackageRecord> {
        self.store.install(source_root)
    }

    /// Enable an installed package and register its assets.
    pub fn enable(
        &mut self,
        package_id: &str,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<InstalledPackageRecord> {
        let mut record = self.store.read(package_id)?;

        self.validate_dependencies(&record)?;
        validate_manifest_assets_under_root(&record.manifest, &record.install_root)?;
        if record.enabled() {
            self.bridge
                .disable_package(package_id, skills, commands, hooks)?;
            self.bridge
                .enable_package(&record, skills, commands, hooks)?;
            return Ok(record);
        }

        self.bridge
            .enable_package(&record, skills, commands, hooks)?;
        record.state = InstalledPackageState::Enabled;

        if let Err(err) = self.store.write_record(&record) {
            if let Err(rollback_err) = self
                .bridge
                .disable_package(package_id, skills, commands, hooks)
            {
                return Err(partial_lifecycle_error(
                    package_id,
                    "enable",
                    InstalledPackageState::Disabled,
                    vec![err, rollback_err],
                ));
            }
            return Err(err);
        }
        Ok(record)
    }

    /// Disable an enabled package and unregister its assets.
    pub fn disable(
        &mut self,
        package_id: &str,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<InstalledPackageRecord> {
        let mut record = self.store.read(package_id)?;
        if !record.enabled() {
            return Ok(record);
        }

        let original = record.clone();
        self.bridge
            .disable_package(package_id, skills, commands, hooks)?;
        record.state = InstalledPackageState::Disabled;
        if let Err(err) = self.store.write_record(&record) {
            if let Err(restore_err) = self
                .bridge
                .enable_package(&original, skills, commands, hooks)
            {
                return Err(partial_lifecycle_error(
                    package_id,
                    "disable",
                    original.state,
                    vec![err, restore_err],
                ));
            }
            return Err(err);
        }
        Ok(record)
    }

    /// Uninstall a package after disabling it if necessary.
    pub fn uninstall(
        &mut self,
        package_id: &str,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<RegisteredPackageAssets> {
        let record = self.store.read(package_id)?;
        let removed = if record.enabled() {
            self.bridge
                .disable_package(package_id, skills, commands, hooks)?
        } else {
            RegisteredPackageAssets::default()
        };
        if let Err(err) = self.store.uninstall(package_id) {
            if record.enabled() {
                if let Err(restore_err) =
                    self.bridge.enable_package(&record, skills, commands, hooks)
                {
                    return Err(partial_lifecycle_error(
                        package_id,
                        "uninstall",
                        record.state,
                        vec![err, restore_err],
                    ));
                }
            }
            return Err(err);
        }
        Ok(removed)
    }

    fn validate_dependencies(&self, record: &InstalledPackageRecord) -> PackageResult<()> {
        for dependency in record
            .manifest
            .dependencies
            .iter()
            .filter(|dep| !dep.optional)
        {
            if !self.store.is_installed(&dependency.id)? {
                return Err(PackageError::MissingDependency {
                    package_id: record.package_id.clone(),
                    dependency_id: dependency.id.clone(),
                });
            }
        }
        Ok(())
    }
}

fn partial_lifecycle_error(
    package_id: &str,
    action: &'static str,
    stored_state: InstalledPackageState,
    failures: Vec<PackageError>,
) -> PackageError {
    PackageError::PartialLifecycle {
        package_id: package_id.to_string(),
        action,
        stored_state: format!("{stored_state:?}"),
        failures: failures
            .into_iter()
            .map(|failure| failure.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandRegistry;
    use crate::hooks::HookRegistry;
    use crate::skills::SkillRegistry;
    use std::fs;
    use tempfile::TempDir;

    fn write_package(root: &Path, dependency: bool) {
        fs::create_dir_all(root.join("skills/reviewer")).unwrap();
        fs::create_dir_all(root.join("commands")).unwrap();
        fs::create_dir_all(root.join("hooks")).unwrap();
        fs::write(root.join("skills/reviewer/SKILL.md"), "review prompt").unwrap();
        fs::write(root.join("commands/review.md"), "review command").unwrap();
        fs::write(
            root.join("hooks/preflight.toml"),
            r#"
pattern = "*"
name = "placeholder"
hook_type = "pre_tool_execution"
type = "command"
command = "echo ok"
"#,
        )
        .unwrap();

        let dependency_block = if dependency {
            r#"
[[dependencies]]
id = "acme.base"
version = "1.0.0"
"#
        } else {
            ""
        };

        fs::write(
            root.join("sage-extension.toml"),
            format!(
                r#"
schema_version = 0
id = "acme.review"
name = "Acme Review"
version = "1.0.0"
permissions = ["skills:read", "commands:run", "hooks:run", "network:mcp"]
{dependency_block}
[[assets.skills]]
id = "reviewer"
path = "skills/reviewer/SKILL.md"
required_permissions = ["skills:read"]

[[assets.commands]]
id = "review"
path = "commands/review.md"
required_permissions = ["commands:run"]

[[assets.hooks]]
id = "preflight"
path = "hooks/preflight.toml"
event = "pre_tool_use"
required_permissions = ["hooks:run"]

[[assets.mcp_servers]]
id = "docs"
transport = "stdio"
command = "node"
required_permissions = ["network:mcp"]
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn lifecycle_install_enable_disable_uninstall() {
        let source = TempDir::new().unwrap();
        let store_root = TempDir::new().unwrap();
        write_package(source.path(), false);

        let mut manager =
            ExtensionPackageManager::new(ExtensionPackageStore::new(store_root.path()));
        let mut skills = SkillRegistry::new(source.path());
        let mut commands = CommandRegistry::new(source.path());
        let hooks = HookRegistry::new();

        manager.install(source.path()).unwrap();
        assert!(!skills.contains("reviewer"));
        assert!(!commands.contains("review"));
        assert!(!hooks.contains_hook_name("preflight"));

        let enabled = manager
            .enable("acme.review", &mut skills, &mut commands, &hooks)
            .unwrap();
        assert!(enabled.enabled());
        assert!(skills.contains("reviewer"));
        assert!(commands.contains("review"));
        assert!(manager.bridge().mcp_server("docs").is_some());

        manager
            .enable("acme.review", &mut skills, &mut commands, &hooks)
            .unwrap();
        assert!(skills.contains("reviewer"));
        assert!(commands.contains("review"));
        assert!(manager.bridge().mcp_server("docs").is_some());

        let mut restarted_manager =
            ExtensionPackageManager::new(ExtensionPackageStore::new(store_root.path()));
        let mut restarted_skills = SkillRegistry::new(source.path());
        let mut restarted_commands = CommandRegistry::new(source.path());
        let restarted_hooks = HookRegistry::new();
        restarted_manager
            .enable(
                "acme.review",
                &mut restarted_skills,
                &mut restarted_commands,
                &restarted_hooks,
            )
            .unwrap();
        assert!(restarted_skills.contains("reviewer"));
        assert!(restarted_commands.contains("review"));
        assert!(restarted_manager.bridge().mcp_server("docs").is_some());

        let disabled = manager
            .disable("acme.review", &mut skills, &mut commands, &hooks)
            .unwrap();
        assert!(!disabled.enabled());
        assert!(!skills.contains("reviewer"));
        assert!(!commands.contains("review"));
        assert!(manager.bridge().mcp_server("docs").is_none());

        manager
            .uninstall("acme.review", &mut skills, &mut commands, &hooks)
            .unwrap();
        assert!(matches!(
            manager.read("acme.review"),
            Err(PackageError::NotFound { .. })
        ));
    }

    #[test]
    fn missing_dependency_fails_before_registry_mutation() {
        let source = TempDir::new().unwrap();
        let store_root = TempDir::new().unwrap();
        write_package(source.path(), true);

        let mut manager =
            ExtensionPackageManager::new(ExtensionPackageStore::new(store_root.path()));
        let mut skills = SkillRegistry::new(source.path());
        let mut commands = CommandRegistry::new(source.path());
        let hooks = HookRegistry::new();
        manager.install(source.path()).unwrap();

        let err = manager
            .enable("acme.review", &mut skills, &mut commands, &hooks)
            .unwrap_err();

        assert!(matches!(err, PackageError::MissingDependency { .. }));
        assert!(!skills.contains("reviewer"));
        assert!(!commands.contains("review"));
        assert!(manager.bridge().mcp_servers().is_empty());
    }
}
