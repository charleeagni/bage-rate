//! The Web Target's Transport: HTTP for queries and mutations, `graphql-ws`
//! for subscriptions, both against the Server Process that served the page.
//!
//! The endpoints are relative, so the browser resolves them against the origin
//! the application was loaded from. That is what removes any API base URL to
//! configure and any CORS surface to review. The endpoints themselves live in
//! `webEndpoints.ts`, because the bundler's development proxy must forward
//! exactly the paths this Transport asks for. This module is only present in
//! the Web bundle; see `targetSelection.ts`.

import { split } from "@apollo/client";
import { HttpLink } from "@apollo/client/link/http";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { getMainDefinition } from "@apollo/client/utilities";
import { createClient } from "graphql-ws";

import type { CreateTransportLink } from "./transportContract.ts";
import { HTTP_ENDPOINT, WEBSOCKET_ENDPOINT } from "./webEndpoints.ts";

/**
 * The WebSocket constructor needs a `ws:`/`wss:` URL, so the relative endpoint
 * is resolved against the page's own origin and its scheme swapped. Reading
 * this per connection rather than once at module load keeps the Transport
 * correct if the page is served from more than one origin.
 */
export const websocketEndpointUrl = (documentUrl: string): string => {
  const endpoint = new URL(WEBSOCKET_ENDPOINT, documentUrl);
  endpoint.protocol = endpoint.protocol === "https:" ? "wss:" : "ws:";
  return endpoint.toString();
};

const isSubscription = (operation: { query: Parameters<typeof getMainDefinition>[0] }) => {
  const definition = getMainDefinition(operation.query);
  return (
    definition.kind === "OperationDefinition" && definition.operation === "subscription"
  );
};

export const createTransportLink: CreateTransportLink = () =>
  split(
    isSubscription,
    new GraphQLWsLink(
      createClient({ url: () => websocketEndpointUrl(window.location.href) }),
    ),
    new HttpLink({ uri: HTTP_ENDPOINT }),
  );
