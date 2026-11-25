/**
 * Test server for the Prime Ecosystem provider.
 * Replicates the Prime v2 API response shapes ({data, meta:{...}} pagination)
 * with generated dummy data. Records carry `url` fields pointing at the
 * `/files/` endpoint, which serves random .txt bodies — so file downloads
 * are exercised end to end.
 */

export const providerId = 'primeeco';

const rand = (n) => Math.random().toString(36).slice(2, 2 + n);
const words = 'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor'.split(' ');
const randText = () => Array.from({ length: 30 + Math.floor(Math.random() * 70) }, () => words[Math.floor(Math.random() * words.length)]).join(' ') + '\n';

const slug = (path) => path.split('/').filter(Boolean).pop() || 'record';

/** Records for a list endpoint — deterministic count derived from the path. */
const totalFor = (path) => 35 + (path.length % 30);

const makeRecord = (path, i, base) => {
  const rec = {
    id: `${slug(path)}-${i + 1}`,
    name: `${slug(path).replace(/-/g, ' ')} ${i + 1}`,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-02T00:00:00Z',
  };
  // every record carries a downloadable url → exercises attachment downloads
  rec.url = `${base}/files/${slug(path)}-${i + 1}-${rand(6)}.txt`;
  return rec;
};

const listBody = (path, page, per, base) => {
  const total = totalFor(path);
  const start = (page - 1) * per;
  const data = [];
  for (let i = start; i < Math.min(start + per, total); i++) {
    data.push(makeRecord(path, i, base));
  }
  return { data, meta: { current_page: page, last_page: Math.max(1, Math.ceil(total / per)), per_page: per, total } };
};

const json = (res, code, body) => {
  const b = JSON.stringify(body);
  res.writeHead(code, { 'content-type': 'application/json' });
  res.end(b);
};

/**
 * Handle a request whose path is already stripped of the /api/{providerId}
 * prefix. `ctx` = { apiKey, base }.
 */
export function handle(req, res, url, ctx) {
  const path = url.pathname || '/';

  // ---- file downloads: random text, no auth needed ----
  if (path.startsWith('/files/')) {
    const name = path.slice('/files/'.length) || `file-${rand(6)}.txt`;
    res.writeHead(200, { 'content-type': 'text/plain', 'content-disposition': `attachment; filename="${name}"` });
    res.end(randText());
    return;
  }

  // ---- single-object child endpoints: /v2/{x}/{id}/{leaf} (3+ segments) ----
  const parts = path.split('/').filter(Boolean); // e.g. ['v2','jobs','jobs-1','notes']
  if (parts.length >= 4) {
    const id = parts[2];
    const leaf = parts.slice(3).join('/');
    if (leaf === 'notes') {
      return json(res, 200, { data: { jobId: id, notes: `Notes for ${id}` } });
    }
    return json(res, 200, { data: [{ id: `${leaf}-${rand(4)}`, parentId: id, name: `${leaf} row` }] });
  }

  // ---- q-filtered child endpoints + plain list endpoints ----
  if (parts.length >= 2 && parts[0] === 'v2') {
    const q = url.searchParams.get('q');
    const page = parseInt(url.searchParams.get('page') || '1', 10) || 1;
    const per = Math.min(200, parseInt(url.searchParams.get('per_page') || '50', 10) || 50);
    if (q) {
      const m = /eq\('([^']+)'\)/.exec(q);
      const parentId = m ? m[1] : '';
      return json(res, 200, {
        data: [{ id: `${slug(path)}-${rand(4)}`, parentId, name: `${slug(path)} filtered row` }],
        meta: { current_page: 1, last_page: 1, per_page: per, total: 1 },
      });
    }
    return json(res, 200, listBody(path, page, per, ctx.base));
  }

  json(res, 404, { message: `Not found: ${path}` });
}
