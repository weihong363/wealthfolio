import { Separator } from "@wealthfolio/ui/components/ui/separator";
import { useTranslation } from "react-i18next";
import { usePlatform } from "@/hooks/use-platform";
import { SettingsHeader } from "../settings-header";
import { AutoUpdateSettings } from "./auto-update-settings";
import { BaseCurrencySettings } from "./currency-settings";
import { ExchangeRatesSettings } from "./exchange-rates/exchange-rates-settings";
import { TimezoneSettings } from "./timezone-settings";

export default function GeneralSettingsPage() {
  const { t } = useTranslation();
  const { isMobile } = usePlatform();

  return (
    <div className="space-y-6">
      <SettingsHeader
        heading={t("settings.general.title")}
        text={t("settings.general.description")}
      />
      <Separator />
      <BaseCurrencySettings />
      <TimezoneSettings />
      <div className="pt-6">
        <ExchangeRatesSettings />
      </div>
      {!isMobile && (
        <div className="pt-6">
          <AutoUpdateSettings />
        </div>
      )}
    </div>
  );
}
