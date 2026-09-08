//! What every Target's Transport module must supply, and nothing more.
//!
//! Downstream of this contract the application is Target-agnostic: the Apollo
//! client, the generated documents, and the Cache Convergence rules are shared
//! unchanged by the Desktop Target and the Web Target.

import type { ApolloLink } from "@apollo/client";

/**
 * Builds the terminating Apollo link a Target speaks over. It is a function
 * rather than a value so that a Transport can defer opening a connection until
 * the client is actually constructed.
 */
export type CreateTransportLink = () => ApolloLink;
