#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRootViewModel {
    pub title_key: &'static str,
}

pub fn build_model() -> InitRootViewModel {
    InitRootViewModel {
        title_key: "init.root.title",
    }
}
