import { SwipablePage, SwipablePageView } from "@/components/page";
import { Icons } from "@wealthfolio/ui";
import { useTranslation } from "react-i18next";
import { FundsTab } from "./components/fund-research-overview";
import { PortfolioLookthroughTab } from "./components/portfolio-lookthrough-tab";
import { ThemeExposureTab } from "./components/theme-exposure-tab";

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
      content: <ThemeExposureTab />,
    },
  ];

  return <SwipablePage views={views} defaultView="funds" />;
}
