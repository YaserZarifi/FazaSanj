// Builds the fast scan helper and puts it where Tauri expects a sidecar binary.
// Usage: node scripts/copy-sidecar.mjs [debug|release]
import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";

const profile = process.argv[2] === "release" ? "release" : "debug";
const triple = "x86_64-pc-windows-msvc";

execSync(`cargo build -p fast-scan-helper ${profile === "release" ? "--release" : ""}`, { stdio: "inherit" });
mkdirSync("src-tauri/binaries", { recursive: true });
copyFileSync(`target/${profile}/fast-scan-helper.exe`, `src-tauri/binaries/fast-scan-helper-${triple}.exe`);
console.log(`sidecar ready (${profile})`);
