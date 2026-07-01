//! Extension package registry bridge.

use super::package_error::{PackageError, PackageResult};
use super::package_manifest::{ExtensionPackageManifest, PackageAssetKind};
use super::package_store::InstalledPackageRecord;
use crate::commands::{CommandRegistry, CommandSource, SlashCommand};
use crate::config::McpServerConfig;
use crate::hooks::{HookEvent, HookMatcher, HookRegistry, HookSource};
use crate::skills::{Skill, SkillRegistry};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[path = "package_registry_bridge_load.rs"]
mod bridge_load;
use bridge_load::{load_command_asset, load_hook_asset, load_mcp_asset, load_skill_asset};

/// MCP declaration registered from an extension package.
#[derive(Debug, Clone)]
pub struct PackageMcpServerRegistration {
    /// Package id.
    pub package_id: String,
    /// Manifest asset id.
    pub asset_id: String,
    /// Installed package root.
    pub package_root: PathBuf,
    /// MCP server config. GH-86 records this but does not connect it.
    pub config: McpServerConfig,
}

/// Summary of package assets registered or unregistered by the bridge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegisteredPackageAssets {
    /// Skill ids.
    pub skills: Vec<String>,
    /// Command ids.
    pub commands: Vec<String>,
    /// Hook ids.
    pub hooks: Vec<String>,
    /// MCP server ids.
    pub mcp_servers: Vec<String>,
}

/// Bridge between package manifests and runtime registries.
#[derive(Debug, Default)]
pub struct PackageRegistryBridge {
    mcp_servers: BTreeMap<String, PackageMcpServerRegistration>,
}

impl PackageRegistryBridge {
    /// Create an empty bridge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a package-sourced MCP declaration.
    pub fn mcp_server(&self, server_id: &str) -> Option<&PackageMcpServerRegistration> {
        self.mcp_servers.get(server_id)
    }

    /// List package-sourced MCP declarations.
    pub fn mcp_servers(&self) -> Vec<&PackageMcpServerRegistration> {
        self.mcp_servers.values().collect()
    }

    /// Load MCP server declarations from an installed package record.
    pub fn load_mcp_servers(
        record: &InstalledPackageRecord,
    ) -> PackageResult<Vec<PackageMcpServerRegistration>> {
        record
            .manifest
            .assets
            .mcp_servers
            .iter()
            .map(|asset| load_mcp_asset(record, asset))
            .collect()
    }

    /// Enable a package by registering every declared asset.
    pub fn enable_package(
        &mut self,
        record: &InstalledPackageRecord,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<RegisteredPackageAssets> {
        let prepared = self.prepare_assets(record, skills, commands, hooks)?;
        self.register_prepared(record, prepared, skills, commands, hooks)
    }

    /// Disable a package by removing package-owned registry entries.
    pub fn disable_package(
        &mut self,
        package_id: &str,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<RegisteredPackageAssets> {
        let removed_skills = skills
            .remove_package_skills(package_id)
            .into_iter()
            .map(|skill| skill.name().to_string())
            .collect();
        let removed_commands = commands
            .remove_package_commands(package_id)
            .into_iter()
            .map(|command| command.name)
            .collect();

        let mut failures = Vec::new();
        let removed_hooks = hooks
            .remove_package_hooks(package_id)
            .map_err(|err| failures.push(err.to_string()))
            .ok()
            .unwrap_or_default();

        let removed_mcp_servers: Vec<String> = self
            .mcp_servers
            .iter()
            .filter(|(_, registration)| registration.package_id == package_id)
            .map(|(server_id, _)| server_id.clone())
            .collect();
        for server_id in &removed_mcp_servers {
            self.mcp_servers.remove(server_id);
        }

        if !failures.is_empty() {
            return Err(PackageError::PartialUnregister {
                package_id: package_id.to_string(),
                failures,
            });
        }

        Ok(RegisteredPackageAssets {
            skills: removed_skills,
            commands: removed_commands,
            hooks: removed_hooks,
            mcp_servers: removed_mcp_servers,
        })
    }

    fn prepare_assets(
        &self,
        record: &InstalledPackageRecord,
        skills: &SkillRegistry,
        commands: &CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<PreparedPackageAssets> {
        let manifest = &record.manifest;
        self.preflight_conflicts(manifest, skills, commands, hooks)?;

        let mut prepared = PreparedPackageAssets::default();
        for asset in &manifest.assets.skills {
            prepared.skills.push(load_skill_asset(record, asset)?);
        }
        for asset in &manifest.assets.commands {
            prepared
                .commands
                .push(load_command_asset(record, commands, asset)?);
        }
        for asset in &manifest.assets.hooks {
            prepared.hooks.push(load_hook_asset(record, asset)?);
        }
        prepared.mcp_servers = Self::load_mcp_servers(record)?;
        Ok(prepared)
    }

    fn preflight_conflicts(
        &self,
        manifest: &ExtensionPackageManifest,
        skills: &SkillRegistry,
        commands: &CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<()> {
        for asset in &manifest.assets.skills {
            if skills.contains(&asset.id) {
                return Err(conflict(
                    manifest,
                    PackageAssetKind::Skill,
                    &asset.id,
                    "skill registry",
                ));
            }
        }
        for asset in &manifest.assets.commands {
            if commands.contains(&asset.id) {
                return Err(conflict(
                    manifest,
                    PackageAssetKind::Command,
                    &asset.id,
                    "command registry",
                ));
            }
        }
        for asset in &manifest.assets.hooks {
            if hooks.contains_hook_name(&asset.id) {
                return Err(conflict(
                    manifest,
                    PackageAssetKind::Hook,
                    &asset.id,
                    "hook registry",
                ));
            }
        }
        for asset in &manifest.assets.mcp_servers {
            if self.mcp_servers.contains_key(&asset.id) {
                return Err(conflict(
                    manifest,
                    PackageAssetKind::McpServer,
                    &asset.id,
                    "mcp package declarations",
                ));
            }
        }
        Ok(())
    }

    fn register_prepared(
        &mut self,
        record: &InstalledPackageRecord,
        prepared: PreparedPackageAssets,
        skills: &mut SkillRegistry,
        commands: &mut CommandRegistry,
        hooks: &HookRegistry,
    ) -> PackageResult<RegisteredPackageAssets> {
        let package_id = &record.package_id;
        let mut registered = RegisteredPackageAssets::default();

        for skill in prepared.skills {
            let name = skill.name().to_string();
            if let Err(err) = skills.try_register(skill) {
                rollback(package_id, skills, commands, hooks, self);
                return Err(PackageError::Registry {
                    package_id: package_id.clone(),
                    message: err.to_string(),
                });
            }
            registered.skills.push(name);
        }

        for command in prepared.commands {
            let name = command.name.clone();
            let source = CommandSource::Package {
                package_id: package_id.clone(),
                asset_id: name.clone(),
                package_root: record.install_root.clone(),
            };
            if let Err(err) = commands.try_register(command, source) {
                rollback(package_id, skills, commands, hooks, self);
                return Err(PackageError::Registry {
                    package_id: package_id.clone(),
                    message: err.to_string(),
                });
            }
            registered.commands.push(name);
        }

        for (asset_id, event, matcher) in prepared.hooks {
            let source = HookSource::Package {
                package_id: package_id.clone(),
                asset_id: asset_id.clone(),
                package_root: record.install_root.clone(),
            };
            if let Err(err) = hooks.register_with_source(event, matcher, source) {
                rollback(package_id, skills, commands, hooks, self);
                return Err(PackageError::Registry {
                    package_id: package_id.clone(),
                    message: err.to_string(),
                });
            }
            registered.hooks.push(asset_id);
        }

        for registration in prepared.mcp_servers {
            let asset_id = registration.asset_id.clone();
            self.mcp_servers.insert(asset_id.clone(), registration);
            registered.mcp_servers.push(asset_id);
        }

        Ok(registered)
    }
}

#[derive(Default)]
struct PreparedPackageAssets {
    skills: Vec<Skill>,
    commands: Vec<SlashCommand>,
    hooks: Vec<(String, HookEvent, HookMatcher)>,
    mcp_servers: Vec<PackageMcpServerRegistration>,
}

fn conflict(
    manifest: &ExtensionPackageManifest,
    kind: PackageAssetKind,
    asset_id: &str,
    existing_source: &str,
) -> PackageError {
    PackageError::RegistryConflict {
        package_id: manifest.id.clone(),
        asset_kind: kind,
        asset_id: asset_id.to_string(),
        existing_source: existing_source.to_string(),
    }
}

fn rollback(
    package_id: &str,
    skills: &mut SkillRegistry,
    commands: &mut CommandRegistry,
    hooks: &HookRegistry,
    bridge: &mut PackageRegistryBridge,
) {
    let _ = skills.remove_package_skills(package_id);
    let _ = commands.remove_package_commands(package_id);
    let _ = hooks.remove_package_hooks(package_id);
    bridge
        .mcp_servers
        .retain(|_, registration| registration.package_id != package_id);
}

#[cfg(test)]
#[path = "package_registry_bridge_tests.rs"]
mod tests;
