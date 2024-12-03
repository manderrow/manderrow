import * as i18n from "@solid-primitives/i18n";
import * as en_ca from "./locales/en_ca.json"; // en_ca has the base keys

export type Locale = "en_ca" | "en_us"; // fully translated locales
export type RawDictionary = typeof en_ca;
export type Dictionary = i18n.Flatten<RawDictionary>;

export async function fetchDictionary(locale: Locale): Promise<RawDictionary> {
  const dict: RawDictionary = (await import(`./locales/${locale}.json`)).default;

  return dict;
}
