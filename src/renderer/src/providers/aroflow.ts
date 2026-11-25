import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';
import type { Provider } from './types';

const ENTITY_SPECS: EntitySpec[] = [
  { id: 'clients', label: 'Clients', path: '/clients', group: 'Clients', downloadable: false },
  { id: 'contacts', label: 'Contacts', path: '/contacts', group: 'Clients', downloadable: false },
  { id: 'locations', label: 'Locations / Sites', path: '/locations', group: 'Clients', downloadable: false },
  { id: 'tasks', label: 'Tasks', path: '/tasks', group: 'Jobs', downloadable: false },
  { id: 'workorders', label: 'Work Orders', path: '/workorders', group: 'Jobs', downloadable: false },
  { id: 'quotes', label: 'Quotes', path: '/quotes', group: 'Financial', downloadable: false },
  { id: 'invoices', label: 'Invoices', path: '/invoices', group: 'Financial', downloadable: false },
  { id: 'purchaseorders', label: 'Purchase Orders', path: '/purchaseorders', group: 'Financial', downloadable: false },
  { id: 'materials', label: 'Materials', path: '/materials', group: 'Inventory', downloadable: false },
  { id: 'users', label: 'Users', path: '/users', group: 'Company', downloadable: false },
];

const CHILD_SPECS: ChildSpec[] = [
  { id: 'task-labour', label: 'Task Labour Entries', parent: 'tasks', single: false, path: '/tasks/{id}/labour' },
  { id: 'task-materials', label: 'Task Materials', parent: 'tasks', single: false, path: '/tasks/{id}/materials' },
  { id: 'task-notes', label: 'Task Notes', parent: 'tasks', single: false, path: '/tasks/{id}/notes' },
  { id: 'invoice-line-items', label: 'Invoice Line Items', parent: 'invoices', single: false, path: '/invoices/{id}/lineitems' },
];

const CATALOG: Catalog = {
  groups: ['Clients', 'Jobs', 'Financial', 'Inventory', 'Company'],
  entities: ENTITY_SPECS.map(({ path: _p, ...e }) => e),
  children: CHILD_SPECS.map(({ path: _p, qParam: _q, params: _pa, ...c }) => c),
};

export const AROFLOW: Provider = {
  id: 'aroflow',
  name: 'AroFlo',
  defaultHost: 'https://api.aroflo.com',
  validatePath: '/users',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://api.aroflo.com' },
    {
      key: 'apiKey',
      label: 'API key',
      placeholder: 'Paste your secret key',
      password: true,
      instructions: {
        title: 'How do I get an API key?',
        steps: [
          'In AroFlo: Site Admin → Integrations → API Access.',
          'Generate/copy your secret key.',
          'Note: the real AroFlo API uses HMAC-signed headers — full signing support is not implemented yet; the test server accepts it as a Bearer token.',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: CHILD_SPECS,
};
