import { ApolloLink, Observable } from "@apollo/client";
import { getOperationAST, print } from "graphql";
import type { DocumentNode, FormattedExecutionResult } from "graphql";

export interface GraphQlTransportProxy {
  graphql_execute(requestJson: string): Promise<string>;
  graphql_subscribe(
    subscriptionId: string,
    requestJson: string,
    onEvent: (response: string) => void,
  ): Promise<string>;
  graphql_unsubscribe(subscriptionId: string): Promise<boolean>;
}

export type CreateGraphQlTransportProxy = () => GraphQlTransportProxy;

interface SubscriptionEvent {
  type: "next" | "complete";
  payload?: FormattedExecutionResult;
}

let subscriptionCounter = 0;

const subscriptionId = (): string => {
  subscriptionCounter += 1;
  return `subscription-${Date.now()}-${subscriptionCounter}`;
};

const wireRequest = (
  query: DocumentNode,
  operationName: string | undefined,
  variables: unknown,
): string =>
  JSON.stringify({
    query: print(query),
    operationName: operationName || null,
    variables,
  });

const decodeObject = (encoded: string, label: string): Record<string, unknown> => {
  const decoded: unknown = JSON.parse(encoded);
  if (decoded === null || typeof decoded !== "object" || Array.isArray(decoded)) {
    throw new TypeError(`${label} must be a JSON object`);
  }
  return decoded as Record<string, unknown>;
};

const unaryObservable = (
  createProxy: CreateGraphQlTransportProxy,
  requestJson: string,
) =>
  new Observable<FormattedExecutionResult>((observer) => {
    let active = true;
    void createProxy()
      .graphql_execute(requestJson)
      .then(
        (responseJson) => {
          if (!active) return;
          try {
            observer.next(
              decodeObject(
                responseJson,
                "the GraphQL response",
              ) as FormattedExecutionResult,
            );
            observer.complete();
          } catch (error) {
            observer.error(error);
          }
        },
        (error: unknown) => {
          if (active) observer.error(error);
        },
      );
    return () => {
      active = false;
    };
  });

const subscriptionObservable = (
  createProxy: CreateGraphQlTransportProxy,
  requestJson: string,
) =>
  new Observable<FormattedExecutionResult>((observer) => {
    const id = subscriptionId();
    const proxy = createProxy();
    let active = true;
    let accepted = false;
    let finished = false;

    const stop = () => {
      if (accepted && !finished) {
        void proxy.graphql_unsubscribe(id);
      }
    };

    void proxy
      .graphql_subscribe(id, requestJson, (encodedEvent) => {
        if (!active) return;
        try {
          const envelope = decodeObject(
            encodedEvent,
            "the subscription event",
          ) as unknown as SubscriptionEvent;
          if (envelope.type === "next" && envelope.payload) {
            observer.next(envelope.payload);
          } else if (envelope.type === "complete") {
            finished = true;
            observer.complete();
          } else {
            throw new TypeError("the subscription event has an unknown shape");
          }
        } catch (error) {
          finished = true;
          observer.error(error);
        }
      })
      .then(
        (encodedAcknowledgement) => {
          const acknowledgement = decodeObject(
            encodedAcknowledgement,
            "the subscription acknowledgement",
          );
          if (acknowledgement.type === "accepted") {
            accepted = true;
            if (!active) stop();
            return;
          }

          finished = true;
          if (active && Array.isArray(acknowledgement.errors)) {
            observer.next(acknowledgement as FormattedExecutionResult);
            observer.complete();
          } else if (active) {
            observer.error(
              new Error(
                `the subscription was refused: ${encodedAcknowledgement}`,
              ),
            );
          }
        },
        (error: unknown) => {
          finished = true;
          if (active) observer.error(error);
        },
      )
      .catch((error: unknown) => {
        finished = true;
        if (active) observer.error(error);
      });

    return () => {
      active = false;
      stop();
    };
  });

export const createTauriGraphQlLink = (
  createProxy: CreateGraphQlTransportProxy,
): ApolloLink =>
  new ApolloLink((operation) => {
    const operationDefinition = getOperationAST(
      operation.query,
      operation.operationName || undefined,
    );
    if (!operationDefinition) {
      return new Observable((observer) => {
        observer.error(new Error("the GraphQL document has no selected operation"));
      });
    }

    const requestJson = wireRequest(
      operation.query,
      operation.operationName,
      operation.variables,
    );
    return operationDefinition.operation === "subscription"
      ? subscriptionObservable(createProxy, requestJson)
      : unaryObservable(createProxy, requestJson);
  });
