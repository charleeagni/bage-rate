import { useMutation } from "@apollo/client/react";

import {
  CreateProjectDocument,
  DeleteProjectDocument,
  RenameProjectDocument,
} from "./generated/graphql.ts";

// Compile-time contract checks. This function is never rendered; `tsc`
// verifies that generated documents constrain Apollo's variable types.
export function generatedOperationTypeContract() {
  const [createProject] = useMutation(CreateProjectDocument);
  const [deleteProject] = useMutation(DeleteProjectDocument);
  const [renameProject] = useMutation(RenameProjectDocument);

  void createProject({
    variables: { name: "Alpha", workspaceRoot: "/tmp/alpha" },
  });
  void renameProject({ variables: { id: 1, name: "Renamed" } });
  void deleteProject({ variables: { id: 1 } });

  // @ts-expect-error workspaceRoot is required by the authored operation.
  void createProject({ variables: { name: "Missing root" } });
  // @ts-expect-error the caller-specific operation does not expose generated id input.
  void createProject({ variables: { id: 1, name: "Alpha", workspaceRoot: "/tmp/alpha" } });
  // @ts-expect-error id is an integer, not a string.
  void deleteProject({ variables: { id: "one" } });
  // @ts-expect-error the authored delete operation requires one concrete identity.
  void deleteProject({ variables: {} });
}
