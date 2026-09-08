import type { ListRecentWorkspacesQuery } from "./generated/graphql.ts";

export type RecentWorkspace =
  ListRecentWorkspacesQuery["workspaces"]["nodes"][number];

interface LaunchScreenProps {
  busy: boolean;
  canOpenFolders: boolean;
  error: string | null;
  loading: boolean;
  recent: RecentWorkspace[];
  unavailable: ReadonlySet<string>;
  onChooseNewFolder: (workspace: RecentWorkspace) => void;
  onForget: (workspace: RecentWorkspace) => void;
  onOpenFolder: () => void;
  onReopen: (workspace: RecentWorkspace) => void;
  onRetry: () => void;
}

export function LaunchScreen({
  busy,
  canOpenFolders,
  error,
  loading,
  recent,
  unavailable,
  onChooseNewFolder,
  onForget,
  onOpenFolder,
  onReopen,
  onRetry,
}: LaunchScreenProps) {
  return (
    <section aria-labelledby="workspaces-heading" className="panel">
      <div>
        <p className="panel-eyebrow">Read-only folder browser</p>
        <h2 id="workspaces-heading">Recent folders</h2>
        <p>
          Browse any folder as a read-only file tree. Cargo roots get automatic
          Rust folder grouping.
        </p>
        <button
          disabled={busy || !canOpenFolders}
          onClick={onOpenFolder}
          type="button"
        >
          Open folder
        </button>
      </div>

      <div>
        {error ? (
          <>
            <p className="error" role="alert">
              {error}
            </p>
            <button className="secondary" onClick={onRetry} type="button">
              Retry recent folders
            </button>
          </>
        ) : null}
        {loading ? <p aria-live="polite">Loading recent folders...</p> : null}
        {!loading && !error && recent.length === 0 ? (
          <p className="empty">No recent folders yet.</p>
        ) : null}
        <ul className="record-list">
          {recent.map((workspace) => {
            const missing = unavailable.has(workspace.id);
            return (
              <li key={workspace.id}>
                <div>
                  <strong>{workspace.displayName}</strong>
                  <code>{workspace.canonicalRoot}</code>
                  {missing ? <span>Unavailable</span> : null}
                </div>
                <div className="actions">
                  {missing ? (
                    <button
                      className="secondary"
                      disabled={busy || !canOpenFolders}
                      onClick={() => onChooseNewFolder(workspace)}
                      type="button"
                    >
                      Choose new folder
                    </button>
                  ) : (
                    <button
                      className="secondary"
                      disabled={busy || !canOpenFolders}
                      onClick={() => onReopen(workspace)}
                      type="button"
                    >
                      Open
                    </button>
                  )}
                  <button
                    aria-label={`Forget ${workspace.displayName}`}
                    className="danger"
                    disabled={busy}
                    onClick={() => onForget(workspace)}
                    type="button"
                  >
                    Forget
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
