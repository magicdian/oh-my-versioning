#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterModel {
    pub help_key: &'static str,
}

impl Default for FooterModel {
    fn default() -> Self {
        Self {
            help_key: "menu.footer.help",
        }
    }
}
