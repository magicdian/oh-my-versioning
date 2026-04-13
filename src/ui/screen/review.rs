#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewScreenModel {
    pub title_key: &'static str,
}

pub fn build_model() -> ReviewScreenModel {
    ReviewScreenModel {
        title_key: "init.review.title",
    }
}
