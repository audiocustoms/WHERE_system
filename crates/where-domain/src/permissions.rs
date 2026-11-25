//! The rights of the permission groups (`img/Admin Menü/Benutzerverwaltung/Berechtigungsgruppe`).
//!
//! The dialog `Einstellungen: Gruppenrechte` shows a tree of the modules of the
//! application; every leaf of that tree has its own list of checkboxes — one
//! screenshot per leaf in `img/Admin Menü/Benutzerverwaltung/Berechtigungsgruppe`.
//! This module is the catalogue behind that tree: one entry per area with the
//! rights of its screenshot, in the order and with the wording of the original.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One tickable right of an area.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionRight {
    /// `<area>.<slug>`, e.g. `adressen.adresse-anlegen`.
    pub key: String,
    pub label: String,
}

/// One leaf of the tree with its rights.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionArea {
    pub key: String,
    pub label: String,
    pub rights: Vec<PermissionRight>,
}

/// One branch of the tree (`Stammdaten`, `Faktura`, …).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionGroup {
    pub key: String,
    pub label: String,
    pub areas: Vec<PermissionArea>,
}

fn area(key: &str, label: &str, rights: &[(&str, &str)]) -> PermissionArea {
    PermissionArea {
        key: key.to_string(),
        label: label.to_string(),
        rights: rights
            .iter()
            .map(|(slug, text)| PermissionRight {
                key: format!("{key}.{slug}"),
                label: (*text).to_string(),
            })
            .collect(),
    }
}

/// The tree of the rights dialog, in the order of the original.
pub fn catalogue() -> Vec<PermissionGroup> {
    vec![
        PermissionGroup {
            key: "stammdaten".to_string(),
            label: "Stammdaten".to_string(),
            areas: vec![
                area(
                    "adressen",
                    "Adressen",
                    &[
                        ("adresse-anlegen", "Adresse anlegen"),
                        ("adresse-lesen", "Adresse lesen"),
                        ("adresse-editieren", "Adresse editieren"),
                        (
                            "adresse-editieren-auth",
                            "Adresse nach Authentifizierung editieren",
                        ),
                        ("adresse-aktiv-setzen", "Adresse aktiv/inaktiv setzen"),
                        ("adresse-loeschen", "Adresse löschen"),
                        ("vips-sehen", "VIPs sehen"),
                        ("kontakthistorie-sehen", "Kontakthistorie sehen"),
                        ("geheimnisse-1", "Geheimnisse sehen - Level 1"),
                        ("geheimnisse-2", "Geheimnisse sehen - Level 2"),
                        ("dublettenverhinderer", "Dublettenverhinderer"),
                    ],
                ),
                area(
                    "interessenten",
                    "Interessenten",
                    &[
                        ("interessent-anlegen", "Interessent anlegen"),
                        ("interessent-lesen", "Interessent lesen"),
                        ("interessent-editieren", "Interessent editieren"),
                        ("umsaetze-sehen", "Umsätze sehen"),
                    ],
                ),
                area(
                    "kunden",
                    "Kunden",
                    &[
                        ("kunde-anlegen", "Kunde anlegen"),
                        ("kunde-lesen", "Kunde lesen"),
                        ("kunde-editieren", "Kunde editieren"),
                        ("umsaetze-sehen", "Umsätze sehen"),
                    ],
                ),
                area(
                    "lieferanten",
                    "Lieferanten",
                    &[
                        ("lieferant-anlegen", "Lieferant anlegen"),
                        ("lieferant-lesen", "Lieferant lesen"),
                        ("lieferant-editieren", "Lieferant editieren"),
                        ("umsaetze-sehen", "Umsätze sehen"),
                    ],
                ),
                area(
                    "artikel",
                    "Artikel",
                    &[
                        ("artikel-anlegen", "Artikel anlegen"),
                        ("artikel-lesen", "Artikel lesen"),
                        ("artikel-editieren", "Artikel editieren"),
                        (
                            "artikel-editieren-auth",
                            "Artikel nach Authentifizierung editieren",
                        ),
                        ("artikel-aktiv-setzen", "Artikel aktiv/inaktiv setzen"),
                        ("artikel-loeschen", "Artikel löschen"),
                        ("preise-sehen", "Preise sehen"),
                        (
                            "warengruppen-dragdrop",
                            "Warengruppen per Drag&Drop in Artikelliste verteilen",
                        ),
                        ("ungeklaerte-geraeteliste", "Ungeklärte Geräteliste"),
                    ],
                ),
                area(
                    "service",
                    "Service",
                    &[
                        ("service-anlegen", "Service anlegen"),
                        ("service-lesen", "Service lesen"),
                        ("service-editieren", "Service editieren"),
                        (
                            "service-editieren-auth",
                            "Service nach Authentifizierung editieren",
                        ),
                        ("service-aktiv-setzen", "Service aktiv/inaktiv setzen"),
                        ("service-loeschen", "Service löschen"),
                        ("preise-sehen", "Preise sehen"),
                    ],
                ),
                area(
                    "personal",
                    "Personal",
                    &[
                        ("personal-anlegen", "Personal anlegen"),
                        ("personal-lesen", "Personal lesen"),
                        ("personal-editieren", "Personal editieren"),
                        (
                            "personal-editieren-auth",
                            "Personal nach Authentifizierung editieren",
                        ),
                        ("personal-aktiv-setzen", "Personal aktiv/inaktiv setzen"),
                        ("personal-loeschen", "Personal löschen"),
                        ("persoenliche-daten-sehen", "Persönliche Daten sehen"),
                    ],
                ),
                area(
                    "personaldispo",
                    "Personal-Disposition",
                    &[
                        ("personaldispo-anlegen", "Personaldispo anlegen"),
                        ("personaldispo-lesen", "Personaldispo lesen"),
                        ("personaldispo-editieren", "Personaldispo editieren"),
                    ],
                ),
                area(
                    "transporte",
                    "Transporte",
                    &[
                        ("transport-anlegen", "Transport anlegen"),
                        ("transport-lesen", "Transport lesen"),
                        ("transport-editieren", "Transport editieren"),
                        ("transport-loeschen", "Transport löschen"),
                        ("transport-aktiv-setzen", "Transport aktiv/inaktiv setzen"),
                        ("summen-sehen", "Summen sehen"),
                    ],
                ),
                area(
                    "fahrzeuge",
                    "Fahrzeuge",
                    &[
                        ("fahrzeug-anlegen", "Fahrzeug anlegen"),
                        ("fahrzeug-lesen", "Fahrzeug lesen"),
                        ("fahrzeug-editieren", "Fahrzeug editieren"),
                        ("fahrzeug-loeschen", "Fahrzeug löschen"),
                        ("fahrzeug-aktiv-setzen", "Fahrzeug aktiv/inaktiv setzen"),
                        ("summen-sehen", "Summen sehen"),
                        ("fahrtenbuch-einsehen", "Fahrtenbuch einsehen"),
                        ("wartungsbuch-einsehen", "Wartungsbuch einsehen"),
                    ],
                ),
            ],
        },
        PermissionGroup {
            key: "projekt".to_string(),
            label: "Projekt".to_string(),
            areas: vec![area(
                "projekt",
                "Projekt",
                &[
                    ("projekt-anlegen", "Projekt anlegen"),
                    ("projekt-lesen", "Projekt lesen"),
                    ("projekt-editieren", "Projekt editieren"),
                    ("projekt-stornieren", "Projekt stornieren"),
                    ("projekt-loeschen", "Projekt löschen"),
                    ("summen-sehen", "Summen sehen"),
                ],
            )],
        },
        PermissionGroup {
            key: "auftrag".to_string(),
            label: "Auftrag".to_string(),
            areas: vec![
                area(
                    "angebot",
                    "Angebot",
                    &[
                        ("angebot-anlegen", "Angebot anlegen"),
                        ("angebot-lesen", "Angebot lesen"),
                        ("angebot-editieren", "Angebot editieren"),
                        ("angebot-stornieren", "Angebot stornieren"),
                        ("angebot-loeschen", "Angebot löschen"),
                        ("summen-sehen", "Summen sehen"),
                    ],
                ),
                area(
                    "auftrag",
                    "Auftrag",
                    &[
                        ("auftrag-anlegen", "Auftrag anlegen"),
                        ("auftrag-lesen", "Auftrag lesen"),
                        ("auftrag-editieren", "Auftrag editieren"),
                        ("auftrag-stornieren", "Auftrag stornieren"),
                        ("auftrag-loeschen", "Auftrag löschen"),
                    ],
                ),
            ],
        },
        PermissionGroup {
            key: "lager".to_string(),
            label: "Lager".to_string(),
            areas: vec![area(
                "packschein",
                "Packschein",
                &[
                    ("packschein-anlegen", "Packschein anlegen"),
                    ("packschein-lesen", "Packschein lesen"),
                    ("packschein-editieren", "Packschein editieren"),
                    ("packschein-loeschen", "Packschein löschen"),
                    ("summen-sehen", "Summen sehen"),
                    ("packschein-paketieren", "Packschein paketieren"),
                    ("packschein-ausbuchen", "Packschein ausbuchen"),
                    ("packschein-einbuchen", "Packschein einbuchen"),
                ],
            )],
        },
        PermissionGroup {
            key: "einkauf".to_string(),
            label: "Einkauf".to_string(),
            areas: vec![area(
                "bestellvorgang",
                "Bestellvorgang",
                &[
                    ("bestellung-anlegen", "Bestellung anlegen"),
                    ("bestellung-lesen", "Bestellung lesen"),
                    ("bestellung-editieren", "Bestellung editieren"),
                    ("bestellung-loeschen", "Bestellung löschen"),
                ],
            )],
        },
        PermissionGroup {
            key: "faktura".to_string(),
            label: "Faktura".to_string(),
            areas: vec![
                area(
                    "ausgangsrechnung",
                    "Ausgangsrechnung",
                    &[
                        ("rechnung-anlegen", "Rechnung anlegen"),
                        ("rechnung-lesen", "Rechnung lesen"),
                        ("rechnung-editieren", "Rechnung editieren"),
                        ("rechnung-buchen", "Rechnung buchen"),
                        ("summen-sehen", "Summen sehen"),
                    ],
                ),
                area(
                    "storno",
                    "Storno & Rechnungskorrektur",
                    &[("storno-anlegen", "Storno & Rechnungskorrektur anlegen")],
                ),
                area(
                    "zahlungseingang",
                    "Zahlungseingang",
                    &[
                        ("zahlungseingang-anlegen", "Zahlungseingang anlegen"),
                        ("zahlungseingang-lesen", "Zahlungseingang lesen"),
                        ("zahlungseingang-loeschen", "Zahlungseingang löschen"),
                    ],
                ),
                area(
                    "eingangsrechnung",
                    "Eingangsrechnung",
                    &[
                        ("eingangsrechnung-anlegen", "Eingangsrechnung anlegen"),
                        ("eingangsrechnung-lesen", "Eingangsrechnung lesen"),
                        ("eingangsrechnung-editieren", "Eingangsrechnung editieren"),
                        ("eingangsrechnung-stornieren", "Eingangsrechnung stornieren"),
                        ("eingangsrechnung-buchen", "Eingangsrechnung buchen"),
                    ],
                ),
                area(
                    "zahlungsausgang",
                    "Zahlungsausgang",
                    &[
                        ("zahlungsausgang-anlegen", "Zahlungsausgang anlegen"),
                        ("zahlungsausgang-lesen", "Zahlungsausgang lesen"),
                        ("zahlungsausgang-loeschen", "Zahlungsausgang löschen"),
                    ],
                ),
                area(
                    "mahnwesen",
                    "Mahnwesen",
                    &[
                        ("mahnlauf-starten", "Mahnlauf starten"),
                        ("mahnwesen-einsehen", "Mahnwesen einsehen"),
                    ],
                ),
                area(
                    "kassenbuch",
                    "Kassenbuch",
                    &[
                        ("kasseneintrag-anlegen", "Kasseneintrag anlegen"),
                        ("kasseneintrag-lesen", "Kasseneintrag lesen"),
                        ("kasseneintrag-editieren", "Kasseneintrag editieren"),
                        ("kasseneintrag-loeschen", "Kasseneintrag löschen"),
                        ("kassenbuch-abrechnen", "Kassenbuch abrechnen"),
                    ],
                ),
                area(
                    "statistiken",
                    "Statistiken",
                    &[("zugriff", "Zugriff erlauben")],
                ),
            ],
        },
        PermissionGroup {
            key: "module".to_string(),
            label: "Module".to_string(),
            areas: vec![
                area(
                    "barcodesystem",
                    "Barcode-System",
                    &[
                        ("zugriff", "Zugriff erlauben"),
                        ("scan-protokoll", "Scan-Protokoll einsehen"),
                    ],
                ),
                area("dguv", "DGUV Prüfungen", &[("zugriff", "Zugriff erlauben")]),
                area(
                    "emailversand",
                    "eMail-Versand",
                    &[("zugriff", "Zugriff erlauben")],
                ),
                area("messenger", "Messenger", &[("zugriff", "Zugriff erlauben")]),
                area(
                    "qm",
                    "Quality Management",
                    &[
                        ("ticket-anlegen", "Ticket anlegen"),
                        ("ticket-lesen", "Ticket lesen"),
                        ("ticket-editieren", "Ticket editieren"),
                        ("ticket-buchen", "Ticket buchen"),
                    ],
                ),
                area(
                    "tapi",
                    "TAPI",
                    &[
                        ("zugriff", "Zugriff erlauben"),
                        (
                            "fremde-gespraeche-liste",
                            "Zeige Liste fremder Gespräche (Controlling)",
                        ),
                        (
                            "fremde-gespraeche-auflegen",
                            "Auflegen fremder Gespräche erlauben (Controlling)",
                        ),
                    ],
                ),
                area(
                    "teamagenda",
                    "TeamAgenda",
                    &[
                        ("termine-anlegen", "Termine anlegen"),
                        ("termine-lesen", "Termine lesen"),
                        ("termine-editieren", "Termine editieren"),
                        ("termine-loeschen", "Termine löschen"),
                    ],
                ),
                area(
                    "textverarbeitung",
                    "Textverarbeitung",
                    &[
                        ("dokument-anlegen", "Dokument anlegen"),
                        ("dokument-lesen", "Dokument lesen"),
                        ("dokument-editieren", "Dokument editieren"),
                        ("dokument-loeschen", "Dokument löschen"),
                    ],
                ),
                area("werkstatt", "Werkstatt", &[("zugriff", "Zugriff erlauben")]),
                area(
                    "scriptmenue",
                    "Scriptmenü",
                    &[
                        ("sicherheitsstufe-1", "Sicherheitsstufe 1"),
                        ("sicherheitsstufe-2", "Sicherheitsstufe 2"),
                        ("sicherheitsstufe-3", "Sicherheitsstufe 3"),
                    ],
                ),
            ],
        },
        PermissionGroup {
            key: "admin".to_string(),
            label: "Admin".to_string(),
            areas: vec![
                area(
                    "persoenliche-einstellungen",
                    "persönliche Einstellungen",
                    &[
                        ("zugriff", "Zugriff erlauben"),
                        ("toolbaranpassung", "Toolbaranpassung erlauben"),
                    ],
                ),
                area(
                    "administrative-einstellungen",
                    "administrative Einstellungen",
                    &[("zugriff", "Zugriff erlauben")],
                ),
                area(
                    "dokumentvorlage",
                    "Dokumentvorlage",
                    &[("zugriff", "Zugriff erlauben")],
                ),
                area(
                    "im-export",
                    "Im- & Export",
                    &[
                        ("import", "Zugriff Import erlauben"),
                        ("export", "Zugriff Export erlauben"),
                        ("datenaustausch", "Zugriff Datenaustausch erlauben"),
                    ],
                ),
            ],
        },
    ]
}

/// Every key of the catalogue — the repository only stores keys from this set.
pub fn all_right_keys() -> Vec<String> {
    catalogue()
        .into_iter()
        .flat_map(|group| group.areas)
        .flat_map(|area| area.rights)
        .map(|right| right.key)
        .collect()
}

/// The number of rights the catalogue offers.
pub fn right_count() -> usize {
    all_right_keys().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tree_has_the_areas_of_the_screenshots() {
        let groups = catalogue();
        let labels: Vec<&str> = groups.iter().map(|group| group.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Stammdaten",
                "Projekt",
                "Auftrag",
                "Lager",
                "Einkauf",
                "Faktura",
                "Module",
                "Admin"
            ]
        );
        let areas: usize = groups.iter().map(|group| group.areas.len()).sum();
        assert_eq!(areas, 37);
        assert_eq!(right_count(), 148);
    }

    #[test]
    fn every_right_key_is_unique_and_prefixed_by_its_area() {
        let mut keys = all_right_keys();
        let before = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), before);
        for group in catalogue() {
            for area in group.areas {
                for right in area.rights {
                    assert!(right.key.starts_with(&format!("{}.", area.key)));
                    assert!(!right.label.trim().is_empty());
                }
            }
        }
    }
}
