import assert from "node:assert/strict";
import test from "node:test";

import { ApolloClient, ApolloLink, InMemoryCache, Observable } from "@apollo/client";

import {
  CreateProjectDocument,
  DeleteProjectDocument,
  ListProjectsDocument,
  RenameProjectDocument,
} from "./generated/graphql.ts";
import {
  evictProject,
  projectListConvergence,
  projectsTypePolicies,
} from "./cache.ts";

const project = (id: number, name: string, workspaceRoot: string) => ({
  __typename: "Projects" as const,
  id,
  name,
  workspaceRoot,
});

const testClient = () => {
  const serverProjects = [project(1, "Alpha", "/alpha"), project(2, "Beta", "/beta")];
  const link = new ApolloLink(
    (operation) =>
      new Observable((observer) => {
        queueMicrotask(() => {
          if (operation.operationName === "ListProjects") {
            observer.next({
              data: {
                projects: {
                  __typename: "ProjectsConnection",
                  nodes: [...serverProjects].sort((left, right) =>
                    left.name.localeCompare(right.name),
                  ),
                },
              },
            });
          } else if (operation.operationName === "CreateProject") {
            const created = project(
              Math.max(...serverProjects.map(({ id }) => id)) + 1,
              String(operation.variables.name),
              String(operation.variables.workspaceRoot),
            );
            serverProjects.push(created);
            observer.next({ data: { projectsCreateOne: created } });
          } else if (operation.operationName === "RenameProject") {
            const renamed = serverProjects.find(
              ({ id }) => id === Number(operation.variables.id),
            );
            assert.ok(renamed);
            renamed.name = String(operation.variables.name);
            observer.next({ data: { projectsUpdate: [renamed] } });
          } else if (operation.operationName === "DeleteProject") {
            const index = serverProjects.findIndex(
              ({ id }) => id === Number(operation.variables.id),
            );
            const deleted = index === -1 ? 0 : serverProjects.splice(index, 1).length;
            observer.next({ data: { projectsDelete: deleted } });
          } else {
            observer.error(new Error(`unexpected operation ${operation.operationName}`));
            return;
          }
          observer.complete();
        });
      }),
  );

  return {
    client: new ApolloClient({
      cache: new InMemoryCache({ typePolicies: projectsTypePolicies }),
      link,
    }),
    serverProjects,
  };
};

const projectNames = (client: ApolloClient) =>
  client.readQuery({ query: ListProjectsDocument })?.projects.nodes.map(({ name }) => name);

const watchProjects = async (client: ApolloClient) => {
  let ready = false;
  let markReady = () => {};
  const initialResult = new Promise<void>((resolve) => {
    markReady = resolve;
  });
  const subscription = client
    .watchQuery({ query: ListProjectsDocument, fetchPolicy: "network-only" })
    .subscribe({
      error(error) {
        throw error;
      },
      next(result) {
        if (!ready && result.data) {
          ready = true;
          markReady();
        }
      },
    });
  await initialResult;
  return subscription;
};

test("create and sort-key updates refetch the ordered project list", async () => {
  const { client } = testClient();
  const subscription = await watchProjects(client);

  await client.mutate({
    mutation: CreateProjectDocument,
    variables: { name: "Aardvark", workspaceRoot: "/aardvark" },
    ...projectListConvergence(),
  });
  assert.deepEqual(projectNames(client), ["Aardvark", "Alpha", "Beta"]);

  await client.mutate({
    mutation: RenameProjectDocument,
    variables: { id: 2, name: "Aaron" },
    ...projectListConvergence(),
  });
  assert.deepEqual(projectNames(client), ["Aardvark", "Aaron", "Alpha"]);
  subscription.unsubscribe();
});

test("delete evicts the known identity even when the row is already absent", async () => {
  const { client, serverProjects } = testClient();
  await client.query({ query: ListProjectsDocument, fetchPolicy: "network-only" });

  serverProjects.splice(
    serverProjects.findIndex(({ id }) => id === 2),
    1,
  );
  await client.mutate({
    mutation: DeleteProjectDocument,
    variables: { id: 2 },
    update(cache, result) {
      if (result.data) evictProject(cache, 2);
    },
  });

  assert.deepEqual(projectNames(client), ["Alpha"]);
});
