// Which window this page is running in. The overlay window loads
// `index.html?view=overlay`, the tray's settings window `?view=settings`;
// every other window shows the control panel.
export type View = "overlay" | "settings" | "panel";

export function currentView(search: string = window.location.search): View {
  const view = new URLSearchParams(search).get("view");
  return view === "overlay" || view === "settings" ? view : "panel";
}
