import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';
import type { Provider } from './types';
export type { Provider } from './types';



const ENTITY_SPECS: EntitySpec[] = [
  {
    'id': 'jobs',
    'label': 'Jobs',
    'path': '/v2/jobs',
    'group': 'Jobs & Work',
    'downloadable': false,
    'description': 'All jobs in the account'
  },
  {
    'id': 'job-types',
    'label': 'Job Types',
    'path': '/v2/job-types',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'job-links',
    'label': 'Job Links',
    'path': '/v2/job-links',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'job-expense-invoices',
    'label': 'Job Expense Invoices',
    'path': '/v2/job-expense-invoices',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'job-custom-fields',
    'label': 'Job Custom Field Values',
    'path': '/v2/job-custom-fields',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'work-orders',
    'label': 'Work Orders',
    'path': '/v2/work-orders',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'upstream-jobs',
    'label': 'Upstream Jobs',
    'path': '/v2/upstream-jobs',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'upstream-work-orders',
    'label': 'Upstream Work Orders',
    'path': '/v2/upstream-work-orders',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'allocations',
    'label': 'Allocations',
    'path': '/v2/allocations',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'allocation-statuses',
    'label': 'Allocation Statuses',
    'path': '/v2/allocation-statuses',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'milestones',
    'label': 'Milestones',
    'path': '/v2/milestones',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'milestone-items',
    'label': 'Milestone Items',
    'path': '/v2/milestone-items',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'todo-items',
    'label': 'To-Do Items',
    'path': '/v2/todo-items',
    'group': 'Jobs & Work',
    'downloadable': false
  },
  {
    'id': 'contacts',
    'label': 'Contacts',
    'path': '/v2/contacts',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'contact-licences',
    'label': 'Contact Licences',
    'path': '/v2/contact-licences',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'contact-relationship-types',
    'label': 'Contact Relationship Types',
    'path': '/v2/contact-relationship-types',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'users',
    'label': 'Users',
    'path': '/v2/users',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'divisions',
    'label': 'Divisions',
    'path': '/v2/divisions',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'regions',
    'label': 'Regions',
    'path': '/v2/regions',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'trades',
    'label': 'Trades',
    'path': '/v2/trades',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'client-portals',
    'label': 'Client Portals',
    'path': '/v2/client-portals',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'external-systems',
    'label': 'External Systems',
    'path': '/v2/external-systems',
    'group': 'People & Companies',
    'downloadable': false
  },
  {
    'id': 'accounts-payable-invoices',
    'label': 'Accounts Payable Invoices',
    'path': '/v2/accounts-payable-invoices',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'accounts-receivable-invoices',
    'label': 'Accounts Receivable Invoices',
    'path': '/v2/accounts-receivable-invoices',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'reserve-categories',
    'label': 'Reserve Categories',
    'path': '/v2/reserve-categories',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'reserve-items',
    'label': 'Reserve Items',
    'path': '/v2/reserve-items',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'reserve-item-transactions',
    'label': 'Reserve Item Transactions',
    'path': '/v2/reserve-item-transactions',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'reserve-item-histories',
    'label': 'Reserve Item Histories',
    'path': '/v2/reserve-item-histories',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'excess-outcome-types',
    'label': 'Excess Outcome Types',
    'path': '/v2/excess-outcome-types',
    'group': 'Financial',
    'downloadable': false
  },
  {
    'id': 'estimates',
    'label': 'Estimates',
    'path': '/v2/estimates',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-items',
    'label': 'Estimate Items',
    'path': '/v2/estimate-items',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-categories',
    'label': 'Estimate Categories',
    'path': '/v2/estimate-categories',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimates-snapshot',
    'label': 'Estimates (Snapshot)',
    'path': '/v2/estimates-snapshot',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-items-snapshot',
    'label': 'Estimate Items (Snapshot)',
    'path': '/v2/estimate-items-snapshot',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-categories-snapshot',
    'label': 'Estimate Categories (Snapshot)',
    'path': '/v2/estimate-categories-snapshot',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-template-items',
    'label': 'Estimate Template Items',
    'path': '/v2/estimate-template-items',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-template-item-categories',
    'label': 'Estimate Template Item Categories',
    'path': '/v2/estimate-template-item-categories',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'estimate-template-item-overrides',
    'label': 'Estimate Template Item Overrides',
    'path': '/v2/estimate-template-item-overrides',
    'group': 'Estimates',
    'downloadable': false
  },
  {
    'id': 'schedules',
    'label': 'Schedules',
    'path': '/v2/schedules',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'appointments',
    'label': 'Appointments',
    'path': '/v2/appointments',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'appointment-types',
    'label': 'Appointment Types',
    'path': '/v2/appointment-types',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'timesheets',
    'label': 'Timesheets',
    'path': '/v2/timesheets',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'timesheet-activities',
    'label': 'Timesheet Activities',
    'path': '/v2/timesheet-activities',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'timesheet-roles',
    'label': 'Timesheet Roles',
    'path': '/v2/timesheet-roles',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'reminders',
    'label': 'Reminders',
    'path': '/v2/reminders',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'reminder-templates',
    'label': 'Reminder Templates',
    'path': '/v2/reminder-templates',
    'group': 'Scheduling',
    'downloadable': false
  },
  {
    'id': 'contents-schedules',
    'label': 'Contents Schedules',
    'path': '/v2/contents-schedules',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-items',
    'label': 'Contents Schedule Items',
    'path': '/v2/contents-schedule-items',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-products',
    'label': 'Contents Schedule Products',
    'path': '/v2/contents-schedule-products',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-discounts',
    'label': 'Contents Schedule Discounts',
    'path': '/v2/contents-schedule-discounts',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-settlements',
    'label': 'Contents Schedule Settlements',
    'path': '/v2/contents-schedule-settlements',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-settlement-types',
    'label': 'Settlement Types',
    'path': '/v2/contents-schedule-settlement-types',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-settlement-statuses',
    'label': 'Settlement Statuses',
    'path': '/v2/contents-schedule-settlement-statuses',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'contents-schedule-settlement-outcomes',
    'label': 'Settlement Outcomes',
    'path': '/v2/contents-schedule-settlement-outcomes',
    'group': 'Contents Schedules',
    'downloadable': false
  },
  {
    'id': 'whs-items',
    'label': 'WHS Items',
    'path': '/v2/whs-items',
    'group': 'Health & Safety',
    'downloadable': false
  },
  {
    'id': 'whs-item-types',
    'label': 'WHS Item Types',
    'path': '/v2/whs-item-types',
    'group': 'Health & Safety',
    'downloadable': false
  },
  {
    'id': 'whs-item-statuses',
    'label': 'WHS Item Statuses',
    'path': '/v2/whs-item-statuses',
    'group': 'Health & Safety',
    'downloadable': false
  },
  {
    'id': 'site-forms',
    'label': 'Site Forms',
    'path': '/v2/site-forms',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'site-form-templates',
    'label': 'Site Form Templates',
    'path': '/v2/site-form-templates',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'feedback-items',
    'label': 'Feedback Items',
    'path': '/v2/feedback-items',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'feedback-item-types',
    'label': 'Feedback Item Types',
    'path': '/v2/feedback-item-types',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'feedback-item-categories',
    'label': 'Feedback Item Categories',
    'path': '/v2/feedback-item-categories',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'feedback-item-statuses',
    'label': 'Feedback Item Statuses',
    'path': '/v2/feedback-item-statuses',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'feedback-item-submission-types',
    'label': 'Feedback Submission Types',
    'path': '/v2/feedback-item-submission-types',
    'group': 'Site Forms & Feedback',
    'downloadable': false
  },
  {
    'id': 'attachment-types',
    'label': 'Attachment Types',
    'path': '/v2/attachment-types',
    'group': 'Files & Attachments',
    'downloadable': false
  },
  {
    'id': 'notifications',
    'label': 'Notifications',
    'path': '/v2/notifications',
    'group': 'Other',
    'downloadable': false
  },
  {
    'id': 'notification-templates',
    'label': 'Notification Templates',
    'path': '/v2/notification-templates',
    'group': 'Other',
    'downloadable': false
  },
  {
    'id': 'comments',
    'label': 'Comments (all)',
    'path': '/v2/comments',
    'group': 'Other',
    'downloadable': false,
    'description': 'All comments; per-job comments are under Per-Job Details'
  },
  {
    'id': 'history-actions',
    'label': 'History Actions',
    'path': '/v2/history-actions',
    'group': 'Other',
    'downloadable': false
  },
  {
    'id': 'powerbi-schema',
    'label': 'PowerBI Schema',
    'path': '/v2/powerbi/schema',
    'group': 'Other',
    'downloadable': false,
    'description': 'Data model metadata (for reference)'
  },
  {
    'id': 'statuses',
    'label': 'Statuses',
    'path': '/v2/statuses',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'perils',
    'label': 'Perils',
    'path': '/v2/perils',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'catastrophe-codes',
    'label': 'Catastrophe Codes',
    'path': '/v2/catastrophe-codes',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'licences',
    'label': 'Licences',
    'path': '/v2/licences',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'licence-classifications',
    'label': 'Licence Classifications',
    'path': '/v2/licence-classifications',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'custom-fields',
    'label': 'Custom Fields',
    'path': '/v2/custom-fields',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'custom-field-options',
    'label': 'Custom Field Options',
    'path': '/v2/custom-field-options',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'filter-types',
    'label': 'Filter Types',
    'path': '/v2/filter-types',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'filter-tags',
    'label': 'Filter Tags',
    'path': '/v2/filter-tags',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'workflows',
    'label': 'Workflows',
    'path': '/v2/workflows',
    'group': 'Reference',
    'downloadable': false
  },
  {
    'id': 'units',
    'label': 'Units',
    'path': '/v2/units',
    'group': 'Reference',
    'downloadable': false
  }
];

const CHILD_SPECS: ChildSpec[] = [
  {
    'id': 'job-notes',
    'label': 'Job Notes (Scribble Pad)',
    'parent': 'jobs',
    'single': true,
    'path': '/v2/jobs/{id}/notes'
  },
  {
    'id': 'job-reserve-transactions',
    'label': 'Job Reserve Transactions',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/jobs/{id}/reserve-transactions'
  },
  {
    'id': 'job-schedules',
    'label': 'Job Schedules',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/schedules/job/{id}'
  },
  {
    'id': 'job-schedule-items',
    'label': 'Job Schedule Items',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/schedule-items/job/{id}'
  },
  {
    'id': 'job-histories',
    'label': 'Job Histories',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/histories',
    "qParam": "'jobId'.eq('{id}')"
  },
  {
    'id': 'job-status-histories',
    'label': 'Job Status Histories',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/status-histories',
    "qParam": "'jobId'.eq('{id}')"
  },
  {
    'id': 'job-contact-relationships',
    'label': 'Job Contact Relationships',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/contact-relationships',
    "qParam": "'jobId'.eq('{id}')"
  },
  {
    'id': 'job-comments',
    'label': 'Job Comments',
    'parent': 'jobs',
    'single': false,
    'path': '/v2/comments',
    "qParam": "'objectType'.eq('Job'),'objectId'.eq('{id}')"
  }
];

const CATALOG: Catalog = {
  groups: [
  'Jobs & Work',
  'People & Companies',
  'Financial',
  'Estimates',
  'Scheduling',
  'Contents Schedules',
  'Health & Safety',
  'Site Forms & Feedback',
  'Files & Attachments',
  'Other',
  'Reference'
],
  entities: ENTITY_SPECS.map(({ path: _p, ...e }) => e),
  children: CHILD_SPECS.map(({ path: _p, qParam: _q, ...c }) => c),
};

export const PRIME_ECO: Provider = {
  id: 'primeeco',
  name: 'Prime Ecosystem',
  defaultHost: 'https://www.primeeco.tech/api.prime',
  validatePath: '/v2/users',
  fields: [
    { key: 'host', label: 'Host', placeholder: 'https://www.primeeco.tech/api.prime' },
    {
      key: 'apiKey',
      label: 'API key',
      placeholder: 'Paste your API key',
      password: true,
      instructions: {
        title: 'How do I get an API key?',
        steps: [
          'Sign in to your API account in a browser.',
          'Open account/profile settings → API / API Tokens / Developer.',
          'Create a token, copy it, paste it above.',
        ],
      },
    },
  ],
  catalog: () => Promise.resolve(CATALOG),
  entitySpecs: ENTITY_SPECS,
  childSpecs: CHILD_SPECS,
};
