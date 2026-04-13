# Localization Guidelines

> i18n contracts for CLI and TUI output in `omv`.

---

## Overview

CLI help, command feedback, error messages, and `omv init` TUI text must support
both Chinese and English from V1. The implementation should follow the same
broad pattern used in the referenced `bridgingio` code:

- embedded `en-US` and `zh-CN` catalogs
- locale normalization
- English fallback
- catalog key parity validation

The code must not hardcode operator-facing strings.

Machine-facing surfaces such as JSON envelope keys and canonical `.omv/ai/*`
artifacts may stay stable and English-first when localization would reduce
automation interoperability.

## Scenario: Shared CLI/TUI Catalogs

### 1. Scope / Trigger

- Trigger: any change to CLI help, command output, status messages, popup copy,
  or locale preference loading/saving
- This is cross-layer because one locale preference affects config, CLI output,
  TUI output, tests, and future generated skills/help text

### 2. Signatures

The implementation should converge on a shape equivalent to:

```rust
const OPERATOR_LOCALE_EN_US: &str = "en-US";
const OPERATOR_LOCALE_ZH_CN: &str = "zh-CN";

struct Catalog { /* locale + primary/fallback maps */ }

fn normalize_operator_locale(input: &str) -> &'static str;
fn is_supported_operator_locale(input: &str) -> bool;
fn supported_operator_locales() -> &'static [&'static str];
fn load_catalog(locale: &str) -> Result<Catalog, OmvError>;
fn validate_catalog_key_parity() -> Result<(), OmvError>;

impl Catalog {
    fn t(&self, key: &str) -> String;
    fn tf(&self, key: &str, vars: &[(&str, &str)]) -> String;
}
```

### 3. Contracts

Catalog locations:

```text
resources/i18n/en-US.toml
resources/i18n/zh-CN.toml
```

Config contract:

```toml
locale = "en-US" # or "zh-CN"
```

Rules:

- `locale` is persisted in `.omv/config.toml`
- CLI and TUI must read the same locale preference
- missing keys in the selected locale fall back to `en-US`
- missing keys in both catalogs should return the key text in debug/test paths
  and should fail parity tests in CI
- format variables must use named placeholders such as `{version}`
- raw user-facing text literals in Rust source are forbidden except in tests
  explicitly validating fallback behavior
- JSON field names such as `ok`, `command`, `data`, and `error` are contract
  keys, not localized copy
- `.omv/ai/instructions.md` and adapter source templates are canonical OMV
  contract artifacts, not operator locale surfaces

### 4. Validation & Error Matrix

| Condition | Behavior | Error / Fallback |
| --- | --- | --- |
| locale input is `ZH-cn` | normalize to `zh-CN` | none |
| locale input unsupported | fall back to `en-US` for runtime, but reject persisted invalid config | `ConfigError::InvalidLocale` |
| key missing in `zh-CN` only | fall back to `en-US` | warn-level log |
| key missing in both catalogs | return key text and fail parity test | `I18nError::MissingKey` in validation path |
| catalogs have different key sets | fail parity validation test | `I18nError::CatalogParity` |
| catalog value malformed | fail load | `I18nError::ParseCatalog` |

### 5. Good/Base/Bad Cases

#### Good

- user selects `zh-CN` during init
- `.omv/config.toml` stores `locale = "zh-CN"`
- later `omv bump` and `omv init` screens both render Chinese text

#### Base

- user never changes locale; default remains `en-US`

#### Bad

- one screen uses `catalog.t("menu.root.title")` while another prints
  `"Main Menu"` directly

### 6. Tests Required

- locale normalization tests for `en-US` and `zh-CN`
- parity test ensuring both catalogs expose identical keys
- fallback test where `zh-CN` omits a key and `en-US` supplies it
- config round-trip test preserving locale preference
- command/output snapshot tests for at least one English and one Chinese path
- tests proving JSON output keys stay stable across locales

Assertion points:

- no unsupported locale is persisted
- both CLI help and TUI status rendering pull from the same catalog API
- placeholder replacement works for localized templates
- structured JSON does not translate contract keys

### 7. Wrong vs Correct

#### Wrong

```rust
println!("Version bumped to {}", version);
```

#### Correct

```rust
println!(
    "{}",
    catalog.tf("cli.bump.success", &[("version", version.as_str())])
);
```

## Project Convention

### Convention: Shared key namespace

Use stable dotted keys grouped by feature, for example:

- `cli.help.title`
- `cli.bump.success`
- `init.language.detected`
- `menu.footer.exit`

Do not create ad hoc keys that only differ by locale file order.
