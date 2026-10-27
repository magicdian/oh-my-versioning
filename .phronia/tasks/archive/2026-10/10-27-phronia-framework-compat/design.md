# Design

## Provider model

`IntegrationProvider::Phronia` is a `SpecWorkflowHost` with the same capability
shape as Trellis:

- `spec-guide` → dedicated file under `.phronia/spec/guides/`
- `spec-index-snippet` → managed block in `.phronia/spec/guides/index.md`
- `finalize-boundary` → verification-only block in Phronia finish-work surfaces

`SpecAdapter::Phronia` keeps the legacy `omv adapter` compatibility surface.

## Finalize-boundary

Phronia commits in workflow Phase 3.4 before `/phronia:finish-work`, so the block
must not bump again. Unlike the Trellis agent-gated target set, Phronia scans
every existing finish-work surface because those files are owned by the Phronia
installation. This also covers platforms OMV does not model as agent providers
(for example ZCode). At least one surface must exist.

## Coexistence policy

New helper: `detect_spec_framework_conflict(project_root)` returns both host
roots when `.trellis` and `.phronia` exist. OMV never picks a winner and never
mutates framework directories or adapter records to resolve the ambiguity.

- `integrate status`: report `spec-framework-conflict` per capability (read-only).
- `integrate apply`: fail the affected capabilities with the same reason code.
- `adapter install --spec trellis|phronia`: fail with `spec_framework_conflict`.
- `adapter refresh`: same guard; a stale record whose own host tree is missing
  fails with `unsupported_adapter_operation` instead of recreating the host.
- `init`: fail before writing config/targets when a workflow provider is
  selected and both host trees exist.

## Sync line endings

`text_matches` compares after normalizing `\r\n` to `\n`. All writes go through
`write_text`, which normalizes to LF. A line-ending-only difference is `Ok` (no
rewrite); a real version change rewrites the generated file with LF.

## Compatibility

- Trellis behavior, including v0.4/v0.5 finish-work path resolution, is unchanged.
- OpenSpec remains a bootstrap-capable spec adapter (host-directory requirement
  applies only to Trellis/Phronia).
