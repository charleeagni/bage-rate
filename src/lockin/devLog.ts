import { emit } from "@tauri-apps/api/event";

export function devLog(message: string): void {
  if (!import.meta.env.DEV) return;
  void emit("lockin:log", message).catch(() => {});
}

export function devLogError(context: string): (reason: unknown) => void {
  return (reason: unknown) => devLog(`${context}: ${String(reason)}`);
}
