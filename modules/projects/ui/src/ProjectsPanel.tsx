import { useState, type FormEvent } from "react";
import { useMutation, useQuery } from "@apollo/client/react";

import {
  CreateProjectDocument,
  DeleteProjectDocument,
  ListProjectsDocument,
  RenameProjectDocument,
} from "./generated/graphql.ts";
import { evictProject, projectListConvergence } from "./cache.ts";

export function ProjectsPanel() {
  const projects = useQuery(ListProjectsDocument);
  const [createProject, createState] = useMutation(
    CreateProjectDocument,
    projectListConvergence(),
  );
  const [renameProject, renameState] = useMutation(
    RenameProjectDocument,
    projectListConvergence(),
  );
  const [deleteProject, deleteState] = useMutation(DeleteProjectDocument);
  const [name, setName] = useState("");
  const [workspaceRoot, setWorkspaceRoot] = useState("");

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const cleanName = name.trim();
    const cleanRoot = workspaceRoot.trim();
    if (!cleanName || !cleanRoot) return;
    await createProject({
      variables: { name: cleanName, workspaceRoot: cleanRoot },
    });
    setName("");
    setWorkspaceRoot("");
  };

  const rename = async (id: number, currentName: string) => {
    const nextName = window.prompt("Project name", currentName)?.trim();
    if (!nextName || nextName === currentName) return;
    await renameProject({
      variables: { id, name: nextName },
    });
  };

  const remove = async (id: number) => {
    await deleteProject({
      variables: { id },
      update(cache, result) {
        if (result.data) evictProject(cache, id);
      },
    });
  };

  const error =
    projects.error ?? createState.error ?? renameState.error ?? deleteState.error;
  const projectMutationLoading = renameState.loading || deleteState.loading;

  return (
    <>
      <section aria-labelledby="new-project-heading" className="panel">
        <div>
          <p className="panel-eyebrow">Projects</p>
          <h2 id="new-project-heading">Create a project</h2>
        </div>
        <form onSubmit={(event) => void submit(event).catch(() => undefined)}>
          <label>
            Name
            <input
              name="name"
              onChange={(event) => setName(event.target.value)}
              placeholder="My desktop app"
              required
              value={name}
            />
          </label>
          <label>
            Workspace root
            <input
              name="workspaceRoot"
              onChange={(event) => setWorkspaceRoot(event.target.value)}
              placeholder="/projects/my-app"
              required
              value={workspaceRoot}
            />
          </label>
          <button disabled={createState.loading} type="submit">
            {createState.loading ? "Creating…" : "Create project"}
          </button>
        </form>
      </section>

      <section aria-labelledby="projects-heading" className="panel">
        <div>
          <p className="panel-eyebrow">Projects</p>
          <h2 id="projects-heading">Persisted projects</h2>
        </div>

        {error ? (
          <p className="error" role="alert">
            {error.message}
          </p>
        ) : null}
        {projects.loading && !projects.data ? (
          <p aria-live="polite">Loading from Rust…</p>
        ) : null}
        {projects.data?.projects.nodes.length === 0 ? (
          <p className="empty">No projects yet. The first mutation is waiting.</p>
        ) : null}
        <ul className="record-list">
          {projects.data?.projects.nodes.map((project) => (
            <li key={project.id}>
              <div>
                <strong>{project.name}</strong>
                <code>{project.workspaceRoot}</code>
              </div>
              <div className="actions">
                <button
                  className="secondary"
                  disabled={projectMutationLoading}
                  onClick={() =>
                    void rename(project.id, project.name).catch(() => undefined)
                  }
                  type="button"
                >
                  Rename
                </button>
                <button
                  className="danger"
                  disabled={projectMutationLoading}
                  onClick={() => void remove(project.id).catch(() => undefined)}
                  type="button"
                >
                  Delete
                </button>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </>
  );
}
