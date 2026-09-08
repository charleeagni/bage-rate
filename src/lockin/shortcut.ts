// Physical letter/digit keys keep recording stable across keyboard layouts.
export function shortcutFromKey(event: Pick<KeyboardEvent, "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">): string | null {
  const key = /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4]))$/.test(event.code)
    ? event.code.replace(/^(Key|Digit)/, "")
    : ({ Space: "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" } as Record<string, string>)[event.code];
  if (!key || !(event.metaKey || event.ctrlKey || event.altKey)) return null;
  return [event.metaKey && "Super", event.ctrlKey && "Control", event.altKey && "Alt", event.shiftKey && "Shift", key].filter(Boolean).join("+");
}
export function shortcutLabel(shortcut: string): string {
  return shortcut.replace(/CommandOrControl|CmdOrCtrl/g, "⌘/Ctrl").replace(/Super/g, "⌘").replace(/Control/g, "Ctrl").replace(/Alt/g, "Option");
}
