/**
 * Test server for the Simpro provider.
 * OAuth token endpoint + {data, meta} paginated company-scoped endpoints.
 */

export const providerId = 'simpro';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'simpro job customer quote site invoice technician'.split(' ');
const randText = () => Array.from({ length: 40 }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';

const slug = (path) => path.split('/').filter(Boolean).pop() || 'record';
const totalFor = (path) => 25 + (path.length % 20);

const json = (res, code, body) => {
  res.writeHead(code, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
};

export function handle(req, res, url, ctx) {
  const path = url.pathname || '/';

  if (path === '/oauth/token' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let parsed = {};
      try { parsed = JSON.parse(body); } catch {}
      const validId = String(parsed.client_id || '').startsWith('test-client') || String(parsed.client_id).toLowerCase().includes('2fa');
      if (!validId || (!String(parsed.client_secret || '').startsWith('test-secret') && parsed.client_secret !== ctx.oauthClientSecret)) {
        return json(res, 401, { error: 'invalid_client', message: 'Invalid client credentials' });
      }
      { const t = `sp-token-${rand(20)}`; ctx.issuedTokens?.add(t); json(res, 200, { access_token: t, token_type: 'Bearer', expires_in: 3600 }); }
    });
    return;
  }

  if (path.startsWith('/files/')) {
    const name = path.slice('/files/'.length) || `file-${rand(6)}.txt`;
    res.writeHead(200, { 'content-type': 'text/plain', 'content-disposition': `attachment; filename="${name}"` });
    return res.end(randText());
  }

  const per = Math.min(250, parseInt(url.searchParams.get('per_page') || url.searchParams.get('pageSize') || '50', 10) || 50);
  const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;
  const total = totalFor(path);
  const start = (page - 1) * per;
  const data = [];
  for (let i = start; i < Math.min(start + per, total); i++) {
    data.push({
      id: `${slug(path)}-${i + 1}`,
      name: `${slug(path).replace(/-/g, ' ')} ${i + 1}`,
      createdAt: '2026-01-01T00:00:00Z',
      url: `${ctx.base}/files/${slug(path)}-${i + 1}-${rand(6)}.txt`,
    });
  }
  json(res, 200, { data, meta: { current_page: page, last_page: Math.max(1, Math.ceil(total / per)), per_page: per, total } });
}
