//! Where the Web Target reaches the Server Process.
//!
//! The paths are relative because the Web Target bundle and its endpoints are
//! served from one origin, which is what removes any API base URL to configure
//! and any CORS surface to review. Development keeps that same-origin model by
//! having the bundler proxy these exact paths to the Server Process, so this
//! module is the single definition both the client and `vite.config.ts` read.
//!
//! It must stay free of any browser or Node-only API, because the bundler
//! configuration imports it.

/** Queries and mutations, same-origin. */
export const HTTP_ENDPOINT = "/graphql";

/** Subscriptions over the standard `graphql-ws` protocol, same-origin. */
export const WEBSOCKET_ENDPOINT = "/graphql/ws";

/**
 * The Server Process as started by the Web Target development command: the
 * loopback default of the `--bind` flag, which is also `DEFAULT_BIND` in the
 * `web-server` crate. Only the bundler's development proxy uses it; nothing in
 * a built bundle does.
 *
 * The two declarations are held in agreement by
 * `scripts/cross-language-constants.test.mjs`, because no build step spans the
 * TypeScript and Rust halves of this coupling.
 */
export const DEVELOPMENT_SERVER_PROCESS_ORIGIN = "http://127.0.0.1:1421";
