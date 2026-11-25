import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';
import type { Provider } from './types';

const ENTITY_SPECS: EntitySpec[] = [
  { id: 'companies', label: 'Companies', path: '/api/v1.0/companies/', group: 'Company', downloadable: false, description: 'Simpro companies — parent for all scoped data' },
];

const CHILD_SPECS: ChildSpec[] = [
  { id: 'jobs', label: 'Jobs', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/jobs/' },
  { id: 'customers', label: 'Customers', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/customers/' },
  { id: 'quotes', label: 'Quotes', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/quotes/' },
  { id: 'sites', label: 'Sites', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/sites/' },
  { id: 'employees', label: 'Employees', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/employees/' },
  { id: 'contractors', label: 'Contractors', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/contractors/' },
  { id: 'contacts', label: 'Contacts', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/contacts/' },
  { id: 'leads', label: 'Leads', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/leads/' },
  { id: 'invoices', label: 'Invoices', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/invoices/' },
  { id: 'customer-invoices', label: 'Customer Invoices', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/customerInvoices/' },
  { id: 'customer-payments', label: 'Customer Payments', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/customerPayments/' },
  { id: 'credit-notes', label: 'Credit Notes', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/creditNotes/' },
  { id: 'catalogs', label: 'Catalog Items', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/catalogs/' },
  { id: 'prebuilds', label: 'Prebuilds', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/prebuilds/' },
  { id: 'plants', label: 'Plant & Equipment', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/plants/' },
  { id: 'activity-schedules', label: 'Activity Schedules', parent: 'companies', single: false, path: '/api/v1.0/companies/{id}/activitySchedules/' },
];

const CATALOG: Catalog = {
  groups: ['Company'],
  entities: ENTITY_SPECS.map(({ path: _p, ...e }) => e),
  children: CHILD_SPECS.map(({ path: _p, qParam: _q, params: _pa, ...c }) => c),
};

export const SIMPRO: Provider = {
  id: 'simpro',
  name: 'Simpro',
  defaultHost: 'https://mysimpro.simprosuite.com',
  validatePath: '/api/v1.0/companies/',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://mysimpro.simprosuite.com' },
    { key: 'clientId', label: 'Client ID', placeholder: 'Paste your client ID' },
    {
      key: 'clientSecret',
      label: 'Client secret',
      placeholder: 'Paste your client secret',
      password: true,
      instructions: {
        title: 'How do I get credentials?',
        steps: [
          'In Simpro: System → Setup → API → API Applications.',
          'Create an application and copy the client ID + secret.',
          'DataFulton exchanges them for a bearer token automatically (POST /oauth/token).',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: CHILD_SPECS,
};
