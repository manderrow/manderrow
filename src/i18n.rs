pub fn get_preferred_locales() -> Vec<String> {
    get_locale::get_preferred_locales()
}

/// Returns the JSON translation dictionary for a locale.
pub fn locale_dictionary(locale: &str) -> Option<&'static str> {
    Some(match locale {
        "en-CA" => include_str!("../locales/en-CA.json"),
        "en-US" => include_str!("../locales/en-US.json"),
        "es" => include_str!("../locales/es.json"),
        "fr-FR" => include_str!("../locales/fr-FR.json"),
        "zh-HANS" => include_str!("../locales/zh-HANS.json"),
        "localeNames" => include_str!("../locales/localeNames.json"),
        _ => return None,
    })
}
