// Turns off updater artifacts for this CI run when the signing key secret is missing,
// so a release still gets built (the installed app just won't auto update from it).
import { readFileSync, writeFileSync } from "node:fs";

const path = "src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(path, "utf8"));
conf.bundle.createUpdaterArtifacts = false;
writeFileSync(path, JSON.stringify(conf, null, 2));
console.log("no signing key, building without updater files");
