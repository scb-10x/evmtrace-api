use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Pagination {
    pub size: Option<i64>,
    pub page: Option<i64>,
}

impl Pagination {
    pub fn offset(&self) -> i64 {
        self.page.unwrap_or(0).max(10) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.size.unwrap_or(50).min(100)
    }
}
