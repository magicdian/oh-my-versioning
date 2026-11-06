# Spec Delta

## Target files
- .phronia/spec/backend/index.md

## Intended changes

Insert two new bullets immediately after the existing bullet that begins
`- \\\`release_mode\\\` in \\\`.omv/config.toml\\\` defaults to \\\`local\\\`.`:

```markdown
- **Config compatibility is declared by the file, not inferred by the binary.**
  `load_config` reads `requires_omv` before parsing any typed field and fails
  with `unsupported_config_capability` when the declared minimum exceeds
  `OMV_VERSION`. A gate compiled into the binary cannot protect an older binary:
  that binary has no arm for the newer value, falls back to the enum default, and
  skips the guard — which is how `omv@2610.27.2` accepted local version writes in
  a `release_mode = "pipeline"` project. The declaration covers the window from
  the release that introduced it onward; binaries predating the key ignore it.
  Gated enums implement `ConfigCapability::minimum_omv` (an exhaustive match, so a
  new variant cannot silently omit its release) and `required_omv_for(&OmvConfig)`
  takes the strictest requirement across fields — one number, because a config is
  read as a whole. `save_config` writes it on `omv init`, and
  `storage::config::reconcile_requirement` backfills it on `omv integrate apply`
  so projects created earlier acquire the declaration. `load_config` rejects an
  understated declaration with `config_declaration_mismatch` and an unparsable
  one with `malformed_config_requirement` — the key decides whether a config may
  be read at all, so it must fail closed. An absent `requires_omv` stays
  compatible.
- Version strings (`2611.2.1`) are date-derived, not semver, and compare as
  zero-padded integer tuples (`capability::compare_versions`). Do not sort them
  lexicographically.
```

## Rationale

- The backend spec is the layer contract for `src/storage` and `src/core`; the
  config-compatibility rule now governs how every future gated config value is
  introduced, so it belongs next to the existing `release_mode` bullet.
- The `SPEC_IN_CODE_TASK` rule forbids committing `.phronia/spec/**` from a code
  task. Earlier commits on this task violated that; they are being reverted so
  this delta is the single record until the post-integration spec sync.

## Verification

- `grep -n "Config compatibility is declared by the file" .phronia/spec/backend/index.md`
  after spec sync.
- `cargo test` / `cargo clippy --all-targets -- -D warnings` are unaffected by the
  spec text itself.
