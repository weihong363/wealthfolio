import { SwipablePage, SwipablePageView } from "@/components/page";
import { Icons } from "@wealthfolio/ui";
import { useTranslation } from "react-i18next";
import { FundsTab } from "./components/fund-research-overview";
import { PortfolioLookthroughTab } from "./components/portfolio-lookthrough-tab";

export default function FundResearchPage() {
  const { t } = useTranslation();

  const views: SwipablePageView[] = [
    {
      value: "funds",
      label: t("fundResearch.tabs.funds"),
      icon: Icons.TrendingUp,
      content: <FundsTab />,
    },
    {
      value: "lookthrough",
      label: t("fundResearch.tabs.portfolioLookthrough"),
      icon: Icons.PieChart,
      content: <PortfolioLookthroughTab />,
    },
    {
      value: "theme",
      label: t("fundResearch.tabs.themeExposure"),
      icon: Icons.BarChart,
      content: (
        <div className="flex h-64 items-center justify-center">
          <p className="text-muted-foreground text-sm">
            {t("fundResearch.themeExposureDesc")}
          </p>
        </div>
      ),
    },
  ];

  return <SwipablePage views={views} defaultView="funds" />;
}
