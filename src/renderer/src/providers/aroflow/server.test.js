/**
 * Test server for the AroFlo provider.
 * Bearer auth + {data, meta} paginated resources at /{resource}.
 */

export const providerId = 'aroflow';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'aroflo task client site invoice labour material'.split(' ');
const randText = () => Array.from({ length: 40 }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';

const slug = (path) => path.split('/').filter(Boolean).pop() || 'record';
const totalFor = (path) => 30 + (path.length % 25);

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

  const parts = path.split('/').filter(Boolean);
  const per = Math.min(200, parseInt(url.searchParams.get('per_page') || '50', 10) || 50);
  const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;

  // child endpoint: /{parent}/{id}/{leaf}
  if (parts.length >= 3) {
    const parentId = parts[1];
    const leaf = parts.slice(2).join('/');
    return json(res, 200, {
      data: [{ id: `${leaf}-${rand(4)}`, parentId, name: `${leaf} row` }],
      meta: { current_page: 1, last_page: 1, per_page: per, total: 1 },
    });
  }

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
