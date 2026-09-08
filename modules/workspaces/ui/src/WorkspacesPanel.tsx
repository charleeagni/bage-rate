import { useMutation, useQuery } from "@apollo/client/react";

import {
  ForgetWorkspaceDocument,
  ListRecentWorkspacesDocument,
} from "./generated/graphql.ts";
import { evictWorkspace } from "./cache.ts";
import { LaunchScreen, type RecentWorkspace } from "./LaunchScreen.tsx";

export interface WorkspacesPanelProps {
  unavailable?: ReadonlySet<string>;
  onChooseNewFolder?: (workspace: RecentWorkspace) => void;
  onOpenFolder?: () => void;
  onReopen?: (workspace: RecentWorkspace) => void;
}

/**
 * The persistent picker half. A host supplies folder capability callbacks;
 * without them the mounted Module still lists and forgets recent folders.
 */
export function WorkspacesPanel({
  unavailable = new Set<string>(),
  onChooseNewFolder,
  onOpenFolder,
  onReopen,
}: WorkspacesPanelProps = {}) {
  const recent = useQuery(ListRecentWorkspacesDocument);
  const [forget, forgetState] = useMutation(ForgetWorkspaceDocument);
  const busy = forgetState.loading;

  const forgetWorkspace = (workspace: RecentWorkspace) => {
    void forget({
      variables: { id: workspace.id },
      update(cache, result) {
        if (result.data) evictWorkspace(cache, workspace.id);
      },
    }).catch(() => undefined);
  };

  return (
    <LaunchScreen
      busy={busy}
      canOpenFolders={Boolean(onOpenFolder && onReopen && onChooseNewFolder)}
      error={recent.error?.message ?? forgetState.error?.message ?? null}
      loading={recent.loading && !recent.data}
      onChooseNewFolder={(workspace) => onChooseNewFolder?.(workspace)}
      onForget={forgetWorkspace}
      onOpenFolder={() => onOpenFolder?.()}
      onReopen={(workspace) => onReopen?.(workspace)}
      onRetry={() => void recent.refetch()}
      recent={[...(recent.data?.workspaces.nodes ?? [])]}
      unavailable={unavailable}
    />
  );
}
