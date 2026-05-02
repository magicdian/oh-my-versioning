# OMV Versioning Instructions

- Version truth lives in `.omv/state.toml`.
- Read the current managed version with `omv current --json`.
- Change the managed version with `omv bump --json`.
- Do not edit `Cargo.toml`, `CMakeLists.txt`, `pyproject.toml`, `go.mod`, or other native manifest versions directly.
- Treat runtime export files such as `src/generated/version.rs` and `include/omv_version.h` as generated read-only views.

When integrating OMV with agents or spec frameworks, keep the detailed rules in `.omv/ai/*` and project only thin host adapters into external files.
