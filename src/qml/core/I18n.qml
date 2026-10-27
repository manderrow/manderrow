pragma Singleton
import QtQuick
import Manderrow

// Translations, ported from i18n/i18n.ts. Calling `t` inside a binding makes the binding depend on
// the current dictionary, so translated text updates when the locale changes.
QtObject {
    id: i18n

    /// Fully translated locales. The first is the default and has every key.
    readonly property var rawLocales: ["en-CA", "en-US", "es", "fr-FR"]
    readonly property string defaultLocale: rawLocales[0]

    readonly property var localeNames: App.localeNames()

    property string locale: defaultLocale

    readonly property var _defaultDict: App.translations(defaultLocale)
    /// The translations by key, with the default locale's for those the locale lacks.
    readonly property var dict: Object.assign({}, _defaultDict, App.translations(locale))

    /// Replaces `{{ name }}` placeholders, like `@solid-primitives/i18n`'s `resolveTemplate`.
    function resolveTemplate(template, args) {
        if (!args)
            return template;
        return template.replace(/{{\s*([^{}\s]+)\s*}}/g, (match, name) => {
            const value = args[name];
            return value === undefined ? match : String(value);
        });
    }

    /// Translates a key. Prefer `Strings`, whose typed properties the editor completes and qmllint
    /// checks; this is for keys that are only known at runtime, like those from the backend.
    function t(key, args) {
        const value = dict[key];
        if (value === undefined)
            return key;
        return resolveTemplate(value, args);
    }

    /// Picks the supported locale for the most preferred language that has one, in the preferred
    /// region if there is one for it.
    function detectLocale(preferredLocales) {
        for (const preferred of preferredLocales) {
            const lang = preferred.slice(0, 2);
            const candidates = rawLocales.filter(candidate => candidate.startsWith(lang));
            if (candidates.length !== 0) {
                locale = candidates.includes(preferred) ? preferred : candidates[0];
                return;
            }
        }
    }
}
