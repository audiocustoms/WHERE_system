//! Flags of the disposition: the short codes PeM writes at the end of an event
//! (`Max und Moritz Vst`, `Cosi fan tutte BO 1`) and the staffing each of them
//! needs.
//!
//! A flag is master data with a code and a name — `Vst` / `Vorstellung`. Its
//! staffing lists the positions a day needs when that flag occurs, with `*`
//! standing for "any position" (`nicht relevant` in the mask). The import
//! recognises the flag of an event against this list, so the demand of a day can
//! be derived from its events.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

use crate::demand::ANY_POSITION;

/// Longest code a flag takes (`Belkorr.` needs a few characters).
pub const MAX_CODE: usize = 20;
/// Longest name a flag takes.
pub const MAX_NAME: usize = 100;
/// Longest position name a staffing row takes.
pub const MAX_POSITION: usize = 100;
/// Most employees one position can need.
pub const MAX_NEEDED: i32 = 99;
/// Most staffing rows one flag may carry.
pub const MAX_STAFFING: usize = 50;

/// Positions the theatre knows; the presets and the mask use them as suggestions.
pub const STAFFING_POSITIONS: [&str; 5] = [
    "Meister",
    "Stellwerker",
    "Oberbeleuchter - Bühne links",
    "Beleuchter - Bühne rechts",
    "Beleuchter - Brücke",
];

/// One position a flag needs in one house.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlagStaffing {
    /// The location the staffing belongs to.
    pub location_id: i64,
    /// A position of the personnel master data, or `*` for "any position".
    pub position: String,
    /// How many employees of that position.
    pub needed: i32,
}

/// A flag with its default staffing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Flag {
    pub id: i64,
    /// The short code as the export writes it, for example `Vst`.
    pub code: String,
    /// What the code means, for example `Vorstellung`.
    pub name: String,
    pub sort_order: i32,
    /// The positions the flag needs, in the order they were entered.
    pub staffing: Vec<FlagStaffing>,
    /// Positions this flag never uses.
    pub excluded_positions: Vec<String>,
}

/// The positions a flag does not use; the list replaces the stored one.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FlagExclusionsInput {
    pub positions: Vec<String>,
}

impl FlagExclusionsInput {
    /// Trims the names, keeps each one once and drops the empty ones.
    pub fn clean(&self) -> Result<Vec<String>, DomainError> {
        if self.positions.len() > MAX_STAFFING {
            return Err(DomainError::Validation(format!(
                "A flag takes at most {MAX_STAFFING} excluded positions."
            )));
        }
        let mut positions: Vec<String> = Vec::with_capacity(self.positions.len());
        for position in &self.positions {
            let name = position.trim();
            if name.is_empty() {
                continue;
            }
            if name.chars().count() > MAX_POSITION {
                return Err(DomainError::Validation(format!(
                    "The position must not be longer than {MAX_POSITION} characters."
                )));
            }
            if positions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(name))
            {
                continue;
            }
            positions.push(name.to_string());
        }
        Ok(positions)
    }
}

/// What the mask sends to create and rename a flag.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FlagInput {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub name: String,
}

/// The staffing of a flag; the list replaces the stored one.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FlagStaffingInput {
    pub entries: Vec<FlagStaffingEntryInput>,
}

/// One row of the staffing as the mask sends it back.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FlagStaffingEntryInput {
    #[serde(default)]
    pub position: String,
    #[serde(default)]
    pub needed: i32,
}

/// A checked flag for the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanFlag {
    pub code: String,
    pub name: String,
}

/// What the presets did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlagPresetReport {
    /// Flags that did not exist yet and were created.
    pub created: usize,
    /// Flags that already existed and were left alone.
    pub kept: usize,
}

/// A flag of the PeM legend with the staffing it usually needs.
pub struct FlagPreset {
    pub code: &'static str,
    pub name: &'static str,
    pub staffing: &'static [(&'static str, i32)],
}

const FIVE: &[(&str, i32)] = &[
    ("Meister", 1),
    ("Stellwerker", 1),
    ("Oberbeleuchter - Bühne links", 1),
    ("Beleuchter - Bühne rechts", 1),
    ("Beleuchter - Brücke", 1),
];

const BELKORR: &[(&str, i32)] = &[("Meister", 1), ("Stellwerker", 1), (ANY_POSITION, 1)];

const ANY: &[(&str, i32)] = &[(ANY_POSITION, 1)];

/// The legend of the theatre: every flag of the export with its default staffing.
pub const PRESETS: &[FlagPreset] = &[
    FlagPreset {
        code: "Vst",
        name: "Vorstellung",
        staffing: FIVE,
    },
    FlagPreset {
        code: "Probe",
        name: "Probe",
        staffing: ANY,
    },
    FlagPreset {
        code: "TE",
        name: "Technische Einrichtung",
        staffing: FIVE,
    },
    FlagPreset {
        code: "BO",
        name: "Bühnenorchesterprobe",
        staffing: ANY,
    },
    FlagPreset {
        code: "BEL",
        name: "Beleuchtungsprobe",
        staffing: FIVE,
    },
    FlagPreset {
        code: "Belkorr.",
        name: "Beleuchtungskorrekturen",
        staffing: BELKORR,
    },
    FlagPreset {
        code: "HP1",
        name: "Hauptprobe 1",
        staffing: FIVE,
    },
    FlagPreset {
        code: "HP2",
        name: "Hauptprobe 2",
        staffing: FIVE,
    },
    FlagPreset {
        code: "GP",
        name: "Generalprobe",
        staffing: FIVE,
    },
    FlagPreset {
        code: "Premiere",
        name: "Premierenvorstellung",
        staffing: FIVE,
    },
    FlagPreset {
        code: "Dern",
        name: "Dernière",
        staffing: FIVE,
    },
];

impl FlagInput {
    /// Trims both fields and checks that neither is empty nor too long.
    pub fn clean(&self) -> Result<CleanFlag, DomainError> {
        let code = self.code.trim();
        if code.is_empty() {
            return Err(DomainError::Validation(
                "The code must not be empty.".to_string(),
            ));
        }
        if code.chars().count() > MAX_CODE {
            return Err(DomainError::Validation(format!(
                "The code must not be longer than {MAX_CODE} characters."
            )));
        }
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "The name must not be empty.".to_string(),
            ));
        }
        if name.chars().count() > MAX_NAME {
            return Err(DomainError::Validation(format!(
                "The name must not be longer than {MAX_NAME} characters."
            )));
        }
        Ok(CleanFlag {
            code: code.to_string(),
            name: name.to_string(),
        })
    }
}

impl FlagStaffingInput {
    /// Trims the positions, drops the zeros and keeps each position once.
    /// `location_id` is the house the rows belong to.
    pub fn clean(&self, location_id: i64) -> Result<Vec<FlagStaffing>, DomainError> {
        if self.entries.len() > MAX_STAFFING {
            return Err(DomainError::Validation(format!(
                "A flag takes at most {MAX_STAFFING} staffing rows."
            )));
        }
        let mut rows: Vec<FlagStaffing> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            if entry.needed <= 0 {
                continue;
            }
            if entry.needed > MAX_NEEDED {
                return Err(DomainError::Validation(format!(
                    "A position needs at most {MAX_NEEDED} employees."
                )));
            }
            let position = entry.position.trim();
            if position.is_empty() {
                return Err(DomainError::Validation(
                    "A staffing row needs a position.".to_string(),
                ));
            }
            if position.chars().count() > MAX_POSITION {
                return Err(DomainError::Validation(format!(
                    "The position must not be longer than {MAX_POSITION} characters."
                )));
            }
            if rows
                .iter()
                .any(|row| row.position.eq_ignore_ascii_case(position))
            {
                return Err(DomainError::Validation(format!(
                    "The position “{position}” appears twice."
                )));
            }
            rows.push(FlagStaffing {
                location_id,
                position: position.to_string(),
                needed: entry.needed,
            });
        }
        Ok(rows)
    }
}

/// How many words before the last one the search still looks: PeM writes `TE Ton`
/// and `BO 4`, so the code does not always end the line.
const SEARCHED_WORDS: usize = 3;

/// `true` when the word names the code, ignoring case and a trailing dot.
fn same_code(code: &str, word: &str) -> bool {
    code.trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case(word.trim().trim_end_matches('.'))
}

/// The preset with that code, ignoring case and a trailing dot.
pub fn preset(code: &str) -> Option<&'static FlagPreset> {
    PRESETS.iter().find(|preset| same_code(preset.code, code))
}

/// The codes an import recognises: what the master data carries plus the legend of
/// PeM, so that a fresh installation recognises the flags of its first disposition.
pub fn vocabulary(stored: &[String]) -> Vec<String> {
    let mut codes: Vec<String> = stored.to_vec();
    for preset in PRESETS {
        if !codes.iter().any(|code| same_code(code, preset.code)) {
            codes.push(preset.code.to_string());
        }
    }
    codes
}

/// The flag of an event title, or `None` when it carries none.
///
/// PeM writes the flag at the end of the first line — `Max und Moritz Vst`,
/// `Cosi fan tutte BO 1`, `Cosi fan tutte Belkorr.`. A bare number behind the
/// code is skipped, so `BO 1` is the flag `BO`, and a word behind it
/// (`Ehebrecherinnen TE Ton`) is looked past as well.
pub fn recognise(title: &str, codes: &[String]) -> Option<String> {
    let line = title.lines().next()?.trim();
    let mut words: Vec<&str> = line.split_whitespace().rev().collect();
    while let Some(word) = words.first() {
        if word.chars().all(|character| character.is_ascii_digit()) {
            words.remove(0);
        } else {
            break;
        }
    }
    for word in words.iter().take(SEARCHED_WORDS) {
        let word = word.trim_matches('.');
        if word.is_empty() {
            continue;
        }
        if let Some(code) = codes.iter().find(|code| same_code(code, word)) {
            return Some(code.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes() -> Vec<String> {
        PRESETS
            .iter()
            .map(|preset| preset.code.to_string())
            .collect()
    }

    #[test]
    fn a_body_with_another_field_name_is_rejected() {
        // A misspelt list would otherwise deserialise as empty and wipe the
        // stored staffing.
        assert!(serde_json::from_str::<FlagStaffingInput>(r#"{"staffing":[]}"#).is_err());
        assert!(serde_json::from_str::<FlagStaffingInput>("{}").is_err());
        assert!(serde_json::from_str::<FlagStaffingInput>(r#"{"entries":[]}"#).is_ok());
        assert!(serde_json::from_str::<FlagExclusionsInput>(r#"{"positions":[]}"#).is_ok());
        assert!(serde_json::from_str::<FlagExclusionsInput>("{}").is_err());
    }

    #[test]
    fn the_code_is_trimmed_and_required() {
        let input = FlagInput {
            code: "  Vst ".into(),
            name: "  Vorstellung  ".into(),
        };
        let clean = input.clean().unwrap();
        assert_eq!(clean.code, "Vst");
        assert_eq!(clean.name, "Vorstellung");
        assert!(FlagInput::default().clean().is_err());
    }

    #[test]
    fn a_code_may_not_exceed_the_column_width() {
        let input = FlagInput {
            code: "x".repeat(MAX_CODE + 1),
            name: "Test".into(),
        };
        assert!(input.clean().is_err());
    }

    #[test]
    fn the_staffing_drops_zeros_and_keeps_each_position_once() {
        let input = FlagStaffingInput {
            entries: vec![
                FlagStaffingEntryInput {
                    position: " Meister ".into(),
                    needed: 1,
                },
                FlagStaffingEntryInput {
                    position: "Stellwerker".into(),
                    needed: 0,
                },
                FlagStaffingEntryInput {
                    position: ANY_POSITION.into(),
                    needed: 2,
                },
            ],
        };
        let rows = input.clean(7).unwrap();
        assert_eq!(
            rows,
            vec![
                FlagStaffing {
                    location_id: 7,
                    position: "Meister".into(),
                    needed: 1
                },
                FlagStaffing {
                    location_id: 7,
                    position: "*".into(),
                    needed: 2
                },
            ]
        );
    }

    #[test]
    fn a_position_twice_is_reported() {
        let input = FlagStaffingInput {
            entries: vec![
                FlagStaffingEntryInput {
                    position: "Meister".into(),
                    needed: 1,
                },
                FlagStaffingEntryInput {
                    position: "meister".into(),
                    needed: 2,
                },
            ],
        };
        assert!(input.clean(1).is_err());
    }

    #[test]
    fn the_flag_at_the_end_of_the_first_line_is_the_one() {
        let codes = codes();
        assert_eq!(recognise("Max und Moritz Vst", &codes), Some("Vst".into()));
        assert_eq!(recognise("Cosi fan tutte BO 1", &codes), Some("BO".into()));
        assert_eq!(
            recognise("Cosi fan tutte HP1\n Fotograf: Fabian Raabe", &codes),
            Some("HP1".into())
        );
        assert_eq!(
            recognise("Cosi fan tutte Belkorr.", &codes),
            Some("Belkorr.".into())
        );
        assert_eq!(recognise("Magic TE", &codes), Some("TE".into()));
        assert_eq!(recognise("Dern", &codes), Some("Dern".into()));
    }

    #[test]
    fn a_word_behind_the_code_does_not_hide_it() {
        let codes = codes();
        assert_eq!(
            recognise("Ehebrecherinnen TE Ton", &codes),
            Some("TE".into())
        );
        assert_eq!(recognise("Così fan tutte BO 4", &codes), Some("BO".into()));
        // Only the first line counts, the notes below it are no flag.
        assert_eq!(
            recognise("Kipppunkte Einführung\nanschließend Probe", &codes),
            None
        );
    }

    #[test]
    fn a_text_without_a_flag_keeps_none() {
        let codes = codes();
        assert_eq!(recognise("Kipppunkte Einführung", &codes), None);
        assert_eq!(recognise("NJK 2027 HP Konzert", &codes), None);
        assert_eq!(recognise("Bühnenreinigung res.", &codes), None);
        assert_eq!(recognise("", &codes), None);
        assert_eq!(recognise("Max und Moritz Vst", &[]), None);
    }

    #[test]
    fn the_vocabulary_is_the_master_data_plus_the_legend() {
        let codes = vocabulary(&["WE".to_string(), "vst".to_string()]);
        assert_eq!(codes[0], "WE");
        // `vst` stands in the master data already, so the legend does not repeat it.
        assert_eq!(
            codes
                .iter()
                .filter(|code| code.eq_ignore_ascii_case("Vst"))
                .count(),
            1
        );
        assert_eq!(codes.len(), 1 + PRESETS.len());
        assert!(preset("belkorr").is_some());
        // The dot is part of the legend's spelling, not of the comparison.
        assert!(preset("Belkorr.").is_some());
        assert!(preset(" BELKORR ").is_some());
        assert!(preset("unbekannt").is_none());
    }

    #[test]
    fn every_preset_is_valid() {
        for preset in PRESETS {
            let clean = FlagInput {
                code: preset.code.to_string(),
                name: preset.name.to_string(),
            }
            .clean()
            .unwrap_or_else(|error| panic!("{}: {error}", preset.code));
            assert_eq!(clean.code, preset.code);
            let staffing = FlagStaffingInput {
                entries: preset
                    .staffing
                    .iter()
                    .map(|(position, needed)| FlagStaffingEntryInput {
                        position: (*position).to_string(),
                        needed: *needed,
                    })
                    .collect(),
            }
            .clean(7)
            .unwrap_or_else(|error| panic!("{}: {error}", preset.code));
            assert_eq!(staffing.len(), preset.staffing.len());
        }
    }
}
