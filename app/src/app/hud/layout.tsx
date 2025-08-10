export default function HudLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  // Nested layouts must not render <html> or <body>; keep this a server component.
  return (
    <div className="w-screen h-screen overflow-hidden bg-transparent antialiased font-sans">
      {children}
    </div>
  );
}