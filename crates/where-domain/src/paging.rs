use serde::{Deserialize, Serialize};

pub const DEFAULT_LIMIT: i64 = 50;
pub const MAX_LIMIT: i64 = 500;

/// Raw paging query parameters (`?limit=&offset=`).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct PageRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl PageRequest {
    /// Clamps the request into a bounded window. Never lets a caller ask for
    /// an unbounded result set: the legacy application had 14k methods and tables with
    /// hundreds of thousands of rows.
    pub fn normalized(&self) -> (i64, i64) {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = self.offset.unwrap_or(0).max(0);
        (limit, offset)
    }
}

/// One page of results plus the total row count of the query.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
