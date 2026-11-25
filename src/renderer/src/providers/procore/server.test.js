/**
 * Test server for the Procore provider.
 * Replicates Procore's shape: POST /oauth/token (client_credentials) and
 * bare-array list responses with page/per_page pagination.
 */

export const providerId = 'procore';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'procore project site drawing submittal inspection punch'.split(' ');
const randText = () => Array.from({ length: 40 }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';

const slug = (path) => path.split('/').filter(Boolean).pop() || 'record';
const totalFor = (path) => 30 + (path.length % 25);

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
      const { client_id, client_secret, otp } = parsed;
      const validId = String(client_id || '').startsWith('test-client') || client_id === ctx.oauthClientId || String(client_id).toLowerCase().includes('2fa');
      if (!validId || !String(client_secret || '').startsWith('test-secret') && client_secret !== ctx.oauthClientSecret) {
        return json(res, 401, { error: 'invalid_client', message: 'Invalid client credentials' });
      }
      { const t = `pc-token-${rand(20)}`; ctx.issuedTokens?.add(t); json(res, 200, { access_token: t, token_type: 'Bearer', expires_in: 3600 }); }
    });
    return;
  }

  if (path.startsWith('/files/')) {
    const name = path.slice('/files/'.length) || `file-${rand(6)}.txt`;
    res.writeHead(200, { 'content-type': 'text/plain', 'content-disposition': `attachment; filename="${name}"` });
    return res.end(randText());
  }

  // Procore returns bare arrays (not {data, meta}) — page/per_page params.
  const per = Math.min(100, parseInt(url.searchParams.get('per_page') || url.searchParams.get('limit') || '25', 10) || 25);
  const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;
  const total = totalFor(path);
  const start = (page - 1) * per;
  const projectId = url.searchParams.get('project_id');
  const data = [];
  for (let i = start; i < Math.min(start + per, total); i++) {
    const rec = {
      id: `${slug(path)}-${i + 1}`,
      name: `${slug(path).replace(/-/g, ' ')} ${i + 1}`,
      created_at: '2026-01-01T00:00:00Z',
      url: `${ctx.base}/files/${slug(path)}-${i + 1}-${rand(6)}.txt`,
    };
    if (projectId) rec.project_id = projectId;
    data.push(rec);
  }
  json(res, 200, data);
}
