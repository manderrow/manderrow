import * as i18n from "@solid-primitives/i18n";
import { BaseDict } from "@solid-primitives/i18n";
import { createResource, createSignal } from "solid-js";

import en_ca from "./locales/en-CA.json"; // en_ca has the base keys
import localeNames from "./locales/localeNames.json";

import { getPreferredLocales } from "../api";

export const RAW_LOCALES = ["en-CA", "en-US", "es", "fr-FR"] as const; // fully translated locales
const DEFAULT_LOCALE = RAW_LOCALES[0];

export type Locale = (typeof RAW_LOCALES)[number];
export type RawDictionary = typeof en_ca;
export type Dictionary = i18n.Flatten<RawDictionary>;
export const localeNamesMap: { [key in Locale]: string } = Object.freeze(localeNames);
export type TranslationKey = keyof RawDictionary;
type FlattenedKnownDictionary = Flatten<Exclude<RawDictionary, undefined>>;

const DEFAULT_FLATTENED_LANG = i18n.flatten(en_ca) as FlattenedKnownDictionary;

function flattenDict(dict: RawDictionary) {
  return { ...DEFAULT_FLATTENED_LANG, ...(i18n.flatten(dict) as FlattenedKnownDictionary) };
}

export async function fetchDictionary(locale: Locale) {
  const dict: RawDictionary = (await import(`./locales/${locale}.json`)).default;

  return flattenDict(dict);
}

export const [locale, setLocale] = createSignal<Locale>(DEFAULT_LOCALE);
export const [dict] = createResource(locale, fetchDictionary, { initialValue: DEFAULT_FLATTENED_LANG });
export const t = i18n.translator(dict, i18n.resolveTemplate);

(async () => {
  let finalLocale: Locale | undefined;

  const preferredLocales = await getPreferredLocales();

  for (const preferredLocale of preferredLocales) {
    const lang = preferredLocale.slice(0, 2);
    let found = false;

    for (const locale of RAW_LOCALES) {
      if (!locale.startsWith(lang)) continue;

      finalLocale = locale;
      if (locale === preferredLocale) {
        found = true;
        break;
      }
    }

    if (found) break;
  }

  if (finalLocale !== undefined) setLocale(finalLocale);
})();

// Workaround for Typescript static analysis bug
type UnionToIntersection<U> = (U extends any ? (k: U) => void : never) extends (k: infer I) => void ? I : never;
type JoinPath<A, B> = A extends string | number
  ? B extends string | number
    ? `${A}.${B}`
    : A
  : B extends string | number
  ? B
  : "";
type Flatten<T extends BaseDict, P = {}> = UnionToIntersection<
  {
    [K in keyof T]: T[K] extends BaseDict ? Flatten<T[K], JoinPath<P, K>> : never;
  }[keyof T]
> & {
  readonly [K in keyof T as JoinPath<P, K>]: T[K];
};
