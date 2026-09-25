// Fails when package.json, Cargo.toml and tauri.conf.json disagree with the tag.
import { readFileSync } from "node:fs";

const tag = (process.argv[2] || "").replace(/^v/, "");
const pkg = JSON.parse(readFileSync("package.json", "utf8")).version;
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")).version;
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8").match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const found = { "package.json": pkg, "tauri.conf.json": conf, "src-tauri/Cargo.toml": cargo };
const bad = Object.entries(found).filter(([, v]) => v !== tag);
if (!tag || bad.length) {
  console.error(`version mismatch for tag v${tag}:`, found);
  process.exit(1);
}
console.log(`version ${tag} ok`);
