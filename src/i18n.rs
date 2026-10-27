use std::collections::HashMap;

pub fn get_preferred_locales() -> Vec<String> {
    get_locale::get_preferred_locales()
}

/// The JSON translation dictionary of a locale, which nests its translations by the parts of
/// their keys.
fn dictionary(locale: &str) -> Option<&'static str> {
    Some(match locale {
        "en-CA" => include_str!("locales/en-CA.json"),
        "en-US" => include_str!("locales/en-US.json"),
        "es" => include_str!("locales/es.json"),
        "fr-FR" => include_str!("locales/fr-FR.json"),
        "zh-HANS" => include_str!("locales/zh-HANS.json"),
        _ => return None,
    })
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Entry {
    Translation(String),
    Group(HashMap<String, Entry>),
}

/// The translations of a locale, by their keys, like `game_select.title`. There are none for a
/// locale that isn't translated.
pub fn translations(locale: &str) -> serde_json::Result<Vec<(String, String)>> {
    fn flatten(prefix: &str, group: HashMap<String, Entry>, out: &mut Vec<(String, String)>) {
        for (key, entry) in group {
            let key = if prefix.is_empty() {
                key
            } else {
                format!("{prefix}.{key}")
            };
            match entry {
                Entry::Translation(translation) => out.push((key, translation)),
                Entry::Group(group) => flatten(&key, group, out),
            }
        }
    }

    let mut translations = Vec::new();
    if let Some(dictionary) = dictionary(locale) {
        flatten("", serde_json::from_str(dictionary)?, &mut translations);
    }
    Ok(translations)
}

/// The names of the locales, each in its own language, by locale.
pub fn locale_names() -> serde_json::Result<HashMap<String, String>> {
    serde_json::from_str(include_str!("locales/localeNames.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_are_valid() {
        for locale in ["en-CA", "en-US", "es", "fr-FR", "zh-HANS"] {
            translations(locale).unwrap();
        }
        locale_names().unwrap();
    }

    #[test]
    fn translations_are_by_their_keys() {
        let translations = translations("en-CA").unwrap();
        assert!(translations
            .iter()
            .any(|(key, translation)| key == "global.phrases.ok" && translation == "OK"));
        assert!(super::translations("xx").unwrap().is_empty());
    }
}
