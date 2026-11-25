import type { Catalog, EntitySpec } from '@shared/types';
import type { Provider } from './types';

/** GraphQL entities — each POSTs /v2 with a query; {page}/{per_page} substitute per request. */
const gql = (name: string, args: string, fields: string, dataPath: string): EntitySpec => ({
  id: name.replace(/_/g, '-'),
  label: name.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase()),
  path: '/v2',
  method: 'POST',
  bodyTemplate: `{"query":"{ ${name}(${args}) { ${fields} } }"}`,
  dataPath: `data.${dataPath}`,
  group: 'Monday.com',
  downloadable: false,
});

const ENTITY_SPECS: EntitySpec[] = [
  gql('boards', 'limit: {per_page}, page: {page}', 'id name state created_at', 'boards'),
  gql('users', 'limit: {per_page}', 'id name email', 'users'),
  gql('workspaces', 'limit: {per_page}', 'id name', 'workspaces'),
  gql('tags', 'limit: {per_page}', 'id name color', 'tags'),
  gql('updates', 'limit: {per_page}, page: {page}', 'id body created_at', 'updates'),
  gql('notifications', 'limit: {per_page}', 'id text created_at', 'notifications'),
];

const CATALOG: Catalog = {
  groups: ['Monday.com'],
  entities: ENTITY_SPECS.map(({ path: _p, method: _m, bodyTemplate: _b, dataPath: _d, ...e }) => e),
  children: [],
};

export const MONDAY: Provider = {
  id: 'monday',
  name: 'Monday.com',
  defaultHost: 'https://api.monday.com',
  validatePath: '/v2/get_schema',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://api.monday.com' },
    {
      key: 'apiKey',
      label: 'API token',
      placeholder: 'Paste your API token',
      password: true,
      instructions: {
        title: 'How do I get an API token?',
        steps: [
          'In Monday.com: Avatar → Administration → API → Personal API Token.',
          'Copy the token and paste it above.',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: [],
};
