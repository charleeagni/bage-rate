#!/usr/bin/env node
import { randomUUID } from "node:crypto";
import { readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const usage = `usage: bun run configure -- --name <package-name> --title <display-title> --identifier <reverse.domain.id>\n   or: npm run configure -- --name <package-name> --title <display-title> --identifier <reverse.domain.id>`;

const values = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  const flag = process.argv[index];
  const value = process.argv[index + 1];
  if (!flag?.startsWith("--") || value === undefined) {
    console.error(usage);
    process.exit(2);
  }
  values.set(flag.slice(2), value);
}

const name = values.get("name");
const title = values.get("title");
const identifier = values.get("identifier");
if (!name || !title || !identifier || values.size !== 3) {
  console.error(usage);
  process.exit(2);
}

if (!/^[a-z0-9][a-z0-9._-]*$/.test(name) || name.length > 214) {
  throw new Error("--name must be a lowercase, unscoped package name");
}
if (title.trim() !== title || title.length > 64 || /[\u0000-\u001f]/.test(title)) {
  throw new Error("--title must be 1-64 printable characters without surrounding whitespace");
}
const identifierSegments = identifier.split(".");
const validSegment = (segment) =>
  /^[A-Za-z0-9]$/.test(segment) ||
  /^[A-Za-z0-9][A-Za-z0-9-]*[A-Za-z0-9]$/.test(segment);
if (identifierSegments.length < 3 || !identifierSegments.every(validSegment)) {
  throw new Error("--identifier must be a reverse-domain identifier with at least three valid segments");
}

const escapeHtml = (value) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");

const root = process.cwd();
const absolutePath = (relativePath) => path.join(root, relativePath);
const parseJson = (relativePath, source) => {
  let parsed;
  try {
    parsed = JSON.parse(source);
  } catch (error) {
    throw new Error(`${relativePath} is not valid JSON`, { cause: error });
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`${relativePath} must contain a JSON object`);
  }
  return parsed;
};

const relativePaths = [
  "package.json",
  "package-lock.json",
  "bun.lock",
  "src-tauri/tauri.conf.json",
  "index.html",
  "src/app-config.ts",
];
const originals = new Map(
  await Promise.all(
    relativePaths.map(async (relativePath) => [
      relativePath,
      await readFile(absolutePath(relativePath), "utf8"),
    ]),
  ),
);

const manifest = parseJson("package.json", originals.get("package.json"));
const packageLock = parseJson("package-lock.json", originals.get("package-lock.json"));
const config = parseJson(
  "src-tauri/tauri.conf.json",
  originals.get("src-tauri/tauri.conf.json"),
);
if (!packageLock.packages?.[""] || typeof packageLock.packages[""] !== "object") {
  throw new Error("package-lock.json does not contain the root workspace package");
}
if (!config.app?.windows?.[0] || typeof config.app.windows[0] !== "object") {
  throw new Error("src-tauri/tauri.conf.json does not contain the main window");
}

manifest.name = name;
packageLock.name = name;
packageLock.packages[""].name = name;
config.productName = title;
config.identifier = identifier;
config.app.windows[0].title = title;

const bunLock = originals.get("bun.lock");
const updatedBunLock = bunLock.replace(
  /^(\s{6}"name":\s*)"[^"]+"/m,
  `$1${JSON.stringify(name)}`,
);
if (updatedBunLock === bunLock) {
  throw new Error("bun.lock does not contain the root workspace name");
}

const html = originals.get("index.html");
const titleMatches = html.match(/<title>[^<]*<\/title>/g) ?? [];
if (titleMatches.length !== 1) {
  throw new Error("index.html must contain exactly one simple <title> element");
}
const appConfig = originals.get("src/app-config.ts");
if (!/^export const APP_TITLE = .+;\r?\n?$/.test(appConfig)) {
  throw new Error("src/app-config.ts does not contain the generated APP_TITLE declaration");
}

const updates = new Map(
  [
    ["package.json", `${JSON.stringify(manifest, null, 2)}\n`],
    ["package-lock.json", `${JSON.stringify(packageLock, null, 2)}\n`],
    ["bun.lock", updatedBunLock],
    ["src-tauri/tauri.conf.json", `${JSON.stringify(config, null, 2)}\n`],
    [
      "index.html",
      html.replace(/<title>[^<]*<\/title>/, `<title>${escapeHtml(title)}</title>`),
    ],
    ["src/app-config.ts", `export const APP_TITLE = ${JSON.stringify(title)};\n`],
  ],
);

// Stage every output beside its target, then replace each target atomically.
// All expected parse/shape failures have already happened before this point;
// if a filesystem replacement still fails, restore any committed originals.
const transactionId = `${process.pid}-${randomUUID()}`;
const staged = [...updates].map(([relativePath, contents]) => ({
  contents,
  file: absolutePath(relativePath),
  original: originals.get(relativePath),
  temporary: `${absolutePath(relativePath)}.${transactionId}.tmp`,
}));
const committed = [];
try {
  await Promise.all(
    staged.map(({ contents, temporary }) => writeFile(temporary, contents, { flag: "wx" })),
  );
  for (const entry of staged) {
    await rename(entry.temporary, entry.file);
    committed.push(entry);
  }
} catch (error) {
  await Promise.allSettled(
    committed.map(({ file, original }) => writeFile(file, original)),
  );
  throw error;
} finally {
  await Promise.allSettled(staged.map(({ temporary }) => rm(temporary, { force: true })));
}

console.log(`configured ${title} (${name}, ${identifier})`);
