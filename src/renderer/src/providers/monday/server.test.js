/**
 * Test server for the Monday.com provider (GraphQL POST /v2).
 * Parses the query name + page/limit from the body and returns {data:{name:[...]}}.
 */

export const providerId = 'monday';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'monday board item pulse update workspace'.split(' ');
const randText = () => Array.from({ length: 40 }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';
const totalFor = (p) => 30 + (p.length % 25);

const json = (res, code, body) => {
  res.writeHead(code, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
};

export function handle(req, res, url, ctx) {
  const path = url.pathname || '/';

  if (path.startsWith('/files/')) {
    const name = path.slice('/files/'.length) || `file-${rand(6)}.txt`;
    res.writeHead(200, { 'content-type': 'text/plain', 'content-disposition': `attachment; filename="${name}"` });
    return res.end(randText());
  }

  if (path === '/v2/get_schema' && req.method === 'GET') {
    return json(res, 200, { data: { __schema: { queryType: { name: 'Query' } } } });
  }

  if (path === '/v2' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      let query = '';
      try { query = JSON.parse(body).query || ''; } catch {}
      const m = /\{\s*(\w+)/.exec(query);
      const name = m ? m[1] : 'boards';
      const limit = Math.min(100, parseInt(/limit:\s*(\d+)/.exec(query)?.[1] || '25', 10));
      const page = parseInt(/page:\s*(\d+)/.exec(query)?.[1] || '1', 10);
      const total = totalFor(name);
      const start = (page - 1) * limit;
      const items = [];
      for (let i = start; i < Math.min(start + limit, total); i++) {
        items.push({
          id: `${name}-${i + 1}`,
          name: `${name} ${i + 1}`,
          created_at: '2026-01-01T00:00:00Z',
          url: `${ctx.base}/files/${name}-${i + 1}-${rand(6)}.txt`,
        });
      }
      json(res, 200, { data: { [name]: items } });
    });
    return;
  }

  json(res, 404, { message: `Not found: ${path}` });
}
