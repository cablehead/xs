// Usage: <Link to="fjall" />
//
// This component is only for reusable EXTERNAL links keyed by the short
// names below. For internal page links use a relative markdown link
// instead, e.g. [text](../section/page/). Passing an unknown key (such as
// an internal route) warns at build time and renders the key verbatim so
// the mistake is visible rather than silently swallowed.

const links = [
  ["fjall", "fjall", "https://github.com/fjall-rs/fjall"],
  ["nu", "Nushell", "https://www.nushell.sh"],
];

const linkMap = new Map(links.map(([short, desc, link]) => [
  short,
  { desc, link },
]));

export const Link = ({ to }) => {
  const link = linkMap.get(to);
  if (!link) {
    console.warn(
      `[Link] no entry for "${to}". Known keys: ${[...linkMap.keys()].join(", ")}. ` +
        `For an internal page link use a relative markdown link, e.g. [text](../section/page/).`,
    );
    return <code>{to}</code>;
  }

  return (
    <a
      href={link.link}
      target="_blank"
      rel="noopener noreferrer"
      title={link.desc}
    >
      <code>{link.desc}</code>
    </a>
  );
};
