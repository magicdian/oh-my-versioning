pub mod capability;
pub mod engine;

use crate::core::schema::OmvConfig;
use crate::generated::version::OMV_VERSION;

use capability::{ConfigCapability, UnsupportedCapability};

/// First release that understands `release_mode = "pipeline"`.
///
/// Kept as published documentation of the value's introduction. It is *not* used
/// to decide whether a config is readable: a binary older than this constant has
/// no such constant at all, so gating on it would only ever protect binaries that
/// already understand the value. See [`capability`] for the mechanism that does
/// work -- the config declaring `requires_omv` itself.
pub const RELEASE_MODE_PIPELINE_SINCE: &str = "2611.2.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReleaseMode {
    #[default]
    Local,
    Pipeline,
}

impl ReleaseMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Pipeline => "pipeline",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "local" => Some(Self::Local),
            "pipeline" => Some(Self::Pipeline),
            _ => None,
        }
    }
}

impl ConfigCapability for ReleaseMode {
    fn minimum_omv(&self) -> Option<&'static str> {
        match self {
            Self::Local => None,
            Self::Pipeline => Some(RELEASE_MODE_PIPELINE_SINCE),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildPolicy {
    #[default]
    DailyReset,
    Continuous,
}

impl BuildPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DailyReset => "daily-reset",
            Self::Continuous => "continuous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "daily-reset" => Some(Self::DailyReset),
            "continuous" => Some(Self::Continuous),
            _ => None,
        }
    }
}

impl ConfigCapability for BuildPolicy {
    fn minimum_omv(&self) -> Option<&'static str> {
        match self {
            Self::DailyReset | Self::Continuous => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VersionOutput {
    #[default]
    DateTriplet,
    Semver,
}

impl VersionOutput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DateTriplet => "date-triplet",
            Self::Semver => "semver",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "date-triplet" => Some(Self::DateTriplet),
            "semver" => Some(Self::Semver),
            _ => None,
        }
    }
}

impl ConfigCapability for VersionOutput {
    fn minimum_omv(&self) -> Option<&'static str> {
        match self {
            Self::DateTriplet | Self::Semver => None,
        }
    }
}

/// Outcome of checking a config file against the running binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// This binary is new enough for the config.
    Supported,
    /// The config declares a minimum release this binary predates.
    UpgradeRequired(UnsupportedCapability),
}

/// Checks the `requires_omv` declaration in a config file.
///
/// `declared` is the raw value of `requires_omv`, if the file carries one. An
/// absent declaration is treated as compatible so every config written before
/// this key existed keeps loading. `culprit` names the key and value responsible
/// for the requirement, used only to make the error actionable.
pub fn gate_declared_requirement(
    culprit: Option<(&'static str, &str)>,
    declared: Option<&str>,
) -> GateOutcome {
    let Some(required) = declared.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return GateOutcome::Supported;
    };

    if capability::requires_upgrade(OMV_VERSION, required) {
        let (field, value) = culprit.unwrap_or((REQUIRE_RELEASE_KEY, ""));
        return GateOutcome::UpgradeRequired(capability::unsupported(
            field,
            Some(value),
            required,
            OMV_VERSION,
        ));
    }

    GateOutcome::Supported
}

/// Config key declaring the minimum `omv` release that can honour the file.
pub const REQUIRE_RELEASE_KEY: &str = "requires_omv";

/// The `requires_omv` value a project should declare for its current config.
///
/// Reads the strictest requirement across every gated field. A config file is
/// read as a whole -- a release either understands all of it or none of it -- so
/// the hardest requirement is the only one a reader needs.
///
/// The field list is maintained by hand, so a newly gated field can be missed.
/// `load_config` compares the written declaration against this function to catch
/// that, but only for fields this binary knows about: a field gated by a *future*
/// release is invisible until that release ships.
pub fn required_omv_for(config: &OmvConfig) -> &'static str {
    let strictest = capability::strongest_requirement(
        gated_values(config).map(|(_, _, requirement)| requirement),
    );

    strictest.unwrap_or(capability::BASELINE_VERSION)
}

/// Every gated `(key, value, minimum_omv)` triple in a config.
fn gated_values(
    config: &OmvConfig,
) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> {
    [
        (
            "release_mode",
            config.release_mode.as_str(),
            config.release_mode.minimum_omv(),
        ),
        (
            "build_policy",
            config.build_policy.as_str(),
            config.build_policy.minimum_omv(),
        ),
        (
            "version_output",
            config.version_output.as_str(),
            config.version_output.minimum_omv(),
        ),
    ]
    .into_iter()
}

#[cfg(test)]
mod tests {
    use crate::core::schema::OmvConfig;

    use super::{GateOutcome, ReleaseMode, gate_declared_requirement, required_omv_for};

    #[test]
    fn absent_declaration_keeps_legacy_configs_readable() {
        assert_eq!(
            gate_declared_requirement(Some(("release_mode", "pipeline")), None),
            GateOutcome::Supported
        );
    }

    #[test]
    fn blank_declaration_counts_as_absent() {
        assert_eq!(
            gate_declared_requirement(None, Some("   ")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn requirement_at_or_below_this_binary_passes() {
        // This binary is 2611.2.1; requiring that or anything older is fine.
        assert_eq!(
            gate_declared_requirement(None, Some(super::OMV_VERSION)),
            GateOutcome::Supported
        );
        assert_eq!(
            gate_declared_requirement(None, Some("2610.27.2")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn requirement_above_this_binary_asks_for_an_upgrade() {
        match gate_declared_requirement(Some(("release_mode", "pipeline")), Some("9999.1.1")) {
            GateOutcome::UpgradeRequired(detail) => {
                assert_eq!(detail.field, "release_mode");
                assert_eq!(detail.value.as_deref(), Some("pipeline"));
                assert_eq!(detail.required_version, "9999.1.1");
                assert_eq!(detail.current_version, super::OMV_VERSION);
            }
            other => panic!("expected upgrade prompt, got {other:?}"),
        }
    }

    #[test]
    fn upgrade_without_a_named_culprit_points_at_the_declaration() {
        match gate_declared_requirement(None, Some("9999.1.1")) {
            GateOutcome::UpgradeRequired(detail) => {
                assert_eq!(detail.field, super::REQUIRE_RELEASE_KEY);
            }
            other => panic!("expected upgrade prompt, got {other:?}"),
        }
    }

    #[test]
    fn malformed_requirement_is_not_an_upgrade_prompt() {
        // The gate compares release numbers; a value it cannot read is not a
        // version request. `load_config` rejects it separately as
        // `malformed_config_requirement` so the failure cannot fail open here.
        assert_eq!(
            gate_declared_requirement(None, Some("newest")),
            GateOutcome::Supported
        );
    }

    #[test]
    fn pipeline_projects_declare_the_release_that_added_pipeline() {
        let config = OmvConfig {
            release_mode: ReleaseMode::Pipeline,
            ..OmvConfig::default()
        };
        assert_eq!(required_omv_for(&config), "2611.2.1");
    }

    #[test]
    fn local_projects_declare_only_the_baseline() {
        let config = OmvConfig {
            release_mode: ReleaseMode::Local,
            ..OmvConfig::default()
        };
        assert_eq!(
            required_omv_for(&config),
            super::capability::BASELINE_VERSION
        );
    }

    #[test]
    fn ungated_fields_never_raise_the_declaration() {
        // Neither field has introduced a versioned value, so the strictest
        // requirement still comes solely from release_mode.
        let config = OmvConfig {
            release_mode: ReleaseMode::Local,
            build_policy: super::BuildPolicy::Continuous,
            version_output: super::VersionOutput::Semver,
            ..OmvConfig::default()
        };
        assert_eq!(
            required_omv_for(&config),
            super::capability::BASELINE_VERSION
        );
    }

    #[test]
    fn strongest_requirement_picks_the_newest_release() {
        let picked = super::capability::strongest_requirement([
            None,
            Some("2611.2.1"),
            Some("2612.1.1"),
            Some("2610.27.2"),
        ]);
        assert_eq!(picked, Some("2612.1.1"));
    }

    #[test]
    fn strongest_requirement_of_nothing_is_none() {
        assert_eq!(super::capability::strongest_requirement([None, None]), None);
    }
}
