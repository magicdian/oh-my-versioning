# OMV Contract API Snapshots

`current/` is the editable latest protobuf contract used by source builds.
Numbered directories are frozen contract snapshots that released OMV builds
must continue to understand at their contract boundary.

Version rules:

* Additive protobuf-compatible changes may update `current/`.
* Any release that ships a changed `current/` must freeze a numbered snapshot.
* `versions/current/contract.proto` must match the newest frozen snapshot after
  a release or bootstrap.
* Do not reuse protobuf tag numbers or enum numeric values.
* Deleted protobuf fields or enum values must be reserved in future snapshots.
* `CONTRACT_VERSION` must match the newest frozen API version compiled into
  the OMV binary.
* Compatibility domains remain separate: project `.omv/*.toml` schema version,
  protobuf contract version, structured JSON contract version, and AI adapter
  contract version are not interchangeable.

Bootstrap snapshots:

* Version 1 is the original language-native target contract.
* Version 2 is the current runtime contract, including generalized target kinds
  and host integration capability metadata.
