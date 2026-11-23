import {
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
  existsSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const metadata = spawnSync(
  "cargo",
  [
    "metadata",
    "--locked",
    "--format-version=1",
    "--manifest-path",
    "src-tauri/Cargo.toml",
  ],
  { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
);
if (metadata.status !== 0)
  throw new Error(metadata.stderr || "Cargo metadata failed");
const packages = JSON.parse(metadata.stdout).packages;
const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
const sections = [
  "NetControl third-party notices",
  "Dependencies retain their original licenses. This list includes build dependencies.",
  readFileSync("LICENSE", "utf8"),
];

function licenseTexts(directory) {
  if (!existsSync(directory)) return [];
  const texts = [];
  for (const name of readdirSync(directory)) {
    if (!/^(licen[sc]e|copying|copyright|notice)([._-]|$)/i.test(name))
      continue;
    const file = join(directory, name);
    const stat = statSync(file);
    if (stat.isFile() && stat.size < 1024 * 1024)
      texts.push(`${name}\n${readFileSync(file, "utf8")}`);
    if (stat.isDirectory()) {
      for (const child of readdirSync(file)) {
        const entry = join(file, child);
        if (statSync(entry).isFile() && statSync(entry).size < 1024 * 1024)
          texts.push(`${name}/${child}\n${readFileSync(entry, "utf8")}`);
      }
    }
  }
  return texts;
}

for (const [path, item] of Object.entries(lock.packages)) {
  if (!path) continue;
  sections.push(
    `npm: ${path.replace(/^node_modules\//, "")} ${item.version}\nLicense: ${item.license || "See upstream package"}`,
    ...licenseTexts(path),
  );
}
for (const item of packages) {
  if (item.name === "netcontrol") continue;
  sections.push(
    `Rust: ${item.name} ${item.version}\nLicense: ${item.license || "See upstream package"}\nRepository: ${item.repository || ""}`,
    ...licenseTexts(dirname(item.manifest_path)),
  );
}
writeFileSync(
  "THIRD_PARTY_NOTICES.txt",
  sections.join("\n\n----------------------------------------\n\n"),
);
