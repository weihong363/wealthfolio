import { getDynamicNavItems, subscribeToNavigationUpdates } from "@/addons/addons-runtime-context";
import { Icons } from "@wealthfolio/ui/components/ui/icons";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

export interface NavLink {
  title: string;
  href: string;
  icon?: React.ReactNode;
  keywords?: string[];
  label?: string; // Optional descriptive label for launcher/search
}

export interface NavigationProps {
  primary: NavLink[];
  secondary?: NavLink[];
  addons?: NavLink[];
}

export function useNavigation() {
  const { t } = useTranslation();
  const [dynamicItems, setDynamicItems] = useState<NavigationProps["addons"]>([]);

  const staticNavigation: NavigationProps = useMemo(
    () => ({
      primary: [
        {
          icon: <Icons.Dashboard className="size-6" />,
          title: t("nav.dashboard"),
          href: "/dashboard",
          keywords: ["home", "overview", "summary"],
          label: t("nav.viewDashboard"),
        },
        {
          icon: <Icons.Insight className="size-6" />,
          title: t("nav.insights"),
          href: "/insights",
          keywords: ["insights", "Analytics"],
          label: t("nav.viewInsights"),
        },
        {
          icon: <Icons.HandCoins className="size-6" />,
          title: t("nav.fundResearch"),
          href: "/fund-research",
          keywords: ["fund", "research", "holdings", "lookthrough"],
          label: t("nav.viewFundResearch"),
        },
        {
          icon: <Icons.Holdings className="size-6" />,
          title: t("nav.holdings"),
          href: "/holdings",
          keywords: ["Holdings", "portfolio", "assets", "positions", "stocks"],
          label: t("nav.viewHoldings"),
        },
        {
          icon: <Icons.Activity className="size-6" />,
          title: t("nav.activities"),
          href: "/activities",
          keywords: ["transactions", "trades", "history"],
          label: t("nav.viewActivities"),
        },
        {
          icon: <Icons.Goals className="size-6" />,
          title: t("nav.goals"),
          href: "/goals",
          keywords: ["goals", "fire", "retire", "retirement", "savings", "planner"],
          label: t("nav.goals"),
        },
        {
          icon: <Icons.Sparkles className="size-6" />,
          title: t("nav.assistant"),
          href: "/assistant",
          keywords: ["ai", "assistant", "chat", "help", "ask"],
          label: t("nav.assistant"),
        },
      ],
      secondary: [
        {
          icon: <Icons.Settings className="size-6" />,
          title: t("nav.settings"),
          href: "/settings",
          keywords: ["preferences", "config", "configuration"],
        },
      ],
    }),
    [t],
  );

  // Subscribe to navigation updates from addons
  useEffect(() => {
    const updateDynamicItems = () => {
      const itemsFromRuntime = getDynamicNavItems();
      setDynamicItems(itemsFromRuntime);
    };

    // Initial load
    updateDynamicItems();

    // Subscribe to updates
    const unsubscribe = subscribeToNavigationUpdates(updateDynamicItems);

    return () => {
      unsubscribe();
    };
  }, []);

  // Spending lives entirely on the dashboard tab (and its deep-linked pages);
  // no top-level nav entry. Combine static navigation items with addons.
  const primary = [...staticNavigation.primary];

  const navigation: NavigationProps = {
    primary,
    secondary: staticNavigation.secondary,
    addons: dynamicItems,
  };

  return navigation;
}

export function isPathActive(pathname: string, href: string): boolean {
  if (!href) {
    return false;
  }

  const ensureLeadingSlash = href.startsWith("/") ? href : `/${href}`;
  const normalize = (value: string) => {
    if (value.length > 1 && value.endsWith("/")) {
      return value.slice(0, -1);
    }
    return value;
  };

  const normalizedHref = normalize(ensureLeadingSlash);
  const normalizedPath = normalize(pathname);

  if (normalizedHref === "/") {
    return normalizedPath === "/";
  }

  // Dashboard and Net Worth are grouped together
  if (normalizedHref === "/dashboard") {
    return (
      normalizedPath === "/" || normalizedPath === "/dashboard" || normalizedPath === "/net-worth"
    );
  }

  return normalizedPath === normalizedHref || normalizedPath.startsWith(`${normalizedHref}/`);
}
