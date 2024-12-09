import * as i18n from "@solid-primitives/i18n";
import { BaseDict } from "@solid-primitives/i18n";
import { createResource, createSignal } from "solid-js";

import en_ca from "./locales/en-CA.json"; // en_ca has the base keys

const rawLocales = ["en-CA", "en-US"] as const; // fully translated locales
export type Locale = (typeof rawLocales)[number];

export type RawDictionary = typeof en_ca;
export type Dictionary = i18n.Flatten<RawDictionary>;

function flattenDict(dict: RawDictionary) {
  return i18n.flatten(dict) as Flatten<Exclude<RawDictionary, undefined>>;
}

export async function fetchDictionary(locale: Locale) {
  const dict: RawDictionary = (await import(`./locales/${locale}.json`)).default;

  return flattenDict(dict);
}

export const [locale, setLocale] = createSignal<Locale>("en-CA");
export const [dict] = createResource(locale, fetchDictionary, { initialValue: flattenDict(en_ca) });
export const t = i18n.translator(dict, i18n.resolveTemplate);

// Workaround for Typescript static analysis bug
type UnionToIntersection<U> = (U extends any ? (k: U) => void : never) extends (k: infer I) => void ? I : never;
type JoinPath<A, B> = A extends string | number ? (B extends string | number ? `${A}.${B}` : A) : B extends string | number ? B : "";
type Flatten<T extends BaseDict, P = {}> = UnionToIntersection<
  {
    [K in keyof T]: T[K] extends BaseDict ? Flatten<T[K], JoinPath<P, K>> : never;
  }[keyof T]
> & {
  readonly [K in keyof T as JoinPath<P, K>]: T[K];
};
