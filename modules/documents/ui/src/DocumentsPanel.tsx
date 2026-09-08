import { useState, type FormEvent } from "react";
import {
  useLazyQuery,
  useMutation,
  useQuery,
  useSubscription,
} from "@apollo/client/react";

import {
  CheckDocumentSaveDocument,
  ForgetDocumentDocument,
  ListTaskDocumentsDocument,
  OnDocumentSavedDocument,
  RegisterDocumentDocument,
  SaveDocumentDocument,
} from "./generated/graphql.ts";
import {
  applySavedDigest,
  documentListConvergence,
  evictDocument,
} from "./cache.ts";

const TASK_ID = "demo-task";

// Stand-ins for whatever really produces a document's content and its
// digest. The point of the panel is the write protocol, not the hashing —
// but the check and the save have to agree on what the content is, or the
// check would be answering about a document nobody is about to save.
const contentOf = (relPath: string) => `# ${relPath}\n`;
const digestOf = (content: string) =>
  Array.from(content).reduce((hash, character) => (hash * 31 + character.charCodeAt(0)) | 0, 7)
    .toString(16);

export function DocumentsPanel() {
  const documents = useQuery(ListTaskDocumentsDocument, {
    variables: { taskId: TASK_ID },
  });
  const [registerDocument, registerState] = useMutation(
    RegisterDocumentDocument,
    documentListConvergence(TASK_ID),
  );
  const [saveDocument, saveState] = useMutation(SaveDocumentDocument);
  const [forgetDocument, forgetState] = useMutation(ForgetDocumentDocument);
  // A check is a point-in-time answer about the Store, so it is read from the
  // network rather than from a cache that may be holding a previous one.
  const [checkSave, checkState] = useLazyQuery(CheckDocumentSaveDocument, {
    fetchPolicy: "network-only",
  });
  const saved = useSubscription(OnDocumentSavedDocument);

  const [relPath, setRelPath] = useState("");
  const [conflict, setConflict] = useState<string | null>(null);
  const [preflight, setPreflight] = useState<string | null>(null);

  const register = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const path = relPath.trim();
    if (!path) return;
    const now = new Date().toISOString();
    await registerDocument({
      variables: {
        id: `${TASK_ID}:${path}`,
        taskId: TASK_ID,
        scope: "design",
        rootDir: "/workspace",
        relPath: path,
        now,
      },
    });
    setRelPath("");
  };

  const check = async (id: string, content: string) => {
    const result = await checkSave({
      variables: { documentId: id, digest: digestOf(content) },
    });
    const answer = result.data?.documentsSaveCheck;
    if (!answer) return;
    setPreflight(
      !answer.known
        ? `${id} is not registered`
        : answer.upToDate
          ? `${id} already holds this content`
          : `${id} holds ${answer.held ?? "nothing yet"}; saving would write`,
    );
  };

  const save = async (id: string, held: string | null, content: string) => {
    const savedAt = new Date().toISOString();
    const result = await saveDocument({
      variables: {
        documentId: id,
        expectedDigest: held ?? "",
        digest: digestOf(content),
        savedAt,
      },
      update(cache, { data }) {
        if (data?.documentsSave.saved) {
          applySavedDigest(cache, id, data.documentsSave.digest, savedAt);
        }
      },
    });
    // The conflict is data, so it is read from the result rather than caught.
    setConflict(
      result.data?.documentsSave.stale
        ? `${id} moved on; it now holds ${result.data.documentsSave.digest}`
        : null,
    );
  };

  const forget = async (id: string) => {
    await forgetDocument({
      variables: { id },
      update(cache, result) {
        if (result.data) evictDocument(cache, id);
      },
    });
  };

  const error =
    documents.error ??
    registerState.error ??
    checkState.error ??
    saveState.error ??
    forgetState.error;
  const busy = checkState.loading || saveState.loading || forgetState.loading;

  return (
    <>
      <section aria-labelledby="register-document-heading" className="panel">
        <div>
          <p className="panel-eyebrow">Documents</p>
          <h2 id="register-document-heading">Register a document</h2>
        </div>
        <form onSubmit={(event) => void register(event).catch(() => undefined)}>
          <label>
            Path under the root
            <input
              name="relPath"
              onChange={(event) => setRelPath(event.target.value)}
              placeholder="docs/design.md"
              required
              value={relPath}
            />
          </label>
          <button disabled={registerState.loading} type="submit">
            {registerState.loading ? "Registering…" : "Register document"}
          </button>
        </form>
        <p className="empty">
          A path that is absolute or climbs out of the root is rejected by the
          Module&rsquo;s write rule, before anything is written.
        </p>
      </section>

      <section aria-labelledby="documents-heading" className="panel">
        <div>
          <p className="panel-eyebrow">Documents</p>
          <h2 id="documents-heading">Saved documents</h2>
        </div>

        {error ? (
          <p className="error" role="alert">
            {error.message}
          </p>
        ) : null}
        {conflict ? (
          <p className="error" role="status">
            {conflict}
          </p>
        ) : null}
        {preflight ? <p aria-live="polite">{preflight}</p> : null}
        {saved.data ? (
          <p aria-live="polite">
            Last save seen live: {saved.data.documentsSaved.documentId} at{" "}
            {saved.data.documentsSaved.savedAt}
          </p>
        ) : null}
        {documents.data?.documents.nodes.length === 0 ? (
          <p className="empty">No documents registered for this task yet.</p>
        ) : null}

        <ul className="record-list">
          {documents.data?.documents.nodes.map((document) => (
            <li key={document.id}>
              <div>
                <strong>{document.relPath}</strong>
                <code>
                  {document.neverSaved
                    ? "never saved"
                    : `digest ${document.contentDigest}`}
                </code>
              </div>
              <div className="actions">
                <button
                  className="secondary"
                  disabled={busy}
                  onClick={() =>
                    void check(
                      document.id,
                      contentOf(document.relPath),
                    ).catch(() => undefined)
                  }
                  type="button"
                >
                  Check
                </button>
                <button
                  className="secondary"
                  disabled={busy}
                  onClick={() =>
                    void save(
                      document.id,
                      document.contentDigest ?? null,
                      contentOf(document.relPath),
                    ).catch(() => undefined)
                  }
                  type="button"
                >
                  Save
                </button>
                <button
                  className="danger"
                  disabled={busy}
                  onClick={() => void forget(document.id).catch(() => undefined)}
                  type="button"
                >
                  Forget
                </button>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </>
  );
}
