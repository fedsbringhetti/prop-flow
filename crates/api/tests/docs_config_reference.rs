// Crate di test: qui `expect` e `panic!` sono lo strumento corretto. Un panic
// segnala una precondizione non soddisfatta e fa fallire il test, che è il
// comportamento desiderato.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Guardia sulla documentazione della configurazione.
//!
//! Le tabelle di [07 — Operazioni](../../../docs/07-operazioni.md) sono l'unico
//! posto dove si cerca quali variabili esistono e con quale default. Finora nulla
//! impediva loro di restare indietro rispetto al codice, ed è successo:
//! `PROPFLOW__TELEMETRY__SERVICE_NAME` non era documentata e il default di
//! `PROPFLOW__TELEMETRY__FILTER` era quello di due versioni prima.
//!
//! L'elenco si deriva dai sorgenti — i campi di `AppConfig` e le `std::env::var`
//! del migratore — e mai da una lista scritta qui: una lista scritta a mano è
//! esattamente la cosa che ci si dimentica di aggiornare, cioè il difetto che
//! questo test esiste per intercettare.
//!
//! Non serve il database: gira in ogni `cargo test`, anche senza Docker.

const CONFIG_RS: &str = include_str!("../src/config.rs");
const MIGRATOR_RS: &str = include_str!("../../migrator/src/main.rs");
const DEFAULTS_TOML: &str = include_str!("../../../config/default.toml");
const OPERAZIONI_MD: &str = include_str!("../../../docs/07-operazioni.md");

/// Le variabili che `AppConfig` sa leggere, nella forma `PROPFLOW__SEZIONE__CAMPO`.
///
/// `AppConfig` dichiara le sezioni come campi (`server: ServerConfig`), quindi la
/// lista si ottiene visitando due livelli di struct e non elencando nulla.
fn settings_from_app_config() -> Vec<String> {
    let mut settings = Vec::new();
    for (section, type_name) in struct_fields("AppConfig") {
        for (field, _) in struct_fields(&type_name) {
            settings.push(format!(
                "PROPFLOW__{}__{}",
                section.to_uppercase(),
                field.to_uppercase()
            ));
        }
    }
    assert!(
        settings.len() > 10,
        "il parser di config.rs non ha trovato le impostazioni: {settings:?}"
    );
    settings
}

/// Le variabili che il migratore legge da sé, senza passare da `AppConfig`.
fn settings_from_migrator() -> Vec<String> {
    let settings: Vec<String> = MIGRATOR_RS
        .split("std::env::var(\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"'))
        .map(|(name, _)| name.to_owned())
        .filter(|name| name.starts_with("PROPFLOW__"))
        .collect();
    assert!(
        !settings.is_empty(),
        "il parser di crates/migrator/src/main.rs non ha trovato variabili"
    );
    settings
}

/// I campi `pub` di una struct, come coppie `(nome, tipo)`.
fn struct_fields(name: &str) -> Vec<(String, String)> {
    let header = format!("pub struct {name} {{");
    let start = CONFIG_RS
        .find(&header)
        .unwrap_or_else(|| panic!("struct {name} non trovata in crates/api/src/config.rs"));
    let body = &CONFIG_RS[start + header.len()..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("fine della struct {name} non trovata"));

    body[..end]
        .lines()
        .filter_map(|line| {
            let (field, type_name) = line.trim().strip_prefix("pub ")?.split_once(": ")?;
            Some((field.to_owned(), type_name.trim_end_matches(',').to_owned()))
        })
        .collect()
}

/// Le righe delle tabelle di configurazione, come coppie `(variabile, default)`.
fn documented_settings() -> Vec<(String, String)> {
    let rows: Vec<(String, String)> = OPERAZIONI_MD
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("| `PROPFLOW__"))
        .filter_map(|line| {
            let mut cells = line.trim_matches('|').split('|');
            let variable = cells.next()?.trim().trim_matches('`').to_owned();
            let default = cells.next()?.trim().trim_matches('`').to_owned();
            Some((variable, default))
        })
        .collect();
    assert!(
        rows.len() > 10,
        "il parser di docs/07-operazioni.md non ha trovato le tabelle: {rows:?}"
    );
    rows
}

/// Le impostazioni che il progetto spedisce in `config/default.toml`.
fn shipped_defaults() -> Vec<(String, String)> {
    let mut section = String::new();
    let mut defaults = Vec::new();
    for line in DEFAULTS_TOML.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_uppercase();
        } else if let Some((key, value)) = line.split_once('=') {
            defaults.push((
                format!("PROPFLOW__{section}__{}", key.trim().to_uppercase()),
                value.trim().trim_matches('"').to_owned(),
            ));
        }
    }
    assert!(
        !defaults.is_empty(),
        "il parser di config/default.toml non ha trovato impostazioni"
    );
    defaults
}

#[test]
fn every_setting_of_app_config_is_documented() {
    let documented = documented_settings();
    let missing: Vec<String> = settings_from_app_config()
        .into_iter()
        .filter(|setting| !documented.iter().any(|(variable, _)| variable == setting))
        .collect();

    assert!(
        missing.is_empty(),
        "impostazioni presenti in crates/api/src/config.rs e assenti dalla tabella \
         di docs/07-operazioni.md: {missing:?}"
    );
}

#[test]
fn every_documented_variable_exists_in_the_code() {
    let mut known = settings_from_app_config();
    known.extend(settings_from_migrator());

    let unknown: Vec<String> = documented_settings()
        .into_iter()
        .map(|(variable, _)| variable)
        .filter(|variable| !known.contains(variable))
        .collect();

    assert!(
        unknown.is_empty(),
        "variabili documentate in docs/07-operazioni.md che nessun binario legge: {unknown:?}"
    );
}

#[test]
fn the_documented_default_is_the_one_the_project_ships() {
    let documented = documented_settings();

    // Solo le impostazioni presenti in `config/default.toml`: per le altre il
    // default sta in una funzione del codice e la tabella riporta `—`.
    let wrong: Vec<String> = shipped_defaults()
        .into_iter()
        .filter_map(|(variable, shipped)| {
            let (_, documented) = documented.iter().find(|(name, _)| *name == variable)?;
            (documented != &shipped)
                .then(|| format!("{variable}: documentato `{documented}`, spedito `{shipped}`"))
        })
        .collect();

    assert!(
        wrong.is_empty(),
        "default divergenti fra config/default.toml e docs/07-operazioni.md: {wrong:?}"
    );
}
