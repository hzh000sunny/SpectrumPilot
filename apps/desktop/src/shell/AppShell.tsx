import { useState } from "react";
import { MenuFoldOutlined, MenuUnfoldOutlined } from "@ant-design/icons";
import { Layout, Menu, Space, Typography } from "antd";
import { Outlet, useLocation, useNavigate } from "react-router-dom";

import { NAV_ITEMS, navItemFromKey, navKeyFromPath, type NavKey } from "./navigation";

const { Header, Sider, Content } = Layout;

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const selectedKey = navKeyFromPath(location.pathname);
  const currentItem = navItemFromKey(selectedKey);

  function handleNavigate(key: string) {
    const nextItem = navItemFromKey(key as NavKey);
    navigate(nextItem.path);
  }

  return (
    <Layout className="shell">
      <Sider
        className="sidebar"
        trigger={null}
        collapsible
        collapsed={collapsed}
        width={248}
      >
        <div className="brand">
          <div className="brand-mark">SP</div>
          {!collapsed && (
            <div className="brand-copy">
              <div className="brand-name">SpectrumPilot</div>
              <div className="brand-subtitle">Wireless Research Assistant</div>
            </div>
          )}
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[selectedKey]}
          items={NAV_ITEMS.map((item) => item.menuItem)}
          onClick={({ key }) => handleNavigate(key)}
        />
      </Sider>

      <Layout>
        <Header className="topbar">
          <Space size={12} align="center">
            <button
              type="button"
              className="icon-button"
              onClick={() => setCollapsed((value) => !value)}
              aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
            </button>
            <div>
              <Typography.Title level={4} className="page-title">
                {currentItem.title}
              </Typography.Title>
              <Typography.Text type="secondary" className="page-subtitle">
                {currentItem.subtitle}
              </Typography.Text>
            </div>
          </Space>
        </Header>

        <Content className="content">
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
