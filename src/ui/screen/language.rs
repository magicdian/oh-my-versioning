#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageScreenModel {
    pub title_key: &'static str,
}

pub fn build_model() -> LanguageScreenModel {
    LanguageScreenModel {
        title_key: "init.language.title",
    }
}
