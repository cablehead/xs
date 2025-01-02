// Usage: <Link to="fjall" />

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
  if (!link) return null;

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
