use regex::Regex;

use crate::core::schema::{OmvV2TargetConfig, YamlScalarTarget};
use crate::core::target::TargetKind;
use crate::errors::{OmvError, TargetError};
use crate::sync::{
    PlanOperation, PlanStatus, PlanTargetResult, V2SyncContext, V2TargetSyncAdapter, planned_write,
    project_relative_path, read_text_if_exists, resolve_target_path,
};

#[derive(Debug, Default)]
pub struct TextScalarAdapter;

#[derive(Debug, Default)]
pub struct RegexReplaceAdapter;

#[derive(Debug, Default)]
pub struct MarkdownManagedBlockAdapter;

#[derive(Debug, Default)]
pub struct YamlScalarAdapter;

#[derive(Debug, Default)]
pub struct CHeaderMacroAdapter;

impl V2TargetSyncAdapter for TextScalarAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::TextScalar
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::TextScalar(config) = &context.target.config else {
            return invalid_config(context.target.id.as_str(), self.kind());
        };
        let path = resolve_v2_path(context, config.path.as_str());
        let expected = render_template(config.template.as_str(), context.version);
        let current = read_text_if_exists(&path)?;
        let status = scalar_status(current.as_deref(), expected.as_str());
        let operations = vec![planned_write(
            context.project_root,
            &path,
            expected.clone(),
            "write scalar text value from .omv version truth",
        )];

        Ok(v2_result(
            context,
            vec![project_relative_path(context.project_root, &path)],
            current
                .as_deref()
                .map(summarize_scalar)
                .unwrap_or_else(|| String::from("missing")),
            summarize_scalar(expected.as_str()),
            status,
            operations,
            diagnostics_for_status(status),
        ))
    }
}

impl V2TargetSyncAdapter for RegexReplaceAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::RegexReplace
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::RegexReplace(config) = &context.target.config else {
            return invalid_config(context.target.id.as_str(), self.kind());
        };
        let path = resolve_v2_path(context, config.path.as_str());
        let Some(current) = read_text_if_exists(&path)? else {
            return Ok(v2_result(
                context,
                vec![project_relative_path(context.project_root, &path)],
                String::from("missing"),
                render_template(config.template.as_str(), context.version),
                PlanStatus::Missing,
                Vec::new(),
                vec![String::from("regex target file is missing")],
            ));
        };
        let regex = Regex::new(config.pattern.as_str()).map_err(|err| {
            TargetError::InvalidTargetRecord(format!(
                "target {}: invalid regex pattern: {err}",
                context.target.id
            ))
        })?;
        let matches = regex.find_iter(current.as_str()).count();
        if matches == 0 {
            return Err(TargetError::InvalidTargetRecord(format!(
                "target {}: regex pattern matched zero ranges",
                context.target.id
            ))
            .into());
        }
        if matches > 1 && !config.allow_multiple {
            return Err(TargetError::InvalidTargetRecord(format!(
                "target {}: regex pattern matched {matches} ranges; set allow_multiple = true to replace all",
                context.target.id
            ))
            .into());
        }

        let replacement = render_template(config.template.as_str(), context.version);
        let expected = if config.allow_multiple {
            regex
                .replace_all(current.as_str(), replacement.as_str())
                .into_owned()
        } else {
            regex
                .replace(current.as_str(), replacement.as_str())
                .into_owned()
        };
        let status = if expected == current {
            PlanStatus::Ok
        } else {
            PlanStatus::Drift
        };

        Ok(v2_result(
            context,
            vec![project_relative_path(context.project_root, &path)],
            format!("{matches} match(es)"),
            replacement,
            status,
            vec![planned_write(
                context.project_root,
                &path,
                expected,
                "replace configured regex match from .omv version truth",
            )],
            diagnostics_for_status(status),
        ))
    }
}

impl V2TargetSyncAdapter for MarkdownManagedBlockAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::MarkdownManagedBlock
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::MarkdownManagedBlock(config) = &context.target.config else {
            return invalid_config(context.target.id.as_str(), self.kind());
        };
        let path = resolve_v2_path(context, config.path.as_str());
        let Some(current) = read_text_if_exists(&path)? else {
            return Ok(v2_result(
                context,
                vec![project_relative_path(context.project_root, &path)],
                String::from("missing"),
                String::from("managed block present"),
                PlanStatus::Missing,
                Vec::new(),
                vec![String::from("markdown managed-block file is missing")],
            ));
        };
        let expected_block = render_template(config.template.as_str(), context.version);
        let expected = replace_managed_block(
            current.as_str(),
            config.begin_marker.as_str(),
            config.end_marker.as_str(),
            expected_block.as_str(),
            context.target.id.as_str(),
        )?;
        let status = if expected == current {
            PlanStatus::Ok
        } else {
            PlanStatus::Drift
        };

        Ok(v2_result(
            context,
            vec![project_relative_path(context.project_root, &path)],
            String::from("managed block present"),
            String::from("managed block updated"),
            status,
            vec![planned_write(
                context.project_root,
                &path,
                expected,
                "replace markdown managed block from .omv version truth",
            )],
            diagnostics_for_status(status),
        ))
    }
}

impl V2TargetSyncAdapter for YamlScalarAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::YamlScalar
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::YamlScalar(config) = &context.target.config else {
            return invalid_config(context.target.id.as_str(), self.kind());
        };
        let path = resolve_v2_path(context, config.path.as_str());
        let Some(current) = read_text_if_exists(&path)? else {
            return Ok(v2_result(
                context,
                vec![project_relative_path(context.project_root, &path)],
                String::from("missing"),
                render_template(config.template.as_str(), context.version),
                PlanStatus::Missing,
                Vec::new(),
                vec![String::from("yaml scalar file is missing")],
            ));
        };
        let (expected, current_value) = replace_yaml_scalar(
            current.as_str(),
            config,
            context.version,
            context.target.id.as_str(),
        )?;
        let status = if expected == current {
            PlanStatus::Ok
        } else {
            PlanStatus::Drift
        };

        Ok(v2_result(
            context,
            vec![project_relative_path(context.project_root, &path)],
            summarize_scalar(current_value.as_str()),
            summarize_scalar(render_template(config.template.as_str(), context.version).as_str()),
            status,
            vec![planned_write(
                context.project_root,
                &path,
                expected,
                "write YAML scalar from .omv version truth",
            )],
            diagnostics_for_status(status),
        ))
    }
}

impl V2TargetSyncAdapter for CHeaderMacroAdapter {
    fn kind(&self) -> TargetKind {
        TargetKind::CHeaderMacro
    }

    fn plan(&self, context: &V2SyncContext<'_>) -> Result<PlanTargetResult, OmvError> {
        let OmvV2TargetConfig::CHeaderMacro(config) = &context.target.config else {
            return invalid_config(context.target.id.as_str(), self.kind());
        };
        let path = resolve_v2_path(context, config.path.as_str());
        let Some(current) = read_text_if_exists(&path)? else {
            return Ok(v2_result(
                context,
                vec![project_relative_path(context.project_root, &path)],
                String::from("missing"),
                render_template(config.template.as_str(), context.version),
                PlanStatus::Missing,
                Vec::new(),
                vec![String::from("C header macro file is missing")],
            ));
        };
        let (expected, current_value) = replace_c_header_macro(
            current.as_str(),
            config.macro_name.as_str(),
            render_template(config.template.as_str(), context.version).as_str(),
            context.target.id.as_str(),
        )?;
        let status = if expected == current {
            PlanStatus::Ok
        } else {
            PlanStatus::Drift
        };

        Ok(v2_result(
            context,
            vec![project_relative_path(context.project_root, &path)],
            summarize_scalar(current_value.as_str()),
            summarize_scalar(render_template(config.template.as_str(), context.version).as_str()),
            status,
            vec![planned_write(
                context.project_root,
                &path,
                expected,
                "write C header macro from .omv version truth",
            )],
            diagnostics_for_status(status),
        ))
    }
}

fn resolve_v2_path(context: &V2SyncContext<'_>, relative_path: &str) -> std::path::PathBuf {
    resolve_target_path(
        context.project_root,
        context.target.root.as_str(),
        relative_path,
    )
}

fn v2_result(
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
        language: String::new(),
        paths,
        current_value_summary,
        expected_value_summary,
        status,
        operations,
        diagnostics,
        required: true,
    }
}

fn invalid_config<T>(target_id: &str, kind: TargetKind) -> Result<T, OmvError> {
    Err(TargetError::InvalidTargetRecord(format!(
        "target {target_id}: config does not match kind {}",
        kind.as_str()
    ))
    .into())
}

fn render_template(template: &str, version: &str) -> String {
    template.replace("{version}", version)
}

fn scalar_status(current: Option<&str>, expected: &str) -> PlanStatus {
    match current {
        None => PlanStatus::Missing,
        Some(current) if current == expected => PlanStatus::Ok,
        Some(_) => PlanStatus::Drift,
    }
}

fn summarize_scalar(value: &str) -> String {
    let first_line = value.lines().next().unwrap_or_default().trim();
    let mut summary: String = first_line.chars().take(80).collect();
    if first_line.chars().count() > 80 {
        summary.push_str("...");
    }
    if summary.is_empty() {
        String::from("<empty>")
    } else {
        summary
    }
}

fn diagnostics_for_status(status: PlanStatus) -> Vec<String> {
    match status {
        PlanStatus::Drift => vec![String::from(
            "target content differs from .omv version truth",
        )],
        PlanStatus::Missing => vec![String::from("target output is missing")],
        _ => Vec::new(),
    }
}

fn replace_managed_block(
    content: &str,
    begin_marker: &str,
    end_marker: &str,
    replacement: &str,
    target_id: &str,
) -> Result<String, OmvError> {
    let begin_matches: Vec<_> = content.match_indices(begin_marker).collect();
    let end_matches: Vec<_> = content.match_indices(end_marker).collect();
    if begin_matches.len() != 1 || end_matches.len() != 1 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: markdown markers must appear exactly once"
        ))
        .into());
    }
    let begin = begin_matches[0].0;
    let end = end_matches[0].0;
    if begin >= end {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: markdown managed-block markers are inverted"
        ))
        .into());
    }

    let begin_line_end = content[begin..]
        .find('\n')
        .map(|offset| begin + offset + 1)
        .unwrap_or(content.len());
    let end_line_start = content[..end]
        .rfind('\n')
        .map(|offset| offset + 1)
        .unwrap_or(0);

    let mut output = String::new();
    output.push_str(&content[..begin_line_end]);
    output.push_str(replacement);
    if !replacement.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(&content[end_line_start..]);
    Ok(output)
}

fn replace_yaml_scalar(
    content: &str,
    config: &YamlScalarTarget,
    version: &str,
    target_id: &str,
) -> Result<(String, String), OmvError> {
    let expected = render_template(config.template.as_str(), version);
    let wanted: Vec<&str> = config
        .key
        .split('.')
        .filter(|part| !part.is_empty())
        .collect();
    if wanted.is_empty() {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: yaml key path cannot be empty"
        ))
        .into());
    }

    let mut output = Vec::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut matches = 0usize;
    let mut current_value = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            output.push(line.to_owned());
            continue;
        }
        if trimmed.starts_with('-') || trimmed.contains('&') || trimmed.contains('*') {
            return Err(TargetError::InvalidTargetRecord(format!(
                "target {target_id}: yaml-scalar supports simple mapping scalars only"
            ))
            .into());
        }

        let indent = line.chars().take_while(|ch| *ch == ' ').count();
        while stack
            .last()
            .map(|(stack_indent, _)| *stack_indent >= indent)
            .unwrap_or(false)
        {
            stack.pop();
        }

        let Some((raw_key, raw_value)) = trimmed.split_once(':') else {
            output.push(line.to_owned());
            continue;
        };
        let key = raw_key
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_owned();
        let value = raw_value.trim();
        stack.push((indent, key));

        let path: Vec<&str> = stack.iter().map(|(_, key)| key.as_str()).collect();
        if path == wanted {
            if value.starts_with('|') || value.starts_with('>') || value.is_empty() {
                return Err(TargetError::InvalidTargetRecord(format!(
                    "target {target_id}: yaml-scalar only updates existing scalar values"
                ))
                .into());
            }
            matches += 1;
            current_value = value.to_owned();
            output.push(format!(
                "{}{}: {}",
                " ".repeat(indent),
                raw_key.trim(),
                expected
            ));
        } else {
            output.push(line.to_owned());
        }
    }

    if matches == 0 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: yaml key path {} was not found",
            config.key
        ))
        .into());
    }
    if matches > 1 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: yaml key path {} matched multiple scalars",
            config.key
        ))
        .into());
    }

    let mut rendered = output.join("\n");
    if content.ends_with('\n') {
        rendered.push('\n');
    }
    Ok((rendered, current_value))
}

fn replace_c_header_macro(
    content: &str,
    macro_name: &str,
    expected_value: &str,
    target_id: &str,
) -> Result<(String, String), OmvError> {
    let mut output = Vec::new();
    let mut matches = 0usize;
    let mut current_value = String::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#define") {
            let mut parts = rest.split_whitespace();
            if parts.next() == Some(macro_name) {
                matches += 1;
                current_value = parts.collect::<Vec<_>>().join(" ");
                let indent = &line[..line.len() - trimmed.len()];
                output.push(format!("{indent}#define {macro_name} {expected_value}"));
                continue;
            }
        }
        output.push(line.to_owned());
    }

    if matches == 0 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: C header macro {macro_name} was not found"
        ))
        .into());
    }
    if matches > 1 {
        return Err(TargetError::InvalidTargetRecord(format!(
            "target {target_id}: C header macro {macro_name} is duplicated"
        ))
        .into());
    }

    let mut rendered = output.join("\n");
    if content.ends_with('\n') {
        rendered.push('\n');
    }
    Ok((rendered, current_value))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::core::schema::{
        CHeaderMacroTarget, MarkdownManagedBlockTarget, OmvV2TargetConfig, OmvV2TargetRecord,
        RegexReplaceTarget, TextScalarTarget, YamlScalarTarget,
    };
    use crate::core::target::{TargetKind, TargetMode};
    use crate::sync::{PlanStatus, V2SyncContext, V2TargetSyncAdapter};

    use super::{
        CHeaderMacroAdapter, MarkdownManagedBlockAdapter, RegexReplaceAdapter, TextScalarAdapter,
        YamlScalarAdapter,
    };

    #[test]
    fn text_scalar_plans_missing_file_write() {
        let root = temp_root("text-scalar");
        let target = target(
            TargetKind::TextScalar,
            OmvV2TargetConfig::TextScalar(TextScalarTarget {
                path: "VERSION".to_owned(),
                selector: "whole-file".to_owned(),
                template: "{version}\n".to_owned(),
            }),
        );
        let plan = TextScalarAdapter
            .plan(&context(&root, &target, "2605.1.1"))
            .expect("text scalar should plan");
        assert_eq!(plan.status, PlanStatus::Missing);
        assert_eq!(plan.operations[0].content, "2605.1.1\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn regex_replace_rejects_ambiguous_matches() {
        let root = temp_root("regex-ambiguous");
        fs::write(
            root.join("README.md"),
            "version-1.0.0-blue version-1.0.1-blue\n",
        )
        .expect("fixture should write");
        let target = target(
            TargetKind::RegexReplace,
            OmvV2TargetConfig::RegexReplace(RegexReplaceTarget {
                path: "README.md".to_owned(),
                pattern: "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue".to_owned(),
                template: "version-{version}-blue".to_owned(),
                allow_multiple: false,
            }),
        );
        let err = RegexReplaceAdapter
            .plan(&context(&root, &target, "2605.1.1"))
            .expect_err("ambiguous regex should fail");
        assert_eq!(err.code(), "invalid_target_record");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn markdown_managed_block_replaces_only_marked_content() {
        let root = temp_root("markdown-block");
        fs::write(
            root.join("README.md"),
            "before\n<!-- BEGIN -->\nold\n<!-- END -->\nafter\n",
        )
        .expect("fixture should write");
        let target = target(
            TargetKind::MarkdownManagedBlock,
            OmvV2TargetConfig::MarkdownManagedBlock(MarkdownManagedBlockTarget {
                path: "README.md".to_owned(),
                begin_marker: "<!-- BEGIN -->".to_owned(),
                end_marker: "<!-- END -->".to_owned(),
                template: "version {version}".to_owned(),
            }),
        );
        let plan = MarkdownManagedBlockAdapter
            .plan(&context(&root, &target, "2605.1.1"))
            .expect("markdown block should plan");
        assert_eq!(plan.status, PlanStatus::Drift);
        assert!(
            plan.operations[0]
                .content
                .contains("before\n<!-- BEGIN -->\nversion 2605.1.1\n<!-- END -->\nafter\n")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn yaml_scalar_updates_nested_scalar() {
        let root = temp_root("yaml-scalar");
        fs::write(root.join("component.yml"), "package:\n  version: 0.1.0\n")
            .expect("fixture should write");
        let target = target(
            TargetKind::YamlScalar,
            OmvV2TargetConfig::YamlScalar(YamlScalarTarget {
                path: "component.yml".to_owned(),
                key: "package.version".to_owned(),
                template: "{version}".to_owned(),
            }),
        );
        let plan = YamlScalarAdapter
            .plan(&context(&root, &target, "2605.1.1"))
            .expect("yaml scalar should plan");
        assert!(plan.operations[0].content.contains("version: 2605.1.1"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn c_header_macro_rejects_duplicate_macro() {
        let root = temp_root("c-header-duplicate");
        fs::write(
            root.join("version.h"),
            "#define EXAMPLE_VERSION \"1\"\n#define EXAMPLE_VERSION \"2\"\n",
        )
        .expect("fixture should write");
        let target = target(
            TargetKind::CHeaderMacro,
            OmvV2TargetConfig::CHeaderMacro(CHeaderMacroTarget {
                path: "version.h".to_owned(),
                macro_name: "EXAMPLE_VERSION".to_owned(),
                template: "\"{version}\"".to_owned(),
            }),
        );
        let err = CHeaderMacroAdapter
            .plan(&context(&root, &target, "2605.1.1"))
            .expect_err("duplicate macro should fail");
        assert_eq!(err.code(), "invalid_target_record");
        let _ = fs::remove_dir_all(root);
    }

    fn target(kind: TargetKind, config: OmvV2TargetConfig) -> OmvV2TargetRecord {
        OmvV2TargetRecord {
            id: "target".to_owned(),
            kind,
            adapter: "test".to_owned(),
            root: ".".to_owned(),
            enabled: true,
            mode: TargetMode::Write,
            config,
        }
    }

    fn context<'a>(
        root: &'a std::path::Path,
        target: &'a OmvV2TargetRecord,
        version: &'a str,
    ) -> V2SyncContext<'a> {
        V2SyncContext {
            project_root: root,
            target,
            version,
        }
    }

    fn temp_root(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("omv-{prefix}-{stamp}"));
        fs::create_dir_all(&root).expect("root should be created");
        root
    }
}
