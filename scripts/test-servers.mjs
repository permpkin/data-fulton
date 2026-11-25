/**
 * Dev test-server launcher.
 * Scans providers/<name>/server.test.js, mounts each under /api/{providerId} on a
 * single port, generates a shared API key, and prints what to connect to.
 */
import http from 'node:http';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const PROVIDERS_DIR = path.join(ROOT, 'src/renderer/src/providers');
const PORT = process.env.FULTON_TEST_PORT ? Number(process.env.FULTON_TEST_PORT) : 5174;
const API_KEY = `fulton-test-${crypto.randomBytes(8).toString('hex')}`;
const OAUTH_CLIENT_ID = 'test-client-datafulton';
const OAUTH_CLIENT_SECRET = 'test-secret-datafulton';
// Tokens issued by /oauth/token endpoints — resource routes accept these too.
const issuedTokens = new Set();

async function discover() {
  const providers = [];
  if (!fs.existsSync(PROVIDERS_DIR)) return providers;
  for (const dir of fs.readdirSync(PROVIDERS_DIR)) {
    const file = path.join(PROVIDERS_DIR, dir, 'server.test.js');
    if (fs.existsSync(file)) {
      const mod = await import(pathToFileURL(file).href);
      providers.push({ dir, id: mod.providerId ?? dir, handle: mod.handle });
    }
  }
  return providers;
}

const providers = await discover();
const base = `http://localhost:${PORT}`;

const server = http.createServer((req, res) => {
  const url = new URL(req.url || '/', base);
  const m = /^\/api\/([^/]+)(\/.*)?$/.exec(url.pathname || '');
  const provider = m && providers.find((p) => p.id === m[1]);
  if (!provider) {
    res.writeHead(404, { 'content-type': 'application/json' });
    return res.end(JSON.stringify({ message: `Unknown provider path: ${url.pathname}` }));
  }
  const rest = m[2] || '/';
  const isFile = rest.startsWith('/files/') || rest.startsWith('/oauth/');
  const auth = req.headers.authorization || '';
  const bearer = auth.replace(/^Bearer\s+/i, '');
  if (!isFile && bearer !== API_KEY && !issuedTokens.has(bearer)) {
    res.writeHead(401, { 'content-type': 'application/json' });
    return res.end(JSON.stringify({ message: 'Unauthenticated.' }));
  }
  url.pathname = rest;
  provider.handle(req, res, url, { apiKey: API_KEY, issuedTokens, oauthClientId: OAUTH_CLIENT_ID, oauthClientSecret: OAUTH_CLIENT_SECRET, base: `${base}/api/${provider.id}` });
});

server.listen(PORT, () => {
  console.log(`\n[datafulton test servers] listening on ${base}`);
  for (const p of providers) {
    console.log(`  ${p.id}: ${base}/api/${p.id}/v2/...   (files: ${base}/api/${p.id}/files/…)`);
  }
  console.log(`\n  API key: ${API_KEY}`);
  console.log(`  OAuth client_id:     ${OAUTH_CLIENT_ID}`);
  console.log(`  OAuth client_secret: ${OAUTH_CLIENT_SECRET}`);
  console.log(`  (buildpass/procore/simpro — any id containing "2fa" also requires otp 123456)\n`);
});
