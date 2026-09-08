import { type TypePolicies } from "@apollo/client";
import { type ComponentType } from "react";

import { workspacesTypePolicies } from "./cache.ts";
import { WorkspacesPanel } from "./WorkspacesPanel.tsx";

export { recentWorkspaceConvergence } from "./cache.ts";
export { LaunchScreen, type RecentWorkspace } from "./LaunchScreen.tsx";
export { WorkspacesPanel, type WorkspacesPanelProps } from "./WorkspacesPanel.tsx";

// Every Module's ui half exports one `moduleDef` of this shape; the
// generated src/generated/modules.ts imports it by convention, exactly as the
// generated registry imports each Rust half's `module_def()`.
export const moduleDef: {
  name: string;
  typePolicies: TypePolicies;
  Component: ComponentType;
} = {
  name: "workspaces",
  typePolicies: workspacesTypePolicies,
  Component: WorkspacesPanel,
};
