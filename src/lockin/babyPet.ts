export const BABY_PET_DOCUMENT = "/pets/baby-overlay.html";

const SEED_BASE = 20260906;
const SEED_STEP = 7919;

export function babyPetSrc(index = 0): string {
  const params = new URLSearchParams({ seed: String(SEED_BASE + index * SEED_STEP) });
  return `${BABY_PET_DOCUMENT}?${params.toString()}`;
}

export function babyTaunt(url?: string | null): string {
  const host = hostname(url);
  return host ? `${host} again?` : "back to work.";
}

function hostname(url?: string | null): string {
  if (!url) return "";
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}
