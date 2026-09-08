import { type ApolloCache, type TypePolicies } from "@apollo/client";

import { ListTaskDocumentsDocument } from "./generated/graphql.ts";

/** This Module's contribution to the one composed Apollo cache. */
export const documentsTypePolicies: TypePolicies = {
  Documents: { keyFields: ["id"] },
  DocumentsConnection: { keyFields: false },
  DocumentsEdge: { keyFields: false },
  // The save outcome, the check, and the saved event are values, not
  // entities: two answers about the same document are two distinct answers,
  // so none of them is normalized.
  DocumentsSaveCheck: { keyFields: false },
  DocumentsSaveOutcome: { keyFields: false },
  DocumentsSavedEvent: { keyFields: false },
  PageInfo: { keyFields: false },
  PaginationInfo: { keyFields: false },
  Query: {
    fields: {
      documents: { merge: false },
    },
  },
};

// The list is filtered by taskId and ordered by relPath, so registering a
// document can change membership and has to refetch.
export const documentListConvergence = (taskId: string) => ({
  awaitRefetchQueries: true,
  refetchQueries: [{ query: ListTaskDocumentsDocument, variables: { taskId } }],
});

// A custom operation returns its own outcome type, not the entity, so the
// usual "converges through the returned entity" rule cannot apply: nothing in
// the result is normalizable. The write touches neither the list's filter
// (taskId) nor its sort key (relPath), so refetching would be waste — the
// declared convergence is to write the two changed fields onto the cached
// document by identity.
export function applySavedDigest(
  cache: ApolloCache,
  documentId: string,
  digest: string,
  savedAt: string,
) {
  cache.modify({
    id: cache.identify({ __typename: "Documents", id: documentId }),
    fields: {
      contentDigest: () => digest,
      updatedAt: () => savedAt,
      neverSaved: () => false,
    },
  });
}

export function evictDocument(cache: ApolloCache, id: string) {
  cache.evict({ id: cache.identify({ __typename: "Documents", id }) });
  cache.gc();
}
