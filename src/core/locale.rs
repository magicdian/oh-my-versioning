#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperatorLocale {
    #[default]
    EnUs,
    ZhCn,
}

impl OperatorLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    pub fn normalize(input: &str) -> &'static str {
        match Self::normalized_token(input).as_str() {
            "en-us" => "en-US",
            "zh-cn" => "zh-CN",
            _ => "en-US",
        }
    }

    pub fn from_input(input: &str) -> Self {
        match Self::normalize(input) {
            "zh-CN" => Self::ZhCn,
            _ => Self::EnUs,
        }
    }

    pub fn is_supported(input: &str) -> bool {
        matches!(Self::normalized_token(input).as_str(), "en-us" | "zh-cn")
    }

    pub fn supported() -> &'static [&'static str] {
        &["en-US", "zh-CN"]
    }

    fn normalized_token(input: &str) -> String {
        input.trim().to_ascii_lowercase().replace('_', "-")
    }
}

#[cfg(test)]
mod tests {
    use super::OperatorLocale;

    #[test]
    fn normalize_accepts_supported_variants() {
        assert_eq!(OperatorLocale::normalize("en-US"), "en-US");
        assert_eq!(OperatorLocale::normalize("EN_us"), "en-US");
        assert_eq!(OperatorLocale::normalize("zh-CN"), "zh-CN");
        assert_eq!(OperatorLocale::normalize("ZH_cn"), "zh-CN");
    }

    #[test]
    fn normalize_falls_back_for_unknown_locale() {
        assert_eq!(OperatorLocale::normalize("fr-FR"), "en-US");
    }

    #[test]
    fn supported_check_rejects_unknown_locale() {
        assert!(OperatorLocale::is_supported("en-US"));
        assert!(OperatorLocale::is_supported("zh-CN"));
        assert!(!OperatorLocale::is_supported("fr-FR"));
    }
}
