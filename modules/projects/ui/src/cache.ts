import { type ApolloCache, type TypePolicies } from "@apollo/client";

import { ListProjectsDocument } from "./generated/graphql.ts";

/** This Module's contribution to the one composed Apollo cache. */
export const projectsTypePolicies: TypePolicies = {
  Projects: { keyFields: ["id"] },
  ProjectsConnection: { keyFields: false },
  ProjectsEdge: { keyFields: false },
  PageInfo: { keyFields: false },
  PaginationInfo: { keyFields: false },
  Query: {
    fields: {
      projects: { merge: false },
    },
  },
};

// Entity normalization updates field values but cannot restore server-defined
// ordering when a mutation changes a sort key. Callers use this explicit
// fallback for creates and sort-key updates.
export const projectListConvergence = () => ({
  awaitRefetchQueries: true,
  refetchQueries: [ListProjectsDocument],
});

export function evictProject(cache: ApolloCache, id: number) {
  cache.evict({ id: cache.identify({ __typename: "Projects", id }) });
  cache.gc();
}
