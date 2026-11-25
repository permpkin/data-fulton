/**
 * Test server for the BuildPass provider.
 * Replicates the BuildPass API ({data, meta} pagination via offset/limit,
 * and the page/per_page params our extractor sends) with dummy records.
 * /files/{name}.txt serves random text for download testing.
 */

export const providerId = 'buildpass';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'buildpass site worker induction photo defect meeting project'.split(' ');
const randText = () => Array.from({ length: 40 }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';

const slug = (path) => path.split('/').filter(Boolean).pop() || 'record';
const totalFor = (path) => 30 + (path.length % 25);

const makeRecord = (path, i, base) => ({
  id: `${slug(path)}-${i + 1}`,
  name: `${slug(path).replace(/-/g, ' ')} ${i + 1}`,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
  url: `${base}/files/${slug(path)}-${i + 1}-${rand(6)}.txt`,
});

const json = (res, code, body) => {
  res.writeHead(code, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
};

export function handle(req, res, url, ctx) {
  const path = url.pathname || '/';

  // OAuth token exchange: POST /oauth/token {client_id, client_secret} → access_token
  if (path === '/oauth/token' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let parsed = {};
      try { parsed = JSON.parse(body); } catch {}
      const { client_id, client_secret, otp } = parsed;
      const validId = String(client_id || '').startsWith('test-client') || client_id === ctx.oauthClientId || client_id.toLowerCase().includes('2fa');
      if (!validId || !String(client_secret || '').startsWith('test-secret') && client_secret !== ctx.oauthClientSecret) {
        res.writeHead(401, { 'content-type': 'application/json' });
        return res.end(JSON.stringify({ error: 'invalid_client', message: 'Invalid client credentials' }));
      }
      // Demo 2FA: any client_id containing "2fa" requires otp 123456
      if (client_id.toLowerCase().includes('2fa') && otp !== '123456') {
        res.writeHead(400, { 'content-type': 'application/json' });
        return res.end(JSON.stringify({ error: 'otp_required', requires_2fa: true, otp_hint: 'code sent to authenticator app (use 123456)' }));
      }
      const t = `bp-token-${rand(20)}`;
      ctx.issuedTokens?.add(t);
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ access_token: t, token_type: 'Bearer', expires_in: 3600 }));
    });
    return;
  }

  if (path.startsWith('/files/')) {
    const name = path.slice('/files/'.length) || `file-${rand(6)}.txt`;
    res.writeHead(200, { 'content-type': 'text/plain', 'content-disposition': `attachment; filename="${name}"` });
    return res.end(randText());
  }

  const parts = path.split('/').filter(Boolean);

  // child endpoints: /{parent}/{id}/{leaf} — list shape
  if (parts.length >= 3 && parts[1] !== 'cost-codes' && parts[1] !== 'time-types') {
    const parentId = parts[1];
    const leafPath = `/${parts.slice(2).join('/')}`;
    const per = Math.min(25, parseInt(url.searchParams.get('per_page') || url.searchParams.get('limit') || '25', 10) || 25);
    const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;
    const data = Array.from({ length: Math.min(per, 3) }, (_, i) => ({
      id: `${slug(leafPath)}-${rand(4)}`,
      parentId,
      name: `${slug(leafPath)} ${i + 1}`,
    }));
    return json(res, 200, { data, meta: { current_page: page, last_page: 1, per_page: per, total: data.length } });
  }

  // list endpoints
  if (parts.length >= 1) {
    const per = Math.min(25, parseInt(url.searchParams.get('per_page') || url.searchParams.get('limit') || '25', 10) || 25);
    const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;
    const total = totalFor(path);
    const start = (page - 1) * per;
    const data = [];
    for (let i = start; i < Math.min(start + per, total); i++) data.push(makeRecord(path, i, ctx.base));
    return json(res, 200, {
      data,
      meta: { current_page: page, last_page: Math.max(1, Math.ceil(total / per)), per_page: per, total },
    });
  }

  json(res, 404, { message: `Not found: ${path}` });
}
