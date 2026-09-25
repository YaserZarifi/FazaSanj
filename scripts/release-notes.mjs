// Prints the CHANGELOG.md section for one version, used as the GitHub release text.
import { readFileSync } from "node:fs";

const version = (process.argv[2] || "").replace(/^v/, "");
const lines = readFileSync("CHANGELOG.md", "utf8").split(/\r?\n/);
const start = lines.findIndex((l) => l.startsWith(`## ${version}`) || l.startsWith(`## v${version}`));
if (start < 0) {
  console.error(`no CHANGELOG entry for ${version}`);
  process.exit(1);
}
const rest = lines.slice(start + 1);
const end = rest.findIndex((l) => l.startsWith("## "));
const body = (end < 0 ? rest : rest.slice(0, end)).join("\n").trim();

const footer = [
  "",
  "### Download",
  "",
  "Grab the `Fazasanj_" + version + "_x64-setup.exe` file below and run it.",
  "",
  "Windows might show a SmartScreen warning because the installer isn't signed yet. Click \"More info\" and then \"Run anyway\".",
].join("\n");

process.stdout.write(body + "\n" + footer + "\n");
