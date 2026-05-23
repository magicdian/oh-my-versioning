use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::json;

use crate::core::adapter::{
    AdapterInstallMode, AdapterKind, AdapterTargetMode, AgentAdapter, SpecAdapter,
};
use crate::core::schema::{OmvAdapterInstallation, OmvAdapterTarget, OmvAdapters};
use crate::errors::{AdapterError, OmvError};
use crate::storage;

pub const AI_DIR: &str = "ai";
pub const CONTRACT_VERSION: u32 = 1;
pub const TRELLIS_FINISH_WORK_BLOCK_NAME: &str = "spec-trellis-finalize-boundary-finish-work";

const MANAGED_BEGIN_PREFIX: &str = "<!-- OMV-MANAGED-BEGIN:";
const MANAGED_END_PREFIX: &str = "<!-- OMV-MANAGED-END:";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdapterSelection {
    pub agents: Vec<AgentAdapter>,
    pub specs: Vec<SpecAdapter>,
}

impl AdapterSelection {
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty() && self.specs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterCatalog {
    pub agents: Vec<String>,
    pub specs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterStatusSummary {
    pub available: AdapterCatalog,
    pub installed: Vec<OmvAdapterInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterInstallSummary {
    pub installed: Vec<InstalledAdapterSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledAdapterSummary {
    pub kind: String,
    pub name: String,
    pub install_mode: String,
    pub targets: Vec<OmvAdapterTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceInstallBehavior {
    FullFileOrManagedBlock,
    DedicatedFile,
    ManagedBlockOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendPreference {
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdapterIdentity<'a> {
    kind: AdapterKind,
    name: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalTarget {
    source_rel: &'static str,
    host_rel: &'static str,
    behavior: SourceInstallBehavior,
}

pub fn ensure_canonical_artifacts(omv_root: &Path) -> Result<(), OmvError> {
    let ai_root = omv_root.join(AI_DIR);
    let contract = json!({
        "contract_version": CONTRACT_VERSION,
        "truth_source": ".omv/state.toml",
        "commands": {
            "current": {
                "read": ["omv current --json", "omv current --output json"]
            },
            "plan": {
                "read": ["omv plan --json", "omv plan --output json"],
                "mutates": false
            },
            "sync_check": {
                "read": ["omv sync --check --json", "omv sync --check --output json"],
                "mutates": false
            },
            "bump": {
                "write": ["omv bump --json", "omv bump --output json"],
                "syncs_targets": true
            },
            "sync": {
                "write": ["omv sync --json", "omv sync --output json"],
                "syncs_targets": true
            },
            "integrate_status": {
                "read": ["omv integrate status --json", "omv integrate status --output json"],
                "mutates": false,
                "state_file": ".omv/integrations.toml"
            },
            "integrate_apply": {
                "write": ["omv integrate apply --json", "omv integrate apply --output json"],
                "re_detects_workspace": true,
                "uses_targeted_worktree_safety": true,
                "state_file": ".omv/integrations.toml"
            },
            "finalize_boundary": {
                "write": ["omv event finalize-boundary --provider <provider> --boundary <name> --change-type <type> --json"],
                "identity_fields": ["provider", "boundary"],
                "source_mapping": "<provider>-<boundary>",
                "task_identity": {
                    "trellis_default": ".trellis/.current-task",
                    "override": "--task-id <task-id>"
                },
                "derived_fields": {
                    "status": "done",
                    "tests": "passed"
                },
                "fingerprint_inputs": ["task_id", "provider", "boundary", "workspace_snapshot_hash"],
                "snapshot_normalization": {
                    "managed_target_metadata": true,
                    "fixed_core_files": [
                        ".omv/state.toml",
                        ".omv/finalizations.toml",
                        ".omv/skills/README.md"
                    ]
                },
                "requires_change_type": true,
                "change_type_values": ["bugfix", "feature", "refactor", "docs", "chore"],
                "does_not_infer_change_type": true,
                "missing_change_type": "pending/manual-action without finalize-task",
                "delegates_to": "omv event finalize-task"
            }
        },
        "adapter_command": {
            "install": "omv adapter install --agent <name> --spec <name>",
            "refresh": "omv adapter refresh",
            "status": "omv adapter status",
            "list": "omv adapter list",
            "transition_policy": "temporary MVP compatibility surface; prefer omv integrate status/apply for new automation where available",
            "compatibility_mapping": {
                "list": "omv integrate status",
                "status": "omv integrate status",
                "install": "omv integrate apply for selected integration capabilities, including project-instructions, host-skill, spec-guide, spec-index-snippet, and finalize-boundary",
                "refresh": "omv integrate apply for already selected projection capabilities"
            }
        },
        "integration_model": {
            "truth_source": ".omv/integrations.toml",
            "providers": {
                "codex": {
                    "provider_type": "agent",
                    "mvp_supported": true,
                    "bootstrap_policy": "may create lightweight instruction host files",
                    "capabilities": ["project-instructions", "host-skill"]
                },
                "opencode": {
                    "provider_type": "agent",
                    "mvp_supported": true,
                    "bootstrap_policy": "may create lightweight instruction host files",
                    "capabilities": ["project-instructions", "host-skill"]
                },
                "trellis": {
                    "provider_type": "spec",
                    "mvp_supported": true,
                    "bootstrap_policy": "requires existing Trellis installation before mutation",
                    "capabilities": ["spec-guide", "spec-index-snippet", "finalize-boundary"]
                },
                "claude": {
                    "provider_type": "agent",
                    "mvp_supported": false,
                    "hidden_from_init": true
                },
                "openspec": {
                    "provider_type": "spec",
                    "mvp_supported": false,
                    "hidden_from_init": true
                }
            },
            "capability_statuses": ["selected", "pending", "installed", "failed"],
            "failure_contract": {
                "reason_code": "stable machine-readable string",
                "message": "human-readable diagnostic"
            },
            "host_files_are_authoritative": false
        },
        "plugin_runtime": {
            "public_runtime_in_mvp": false,
            "policy": "providers are internal registry entries in MVP; third-party plugin runtime is future work"
        },
        "rules": {
            "native_manifests_are_derived_outputs": true,
            "runtime_exports_are_read_only_views": true,
            "do_not_edit_native_manifest_versions_directly": true,
            "host_adapter_files_are_derived_outputs": true,
            "do_not_treat_host_files_as_authority": true,
            "generalized_target_kinds": [
                "text-scalar",
                "regex-replace",
                "markdown-managed-block",
                "yaml-scalar",
                "c-header-macro",
                "cargo-workspace"
            ]
        }
    });
    storage::atomic::write_atomically(
        &ai_root.join("contract.json"),
        serde_json::to_string_pretty(&contract)
            .expect("static contract json should serialize")
            .as_bytes(),
    )?;

    let instructions = [
        "# OMV Versioning Instructions",
        "",
        "- Version truth lives in `.omv/state.toml`.",
        "- Integration desired state and last detection snapshot live in `.omv/integrations.toml`.",
        "- Read the current managed version with `omv current --json`.",
        "- Preview target drift and proposed writes with `omv plan --json`.",
        "- Check target drift without mutation with `omv sync --check --json`.",
        "- Inspect host integration provider/capability status with `omv integrate status --json` when that command is available.",
        "- Apply selected or pending host integration capabilities with `omv integrate apply --json` when that command is available.",
        "- Change the managed version with `omv bump --json`.",
        "- At completion boundaries (during commit confirmation when a unit of work is complete), use the OMV finalize-boundary helper advertised in `.omv/ai/contract.json`; provide an explicit `change_type` value and do not infer or default it.",
        "- `.omv/targets.toml` kind-based targets can manage text scalars, regex replacements, Markdown managed blocks, YAML scalars, C header macros, and Cargo workspaces; update OMV if a configured kind is reported as unsupported.",
        "- Do not edit `Cargo.toml`, `CMakeLists.txt`, `pyproject.toml`, `go.mod`, or other native manifest versions directly.",
        "- Before release-sensitive edits, run `omv plan --json`; before committing or publishing, run `omv sync --check --json`.",
        "- Treat runtime export files such as `src/generated/version.rs` and `include/omv_version.h` as generated read-only views.",
        "- Treat host files such as `AGENTS.md`, `CLAUDE.md`, `.codex/skills/*`, and Trellis/OpenSpec guides as derived projections, not OMV authority.",
        "",
        "When integrating OMV with agents or spec frameworks, keep the detailed rules in `.omv/ai/*` and project only thin host adapters into external files. Legacy `omv adapter ...` commands remain temporary compatibility commands during the MVP transition; new automation should prefer `omv integrate status/apply` where available.",
        "",
    ]
    .join("\n");
    storage::atomic::write_atomically(&ai_root.join("instructions.md"), instructions.as_bytes())?;

    for (path, content) in canonical_sources() {
        let target = ai_root.join(path);
        storage::atomic::write_atomically(&target, content.as_bytes())?;
    }

    Ok(())
}

pub fn available_catalog() -> AdapterCatalog {
    AdapterCatalog {
        agents: AgentAdapter::all()
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        specs: SpecAdapter::all()
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
    }
}

pub fn status(omv_root: &Path) -> Result<AdapterStatusSummary, OmvError> {
    ensure_canonical_artifacts(omv_root)?;
    let installed = storage::adapters::load_adapters_if_exists(omv_root)?.installations;
    Ok(AdapterStatusSummary {
        available: available_catalog(),
        installed,
    })
}

pub fn install_selected(
    omv_root: &Path,
    project_root: &Path,
    selection: &AdapterSelection,
) -> Result<AdapterInstallSummary, OmvError> {
    ensure_canonical_artifacts(omv_root)?;

    let mut registry = storage::adapters::load_adapters_if_exists(omv_root)?;
    let mut installed = Vec::new();

    for agent in &selection.agents {
        let installation =
            install_agent_adapter(omv_root, project_root, *agent, BackendPreference::Auto)?;
        upsert_installation(&mut registry, installation.clone());
        installed.push(to_summary(&installation));
    }

    for spec in &selection.specs {
        let installation =
            install_spec_adapter(omv_root, project_root, *spec, BackendPreference::Auto)?;
        upsert_installation(&mut registry, installation.clone());
        installed.push(to_summary(&installation));
    }

    storage::adapters::save_adapters(omv_root, &registry)?;
    Ok(AdapterInstallSummary { installed })
}

pub fn refresh_selected(
    omv_root: &Path,
    project_root: &Path,
    selection: &AdapterSelection,
) -> Result<AdapterInstallSummary, OmvError> {
    let effective = if selection.is_empty() {
        selection_from_registry(omv_root)?
    } else {
        selection.clone()
    };
    install_selected(omv_root, project_root, &effective)
}

pub fn trellis_finish_work_finalize_block() -> String {
    [
        "## OMV Finalize Boundary",
        "",
        "- [ ] Choose exactly one OMV `change_type`: `bugfix`, `feature`, `refactor`, `docs`, or `chore`.",
        "- [ ] Run `omv sync --check --json` and treat any required target drift as blocking.",
        "- [ ] If drift is expected and target files should be updated, run `omv sync --json`, then rerun `omv sync --check --json` before finalizing.",
        "- [ ] After the required finish-work checks pass, run `omv event finalize-boundary --provider trellis --boundary finish-work --change-type <change_type> --json`.",
        "- [ ] Do not treat `finalize-boundary` as target sync: non-semantic change types record a no-op finalization and do not write target files.",
        "- [ ] If `change_type` is unresolved, leave OMV in pending/manual-action state; do not infer a value or call `finalize-task` directly with guessed fields.",
    ]
    .join("\n")
}

pub fn upsert_trellis_finish_work_finalize_block(existing: &str) -> String {
    let begin = format!("{MANAGED_BEGIN_PREFIX}{TRELLIS_FINISH_WORK_BLOCK_NAME} -->");
    let end = format!("{MANAGED_END_PREFIX}{TRELLIS_FINISH_WORK_BLOCK_NAME} -->");
    let block = format!("{begin}\n{}\n{end}\n", trellis_finish_work_finalize_block());

    let without_existing = remove_managed_block(existing, &begin, &end);
    let marker = "\n## Quick Check Flow";
    if let Some(index) = without_existing.find(marker) {
        let mut output = String::new();
        output.push_str(without_existing[..index].trim_end());
        output.push_str("\n\n");
        output.push_str(&block);
        output.push('\n');
        output.push_str(without_existing[index + 1..].trim_start_matches('\n'));
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }

    replace_or_append_managed_block(&without_existing, &begin, &end, &block)
}

fn selection_from_registry(omv_root: &Path) -> Result<AdapterSelection, OmvError> {
    let registry = storage::adapters::load_adapters_if_exists(omv_root)?;
    let mut selection = AdapterSelection::default();
    for installation in registry.installations {
        match installation.kind {
            AdapterKind::Agent => {
                if let Some(agent) = AgentAdapter::parse(&installation.name) {
                    selection.agents.push(agent);
                }
            }
            AdapterKind::Spec => {
                if let Some(spec) = SpecAdapter::parse(&installation.name) {
                    selection.specs.push(spec);
                }
            }
        }
    }
    Ok(selection)
}

fn install_agent_adapter(
    omv_root: &Path,
    project_root: &Path,
    adapter: AgentAdapter,
    preference: BackendPreference,
) -> Result<OmvAdapterInstallation, OmvError> {
    let targets = match adapter {
        AgentAdapter::Claude => vec![CanonicalTarget {
            source_rel: "adapters/claude/CLAUDE.md",
            host_rel: "CLAUDE.md",
            behavior: SourceInstallBehavior::FullFileOrManagedBlock,
        }],
        AgentAdapter::Codex => vec![
            CanonicalTarget {
                source_rel: "adapters/project-instructions.md",
                host_rel: "AGENTS.md",
                behavior: SourceInstallBehavior::FullFileOrManagedBlock,
            },
            CanonicalTarget {
                source_rel: "adapters/codex/SKILL.md",
                host_rel: ".codex/skills/omv-versioning/SKILL.md",
                behavior: SourceInstallBehavior::DedicatedFile,
            },
        ],
        AgentAdapter::OpenCode => vec![
            CanonicalTarget {
                source_rel: "adapters/project-instructions.md",
                host_rel: "AGENTS.md",
                behavior: SourceInstallBehavior::FullFileOrManagedBlock,
            },
            CanonicalTarget {
                source_rel: "adapters/opencode/SKILL.md",
                host_rel: ".opencode/skills/omv-versioning/SKILL.md",
                behavior: SourceInstallBehavior::DedicatedFile,
            },
        ],
    };

    install_plan(
        omv_root,
        project_root,
        AdapterKind::Agent,
        adapter.as_str(),
        &targets,
        preference,
    )
}

fn install_spec_adapter(
    omv_root: &Path,
    project_root: &Path,
    adapter: SpecAdapter,
    preference: BackendPreference,
) -> Result<OmvAdapterInstallation, OmvError> {
    let targets = match adapter {
        SpecAdapter::OpenSpec => vec![
            CanonicalTarget {
                source_rel: "adapters/openspec/project.md",
                host_rel: "openspec/project.md",
                behavior: SourceInstallBehavior::FullFileOrManagedBlock,
            },
            CanonicalTarget {
                source_rel: "adapters/openspec/versioning-source-unification.spec.md",
                host_rel: "openspec/specs/versioning-source-unification/spec.md",
                behavior: SourceInstallBehavior::DedicatedFile,
            },
        ],
        SpecAdapter::Trellis => vec![
            CanonicalTarget {
                source_rel: "adapters/trellis/guide.md",
                host_rel: ".trellis/spec/guides/omv-versioning-guide.md",
                behavior: SourceInstallBehavior::DedicatedFile,
            },
            CanonicalTarget {
                source_rel: "adapters/trellis/index-snippet.md",
                host_rel: ".trellis/spec/guides/index.md",
                behavior: SourceInstallBehavior::ManagedBlockOnly,
            },
        ],
    };

    install_plan(
        omv_root,
        project_root,
        AdapterKind::Spec,
        adapter.as_str(),
        &targets,
        preference,
    )
}

fn install_plan(
    omv_root: &Path,
    project_root: &Path,
    kind: AdapterKind,
    name: &str,
    targets: &[CanonicalTarget],
    preference: BackendPreference,
) -> Result<OmvAdapterInstallation, OmvError> {
    let mut installed_targets = Vec::new();
    let identity = AdapterIdentity { kind, name };

    for target in targets {
        let source_path = omv_root.join(AI_DIR).join(target.source_rel);
        let host_path = project_root.join(target.host_rel);
        let rendered = fs::read_to_string(&source_path)?;
        let mode = install_target(
            &source_path,
            &host_path,
            target.source_rel,
            &rendered,
            identity,
            target.behavior,
            preference,
        )?;

        installed_targets.push(OmvAdapterTarget {
            path: target.host_rel.to_owned(),
            source_path: format!(".omv/{AI_DIR}/{}", target.source_rel),
            mode,
        });
    }

    let install_mode = derive_install_mode(&installed_targets);
    Ok(OmvAdapterInstallation {
        kind,
        name: name.to_owned(),
        install_mode,
        source_contract_version: CONTRACT_VERSION,
        targets: installed_targets,
    })
}

fn install_target(
    source_path: &Path,
    host_path: &Path,
    source_rel: &str,
    rendered: &str,
    identity: AdapterIdentity<'_>,
    behavior: SourceInstallBehavior,
    preference: BackendPreference,
) -> Result<AdapterTargetMode, OmvError> {
    match behavior {
        SourceInstallBehavior::ManagedBlockOnly => {
            write_managed_block(
                host_path,
                managed_block_name(identity.kind, identity.name, source_rel),
                rendered,
            )?;
            Ok(AdapterTargetMode::ManagedBlock)
        }
        SourceInstallBehavior::DedicatedFile => {
            install_dedicated_file(source_path, host_path, source_rel, rendered, preference)
        }
        SourceInstallBehavior::FullFileOrManagedBlock => {
            if host_path.exists() {
                let content = fs::read_to_string(host_path).unwrap_or_default();
                if is_omv_managed_file(&content) || is_same_symlink(host_path, source_path) {
                    install_dedicated_file(source_path, host_path, source_rel, rendered, preference)
                } else {
                    write_managed_block(
                        host_path,
                        managed_block_name(identity.kind, identity.name, source_rel),
                        rendered,
                    )?;
                    Ok(AdapterTargetMode::ManagedBlock)
                }
            } else {
                install_dedicated_file(source_path, host_path, source_rel, rendered, preference)
            }
        }
    }
}

fn install_dedicated_file(
    source_path: &Path,
    host_path: &Path,
    source_rel: &str,
    rendered: &str,
    preference: BackendPreference,
) -> Result<AdapterTargetMode, OmvError> {
    if host_path.exists() {
        let content = fs::read_to_string(host_path).unwrap_or_default();
        if !is_omv_managed_file(&content) && !is_same_symlink(host_path, source_path) {
            return Err(AdapterError::Conflict {
                path: host_path.to_path_buf(),
                reason: String::from("existing file is not OMV-managed"),
            }
            .into());
        }
        let metadata = fs::symlink_metadata(host_path)?;
        if metadata.file_type().is_symlink() {
            fs::remove_file(host_path)?;
        }
    }

    match preference {
        BackendPreference::Auto => {
            if try_install_symlink(source_path, host_path)? {
                return Ok(AdapterTargetMode::Link);
            }
        }
    }

    let materialized = wrap_managed_file(source_rel, rendered);
    storage::atomic::write_atomically(host_path, materialized.as_bytes())?;
    Ok(AdapterTargetMode::Materialize)
}

fn try_install_symlink(source_path: &Path, host_path: &Path) -> Result<bool, OmvError> {
    #[cfg(unix)]
    {
        if let Some(parent) = host_path.parent() {
            fs::create_dir_all(parent)?;
        }

        use std::os::unix::fs::symlink;
        match symlink(source_path, host_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    #[cfg(not(unix))]
    let _ = (source_path, host_path);
    #[cfg(not(unix))]
    Ok(false)
}

fn is_same_symlink(host_path: &Path, source_path: &Path) -> bool {
    if let Ok(link) = fs::read_link(host_path)
        && let Ok(canonical_link) = link.canonicalize()
        && let Ok(canonical_source) = source_path.canonicalize()
    {
        return canonical_link == canonical_source;
    }
    false
}

fn derive_install_mode(targets: &[OmvAdapterTarget]) -> AdapterInstallMode {
    let all_link = targets
        .iter()
        .all(|target| target.mode == AdapterTargetMode::Link);
    let all_materialized = targets.iter().all(|target| {
        matches!(
            target.mode,
            AdapterTargetMode::Materialize | AdapterTargetMode::ManagedBlock
        )
    });

    if all_link {
        AdapterInstallMode::Link
    } else if all_materialized {
        AdapterInstallMode::Materialize
    } else {
        AdapterInstallMode::Hybrid
    }
}

fn upsert_installation(registry: &mut OmvAdapters, installation: OmvAdapterInstallation) {
    if let Some(existing) = registry
        .installations
        .iter_mut()
        .find(|item| item.kind == installation.kind && item.name == installation.name)
    {
        *existing = installation;
        return;
    }

    registry.installations.push(installation);
}

fn to_summary(installation: &OmvAdapterInstallation) -> InstalledAdapterSummary {
    InstalledAdapterSummary {
        kind: installation.kind.as_str().to_owned(),
        name: installation.name.clone(),
        install_mode: installation.install_mode.as_str().to_owned(),
        targets: installation.targets.clone(),
    }
}

fn write_managed_block(path: &Path, block_name: String, rendered: &str) -> Result<(), OmvError> {
    let begin = format!("{MANAGED_BEGIN_PREFIX}{block_name} -->");
    let end = format!("{MANAGED_END_PREFIX}{block_name} -->");
    let block = format!("{begin}\n{rendered}\n{end}\n");

    let content = match fs::read_to_string(path) {
        Ok(existing) => replace_or_append_managed_block(&existing, &begin, &end, &block),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => block,
        Err(err) => return Err(err.into()),
    };

    storage::atomic::write_atomically(path, content.as_bytes())
}

fn replace_or_append_managed_block(
    existing: &str,
    begin: &str,
    end: &str,
    replacement: &str,
) -> String {
    if let Some(start) = existing.find(begin)
        && let Some(end_idx) = existing[start..].find(end)
    {
        let absolute_end = start + end_idx + end.len();
        let mut output = String::with_capacity(existing.len() + replacement.len());
        output.push_str(&existing[..start]);
        if !output.ends_with('\n') && !output.is_empty() {
            output.push('\n');
        }
        output.push_str(replacement);
        if absolute_end < existing.len() {
            let tail = &existing[absolute_end..];
            if !tail.starts_with('\n') && !tail.is_empty() {
                output.push('\n');
            }
            output.push_str(tail.trim_start_matches('\n'));
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }

    let mut output = existing.trim_end().to_owned();
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(replacement);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn remove_managed_block(existing: &str, begin: &str, end: &str) -> String {
    if let Some(start) = existing.find(begin)
        && let Some(end_idx) = existing[start..].find(end)
    {
        let absolute_end = start + end_idx + end.len();
        let mut output = String::new();
        output.push_str(existing[..start].trim_end());
        if absolute_end < existing.len() {
            output.push_str("\n\n");
            output.push_str(existing[absolute_end..].trim_start_matches('\n'));
        }
        if !output.ends_with('\n') {
            output.push('\n');
        }
        return output;
    }

    existing.to_owned()
}

pub(crate) fn wrap_managed_file(source_rel: &str, rendered: &str) -> String {
    let marker = format!(
        "<!-- OMV-MANAGED-FILE source=.omv/{AI_DIR}/{source_rel} contract={CONTRACT_VERSION} -->"
    );
    prepend_managed_marker(&marker, rendered)
}

fn prepend_managed_marker(marker: &str, rendered: &str) -> String {
    if let Some(close_start) = rendered
        .strip_prefix("---\n")
        .and_then(|rest| rest.find("\n---\n").map(|index| index + "---\n".len()))
    {
        let close_end = close_start + "\n---\n".len();
        return format!(
            "{}{}\n{}",
            &rendered[..close_end],
            marker,
            &rendered[close_end..]
        );
    }

    format!("{marker}\n{rendered}")
}

fn is_omv_managed_file(content: &str) -> bool {
    content.contains("<!-- OMV-MANAGED-FILE") || content.contains(MANAGED_BEGIN_PREFIX)
}

fn managed_block_name(kind: AdapterKind, name: &str, source_rel: &str) -> String {
    format!(
        "{}-{}-{}",
        kind.as_str(),
        name,
        source_rel.replace('/', "-")
    )
}

fn canonical_sources() -> Vec<(&'static str, String)> {
    vec![
        (
            "adapters/claude/CLAUDE.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/claude/CLAUDE.md contract=1 -->",
                "# OMV Claude Adapter",
                "",
                "@./.omv/ai/instructions.md",
                "",
                "Use `omv current --json` to read version truth, `omv plan --json` to preview target sync, `omv sync --check --json` to detect drift, and `omv bump --json` to update it.",
                "",
                "For host integration state, prefer `omv integrate status --json` and `omv integrate apply --json` where available. Treat this file as a projection from `.omv/ai/*`, not authority.",
            ]
            .join("\n"),
        ),
        (
            "adapters/codex/AGENTS.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/codex/AGENTS.md contract=1 -->",
                "# OMV Codex Adapter",
                "",
                "Read `./.omv/ai/instructions.md` before touching project versions.",
                "",
                "- Use `omv current --json` to inspect the managed version.",
                "- Use `omv plan --json` before editing version-sensitive surfaces.",
                "- Use `omv sync --check --json` to verify target drift without writing.",
                "- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability status where available.",
                "- At finalize boundaries, call the OMV finalize-boundary helper from `.omv/ai/contract.json` only after tests pass and only with an explicit `change_type`.",
                "- Use `omv bump --json` to advance the managed version.",
                "- Do not edit native manifest versions directly.",
                "- Treat this host file as a derived projection; `.omv/*` and `.omv/ai/*` remain authoritative.",
            ]
            .join("\n"),
        ),
        (
            "adapters/codex/SKILL.md",
            [
                "---",
                "name: omv-versioning",
                "description: \"Use OMV as the version source of truth for this project.\"",
                "---",
                "",
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/codex/SKILL.md contract=1 -->",
                "",
                "1. Read `./.omv/ai/instructions.md`.",
                "2. Use `omv current --json` to inspect current version truth.",
                "3. Use `omv plan --json` or `omv sync --check --json` before changing version-sensitive files.",
                "4. Use `omv integrate status --json` to inspect host integration state and `omv integrate apply --json` to apply selected capabilities where available.",
                "5. At completion boundaries, call the OMV finalize-boundary helper from `./.omv/ai/contract.json` only with an explicit `change_type`; ask the user when the value is missing.",
                "6. Use `omv bump --json` to mutate version truth.",
                "7. Do not hand-edit manifest versions or treat host adapter files as authority.",
            ]
            .join("\n"),
        ),
        (
            "adapters/openspec/project.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/openspec/project.md contract=1 -->",
                "# OMV Version Governance",
                "",
                "This project uses `omv` as the authoritative version source.",
                "",
                "- Version truth: `.omv/state.toml`",
                "- Read current version: `omv current --json`",
                "- Preview sync plan: `omv plan --json`",
                "- Check drift without writes: `omv sync --check --json`",
                "- Check host integration status: `omv integrate status --json` where available",
                "- Apply selected host integration capabilities: `omv integrate apply --json` where available",
                "- Update version truth: `omv bump --json`",
                "- Native manifests are synchronized outputs, not authority",
                "- Host adapter/spec files are derived projections, not authority",
                "",
                "See `./.omv/ai/instructions.md` for the canonical workflow.",
            ]
            .join("\n"),
        ),
        (
            "adapters/openspec/versioning-source-unification.spec.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/openspec/versioning-source-unification.spec.md contract=1 -->",
                "# Spec: Versioning Source Unification",
                "",
                "## Requirements",
                "",
                "- The project MUST treat `.omv/state.toml` as version truth.",
                "- Workflows MUST read current version via `omv current --json`.",
                "- Workflows SHOULD preview target changes via `omv plan --json`.",
                "- Workflows SHOULD gate drift via `omv sync --check --json` before manual edits or CI checks.",
                "- Workflows SHOULD use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability state where available.",
                "- Workflows MUST update managed version via `omv bump --json`.",
                "- Native manifests and runtime export files MUST be treated as derived outputs.",
                "- Host adapter/spec files MUST be treated as derived projections of `.omv/ai/*`.",
            ]
            .join("\n"),
        ),
        (
            "adapters/trellis/guide.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/trellis/guide.md contract=1 -->",
                "# OMV Versioning Guide",
                "",
                "- `.omv/state.toml` is the version source of truth.",
                "- Use `omv current --json` for reads.",
                "- Use `omv plan --json` to preview target changes.",
                "- Use `omv sync --check --json` to verify drift without mutation.",
                "- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability state where available.",
                "- **Trellis v0.5+:** If the Trellis finalize-boundary capability is installed, call the OMV helper advertised in `.omv/ai/contract.json` during Phase 3.4 commit confirmation, before `/trellis:finish-work`. When the user confirms a commit during Phase 3.4, run `omv event finalize-boundary --provider trellis --boundary finish-work --change-type <type> --json`, then commit the resulting bump files. Supply an explicit `change_type`; do not infer it.",
                "- **Trellis v0.4:** When the finalize-boundary block is present in the finish-work skill, the `/trellis:finish-work` flow may trigger `finalize-boundary` automatically. If it does not, invoke it explicitly after finish-work succeeds.",
                "- Use `omv bump --json` for writes.",
                "- Do not trust manifest versions as authority.",
                "- Do not treat this guide or other host files as OMV authority.",
                "",
                "Canonical reference: `./.omv/ai/instructions.md`",
            ]
            .join("\n"),
        ),
        (
            "adapters/trellis/index-snippet.md",
            [
                "## OMV",
                "",
                "- [OMV Versioning Guide](./omv-versioning-guide.md) | Managed version and integration source rules",
            ]
            .join("\n"),
        ),
        (
            "adapters/project-instructions.md",
            [
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/project-instructions.md contract=1 -->",
                "# OMV Agent Instructions",
                "",
                "Read `./.omv/ai/instructions.md` before touching project versions.",
                "",
                "- Use `omv current --json` to inspect the managed version.",
                "- Use `omv plan --json` before editing version-sensitive surfaces.",
                "- Use `omv sync --check --json` to verify target drift without writing.",
                "- Use `omv integrate status --json` and `omv integrate apply --json` for host integration provider/capability status where available.",
                "- During commit confirmation (when a unit of work is complete and ready to commit), call the OMV finalize-boundary helper from `.omv/ai/contract.json` only after tests pass and only with an explicit `change_type`. Do not wait until a separate finish-work step — call finalize-boundary as soon as the commit is confirmed.",
                "- Use `omv bump --json` to advance the managed version.",
                "- Do not edit native manifest versions directly.",
                "- Treat this host file as a derived projection; `.omv/*` and `.omv/ai/*` remain authoritative.",
            ]
            .join("\n"),
        ),
        (
            "adapters/opencode/SKILL.md",
            [
                "---",
                "name: omv-versioning",
                "description: \"Use OMV as the version source of truth for this project.\"",
                "---",
                "",
                "<!-- OMV-MANAGED-FILE source=.omv/ai/adapters/opencode/SKILL.md contract=1 -->",
                "",
                "1. Read `./.omv/ai/instructions.md`.",
                "2. Use `omv current --json` to inspect current version truth.",
                "3. Use `omv plan --json` or `omv sync --check --json` before changing version-sensitive files.",
                "4. Use `omv integrate status --json` to inspect host integration state and `omv integrate apply --json` to apply selected capabilities where available.",
                "5. At completion boundaries, call the OMV finalize-boundary helper from `./.omv/ai/contract.json` only with an explicit `change_type`; ask the user when the value is missing.",
                "6. Use `omv bump --json` to mutate version truth.",
                "7. Do not hand-edit manifest versions or treat host adapter files as authority.",
            ]
            .join("\n"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::adapter::AdapterTargetMode;
    use crate::storage;

    use super::{
        AdapterSelection, ensure_canonical_artifacts, install_selected, refresh_selected, status,
        upsert_trellis_finish_work_finalize_block,
    };

    #[test]
    fn canonical_artifacts_are_generated_under_omv_ai() {
        let omv_root = temp_omv_root("canonical");

        ensure_canonical_artifacts(&omv_root).expect("canonical artifacts should generate");

        let contract =
            fs::read_to_string(omv_root.join("ai/contract.json")).expect("contract should exist");
        assert!(contract.contains("\"contract_version\": 1"));
        let instructions = fs::read_to_string(omv_root.join("ai/instructions.md"))
            .expect("instructions should exist");
        assert!(instructions.contains("omv current --json"));
        assert!(instructions.contains("omv plan --json"));
        assert!(instructions.contains("omv sync --check --json"));
        assert!(instructions.contains("omv integrate status --json"));
        assert!(instructions.contains("finalize-boundary helper"));
        assert!(instructions.contains("derived projections"));
        assert!(contract.contains("\"integration_model\""));
        assert!(contract.contains("\"public_runtime_in_mvp\": false"));
        assert!(contract.contains("\"finalize_boundary\""));
        assert!(contract.contains("\"missing_change_type\""));
        let codex_skill = fs::read_to_string(omv_root.join("ai/adapters/codex/SKILL.md"))
            .expect("codex skill source should exist");
        assert!(codex_skill.starts_with("---\n"));
        assert!(codex_skill.contains("<!-- OMV-MANAGED-FILE"));

        cleanup_root(&omv_root);
    }

    #[test]
    fn trellis_finish_work_managed_block_is_inserted_once_before_quick_check() {
        let input = "# Finish Work\n\n## Checklist\n\n## Quick Check Flow\n\nbody\n";
        let once = upsert_trellis_finish_work_finalize_block(input);
        let twice = upsert_trellis_finish_work_finalize_block(&once);

        assert_eq!(once, twice);
        assert_eq!(twice.matches("OMV-MANAGED-BEGIN").count(), 1);
        assert!(twice.contains("omv sync --check --json"));
        assert!(twice.contains("omv sync --json"));
        assert!(twice.contains("do not write target files"));
        let block = twice
            .find("OMV Finalize Boundary")
            .expect("managed block should exist");
        let quick = twice
            .find("## Quick Check Flow")
            .expect("quick check should exist");
        assert!(block < quick);
    }

    #[test]
    fn install_codex_creates_registry_and_host_files() {
        let root = temp_project_root("install-codex");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");

        let selection = AdapterSelection {
            agents: vec![crate::core::adapter::AgentAdapter::Codex],
            specs: Vec::new(),
        };
        let summary = install_selected(&omv_root, &root, &selection).expect("install should work");
        assert_eq!(summary.installed.len(), 1);

        let registry = storage::adapters::load_adapters(&omv_root).expect("registry should load");
        assert_eq!(registry.installations.len(), 1);
        assert!(root.join("AGENTS.md").exists());
        assert!(root.join(".codex/skills/omv-versioning/SKILL.md").exists());
        let codex_skill = fs::read_to_string(root.join(".codex/skills/omv-versioning/SKILL.md"))
            .expect("codex skill host file should exist");
        assert!(codex_skill.starts_with("---\n"));
        assert!(codex_skill.contains("<!-- OMV-MANAGED-FILE"));

        cleanup_project_root(&root);
    }

    #[test]
    fn install_claude_into_existing_file_uses_managed_block() {
        let root = temp_project_root("install-claude-block");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");
        fs::write(root.join("CLAUDE.md"), "# Existing\n").expect("seed claude file");

        let selection = AdapterSelection {
            agents: vec![crate::core::adapter::AgentAdapter::Claude],
            specs: Vec::new(),
        };
        let summary = install_selected(&omv_root, &root, &selection).expect("install should work");
        let target = &summary.installed[0].targets[0];
        assert_eq!(target.mode, AdapterTargetMode::ManagedBlock);

        let content = fs::read_to_string(root.join("CLAUDE.md")).expect("claude file should exist");
        assert!(content.contains("OMV-MANAGED-BEGIN"));
        assert!(content.contains("# Existing"));

        cleanup_project_root(&root);
    }

    #[test]
    fn status_reports_available_and_installed_adapters() {
        let root = temp_project_root("status");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");

        let selection = AdapterSelection {
            agents: vec![crate::core::adapter::AgentAdapter::Codex],
            specs: vec![crate::core::adapter::SpecAdapter::OpenSpec],
        };
        install_selected(&omv_root, &root, &selection).expect("install should work");
        let status = status(&omv_root).expect("status should succeed");
        assert!(status.available.agents.contains(&String::from("codex")));
        assert_eq!(status.installed.len(), 2);

        cleanup_project_root(&root);
    }

    #[test]
    fn refresh_without_selection_reuses_registry_installations() {
        let root = temp_project_root("refresh-registry");
        let omv_root = root.join(".omv");
        fs::create_dir_all(&omv_root).expect("omv root should exist");

        let selection = AdapterSelection {
            agents: vec![crate::core::adapter::AgentAdapter::Codex],
            specs: vec![crate::core::adapter::SpecAdapter::OpenSpec],
        };
        install_selected(&omv_root, &root, &selection).expect("install should work");

        fs::remove_file(root.join("AGENTS.md")).expect("managed agent file should be removable");
        fs::remove_dir_all(root.join("openspec")).expect("spec tree should be removable");

        let refreshed = refresh_selected(&omv_root, &root, &AdapterSelection::default())
            .expect("refresh should read registry and recreate targets");

        assert_eq!(refreshed.installed.len(), 2);
        assert!(root.join("AGENTS.md").exists());
        assert!(root.join("openspec/project.md").exists());

        cleanup_project_root(&root);
    }

    fn temp_omv_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir()
            .join(format!("omv-{prefix}-{stamp}"))
            .join(".omv");
        fs::create_dir_all(&root).expect("temp root should be created");
        root
    }

    fn temp_project_root(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("omv-adapter-{prefix}-{stamp}"));
        fs::create_dir_all(&root).expect("temp project should be created");
        root
    }

    fn cleanup_project_root(root: &Path) {
        let _ = fs::remove_dir_all(root);
    }

    fn cleanup_root(root: &Path) {
        if let Some(parent) = root.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
