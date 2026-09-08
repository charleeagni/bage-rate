import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ApolloProvider } from "@apollo/client/react";

import { App } from "./App.tsx";
import { apolloClient } from "./graphql/client.ts";
import "./styles.css";
import "./lockin/lockin.css";
import "./lockin/settings.css";

const root = document.getElementById("root");
if (!root) throw new Error("the root element is missing");

createRoot(root).render(
  <StrictMode>
    <ApolloProvider client={apolloClient}>
      <App />
    </ApolloProvider>
  </StrictMode>,
);
