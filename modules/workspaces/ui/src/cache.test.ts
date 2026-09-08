import assert from "node:assert/strict";
import test from "node:test";

import { InMemoryCache } from "@apollo/client";

import { ListRecentWorkspacesDocument } from "./generated/graphql.ts";
import {
  evictWorkspace,
  recentWorkspaceConvergence,
  workspacesTypePolicies,
} from "./cache.ts";

const workspace = {
  __typename: "Workspaces" as const,
  canonicalRoot: "/tmp/rust",
  createdAt: "2026-08-24T00:00:00Z",
  displayName: "rust",
  id: "workspace",
  lastOpenedAt: "2026-08-24T00:00:00Z",
};

test("create and touch refetch the ordered recent-workspaces list", () => {
  assert.deepEqual(recentWorkspaceConvergence(), {
    awaitRefetchQueries: true,
    refetchQueries: [ListRecentWorkspacesDocument],
  });
});

test("forget evicts the known workspace identity", () => {
  const cache = new InMemoryCache({ typePolicies: workspacesTypePolicies });
  cache.writeQuery({
    data: {
      workspaces: {
        __typename: "WorkspacesConnection",
        nodes: [workspace],
      },
    },
    query: ListRecentWorkspacesDocument,
  });

  const cacheId = cache.identify(workspace);
  assert.ok(cacheId);
  evictWorkspace(cache, workspace.id);
  assert.equal(cache.extract()[cacheId], undefined);
});
