//! Asset loading helpers for extension package registry bridge.

use super::PackageMcpServerRegistration;
use crate::commands::{CommandRegistry, SlashCommand};
use crate::config::McpServerConfig;
use crate::hooks::{HookEvent, HookMatcher};
use crate::plugins::package_error::{PackageError, PackageResult};
use crate::plugins::package_manifest::{PackageFileAsset, PackageHookAsset, PackageMcpServerAsset};
use crate::plugins::package_store::InstalledPackageRecord;
use crate::skills::{Skill, SkillRegistry, SkillSourceType};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn load_skill_asset(
    record: &InstalledPackageRecord,
    asset: &PackageFileAsset,
) -> PackageResult<Skill> {
    let path = record.install_root.join(&asset.path);
    let content =
        fs::read_to_string(&path).map_err(|err| registry_error(record, err.to_string()))?;
    let fallback_description = asset
        .metadata
        .get("description")
        .and_then(|value| value.as_str())
        .or(record.manifest.description.as_deref())
        .unwrap_or(&asset.id);

    Ok(SkillRegistry::skill_from_content(
        &asset.id,
        &content,
        SkillSourceType::Package {
            package_id: record.package_id.clone(),
            asset_id: asset.id.clone(),
            package_root: record.install_root.clone(),
        },
        Some(&asset_parent(record, &path)?),
        fallback_description.to_string(),
    ))
}

pub(super) fn load_command_asset(
    record: &InstalledPackageRecord,
    commands: &CommandRegistry,
    asset: &PackageFileAsset,
) -> PackageResult<SlashCommand> {
    let path = record.install_root.join(&asset.path);
    let content =
        fs::read_to_string(&path).map_err(|err| registry_error(record, err.to_string()))?;
    let (metadata, prompt_template) = commands.parse_command_file(&content);
    let mut command = SlashCommand::new(&asset.id, prompt_template).with_source_path(path);
    if let Some(description) = metadata.get("description") {
        command = command.with_description(description.clone());
    }
    Ok(command)
}

pub(super) fn load_hook_asset(
    record: &InstalledPackageRecord,
    asset: &PackageHookAsset,
) -> PackageResult<(String, HookEvent, HookMatcher)> {
    let path = record.install_root.join(&asset.path);
    let content =
        fs::read_to_string(&path).map_err(|err| registry_error(record, err.to_string()))?;
    let mut matcher: HookMatcher =
        toml::from_str(&content).map_err(|err| registry_error(record, err.to_string()))?;
    matcher.hook.name = asset.id.clone();
    let event = asset
        .event
        .as_deref()
        .and_then(|value| parse_hook_event(Some(value)))
        .ok_or_else(|| registry_error(record, "hook asset requires explicit event"))?;
    Ok((asset.id.clone(), event, matcher))
}

pub(super) fn load_mcp_asset(
    record: &InstalledPackageRecord,
    asset: &PackageMcpServerAsset,
) -> PackageResult<PackageMcpServerRegistration> {
    let config = if let Some(path) = &asset.path {
        let content = fs::read_to_string(record.install_root.join(path))
            .map_err(|err| registry_error(record, err.to_string()))?;
        let config: McpServerConfig =
            toml::from_str(&content).map_err(|err| registry_error(record, err.to_string()))?;
        validate_mcp_config(record, &asset.id, &config)?;
        config
    } else {
        inline_mcp_config(record, asset)?
    };

    Ok(PackageMcpServerRegistration {
        package_id: record.package_id.clone(),
        asset_id: asset.id.clone(),
        package_root: record.install_root.clone(),
        config,
    })
}

fn validate_mcp_config(
    record: &InstalledPackageRecord,
    asset_id: &str,
    config: &McpServerConfig,
) -> PackageResult<()> {
    match config.transport.as_str() {
        "stdio" => require_config_field(
            record,
            asset_id,
            "command",
            config.command.as_deref().unwrap_or_default(),
        ),
        "http" | "websocket" => require_config_field(
            record,
            asset_id,
            "url",
            config.url.as_deref().unwrap_or_default(),
        ),
        other => Err(registry_error(
            record,
            format!("MCP asset '{asset_id}' has unsupported transport '{other}'"),
        )),
    }
}

fn require_config_field(
    record: &InstalledPackageRecord,
    asset_id: &str,
    field: &str,
    value: &str,
) -> PackageResult<()> {
    if value.trim().is_empty() {
        Err(registry_error(
            record,
            format!("MCP asset '{asset_id}' requires {field}"),
        ))
    } else {
        Ok(())
    }
}

fn inline_mcp_config(
    record: &InstalledPackageRecord,
    asset: &PackageMcpServerAsset,
) -> PackageResult<McpServerConfig> {
    let transport = asset
        .transport
        .as_deref()
        .ok_or_else(|| registry_error(record, "inline MCP server requires transport"))?;
    let mut config = match transport {
        "stdio" => McpServerConfig::stdio(
            asset
                .command
                .clone()
                .ok_or_else(|| registry_error(record, "stdio MCP server requires command"))?,
            asset.args.clone(),
        ),
        "http" => McpServerConfig::http(
            asset
                .url
                .clone()
                .ok_or_else(|| registry_error(record, "http MCP server requires url"))?,
        ),
        "websocket" => McpServerConfig::websocket(
            asset
                .url
                .clone()
                .ok_or_else(|| registry_error(record, "websocket MCP server requires url"))?,
        ),
        other => {
            return Err(registry_error(
                record,
                format!("unsupported MCP transport '{other}'"),
            ));
        }
    };
    config.env = asset.env.clone().into_iter().collect();
    config.headers = asset.headers.clone().into_iter().collect();
    config.timeout_secs = asset.timeout_secs;
    Ok(config)
}

fn parse_hook_event(value: Option<&str>) -> Option<HookEvent> {
    match value? {
        "pre_tool_use" | "PreToolUse" => Some(HookEvent::PreToolUse),
        "post_tool_use" | "PostToolUse" => Some(HookEvent::PostToolUse),
        "post_tool_use_failure" | "PostToolUseFailure" => Some(HookEvent::PostToolUseFailure),
        "user_prompt_submit" | "UserPromptSubmit" => Some(HookEvent::UserPromptSubmit),
        "session_start" | "SessionStart" => Some(HookEvent::SessionStart),
        "session_end" | "SessionEnd" => Some(HookEvent::SessionEnd),
        "subagent_start" | "SubagentStart" => Some(HookEvent::SubagentStart),
        "subagent_stop" | "SubagentStop" => Some(HookEvent::SubagentStop),
        "permission_request" | "PermissionRequest" => Some(HookEvent::PermissionRequest),
        "pre_compact" | "PreCompact" => Some(HookEvent::PreCompact),
        "notification" | "Notification" => Some(HookEvent::Notification),
        "stop" | "Stop" => Some(HookEvent::Stop),
        "status_line" | "StatusLine" => Some(HookEvent::StatusLine),
        _ => None,
    }
}

fn asset_parent(record: &InstalledPackageRecord, path: &Path) -> PackageResult<PathBuf> {
    path.parent().map(Path::to_path_buf).ok_or_else(|| {
        registry_error(
            record,
            format!("asset path has no parent: {}", path.display()),
        )
    })
}

fn registry_error(record: &InstalledPackageRecord, message: impl Into<String>) -> PackageError {
    PackageError::Registry {
        package_id: record.package_id.clone(),
        message: message.into(),
    }
}
