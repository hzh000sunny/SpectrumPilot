import { Typography } from "antd";

export function DashboardPage() {
  return (
    <section className="section section-main">
      <Typography.Title level={5}>Dashboard</Typography.Title>
      <Typography.Paragraph className="muted">
        Workspace metrics and recent research activity will be summarized here.
      </Typography.Paragraph>
    </section>
  );
}
