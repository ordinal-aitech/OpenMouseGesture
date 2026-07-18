#!/usr/bin/env node
// Repository-level Windows distribution export.
//
// Copies the already-built Tauri release EXE and NSIS installer out of the
// deeply nested `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/...`
// tree into a conventional, easy-to-find `dist/windows/` at the repo root,
// alongside SHA-256 hashes and lightweight build metadata.
//
// This script does NOT build the app. Run `npm run tauri build` inside
// `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/` first (see README.md).
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync, copyFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sha256File, formatSha256SumsFile, buildMetadata, resolveInstallerFileName } from './dist-windows-lib.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, '..');
const SOURCE_DIR = join(REPO_ROOT, 'source-v1.0.1', '7-rate-OpenMouseGesture-b8f5357');
const SRC_TAURI_DIR = join(SOURCE_DIR, 'src-tauri');
const RELEASE_DIR = join(SRC_TAURI_DIR, 'target', 'release');
const NSIS_DIR = join(RELEASE_DIR, 'bundle', 'nsis');
const DIST_DIR = join(REPO_ROOT, 'dist', 'windows');

const OUTPUT_EXE_NAME = 'OpenMouseGesture-x64.exe';
const OUTPUT_INSTALLER_NAME = 'OpenMouseGesture-Setup-x64.exe';

function fail(message) {
  console.error(`[dist:windows] ERROR: ${message}`);
  process.exit(1);
}

function readTauriConfig() {
  const confPath = join(SRC_TAURI_DIR, 'tauri.conf.json');
  if (!existsSync(confPath)) {
    fail(`tauri.conf.json not found at ${confPath}`);
  }
  const conf = JSON.parse(readFileSync(confPath, 'utf8'));
  return {
    version: conf.version,
    productName: conf.productName,
    mainBinaryName: conf.mainBinaryName || conf.productName,
  };
}

function gitCommitSha() {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
  } catch {
    return null;
  }
}

async function main() {
  const { version, productName, mainBinaryName } = readTauriConfig();

  const releaseExePath = join(RELEASE_DIR, `${mainBinaryName}.exe`);
  if (!existsSync(releaseExePath)) {
    fail(
      `Release EXE not found at ${releaseExePath}.\n` +
        `Run "npm run tauri build" inside ${SOURCE_DIR} first, then re-run "npm run dist:windows".`,
    );
  }

  if (!existsSync(NSIS_DIR)) {
    fail(
      `NSIS bundle output not found at ${NSIS_DIR}.\n` +
        `Run "npm run tauri build" inside ${SOURCE_DIR} first, then re-run "npm run dist:windows".`,
    );
  }
  const installerFileName = resolveInstallerFileName(readdirSync(NSIS_DIR));
  const installerPath = join(NSIS_DIR, installerFileName);

  // Clean only the repo-level dist/windows output; never touch the Tauri target tree.
  if (existsSync(DIST_DIR)) {
    rmSync(DIST_DIR, { recursive: true, force: true });
  }
  mkdirSync(DIST_DIR, { recursive: true });

  const exeDest = join(DIST_DIR, OUTPUT_EXE_NAME);
  const installerDest = join(DIST_DIR, OUTPUT_INSTALLER_NAME);
  copyFileSync(releaseExePath, exeDest);
  copyFileSync(installerPath, installerDest);

  const artifacts = [];
  for (const [name, srcPath, destPath] of [
    [OUTPUT_EXE_NAME, releaseExePath, exeDest],
    [OUTPUT_INSTALLER_NAME, installerPath, installerDest],
  ]) {
    const sourceHash = await sha256File(srcPath);
    const destHash = await sha256File(destPath);
    if (sourceHash !== destHash) {
      fail(`SHA-256 mismatch after copying ${name}: source=${sourceHash} dest=${destHash}`);
    }
    artifacts.push({ name, sha256: destHash, sizeBytes: statSync(destPath).size });
  }

  writeFileSync(join(DIST_DIR, 'SHA256SUMS.txt'), formatSha256SumsFile(artifacts));

  const metadata = buildMetadata({
    version,
    productName,
    buildTimestamp: new Date().toISOString(),
    gitCommit: gitCommitSha(),
    artifacts,
  });
  writeFileSync(join(DIST_DIR, 'build-info.json'), JSON.stringify(metadata, null, 2) + '\n');

  console.log(`[dist:windows] Exported to ${DIST_DIR}`);
  for (const a of artifacts) {
    console.log(`  ${a.name}  (${a.sizeBytes} bytes)  sha256=${a.sha256}`);
  }
  console.log(`  SHA256SUMS.txt`);
  console.log(`  build-info.json  (version=${metadata.version}, commit=${metadata.gitCommit ?? 'unknown'})`);
}

main().catch((err) => fail(err.stack || String(err)));
