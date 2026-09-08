import { type ApolloCache, type TypePolicies } from "@apollo/client";

import { ListRecentWorkspacesDocument } from "./generated/graphql.ts";

/** This Module's contribution to the one composed Apollo cache. */
export const workspacesTypePolicies: TypePolicies = {
  Workspaces: { keyFields: ["id"] },
  WorkspacesConnection: { keyFields: false },
  WorkspacesEdge: { keyFields: false },
  PageInfo: { keyFields: false },
  PaginationInfo: { keyFields: false },
  Query: {
    fields: {
      workspaces: { merge: false },
    },
  },
};

// Create and touch can change membership or last-opened ordering.
export const recentWorkspaceConvergence = () => ({
  awaitRefetchQueries: true,
  refetchQueries: [ListRecentWorkspacesDocument],
});

export function evictWorkspace(cache: ApolloCache, id: string) {
  cache.evict({ id: cache.identify({ __typename: "Workspaces", id }) });
  cache.gc();
}
