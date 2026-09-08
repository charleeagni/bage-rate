import { type TypePolicies } from "@apollo/client";
import { type ComponentType } from "react";

import { documentsTypePolicies } from "./cache.ts";
import { DocumentsPanel } from "./DocumentsPanel.tsx";

// Every Module's ui half exports one `moduleDef` of this shape; the generated
// src/generated/modules.ts imports it by convention, exactly as the generated
// registry imports each Rust half's `module_def()`.
export const moduleDef: {
  name: string;
  typePolicies: TypePolicies;
  Component: ComponentType;
} = {
  name: "documents",
  typePolicies: documentsTypePolicies,
  Component: DocumentsPanel,
};
