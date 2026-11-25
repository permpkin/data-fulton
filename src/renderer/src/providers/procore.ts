import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';
import type { Provider } from './types';

const ENTITY_SPECS: EntitySpec[] = [
  { id: 'companies', label: 'Companies', path: '/rest/v1.0/companies', group: 'Company', downloadable: false },
  { id: 'users', label: 'Company Users', path: '/rest/v1.0/users', group: 'Company', downloadable: false },
  { id: 'vendors', label: 'Vendors', path: '/rest/v1.0/vendors', group: 'Company', downloadable: false },
  { id: 'projects', label: 'Projects', path: '/rest/v1.0/projects', group: 'Projects', downloadable: false, description: 'All projects in the account' },
  { id: 'commitments', label: 'Commitments', path: '/rest/v1.0/commitments', group: 'Financial', downloadable: false },
  { id: 'contract-payments', label: 'Contract Payments', path: '/rest/v1.0/contract_payments', group: 'Financial', downloadable: false },
  { id: 'budget-views', label: 'Budget Views', path: '/rest/v1.0/budget_views', group: 'Financial', downloadable: false },
  { id: 'cost-codes', label: 'Cost Codes', path: '/rest/v1.0/cost_codes', group: 'Financial', downloadable: false },
  { id: 'checklists', label: 'Checklists / Inspections', path: '/rest/v1.0/checklist/lists', group: 'Quality & Safety', downloadable: false },
  { id: 'observations', label: 'Observations', path: '/rest/v1.0/observations/items', group: 'Quality & Safety', downloadable: false },
  { id: 'spec-sections', label: 'Specification Sections', path: '/rest/v1.0/specification_sections', group: 'Specifications', downloadable: false },
];

const CHILD_SPECS: ChildSpec[] = [
  { id: 'project-punch-items', label: 'Punch Items', parent: 'projects', single: false, path: '/rest/v1.0/punch_items', params: [['project_id', '{id}']] },
  { id: 'project-meetings', label: 'Meetings', parent: 'projects', single: false, path: '/rest/v1.0/meetings', params: [['project_id', '{id}']] },
  { id: 'project-checklists', label: 'Checklists', parent: 'projects', single: false, path: '/rest/v1.0/checklist/lists', params: [['project_id', '{id}']] },
];

const CATALOG: Catalog = {
  groups: ['Company', 'Projects', 'Financial', 'Quality & Safety', 'Specifications'],
  entities: ENTITY_SPECS.map(({ path: _p, ...e }) => e),
  children: CHILD_SPECS.map(({ path: _p, qParam: _q, params: _pa, ...c }) => c),
};

export const PROCORE: Provider = {
  id: 'procore',
  name: 'Procore',
  defaultHost: 'https://api.procore.com',
  validatePath: '/rest/v1.0/companies',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://api.procore.com' },
    { key: 'clientId', label: 'Client ID', placeholder: 'Paste your client ID' },
    {
      key: 'clientSecret',
      label: 'Client secret',
      placeholder: 'Paste your client secret',
      password: true,
      instructions: {
        title: 'How do I get credentials?',
        steps: [
          'Create an app in your Procore Developer Portal (company level).',
          'Copy the generated client ID + secret.',
          'DataFulton exchanges them for a bearer token automatically (POST /oauth/token).',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: CHILD_SPECS,
};
