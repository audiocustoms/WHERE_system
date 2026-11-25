//! Bank master data (`img/Admin Menü/Bank - Swift BIC Liste`).
//!
//! The window is a list of SWIFT/BIC codes with country, post code and town. It is
//! filled from an online list and can be edited by hand, so this module carries both
//! the entity and the parser for the two shapes such a list arrives in: a CSV with a
//! header row and the German "BLZ-Datei" without one.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use where_domain::DomainError;

/// Longest BIC the legacy column holds.
pub const MAX_BIC: usize = 20;
/// Longest name the legacy column holds.
pub const MAX_NAME: usize = 255;

/// One row of the list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Bank {
    pub id: i64,
    /// SWIFT/BIC, e.g. `AABAFI22XXX`.
    pub bic: String,
    /// Name of the bank.
    pub name: String,
    /// Country as ISO-3, e.g. `DEU`.
    pub country: Option<String>,
    /// Bank sort code ("Bankleitzahl").
    pub blz: Option<String>,
    pub city: Option<String>,
    /// Name the list carries as an alternative.
    pub alt_name: Option<String>,
}

/// What the mask sends for a new or changed row.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BankInput {
    pub bic: String,
    pub name: String,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub blz: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub alt_name: Option<String>,
}

/// Trimmed value; an empty text is no value.
fn optional(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

impl BankInput {
    /// Checks and normalises a row: the BIC is upper case, the bank needs a name.
    pub fn prepare(self) -> Result<Bank, DomainError> {
        let bic = self.bic.trim().to_uppercase();
        if bic.is_empty() {
            return Err(DomainError::Validation(
                "Für die Bank fehlt der SWIFT/BIC.".to_string(),
            ));
        }
        if !looks_like_bic(&bic) {
            return Err(DomainError::Validation(format!(
                "„{bic}\u{201c} ist kein SWIFT/BIC (8 oder 11 Zeichen, nur Buchstaben und Ziffern)."
            )));
        }
        let name = self.name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Für die Bank fehlt der Name.".to_string(),
            ));
        }
        if name.chars().count() > MAX_NAME {
            return Err(DomainError::Validation(format!(
                "Der Name ist länger als {MAX_NAME} Zeichen."
            )));
        }
        let country = optional(self.country).map(|code| code.to_uppercase());
        let mut blz = optional(self.blz);
        let mut city = optional(self.city);
        // "52001 Aachen" in one field is split; the view joins both again.
        if city.is_none()
            && let Some((plz, ort)) = blz.as_deref().and_then(split_place)
        {
            blz = Some(plz);
            city = Some(ort);
        }
        Ok(Bank {
            id: 0,
            bic,
            name,
            country,
            blz,
            city,
            alt_name: optional(self.alt_name),
        })
    }
}

/// Splits "52001 Aachen" into post code and town.
fn split_place(text: &str) -> Option<(String, String)> {
    let (first, rest) = text.split_once(' ')?;
    let first = first.trim();
    let rest = rest.trim();
    if first.is_empty() || rest.is_empty() || !first.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((first.to_string(), rest.to_string()))
}

/// What the `Update` button sends: a URL, a pasted list, or both.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct BankSyncRequest {
    /// Source of the list; empty means "take `csv`".
    #[serde(default)]
    pub url: Option<String>,
    /// The list itself, for a download done by hand.
    #[serde(default)]
    pub csv: Option<String>,
}

/// Outcome of one synchronisation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BankSyncReport {
    /// Where the list came from: the URL or `Eingabe`.
    pub source: String,
    /// Rows that were read from the list.
    pub read: u32,
    pub inserted: u32,
    pub updated: u32,
    pub unchanged: u32,
    /// Rows without a BIC or a bank name.
    pub skipped: u32,
}

/// One parsed row, ready to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankImportRow {
    pub bic: String,
    pub name: String,
    pub country: Option<String>,
    pub blz: Option<String>,
    pub city: Option<String>,
    pub alt_name: Option<String>,
}

/// The Bundesbank file separates with `;`, generic exports with `,`.
fn delimiter(first_line: &str) -> char {
    if first_line.matches(';').count() >= first_line.matches(',').count() {
        ';'
    } else {
        ','
    }
}

/// Index of the column whose heading contains one of `needles` but none of `exclude`.
fn column(headers: &[String], needles: &[&str], exclude: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        let lower = header.to_lowercase();
        needles.iter().any(|needle| lower.contains(needle))
            && !exclude.iter().any(|word| lower.contains(word))
    })
}

/// Reads a list with a header row (`bic,name,city`) or the German BLZ file
/// (`BLZ;Merkmal;Bezeichnung;PLZ;Ort;Kurzbezeichnung;PAN;BIC;Prüfziffer`).
///
/// Returns the rows and the number of lines without a BIC or a bank name.
pub fn parse_banks(text: &str) -> (Vec<BankImportRow>, u32) {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(first) = lines.next() else {
        return (Vec::new(), 0);
    };
    let sep = delimiter(first);
    let lower = first.to_lowercase();
    let has_header =
        lower.contains("bic") || lower.contains("swift") || lower.contains("bezeichnung");

    let mut rows = Vec::new();
    let mut skipped = 0;

    if has_header {
        let headers = split_fields(first, sep);
        let bic = column(&headers, &["bic", "swift"], &[]);
        // "Bank" is also part of "Bankleitzahl" — the numeric columns are no name.
        let name = column(
            &headers,
            &["bezeichnung", "bank", "name"],
            &["alt", "kurz", "bankleitzahl", "blz", "plz"],
        );
        let blz = column(
            &headers,
            &["blz", "bankleitzahl", "plz", "postleitzahl"],
            &[],
        );
        let city = column(&headers, &["ort", "city", "stadt"], &[]);
        let country = column(&headers, &["land", "country", "iso"], &[]);
        let alt = column(&headers, &["alternativ", "altname", "kurzbezeichnung"], &[]);
        let (Some(bic), Some(name)) = (bic, name) else {
            return (Vec::new(), 0);
        };
        for line in lines {
            let fields = split_fields(line, sep);
            let row = clean(BankImportRow {
                bic: field(&fields, bic).unwrap_or_default().to_uppercase(),
                name: field(&fields, name).unwrap_or_default(),
                // Without a country column the list is the German bank sort code file.
                country: country
                    .and_then(|index| field(&fields, index))
                    .or_else(|| Some("DEU".to_string())),
                blz: blz.and_then(|index| field(&fields, index)),
                city: city.and_then(|index| field(&fields, index)),
                alt_name: alt.and_then(|index| field(&fields, index)),
            });
            if row.bic.is_empty() || row.name.is_empty() {
                skipped += 1;
                continue;
            }
            rows.push(row);
        }
    } else {
        for line in std::iter::once(first).chain(lines) {
            let fields = split_fields(line, sep);
            let values: Vec<&str> = fields.iter().map(|value| value.trim()).collect();
            match blz_file_row(&values) {
                Some(row) => rows.push(clean(row)),
                None => skipped += 1,
            }
        }
    }

    (rows, skipped)
}

/// One line of the BLZ file: fixed columns when the line has the full width,
/// otherwise the BIC is found by its shape and the name is the first text column.
fn blz_file_row(values: &[&str]) -> Option<BankImportRow> {
    let is_number = |value: &str| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit());
    if values.len() >= 8 && is_number(values[0]) && looks_like_bic(values[7]) {
        return Some(BankImportRow {
            bic: values[7].to_uppercase(),
            name: values[2].to_string(),
            country: Some("DEU".to_string()),
            blz: Some(values[0].to_string()),
            city: Some(values[4].to_string()),
            alt_name: Some(values[5].to_string()),
        });
    }
    let bic_index = values.iter().position(|value| looks_like_bic(value))?;
    let name = values
        .iter()
        .skip(1)
        .take(bic_index.saturating_sub(1))
        .find(|value| value.len() > 3 && !is_number(value))?;
    Some(BankImportRow {
        bic: values[bic_index].to_uppercase(),
        name: (*name).to_string(),
        country: Some("DEU".to_string()),
        blz: values
            .first()
            .filter(|value| is_number(value))
            .map(|value| value.to_string()),
        city: values
            .get(bic_index.saturating_sub(1))
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty() && !is_number(value)),
        alt_name: None,
    })
}

/// Drops empty fields and an alternative name that repeats the name.
fn clean(mut row: BankImportRow) -> BankImportRow {
    row.name = row.name.trim().to_string();
    row.bic = row.bic.trim().to_string();
    row.blz = optional(row.blz);
    row.city = optional(row.city);
    row.country = optional(row.country).map(|value| value.to_uppercase());
    row.alt_name = optional(row.alt_name).filter(|value| *value != row.name);
    row
}

/// A BIC is 8 or 11 alphanumeric characters, at least one letter.
fn looks_like_bic(value: &str) -> bool {
    let value = value.trim();
    (8..=11).contains(&value.len())
        && value.chars().all(|c| c.is_ascii_alphanumeric())
        && value.chars().any(|c| c.is_ascii_alphabetic())
}

/// Cuts one line into fields; quotes are honoured so a name with a separator survives.
fn split_fields(line: &str, sep: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c == sep && !quoted => {
                fields.push(current.clone());
                current.clear();
            }
            c => current.push(c),
        }
    }
    fields.push(current);
    fields
}

fn field(fields: &[String], index: usize) -> Option<String> {
    fields
        .get(index)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_needs_a_bic_and_a_name() {
        let input = BankInput {
            bic: "  aabafi22xxx ".into(),
            name: " Bank of Åland ".into(),
            country: Some("deu".into()),
            blz: None,
            city: None,
            alt_name: None,
        };
        let bank = input.prepare().unwrap();
        assert_eq!(bank.bic, "AABAFI22XXX");
        assert_eq!(bank.name, "Bank of Åland");
        assert_eq!(bank.country.as_deref(), Some("DEU"));

        let ohne_bic = BankInput {
            bic: " ".into(),
            name: "X".into(),
            country: None,
            blz: None,
            city: None,
            alt_name: None,
        };
        assert!(ohne_bic.prepare().is_err());

        let ohne_namen = BankInput {
            bic: "AABAFI22".into(),
            name: "  ".into(),
            country: None,
            blz: None,
            city: None,
            alt_name: None,
        };
        assert!(ohne_namen.prepare().is_err());
    }

    #[test]
    fn place_in_one_field_is_split() {
        let bank = BankInput {
            bic: "AACSDE33XXX".into(),
            name: "Sparkasse Aachen".into(),
            country: Some("DEU".into()),
            blz: Some("52001 Aachen".into()),
            city: None,
            alt_name: None,
        }
        .prepare()
        .unwrap();
        assert_eq!(bank.blz.as_deref(), Some("52001"));
        assert_eq!(bank.city.as_deref(), Some("Aachen"));
    }

    #[test]
    fn reads_a_list_with_a_header() {
        let csv = "bic;name;blz;city;country;altname\n\
                   AABAFI22XXX;Bank of Åland;65011;Wiesbaden;DEU;Åland Bank\n\
                   ;Ohne BIC;1;Ort;DEU;\n\
                   AACSDE33XXX;Sparkasse Aachen;52059;Aachen;DEU;\n";
        let (rows, skipped) = parse_banks(csv);
        assert_eq!(rows.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!(rows[0].bic, "AABAFI22XXX");
        assert_eq!(rows[0].country.as_deref(), Some("DEU"));
        assert_eq!(rows[0].alt_name.as_deref(), Some("Åland Bank"));
        assert_eq!(rows[1].blz.as_deref(), Some("52059"));
        assert_eq!(rows[1].alt_name, None);
    }

    #[test]
    fn reads_the_german_blz_file() {
        let csv = "52001;1;Aachener Bausparkasse;52001;Aachen;Aachener Bausparkasse;500;AABSDE31XXX;00\n\
                   65011;;Aareal Bank;65011;Wiesbaden;Aareal Bank;;AARBDE5WXXX;00\n";
        let (rows, skipped) = parse_banks(csv);
        assert_eq!(skipped, 0);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bic, "AABSDE31XXX");
        assert_eq!(rows[0].name, "Aachener Bausparkasse");
        assert_eq!(rows[0].blz.as_deref(), Some("52001"));
        assert_eq!(rows[0].city.as_deref(), Some("Aachen"));
        assert_eq!(rows[1].bic, "AARBDE5WXXX");
        assert_eq!(rows[1].name, "Aareal Bank");
    }

    #[test]
    fn a_quoted_name_keeps_its_separator() {
        let csv = "bic,name\nAABAFI22XXX,\"Bank, Filiale Nord\"\n";
        let (rows, _) = parse_banks(csv);
        assert_eq!(rows[0].name, "Bank, Filiale Nord");
    }

    #[test]
    fn a_bic_needs_eight_or_eleven_characters() {
        assert!(looks_like_bic("AABAFI22TMS"));
        assert!(looks_like_bic("AABAFI22"));
        assert!(!looks_like_bic("52001"));
        assert!(!looks_like_bic("AABAFI"));
    }

    #[test]
    fn reads_the_bundesbank_file_with_a_header() {
        // This is what the Bundesbank file looks like: header row, quotes, `;`.
        let csv = "Bankleitzahl;Merkmal;Bezeichnung;PLZ;Ort;Kurzbezeichnung;PAN;BIC;Prüfzifferberechnungsmethode;Datensatznummer;Änderungskennzeichen;Bankleitzahllöschung;Nachfolge-Bankleitzahl\n\
                   \"10000000\";\"1\";\"Bundesbank\";\"10591\";\"Berlin\";\"BBk Berlin\";\"20100\";\"MARKDEF1100\";\"09\";\"011380\";\"U\";\"0\";\"00000000\"\n\
                   \"10050000\";\"1\";\"Berliner Sparkasse\";\"10050\";\"Berlin\";\"Berliner Sparkasse\";;\"\";\"09\";\"000000\";\"U\";\"0\";\"00000000\"\n";
        let (rows, skipped) = parse_banks(csv);
        assert_eq!(skipped, 1, "ohne BIC wird übersprungen");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bic, "MARKDEF1100");
        assert_eq!(rows[0].name, "Bundesbank", "nicht die Bankleitzahl");
        assert_eq!(rows[0].blz.as_deref(), Some("10000000"));
        assert_eq!(rows[0].city.as_deref(), Some("Berlin"));
        assert_eq!(rows[0].alt_name.as_deref(), Some("BBk Berlin"));
        assert_eq!(
            rows[0].country.as_deref(),
            Some("DEU"),
            "ohne Landesspalte: DEU"
        );
    }

    #[test]
    fn an_empty_list_yields_nothing() {
        assert_eq!(parse_banks(""), (Vec::new(), 0));
        assert_eq!(parse_banks("   \n\n").1, 0);
    }
}
