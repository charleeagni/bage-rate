import assert from "node:assert/strict";
import test from "node:test";

import { ApolloClient, InMemoryCache, gql } from "@apollo/client";
import type { TypedDocumentNode } from "@apollo/client";

import {
  createTauriGraphQlLink,
  type GraphQlTransportProxy,
} from "../src/index.ts";

const PING = gql`
  query Ping {
    ping
  }
` as TypedDocumentNode<{ ping: string }>;

const TICKS = gql`
  subscription Ticks {
    ticks
  }
` as TypedDocumentNode<{ ticks: number }>;

test("queries cross the unary transport as GraphQL JSON", async () => {
  let captured = "";
  const proxy: GraphQlTransportProxy = {
    async graphql_execute(requestJson) {
      captured = requestJson;
      return JSON.stringify({ data: { ping: "pong" } });
    },
    async graphql_subscribe() {
      throw new Error("not used");
    },
    async graphql_unsubscribe() {
      return false;
    },
  };
  const client = new ApolloClient({
    link: createTauriGraphQlLink(() => proxy),
    cache: new InMemoryCache(),
  });

  const result = await client.query({ query: PING });
  assert.equal(result.data?.ping, "pong");
  const request = JSON.parse(captured) as {
    query: string;
    operationName: string;
  };
  assert.match(request.query, /query Ping/);
  assert.equal(request.operationName, "Ping");
});

test("subscriptions forward channel events and unsubscribe explicitly", async () => {
  let callback: ((event: string) => void) | undefined;
  let activeId = "";
  const stopped: string[] = [];
  const proxy: GraphQlTransportProxy = {
    async graphql_execute() {
      throw new Error("not used");
    },
    async graphql_subscribe(id, _request, onEvent) {
      activeId = id;
      callback = onEvent;
      return JSON.stringify({ type: "accepted" });
    },
    async graphql_unsubscribe(id) {
      stopped.push(id);
      return true;
    },
  };
  const client = new ApolloClient({
    link: createTauriGraphQlLink(() => proxy),
    cache: new InMemoryCache(),
  });

  const received = new Promise<number>((resolve, reject) => {
    const subscription = client.subscribe({ query: TICKS }).subscribe({
      next(result) {
        resolve(result.data?.ticks ?? -1);
        subscription.unsubscribe();
      },
      error: reject,
    });
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(callback);
  callback(
    JSON.stringify({
      type: "next",
      payload: { data: { ticks: 1 } },
    }),
  );

  assert.equal(await received, 1);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(stopped, [activeId]);
});

test("cancellation during subscribe acknowledgement cannot leak a Rust stream", async () => {
  let resolveAcknowledgement: ((value: string) => void) | undefined;
  let activeId = "";
  const stopped: string[] = [];
  const proxy: GraphQlTransportProxy = {
    async graphql_execute() {
      throw new Error("not used");
    },
    graphql_subscribe(id) {
      activeId = id;
      return new Promise((resolve) => {
        resolveAcknowledgement = resolve;
      });
    },
    async graphql_unsubscribe(id) {
      stopped.push(id);
      return true;
    },
  };
  const client = new ApolloClient({
    link: createTauriGraphQlLink(() => proxy),
    cache: new InMemoryCache(),
  });

  const subscription = client.subscribe({ query: TICKS }).subscribe();
  subscription.unsubscribe();
  assert.ok(resolveAcknowledgement);
  resolveAcknowledgement(JSON.stringify({ type: "accepted" }));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.deepEqual(stopped, [activeId]);
});
