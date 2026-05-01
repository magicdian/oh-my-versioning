use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use crate::core::schema::{CargoWorkspaceTarget, OmvV2TargetConfig};
use crate::core::target::{CargoLockfileStrategy, CargoVersionLocation, TargetKind};
use crate::errors::{OmvError, TargetError};
use crate::sync::{
    PlanOperation, PlanStatus, PlanTargetResult, V2SyncContext, V2TargetSyncAdapter, planned_write,
    project_relative_path, read_text_if_exists, resolve_target_path,
};

#[derive(Debug, Default)]
pub struct CargoWorkspaceAdapter;

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemberManifest {
    name: String,
    path: PathBuf,
    content: String,
}

impl V2TargetSyncAdapter for CargoWorkspaceAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::CargoWorkspace
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::CargoWorkspace(config) = &context.target.config else {
            return Err(TargetError::InvalidTargetRecord(format!(
                "target {}: config does not match kind cargo-workspace",
                context.target.id
            ))
            .into());
        };

        let workspace_root = resolve_target_path(context.project_root, config.root.as_str(), "");
        let workspace_manifest_path = workspace_root.join("Cargo.toml");
        let Some(workspace_manifest) = read_text_if_exists(&workspace_manifest_path)? else {
            return Ok(result(
                context,
                vec![project_relative_path(
                    context.project_root,
                    &workspace_manifest_path,
                )],
                String::from("missing workspace Cargo.toml"),
                format!("workspace version {}", context.version),
                PlanStatus::Missing,
                Vec::new(),
                vec![String::from("workspace Cargo.toml is missing")],
            ));
        };

        match config.members {
            crate::core::target::CargoMembers::All => {}
        }
        match config.version_policy {
            crate::core::target::CargoVersionPolicy::Same => {}
        }
        let members = discover_members(&workspace_root, workspace_manifest.as_str())?;
        let member_names: Vec<String> = members.iter().map(|member| member.name.clone()).collect();
        let version_location = resolve_version_location(config, workspace_manifest.as_str())?;
        let mut paths = vec![project_relative_path(
            context.project_root,
            &workspace_manifest_path,
        )];
        let mut operations = Vec::new();
        let mut diagnostics = Vec::new();
        let mut drift = false;
        let mut current_summaries = Vec::new();

        match version_location {
            CargoVersionLocation::WorkspacePackage => {
                let (updated, current) = replace_workspace_package_version(
                    workspace_manifest.as_str(),
                    context.version,
                    context.target.id.as_str(),
                )?;
                current_summaries.push(format!("workspace.package={current}"));
                if updated != workspace_manifest {
                    drift = true;
                }
                operations.push(planned_write(
                    context.project_root,
                    &workspace_manifest_path,
                    updated,
                    "write [workspace.package] version from .omv version truth",
                ));
            }
            CargoVersionLocation::MemberPackages | CargoVersionLocation::Auto => {
                for member in members {
                    paths.push(project_relative_path(context.project_root, &member.path));
                    let (updated, current) = replace_package_version(
                        member.content.as_str(),
                        context.version,
                        context.target.id.as_str(),
                        member.path.as_path(),
                    )?;
                    current_summaries.push(format!("{}={current}", member.name));
                    if updated != member.content {
                        drift = true;
                    }
                    operations.push(planned_write(
                        context.project_root,
                        &member.path,
                        updated,
                        format!("write package version for {}", member.name),
                    ));
                }
            }
        }

        if config.lockfile != CargoLockfileStrategy::Ignore {
            let lock_path = workspace_root.join("Cargo.lock");
            if let Some(lock_content) = read_text_if_exists(&lock_path)? {
                paths.push(project_relative_path(context.project_root, &lock_path));
                let (updated, changed) =
                    update_lockfile_versions(lock_content.as_str(), &member_names, context.version);
                if changed {
                    drift = true;
                    diagnostics.push(String::from("Cargo.lock workspace package versions drift"));
                    if config.lockfile == CargoLockfileStrategy::Update {
                        operations.push(planned_write(
                            context.project_root,
                            &lock_path,
                            updated,
                            "update Cargo.lock workspace package versions",
                        ));
                    }
                }
            } else if config.lockfile == CargoLockfileStrategy::Check {
                diagnostics.push(String::from("Cargo.lock is missing for check strategy"));
            }
        }

        let status = if drift {
            PlanStatus::Drift
        } else {
            PlanStatus::Ok
        };
        if status == PlanStatus::Drift {
            diagnostics.push(String::from(
                "cargo workspace differs from .omv version truth",
            ));
        }

        Ok(result(
            context,
            paths,
            current_summaries.join(", "),
            format!("workspace members use {}", context.version),
            status,
            operations,
            diagnostics,
        ))
    }
}

fn result(
    context: &V2SyncContext<'_>,
    paths: Vec<String>,
    current_value_summary: String,
    expected_value_summary: String,
    status: PlanStatus,
    operations: Vec<PlanOperation>,
    diagnostics: Vec<String>,
) -> PlanTargetResult {
    PlanTargetResult {
        id: context.target.id.clone(),
        adapter: context.target.adapter.clone(),
        kind: context.target.kind.as_str().to_owned(),
        language: String::from("rust"),
        paths,
        current_value_summary,
        expected_value_summary,
        status,
        operations,
        diagnostics,
        required: true,
    }
}

fn resolve_version_location(
    config: &CargoWorkspaceTarget,
    workspace_manifest: &str,
) -> Result<CargoVersionLocation, OmvError> {
    if config.version_location != CargoVersionLocation::Auto {
        return Ok(config.version_location);
    }

    let parsed = parse_toml(workspace_manifest)?;
    if parsed
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("package"))
        .and_then(Value::as_table)
        .and_then(|package| package.get("version"))
        .is_some()
    {
        Ok(CargoVersionLocation::WorkspacePackage)
    } else {
        Ok(CargoVersionLocation::MemberPackages)
    }
}

fn discover_members(
    workspace_root: &Path,
    workspace_manifest: &str,
) -> Result<Vec<MemberManifest>, OmvError> {
    let parsed = parse_toml(workspace_manifest)?;
    let members = parsed
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            TargetError::InvalidTargetRecord(String::from(
                "cargo-workspace requires [workspace] members array",
            ))
        })?;

    let mut manifests = Vec::new();
    for member in members {
        let member = member.as_str().ok_or_else(|| {
            TargetError::InvalidTargetRecord(String::from(
                "cargo-workspace members entries must be strings",
            ))
        })?;
        if let Some(prefix) = member.strip_suffix("/*") {
            let dir = workspace_root.join(prefix);
            for entry in fs::read_dir(&dir).map_err(|err| {
                TargetError::InvalidTargetRecord(format!(
                    "failed to read cargo workspace member glob {member}: {err}"
                ))
            })? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    push_member_manifest(&mut manifests, &entry.path().join("Cargo.toml"))?;
                }
            }
        } else {
            push_member_manifest(
                &mut manifests,
                &workspace_root.join(member).join("Cargo.toml"),
            )?;
        }
    }
    manifests.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(manifests)
}

fn push_member_manifest(manifests: &mut Vec<MemberManifest>, path: &Path) -> Result<(), OmvError> {
    let content = fs::read_to_string(path).map_err(|err| {
        TargetError::InvalidTargetRecord(format!(
            "failed to read cargo member manifest {}: {err}",
            path.display()
        ))
    })?;
    let parsed = parse_toml(content.as_str())?;
    let name = parsed
        .get("package")
        .and_then(Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            TargetError::InvalidTargetRecord(format!(
                "cargo member manifest {} is missing [package] name",
                path.display()
            ))
        })?
        .to_owned();
    manifests.push(MemberManifest {
        name,
        path: path.to_path_buf(),
        content,
    });
    Ok(())
}

fn replace_workspace_package_version(
    content: &str,
    version: &str,
    target_id: &str,
) -> Result<(String, String), OmvError> {
    replace_section_version(content, "workspace.package", version, target_id, None)
}

fn replace_package_version(
    content: &str,
    version: &str,
    target_id: &str,
    path: &Path,
) -> Result<(String, String), OmvError> {
    replace_section_version(content, "package", version, target_id, Some(path))
}

fn replace_section_version(
    content: &str,
    section: &str,
    version: &str,
    target_id: &str,
    path: Option<&Path>,
) -> Result<(String, String), OmvError> {
    let mut output = Vec::new();
    let mut in_section = false;
    let mut matched = false;
    let mut current = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == format!("[{section}]");
        }

        if in_section && trimmed.starts_with("version") {
            let Some((_, value)) = trimmed.split_once('=') else {
                output.push(line.to_owned());
                continue;
            };
            matched = true;
            current = value.trim().trim_matches('"').to_owned();
            let indent = &line[..line.len() - line.trim_start().len()];
            output.push(format!("{indent}version = \"{version}\""));
        } else {
            output.push(line.to_owned());
        }
    }

    if !matched {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: {} section is missing version{}",
            section,
            path.map(|value| format!(" in {}", value.display()))
                .unwrap_or_default()
        ))
        .into());
    }

    let mut rendered = output.join("\n");
    if content.ends_with('\n') {
        rendered.push('\n');
    }
    Ok((rendered, current))
}

fn update_lockfile_versions(
    content: &str,
    package_names: &[String],
    version: &str,
) -> (String, bool) {
    let mut output = Vec::new();
    let mut in_package = false;
    let mut current_package_matches = false;
    let mut changed = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_package = true;
            current_package_matches = false;
            output.push(line.to_owned());
            continue;
        }
        if in_package && trimmed.starts_with("name =") {
            let name = trimmed
                .split_once('=')
                .map(|(_, value)| value.trim().trim_matches('"'))
                .unwrap_or_default();
            current_package_matches = package_names.iter().any(|package| package == name);
            output.push(line.to_owned());
            continue;
        }
        if in_package && current_package_matches && trimmed.starts_with("version =") {
            let expected_line = format!("version = \"{version}\"");
            if trimmed != expected_line {
                changed = true;
            }
            let indent = &line[..line.len() - line.trim_start().len()];
            output.push(format!("{indent}{expected_line}"));
            continue;
        }
        output.push(line.to_owned());
    }

    let mut rendered = output.join("\n");
    if content.ends_with('\n') {
        rendered.push('\n');
    }
    (rendered, changed)
}

fn parse_toml(content: &str) -> Result<Value, OmvError> {
    content.parse::<Value>().map_err(|err| {
        TargetError::InvalidTargetRecord(format!("failed to parse cargo manifest: {err}")).into()
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::SystemTime;

    use crate::core::schema::{CargoWorkspaceTarget, OmvV2TargetConfig, OmvV2TargetRecord};
    use crate::core::target::{
        CargoLockfileStrategy, CargoMembers, CargoVersionLocation, CargoVersionPolicy, TargetKind,
        TargetMode,
    };
    use crate::sync::{PlanStatus, V2SyncContext, V2TargetSyncAdapter};

    use super::CargoWorkspaceAdapter;

    #[test]
    fn cargo_workspace_updates_members_and_lockfile_narrowly() {
        let root = temp_root("cargo-workspace");
        fs::create_dir_all(root.join("crates/a")).expect("member should exist");
        fs::create_dir_all(root.join("crates/b")).expect("member should exist");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n",
        )
        .expect("workspace should write");
        fs::write(
            root.join("crates/a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
        )
        .expect("member a should write");
        fs::write(
            root.join("crates/b/Cargo.toml"),
            "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
        )
        .expect("member b should write");
        fs::write(
            root.join("Cargo.lock"),
            "[[package]]\nname = \"a\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"external\"\nversion = \"9.9.9\"\n",
        )
        .expect("lockfile should write");

        let target = OmvV2TargetRecord {
            id: "rust-workspace".to_owned(),
            kind: TargetKind::CargoWorkspace,
            adapter: "cargo".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config: OmvV2TargetConfig::CargoWorkspace(CargoWorkspaceTarget {
                root: ".".to_owned(),
                members: CargoMembers::All,
                version_policy: CargoVersionPolicy::Same,
                version_location: CargoVersionLocation::MemberPackages,
                lockfile: CargoLockfileStrategy::Update,
            }),
        };

        let plan = CargoWorkspaceAdapter
            .plan(&V2SyncContext {
                project_root: &root,
                target: &target,
                version: "2605.1.1",
            })
            .expect("workspace should plan");
        assert_eq!(plan.status, PlanStatus::Drift);
        assert!(plan.paths.iter().any(|path| path == "crates/a/Cargo.toml"));
        assert!(
            plan.operations
                .iter()
                .any(|operation| operation.path == "Cargo.lock")
        );
        let lock_op = plan
            .operations
            .iter()
            .find(|operation| operation.path == "Cargo.lock")
            .expect("lockfile operation should exist");
        assert!(lock_op.content.contains("version = \"2605.1.1\""));
        assert!(lock_op.content.contains("version = \"9.9.9\""));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("omv-{prefix}-{stamp}"));
        fs::create_dir_all(&root).expect("root should be created");
        root
    }
}
