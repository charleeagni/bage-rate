import { ApolloClient, InMemoryCache, type TypePolicies } from "@apollo/client";
import { createTransportLink } from "virtual:target-transport";

import { modules } from "../generated/modules.ts";

// One composed cache, exactly as there is one composed schema: each Module
// contributes the policies for its own prefixed types. Shared type names —
// realistically only Query — merge field by field so no Module's field
// policies shadow another's.
function composedTypePolicies(): TypePolicies {
  const merged: TypePolicies = {};
  for (const module of modules) {
    for (const [typeName, policy] of Object.entries(module.typePolicies)) {
      const existing = merged[typeName];
      merged[typeName] = existing
        ? { ...existing, ...policy, fields: { ...existing.fields, ...policy.fields } }
        : policy;
    }
  }
  return merged;
}

export const apolloClient = new ApolloClient({
  link: createTransportLink(),
  cache: new InMemoryCache({ typePolicies: composedTypePolicies() }),
  defaultOptions: {
    watchQuery: { fetchPolicy: "cache-first" },
    query: { fetchPolicy: "network-only" },
  },
});
