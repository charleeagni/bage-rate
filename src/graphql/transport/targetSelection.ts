//! Which Transport module a bundle is built from.
//!
//! The choice is made once, by the bundler, from a build-time flag — never by
//! sniffing for a Tauri global at runtime. The bundler resolves the module
//! identifier below to exactly one of the Transport modules, so the other
//! Target's Transport is not in the module graph at all and cannot be
//! delivered to the wrong Target.
//!
//! This module is imported by `vite.config.ts`; it must stay free of any
//! browser or Node-only API.

/** The module identifier the application imports its Transport from. */
export const SELECTED_TRANSPORT_MODULE_ID = "virtual:target-transport";

/** The build-time flag naming the Target. */
export const TARGET_ENV_VAR = "APP_TARGET";

export const TARGETS = ["desktop", "web"] as const;

export type Target = (typeof TARGETS)[number];

/** The Desktop Target is the historical build, so an unset flag still means it. */
export const DEFAULT_TARGET: Target = "desktop";

const TRANSPORT_MODULE_BY_TARGET: Record<Target, string> = {
  desktop: "./src/graphql/transport/desktopTransport.ts",
  web: "./src/graphql/transport/webTransport.ts",
};

const isTarget = (value: string): value is Target =>
  (TARGETS as readonly string[]).includes(value);

/**
 * Refuses an unrecognised Target rather than falling back, because a typo that
 * silently produced the other Target's bundle is the failure this whole
 * mechanism exists to prevent.
 */
export const parseTarget = (value: string | undefined): Target => {
  if (value === undefined || value === "") return DEFAULT_TARGET;
  if (isTarget(value)) return value;
  throw new Error(
    `${TARGET_ENV_VAR} must be one of ${TARGETS.join(", ")}, but was ${JSON.stringify(value)}`,
  );
};

/** The repository-relative Transport module the given Target is built from. */
export const transportModuleForTarget = (target: Target): string =>
  TRANSPORT_MODULE_BY_TARGET[target];
