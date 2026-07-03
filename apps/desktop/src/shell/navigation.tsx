import {
  DatabaseOutlined,
  FileSearchOutlined,
  FolderOpenOutlined,
  SettingOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import type { ItemType } from "antd/es/menu/interface";

export type NavKey = "dashboard" | "3gpp" | "library" | "watchlist" | "settings";

export type NavItem = {
  key: NavKey;
  path: string;
  title: string;
  subtitle: string;
  menuItem: ItemType;
};

export const NAV_ITEMS: NavItem[] = [
  {
    key: "dashboard",
    path: "/dashboard",
    title: "Dashboard",
    subtitle: "Research workspace overview",
    menuItem: {
      key: "dashboard",
      icon: <DatabaseOutlined />,
      label: "Dashboard",
    },
  },
  {
    key: "3gpp",
    path: "/3gpp",
    title: "3GPP Ftp",
    subtitle: "3GPP FTP document lookup and download",
    menuItem: {
      key: "3gpp",
      icon: <FolderOpenOutlined />,
      label: "3GPP Ftp",
    },
  },
  {
    key: "library",
    path: "/library",
    title: "Proposal Library",
    subtitle: "Downloaded and indexed proposal files",
    menuItem: {
      key: "library",
      icon: <FileSearchOutlined />,
      label: "Proposal Library",
    },
  },
  {
    key: "watchlist",
    path: "/watchlist",
    title: "Keyword Watchlist",
    subtitle: "Saved monitoring rules and research signals",
    menuItem: {
      key: "watchlist",
      icon: <ThunderboltOutlined />,
      label: "Keyword Watchlist",
    },
  },
  {
    key: "settings",
    path: "/settings",
    title: "Settings",
    subtitle: "Application paths, updates, and preferences",
    menuItem: {
      key: "settings",
      icon: <SettingOutlined />,
      label: "Settings",
    },
  },
];

export function navKeyFromPath(pathname: string): NavKey {
  const item = NAV_ITEMS.find((entry) => pathname.startsWith(entry.path));
  return item?.key ?? "3gpp";
}

export function navItemFromKey(key: NavKey): NavItem {
  return NAV_ITEMS.find((item) => item.key === key) ?? NAV_ITEMS[1];
}
