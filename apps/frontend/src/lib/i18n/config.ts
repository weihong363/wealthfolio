import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";

const resources = {
  "zh-CN": { translation: zhCN },
  en: { translation: en },
};

export const supportedLngs = ["zh-CN", "en"] as const;
export type SupportedLanguage = (typeof supportedLngs)[number];

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: "zh-CN",
    lng: "zh-CN",
    supportedLngs: supportedLngs as unknown as string[],
    debug: false,
    detection: {
      order: ["localStorage", "navigator"],
      caches: ["localStorage"],
      lookupLocalStorage: "wealthfolio-language",
    },
    interpolation: {
      escapeValue: false,
    },
  });

export default i18n;
