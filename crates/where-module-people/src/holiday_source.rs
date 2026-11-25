//! The online source of the holidays.
//!
//! Both lists come from the OpenHolidays API, which needs no key and no
//! address the operator would have to type in. The URLs are built here; a
//! failed request becomes [`DomainError::Upstream`] and the caller decides
//! whether the import continues with the other list.

use std::time::Duration;

use reqwest::Client;
use where_domain::DomainError;

use crate::holidays::{self, Holiday, SchoolHoliday};

/// Where the two lists live.
const BASE: &str = "https://openholidaysapi.org";

/// Germany, that is all the calendar is about.
const COUNTRY: &str = "DE";

/// Longest wait for one answer.
const TIMEOUT_SECONDS: u64 = 20;

/// Reads the holidays of one year from OpenHolidays.
pub struct HolidaySource {
    client: Client,
}

impl HolidaySource {
    pub fn new() -> Result<Self, DomainError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECONDS))
            .user_agent(concat!("WHERE/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| DomainError::Upstream(format!("HTTP client: {error}")))?;
        Ok(Self { client })
    }

    /// The public holidays of `year`. With a `state` the answer also carries the
    /// regional holidays of that federal state.
    pub async fn public(
        &self,
        year: i32,
        state: Option<&str>,
    ) -> Result<Vec<Holiday>, DomainError> {
        let url = format!(
            "{BASE}/PublicHolidays?countryIsoCode={COUNTRY}&languageIsoCode=DE\
             &validFrom={year}-01-01&validTo={year}-12-31{}",
            match state {
                Some(code) => format!("&subdivisionCode={COUNTRY}-{code}"),
                None => String::new(),
            }
        );
        let body = self.get(&url).await?;
        holidays::parse_public(&body, state)
    }

    /// The school holidays of `year` in `state`.
    pub async fn school(&self, year: i32, state: &str) -> Result<Vec<SchoolHoliday>, DomainError> {
        let url = format!(
            "{BASE}/SchoolHolidays?countryIsoCode={COUNTRY}&languageIsoCode=DE\
             &validFrom={year}-01-01&validTo={year}-12-31&subdivisionCode={COUNTRY}-{state}"
        );
        let body = self.get(&url).await?;
        holidays::parse_school(&body, state)
    }

    async fn get(&self, url: &str) -> Result<String, DomainError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| DomainError::Upstream(format!("{url}: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(DomainError::Upstream(format!(
                "{url}: the server answers with {status}"
            )));
        }
        response
            .text()
            .await
            .map_err(|error| DomainError::Upstream(format!("{url}: {error}")))
    }
}
