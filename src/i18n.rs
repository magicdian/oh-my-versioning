use std::collections::{BTreeMap, BTreeSet};

use crate::core::locale::OperatorLocale;
use crate::errors::{I18nError, OmvError};

pub const OPERATOR_LOCALE_EN_US: &str = "en-US";
pub const OPERATOR_LOCALE_ZH_CN: &str = "zh-CN";

const EN_CATALOG_RAW: &str = include_str!("../resources/i18n/en-US.toml");
const ZH_CATALOG_RAW: &str = include_str!("../resources/i18n/zh-CN.toml");

#[derive(Debug, Clone)]
pub struct Catalog {
    pub locale: OperatorLocale,
    primary: BTreeMap<String, String>,
    fallback: BTreeMap<String, String>,
}

pub fn normalize_operator_locale(input: &str) -> &'static str {
    OperatorLocale::normalize(input)
}

pub fn is_supported_operator_locale(input: &str) -> bool {
    OperatorLocale::is_supported(input)
}

pub fn supported_operator_locales() -> &'static [&'static str] {
    OperatorLocale::supported()
}

pub fn load_catalog(locale: &str) -> Result<Catalog, OmvError> {
    let normalized = normalize_operator_locale(locale);
    let en = parse_catalog(OPERATOR_LOCALE_EN_US, EN_CATALOG_RAW)?;

    if normalized == OPERATOR_LOCALE_ZH_CN {
        let zh = parse_catalog(OPERATOR_LOCALE_ZH_CN, ZH_CATALOG_RAW)?;
        return Ok(Catalog {
            locale: OperatorLocale::ZhCn,
            primary: zh,
            fallback: en,
        });
    }

    Ok(Catalog {
        locale: OperatorLocale::EnUs,
        primary: en.clone(),
        fallback: en,
    })
}

pub fn validate_catalog_key_parity() -> Result<(), OmvError> {
    let en = parse_catalog(OPERATOR_LOCALE_EN_US, EN_CATALOG_RAW)?;
    let zh = parse_catalog(OPERATOR_LOCALE_ZH_CN, ZH_CATALOG_RAW)?;

    let en_keys: BTreeSet<_> = en.keys().cloned().collect();
    let zh_keys: BTreeSet<_> = zh.keys().cloned().collect();

    let missing_in_en: Vec<_> = zh_keys.difference(&en_keys).cloned().collect();
    let missing_in_zh: Vec<_> = en_keys.difference(&zh_keys).cloned().collect();

    if missing_in_en.is_empty() && missing_in_zh.is_empty() {
        return Ok(());
    }

    Err(I18nError::CatalogParity {
        missing_in_en,
        missing_in_zh,
    }
    .into())
}

impl Catalog {
    pub fn t(&self, key: &str) -> String {
        self.primary
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn tf(&self, key: &str, vars: &[&str]) -> String {
        let mut text = self.t(key);

        for chunk in vars.chunks_exact(2) {
            let token = format!("{{{}}}", chunk[0]);
            text = text.replace(&token, chunk[1]);
        }

        text
    }
}

fn parse_catalog(locale: &str, raw: &str) -> Result<BTreeMap<String, String>, OmvError> {
    let mut map = BTreeMap::new();

    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(I18nError::ParseCatalog {
                locale: locale.to_owned(),
                reason: format!("line {} is malformed", index + 1),
            }
            .into());
        };

        map.insert(
            key.trim().to_owned(),
            value.trim().trim_matches('"').to_owned(),
        );
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::core::locale::OperatorLocale;

    use super::{Catalog, OPERATOR_LOCALE_ZH_CN, load_catalog, validate_catalog_key_parity};

    #[test]
    fn catalog_parity_validation_passes_for_embedded_catalogs() {
        assert!(validate_catalog_key_parity().is_ok());
    }

    #[test]
    fn load_catalog_uses_requested_locale() {
        let zh = load_catalog(OPERATOR_LOCALE_ZH_CN).expect("zh catalog should load");
        assert_eq!(zh.locale, OperatorLocale::ZhCn);
        assert_eq!(zh.t("cli.help.title"), "omv - 本地优先版本管理器");
    }

    #[test]
    fn catalog_falls_back_to_english_when_key_missing_in_primary() {
        let mut primary = BTreeMap::new();
        primary.insert("existing.key".to_owned(), "主语言".to_owned());

        let mut fallback = BTreeMap::new();
        fallback.insert("fallback.key".to_owned(), "fallback".to_owned());

        let catalog = Catalog {
            locale: OperatorLocale::ZhCn,
            primary,
            fallback,
        };

        assert_eq!(catalog.t("fallback.key"), "fallback");
    }

    #[test]
    fn catalog_returns_key_when_missing_in_both_catalogs() {
        let catalog = Catalog {
            locale: OperatorLocale::EnUs,
            primary: BTreeMap::new(),
            fallback: BTreeMap::new(),
        };

        assert_eq!(catalog.t("missing.key"), "missing.key");
    }

    #[test]
    fn tf_replaces_named_placeholders() {
        let mut primary = BTreeMap::new();
        primary.insert(
            "cli.bump.success".to_owned(),
            "version {version} from {source}".to_owned(),
        );
        let catalog = Catalog {
            locale: OperatorLocale::EnUs,
            primary,
            fallback: BTreeMap::new(),
        };

        let text = catalog.tf(
            "cli.bump.success",
            &["version", "2604.13.1", "source", "ntp"],
        );
        assert_eq!(text, "version 2604.13.1 from ntp");
    }
}
