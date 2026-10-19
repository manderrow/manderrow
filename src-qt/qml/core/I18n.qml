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

    readonly property var localeNames: JSON.parse(Backend.localeDictionary("localeNames"))

    property string locale: defaultLocale

    readonly property var _defaultDict: _flatten(JSON.parse(Backend.localeDictionary(defaultLocale)))
    readonly property var dict: Object.assign({}, _defaultDict, _flatten(JSON.parse(Backend.localeDictionary(locale) || "{}")))

    function _flatten(obj, prefix, out) {
        out = out || {};
        for (const key of Object.keys(obj)) {
            const path = prefix ? `${prefix}.${key}` : key;
            const value = obj[key];
            if (value !== null && typeof value === "object")
                _flatten(value, path, out);
            else
                out[path] = value;
        }
        return out;
    }

    /// Replaces `{{ name }}` placeholders, like `@solid-primitives/i18n`'s `resolveTemplate`.
    function resolveTemplate(template, args) {
        if (!args)
            return template;
        return template.replace(/{{\s*([^{}\s]+)\s*}}/g, (match, name) => {
            const value = args[name];
            return value === undefined ? match : String(value);
        });
    }

    function t(key, args) {
        const value = dict[key];
        if (value === undefined)
            return key;
        return resolveTemplate(value, args);
    }

    /// Picks the best supported locale for the user's preferences.
    function detectLocale(preferredLocales) {
        let finalLocale;
        for (const preferred of preferredLocales) {
            const lang = preferred.slice(0, 2);
            let found = false;
            for (const candidate of rawLocales) {
                if (!candidate.startsWith(lang))
                    continue;
                finalLocale = candidate;
                if (candidate === preferred) {
                    found = true;
                    break;
                }
            }
            if (found)
                break;
        }
        if (finalLocale !== undefined)
            locale = finalLocale;
    }
}
