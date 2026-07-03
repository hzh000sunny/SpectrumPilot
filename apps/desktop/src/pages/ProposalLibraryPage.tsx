import { Typography } from "antd";

export function ProposalLibraryPage() {
  return (
    <section className="section section-main">
      <Typography.Title level={5}>Proposal Library</Typography.Title>
      <Typography.Paragraph className="muted">
        Downloaded proposals and normalized metadata will be searchable from this page.
      </Typography.Paragraph>
    </section>
  );
}
