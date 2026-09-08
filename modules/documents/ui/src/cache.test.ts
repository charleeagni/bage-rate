// Cache Convergence for a write whose result is not the entity.
//
// `documentsSave` returns an outcome, not a `Documents`, so Apollo has nothing
// to normalize and the usual "converges through the returned entity" rule
// cannot apply. These tests pin the two halves of what the Module declared
// instead: a successful save writes its two changed fields onto the cached
// document by identity, and a stale save writes nothing.

import assert from "node:assert/strict";
import test from "node:test";

import { ApolloClient, ApolloLink, InMemoryCache, Observable } from "@apollo/client";

import {
  ListTaskDocumentsDocument,
  SaveDocumentDocument,
} from "./generated/graphql.ts";
import { applySavedDigest, documentsTypePolicies } from "./cache.ts";

const TASK_ID = "t";

const document = (id: string, relPath: string, contentDigest: string | null) => ({
  __typename: "Documents" as const,
  id,
  taskId: TASK_ID,
  scope: "design",
  rootDir: "/workspace",
  relPath,
  contentDigest,
  neverSaved: contentDigest === null,
  updatedAt: "2026-08-24T00:00:00Z",
});

const testClient = (held: string | null) => {
  const stored = [document("d1", "docs/design.md", held)];
  const link = new ApolloLink(
    (operation) =>
      new Observable((observer) => {
        queueMicrotask(() => {
          if (operation.operationName === "ListTaskDocuments") {
            observer.next({
              data: {
                documents: { __typename: "DocumentsConnection", nodes: [...stored] },
              },
            });
          } else if (operation.operationName === "SaveDocument") {
            const [target] = stored;
            assert.ok(target);
            const stale = (target.contentDigest ?? "") !== operation.variables.expectedDigest;
            observer.next({
              data: {
                documentsSave: {
                  __typename: "DocumentsSaveOutcome",
                  documentId: target.id,
                  digest: stale
                    ? (target.contentDigest ?? "")
                    : String(operation.variables.digest),
                  saved: !stale,
                  stale,
                },
              },
            });
          } else {
            observer.error(new Error(`unexpected operation ${operation.operationName}`));
            return;
          }
          observer.complete();
        });
      }),
  );

  return new ApolloClient({
    cache: new InMemoryCache({ typePolicies: documentsTypePolicies }),
    link,
  });
};

const cachedDocument = (client: ApolloClient) =>
  client.readQuery({
    query: ListTaskDocumentsDocument,
    variables: { taskId: TASK_ID },
  })?.documents.nodes[0];

const save = (client: ApolloClient, expectedDigest: string, digest: string) =>
  client.mutate({
    mutation: SaveDocumentDocument,
    variables: {
      documentId: "d1",
      expectedDigest,
      digest,
      savedAt: "2026-08-24T01:00:00Z",
    },
    update(cache, { data }) {
      if (data?.documentsSave.saved) {
        applySavedDigest(cache, "d1", data.documentsSave.digest, "2026-08-24T01:00:00Z");
      }
    },
  });

test("a successful save converges the cached document without refetching", async () => {
  const client = testClient(null);
  await client.query({
    query: ListTaskDocumentsDocument,
    variables: { taskId: TASK_ID },
    fetchPolicy: "network-only",
  });
  assert.equal(cachedDocument(client)?.neverSaved, true);

  await save(client, "", "digest-1");

  const converged = cachedDocument(client);
  assert.equal(converged?.contentDigest, "digest-1");
  assert.equal(converged?.updatedAt, "2026-08-24T01:00:00Z");
  assert.equal(converged?.neverSaved, false);
});

test("a stale save leaves the cache holding what the Store holds", async () => {
  const client = testClient("digest-1");
  await client.query({
    query: ListTaskDocumentsDocument,
    variables: { taskId: TASK_ID },
    fetchPolicy: "network-only",
  });

  const result = await save(client, "digest-stale", "digest-mine");

  assert.equal(result.data?.documentsSave.stale, true);
  assert.equal(result.data?.documentsSave.digest, "digest-1");
  assert.equal(cachedDocument(client)?.contentDigest, "digest-1");
});
