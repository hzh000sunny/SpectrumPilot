import { Typography } from "antd";

export function KeywordWatchlistPage() {
  return (
    <section className="section section-main">
      <Typography.Title level={5}>Keyword Watchlist</Typography.Title>
      <Typography.Paragraph className="muted">
        Saved keywords, companies, work items, and meeting watches will be managed here.
      </Typography.Paragraph>
    </section>
  );
}
