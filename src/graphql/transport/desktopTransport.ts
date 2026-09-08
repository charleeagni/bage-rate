//! The Desktop Target's Transport: GraphQL over the Tauri IPC channel.
//!
//! Nothing here reaches the network, which is what keeps the Desktop Target
//! working with no server and no connection. This module is only present in
//! the Desktop bundle; see `targetSelection.ts`.

import { createTauriGraphQlLink } from "@tauri-graphql-template/apollo";

import { createTauRPCProxy } from "../../generated/taurpc.ts";
import type { CreateTransportLink } from "./transportContract.ts";

export const createTransportLink: CreateTransportLink = () =>
  createTauriGraphQlLink(createTauRPCProxy);
