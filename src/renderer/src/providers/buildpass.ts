import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';
import type { Provider } from './types';

const ENTITY_SPECS: EntitySpec[] = [
  { id: 'builders', label: 'Builders', path: '/builders', group: 'Builders', downloadable: false, description: 'All builders on the account' },
  { id: 'subcontractors', label: 'Subcontractors', path: '/subcontractors', group: 'Subcontractors', downloadable: false, description: 'All subcontractors' },
  { id: 'timesheets', label: 'Timesheets', path: '/timesheets', group: 'Timesheets', downloadable: false },
  { id: 'time-codes', label: 'Time Codes', path: '/timesheets/cost-codes', group: 'Timesheets', downloadable: false },
  { id: 'time-types', label: 'Time Types', path: '/timesheets/time-types', group: 'Timesheets', downloadable: false },
];

const CHILD_SPECS: ChildSpec[] = [
  { id: 'builder-projects', label: 'Projects', parent: 'builders', single: false, path: '/builders/{id}/projects' },
  { id: 'builder-defects', label: 'Defects', parent: 'builders', single: false, path: '/builders/{id}/defects' },
  { id: 'builder-meetings', label: 'Meetings', parent: 'builders', single: false, path: '/builders/{id}/meetings' },
  { id: 'builder-photo-folders', label: 'Photo Folders', parent: 'builders', single: false, path: '/builders/{id}/photo-folders' },
  { id: 'builder-photos', label: 'Photos', parent: 'builders', single: false, path: '/builders/{id}/photos' },
  { id: 'sub-contacts', label: 'Contacts', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/contacts' },
  { id: 'sub-inductions', label: 'Inductions', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/inductions' },
  { id: 'sub-insurances', label: 'Insurances', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/insurances' },
  { id: 'sub-prequalifications', label: 'Prequalifications', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/prequalifications' },
  { id: 'sub-projects', label: 'Projects', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/projects' },
  { id: 'sub-swms', label: 'SWMS Documents', parent: 'subcontractors', single: false, path: '/subcontractors/{id}/swms' },
];

const CATALOG: Catalog = {
  groups: ['Builders', 'Subcontractors', 'Timesheets'],
  entities: ENTITY_SPECS.map(({ path: _p, ...e }) => e),
  children: CHILD_SPECS.map(({ path: _p, qParam: _q, ...c }) => c),
};

export const BUILDPASS: Provider = {
  id: 'buildpass',
  name: 'BuildPass',
  defaultHost: 'https://api.buildpass.global',
  validatePath: '/builders',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://api.buildpass.global' },
    { key: 'clientId', label: 'Client ID', placeholder: 'Paste your client ID' },
    {
      key: 'clientSecret',
      label: 'Client secret',
      placeholder: 'Paste your client secret',
      password: true,
      instructions: {
        title: 'How do I get credentials?',
        steps: [
          'Request API access from BuildPass (approved integrators only).',
          'Copy your client ID + secret from the integration dashboard.',
          'DataFulton exchanges them for a bearer token automatically (POST /oauth/token).',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: CHILD_SPECS,
};
