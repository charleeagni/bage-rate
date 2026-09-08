// The module the bundler resolves to one Target's Transport at build time.
// Type checking sees the contract rather than either Target's implementation,
// so nothing downstream can come to depend on one Target's Transport; each
// implementation is checked against the same contract in its own module.

declare module "virtual:target-transport" {
  export const createTransportLink: import("./transportContract.ts").CreateTransportLink;
}
