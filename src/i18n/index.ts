import { en } from "./en";
import { zh } from "./zh";

export type MsgKey = keyof typeof en;
export type Locale = "en" | "zh";

export const LOCALES: Locale[] = ["en", "zh"];

const dictionaries: Record<Locale, Record<MsgKey, string>> = { en, zh };

let current: Locale = "en";

export function setLocale(locale: Locale): void {
  current = locale;
}

export function getLocale(): Locale {
  return current;
}

export function t(key: MsgKey, params?: Record<string, string | number>): string {
  const template = dictionaries[current][key] ?? en[key] ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = params[name];
    return value === undefined ? match : String(value);
  });
}
