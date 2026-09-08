import type { CodegenConfig } from "@graphql-codegen/cli";

const output = process.env.CODEGEN_OUTPUT ?? "./src/generated/graphql.ts";
const schema = process.env.CODEGEN_SCHEMA ?? "./schema.graphql";
const documents = process.env.CODEGEN_DOCUMENTS ?? "./src/**/*.graphql";

const config: CodegenConfig = {
  overwrite: true,
  schema,
  documents: [documents],
  generates: {
    [output]: {
      plugins: ["typescript-operations", "typed-document-node"],
      config: {
        avoidOptionals: {
          field: true,
          inputValue: false,
        },
        defaultScalarType: "unknown",
        nonOptionalTypename: true,
        skipTypeNameForRoot: true,
        useTypeImports: true,
      },
    },
  },
};

export default config;
