# Complex Project Kind Targets Example

This sample shows a generic `.omv/targets.toml` kind-based setup for a project
with documentation, a component manifest, a public C header, and a Rust
workspace. Users choose concrete target `kind` values; `schema_version` is
internal compatibility metadata and is not a user-facing feature gate.

```toml
schema_version = 1

[[targets]]
id = "root-version-file"
kind = "text-scalar"
adapter = "text"
path = "VERSION"
selector = "whole-file"
template = "{version}\n"
mode = "write"

[[targets]]
id = "readme-version-badge"
kind = "regex-replace"
adapter = "markdown"
path = "README.md"
pattern = "version-[0-9]+\\.[0-9]+\\.[0-9]+-blue"
template = "version-{version}-blue"
mode = "write"

[[targets]]
id = "release-notes-version-block"
kind = "markdown-managed-block"
adapter = "markdown"
path = "docs/release.md"
begin_marker = "<!-- OMV:BEGIN version -->"
end_marker = "<!-- OMV:END version -->"
template = "Managed version: {version}"
mode = "write"

[[targets]]
id = "component-manifest"
kind = "yaml-scalar"
adapter = "yaml"
path = "components/example/component.yml"
key = "package.version"
template = "{version}"
mode = "write"

[[targets]]
id = "public-header-version"
kind = "c-header-macro"
adapter = "c-header"
path = "include/example_version.h"
macro = "EXAMPLE_VERSION"
template = "\"{version}\""
mode = "write"

[[targets]]
id = "rust-workspace"
kind = "cargo-workspace"
adapter = "cargo"
root = "tools/example"
members = "all"
version_policy = "same"
version_location = "member-packages"
lockfile = "update"
mode = "write"
```

Run `omv plan --json` to preview changes, `omv sync --check --json` to gate
drift without mutation, and `omv sync` to apply the planned writes.

Current implementation notes:

- `yaml-scalar` supports simple mapping scalar paths such as
  `package.version`. Sequences, anchors, aliases, and block scalars are rejected.
- `cargo-workspace` discovers members from `[workspace].members`, including
  exact paths and one-level `prefix/*` entries.
- `Cargo.lock` updates are narrow and deterministic: OMV updates matching
  workspace package version lines only. It does not run `cargo update`.
