#!/usr/bin/env node
/**
 * Baixa os componentes externos do Vibax a partir das fontes OFICIAIS e
 * valida a integridade (SHA-256) antes de colocá-los em src-tauri/tools.
 *
 *   yt-dlp  -> github.com/yt-dlp/yt-dlp          (Unlicense)
 *   FFmpeg  -> github.com/yt-dlp/FFmpeg-Builds   (GPLv3, build "shared")
 *   Deno    -> github.com/denoland/deno          (MIT) — runtime JS exigido pelo yt-dlp
 *
 * Uso: npm run fetch-binaries [-- --force]
 */
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync, copyFileSync, readFileSync, statSync, cpSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const toolsDir = join(root, "src-tauri", "tools");
const force = process.argv.includes("--force");

const UA = { "User-Agent": "vibax-build-script" };

async function fetchBuffer(url) {
  const res = await fetch(url, { headers: UA, redirect: "follow" });
  if (!res.ok) throw new Error(`HTTP ${res.status} ao baixar ${url}`);
  return Buffer.from(await res.arrayBuffer());
}

async function fetchText(url) {
  return (await fetchBuffer(url)).toString("utf8");
}

function sha256(buf) {
  return createHash("sha256").update(buf).digest("hex");
}

function expectHash(name, buf, expected) {
  const got = sha256(buf);
  if (!expected || got.toLowerCase() !== expected.toLowerCase()) {
    throw new Error(`Falha de integridade em ${name}: esperado ${expected}, obtido ${got}`);
  }
  console.log(`  ✓ SHA-256 verificado (${name})`);
}

function hashFromSums(sumsText, fileName) {
  for (const line of sumsText.split(/\r?\n/)) {
    const [hash, ...rest] = line.trim().split(/\s+/);
    if (rest.join(" ").replace(/^\*/, "") === fileName) return hash;
  }
  return null;
}

function extractZip(buf, label) {
  const dir = join(tmpdir(), `vibax-${label}-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  const zip = join(dir, "pkg.zip");
  writeFileSync(zip, buf);
  // bsdtar do Windows 10+ extrai .zip (o "tar" do Git Bash é GNU tar e não serve)
  const sysTar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  execFileSync(existsSync(sysTar) ? sysTar : "tar", ["-xf", zip, "-C", dir], { stdio: "inherit" });
  rmSync(zip);
  return dir;
}

function findFile(dir, name) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) {
      const r = findFile(p, name);
      if (r) return r;
    } else if (entry.name.toLowerCase() === name.toLowerCase()) return p;
  }
  return null;
}

const manifest = !force && existsSync(join(toolsDir, "manifest.json"))
  ? JSON.parse(readFileSync(join(toolsDir, "manifest.json"), "utf8"))
  : {};

/**
 * Usa a build "onedir" oficial (yt-dlp_win.zip): inicia mais rápido que o .exe
 * onefile e não extrai arquivos temporários em %TEMP% a cada execução — o que
 * também evita lixo quando um download é cancelado (processo encerrado).
 */
async function ytdlp() {
  const targetDir = join(toolsDir, "yt-dlp");
  if (existsSync(join(targetDir, "yt-dlp.exe")) && !force) return console.log("• yt-dlp já presente");
  console.log("• Baixando yt-dlp (release oficial mais recente, build onedir)…");
  const rel = JSON.parse(await fetchText("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest"));
  const base = `https://github.com/yt-dlp/yt-dlp/releases/download/${rel.tag_name}`;
  const name = "yt-dlp_win.zip";
  const sums = await fetchText(`${base}/SHA2-256SUMS`);
  const zip = await fetchBuffer(`${base}/${name}`);
  expectHash(name, zip, hashFromSums(sums, name));
  const dir = extractZip(zip, "ytdlp");
  const exeDir = dirname(findFile(dir, "yt-dlp.exe"));
  rmSync(targetDir, { recursive: true, force: true });
  cpSync(exeDir, targetDir, { recursive: true });
  rmSync(dir, { recursive: true, force: true });
  writeFileSync(join(toolsDir, "licenses", "yt-dlp-LICENSE.txt"), await fetchText(`https://raw.githubusercontent.com/yt-dlp/yt-dlp/${rel.tag_name}/LICENSE`));
  manifest["yt-dlp"] = { version: rel.tag_name, source: base };
}

async function ffmpeg() {
  const target = join(toolsDir, "ffmpeg.exe");
  if (existsSync(target) && !force) return console.log("• FFmpeg já presente");
  console.log("• Baixando FFmpeg (yt-dlp/FFmpeg-Builds, win64 gpl-shared)…");
  const name = "ffmpeg-master-latest-win64-gpl-shared.zip";
  const base = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest";
  const sums = await fetchText(`${base}/checksums.sha256`);
  const zip = await fetchBuffer(`${base}/${name}`);
  expectHash(name, zip, hashFromSums(sums, name));
  const dir = extractZip(zip, "ffmpeg");
  const binDir = dirname(findFile(dir, "ffmpeg.exe"));
  for (const f of readdirSync(binDir)) {
    if (/\.(exe|dll)$/i.test(f) && f.toLowerCase() !== "ffplay.exe") copyFileSync(join(binDir, f), join(toolsDir, f));
  }
  const lic = findFile(dir, "LICENSE.txt");
  if (lic) copyFileSync(lic, join(toolsDir, "licenses", "FFmpeg-LICENSE.txt"));
  rmSync(dir, { recursive: true, force: true });
  manifest.ffmpeg = {
    version: "master-latest (win64-gpl-shared)",
    source: `${base}/${name}`,
    sourceCode: "https://github.com/yt-dlp/FFmpeg-Builds  |  https://ffmpeg.org/download.html#get-sources",
  };
}

async function deno() {
  const target = join(toolsDir, "deno.exe");
  if (existsSync(target) && !force) return console.log("• Deno já presente");
  console.log("• Baixando Deno (runtime JS exigido pelo yt-dlp para o YouTube)…");
  const rel = JSON.parse(await fetchText("https://api.github.com/repos/denoland/deno/releases/latest"));
  const name = "deno-x86_64-pc-windows-msvc.zip";
  const base = `https://github.com/denoland/deno/releases/download/${rel.tag_name}`;
  const sumText = await fetchText(`${base}/${name}.sha256sum`);
  const expected = sumText.match(/[a-f0-9]{64}/i)?.[0];
  const zip = await fetchBuffer(`${base}/${name}`);
  expectHash(name, zip, expected);
  const dir = extractZip(zip, "deno");
  copyFileSync(findFile(dir, "deno.exe"), target);
  rmSync(dir, { recursive: true, force: true });
  writeFileSync(join(toolsDir, "licenses", "Deno-LICENSE.md"), await fetchText(`https://raw.githubusercontent.com/denoland/deno/${rel.tag_name}/LICENSE.md`));
  manifest.deno = { version: rel.tag_name, source: base };
}

mkdirSync(join(toolsDir, "licenses"), { recursive: true });
await ytdlp();
await ffmpeg();
await deno();
writeFileSync(join(toolsDir, "manifest.json"), JSON.stringify(manifest, null, 2));

const total = readdirSync(toolsDir)
  .filter((f) => /\.(exe|dll)$/i.test(f))
  .reduce((acc, f) => acc + statSync(join(toolsDir, f)).size, 0);
console.log(`\nComponentes prontos em src-tauri/tools (${(total / 1024 / 1024).toFixed(1)} MB).`);
