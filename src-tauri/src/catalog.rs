use crate::types::{Catalog, ChildInfo, EntityInfo};

#[derive(Debug, Clone, Copy)]
pub struct EntityDef {
    pub id: &'static str,
    pub label: &'static str,
    pub path: &'static str,
    pub group: &'static str,
    pub description: Option<&'static str>,
    /// Records may carry a downloadable file `url` field.
    pub downloadable: bool,
}

#[derive(Debug, Clone)]
pub struct ChildReq {
    pub path: String,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy)]
pub struct ChildDef {
    pub id: &'static str,
    pub label: &'static str,
    /// parent entity id, e.g. "jobs"
    pub parent: &'static str,
    pub description: Option<&'static str>,
    /// Response is a single object rather than a list.
    pub single: bool,
    /// Build the request for one parent record.
    pub request: fn(&str) -> ChildReq,
}

fn path_req(p: String) -> ChildReq {
    ChildReq {
        path: p,
        params: vec![],
    }
}

fn q_req(field: &str, value: &str) -> String {
    format!("'{}'.eq('{}')", field, value)
}

/**
 * Every GET list endpoint in the Prime v2 API, grouped for the UI.
 * Write endpoints (POST/PUT/PATCH/DELETE) are deliberately not represented —
 * this tool never modifies account data.
 */
pub const ENTITIES: &[EntityDef] = &[
    // Jobs & work
    EntityDef { id: "jobs", label: "Jobs", path: "/v2/jobs", group: "Jobs & Work", description: Some("All jobs in the account"), downloadable: false },
    EntityDef { id: "job-types", label: "Job Types", path: "/v2/job-types", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "job-links", label: "Job Links", path: "/v2/job-links", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "job-expense-invoices", label: "Job Expense Invoices", path: "/v2/job-expense-invoices", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "job-custom-fields", label: "Job Custom Field Values", path: "/v2/job-custom-fields", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "work-orders", label: "Work Orders", path: "/v2/work-orders", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "upstream-jobs", label: "Upstream Jobs", path: "/v2/upstream-jobs", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "upstream-work-orders", label: "Upstream Work Orders", path: "/v2/upstream-work-orders", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "allocations", label: "Allocations", path: "/v2/allocations", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "allocation-statuses", label: "Allocation Statuses", path: "/v2/allocation-statuses", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "milestones", label: "Milestones", path: "/v2/milestones", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "milestone-items", label: "Milestone Items", path: "/v2/milestone-items", group: "Jobs & Work", description: None, downloadable: false },
    EntityDef { id: "todo-items", label: "To-Do Items", path: "/v2/todo-items", group: "Jobs & Work", description: None, downloadable: false },
    // People & companies
    EntityDef { id: "contacts", label: "Contacts", path: "/v2/contacts", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "contact-licences", label: "Contact Licences", path: "/v2/contact-licences", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "contact-relationship-types", label: "Contact Relationship Types", path: "/v2/contact-relationship-types", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "users", label: "Users", path: "/v2/users", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "divisions", label: "Divisions", path: "/v2/divisions", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "regions", label: "Regions", path: "/v2/regions", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "trades", label: "Trades", path: "/v2/trades", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "client-portals", label: "Client Portals", path: "/v2/client-portals", group: "People & Companies", description: None, downloadable: false },
    EntityDef { id: "external-systems", label: "External Systems", path: "/v2/external-systems", group: "People & Companies", description: None, downloadable: false },
    // Financial
    EntityDef { id: "accounts-payable-invoices", label: "Accounts Payable Invoices", path: "/v2/accounts-payable-invoices", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "accounts-receivable-invoices", label: "Accounts Receivable Invoices", path: "/v2/accounts-receivable-invoices", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "reserve-categories", label: "Reserve Categories", path: "/v2/reserve-categories", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "reserve-items", label: "Reserve Items", path: "/v2/reserve-items", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "reserve-item-transactions", label: "Reserve Item Transactions", path: "/v2/reserve-item-transactions", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "reserve-item-histories", label: "Reserve Item Histories", path: "/v2/reserve-item-histories", group: "Financial", description: None, downloadable: false },
    EntityDef { id: "excess-outcome-types", label: "Excess Outcome Types", path: "/v2/excess-outcome-types", group: "Financial", description: None, downloadable: false },
    // Estimates
    EntityDef { id: "estimates", label: "Estimates", path: "/v2/estimates", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-items", label: "Estimate Items", path: "/v2/estimate-items", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-categories", label: "Estimate Categories", path: "/v2/estimate-categories", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimates-snapshot", label: "Estimates (Snapshot)", path: "/v2/estimates-snapshot", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-items-snapshot", label: "Estimate Items (Snapshot)", path: "/v2/estimate-items-snapshot", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-categories-snapshot", label: "Estimate Categories (Snapshot)", path: "/v2/estimate-categories-snapshot", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-template-items", label: "Estimate Template Items", path: "/v2/estimate-template-items", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-template-item-categories", label: "Estimate Template Item Categories", path: "/v2/estimate-template-item-categories", group: "Estimates", description: None, downloadable: false },
    EntityDef { id: "estimate-template-item-overrides", label: "Estimate Template Item Overrides", path: "/v2/estimate-template-item-overrides", group: "Estimates", description: None, downloadable: false },
    // Scheduling
    EntityDef { id: "schedules", label: "Schedules", path: "/v2/schedules", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "appointments", label: "Appointments", path: "/v2/appointments", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "appointment-types", label: "Appointment Types", path: "/v2/appointment-types", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "timesheets", label: "Timesheets", path: "/v2/timesheets", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "timesheet-activities", label: "Timesheet Activities", path: "/v2/timesheet-activities", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "timesheet-roles", label: "Timesheet Roles", path: "/v2/timesheet-roles", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "reminders", label: "Reminders", path: "/v2/reminders", group: "Scheduling", description: None, downloadable: false },
    EntityDef { id: "reminder-templates", label: "Reminder Templates", path: "/v2/reminder-templates", group: "Scheduling", description: None, downloadable: false },
    // Contents schedules
    EntityDef { id: "contents-schedules", label: "Contents Schedules", path: "/v2/contents-schedules", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-items", label: "Contents Schedule Items", path: "/v2/contents-schedule-items", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-products", label: "Contents Schedule Products", path: "/v2/contents-schedule-products", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-discounts", label: "Contents Schedule Discounts", path: "/v2/contents-schedule-discounts", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-settlements", label: "Contents Schedule Settlements", path: "/v2/contents-schedule-settlements", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-settlement-types", label: "Settlement Types", path: "/v2/contents-schedule-settlement-types", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-settlement-statuses", label: "Settlement Statuses", path: "/v2/contents-schedule-settlement-statuses", group: "Contents Schedules", description: None, downloadable: false },
    EntityDef { id: "contents-schedule-settlement-outcomes", label: "Settlement Outcomes", path: "/v2/contents-schedule-settlement-outcomes", group: "Contents Schedules", description: None, downloadable: false },
    // Health & safety
    EntityDef { id: "whs-items", label: "WHS Items", path: "/v2/whs-items", group: "Health & Safety", description: None, downloadable: false },
    EntityDef { id: "whs-item-types", label: "WHS Item Types", path: "/v2/whs-item-types", group: "Health & Safety", description: None, downloadable: false },
    EntityDef { id: "whs-item-statuses", label: "WHS Item Statuses", path: "/v2/whs-item-statuses", group: "Health & Safety", description: None, downloadable: false },
    // Site forms & feedback
    EntityDef { id: "site-forms", label: "Site Forms", path: "/v2/site-forms", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "site-form-templates", label: "Site Form Templates", path: "/v2/site-form-templates", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "feedback-items", label: "Feedback Items", path: "/v2/feedback-items", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "feedback-item-types", label: "Feedback Item Types", path: "/v2/feedback-item-types", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "feedback-item-categories", label: "Feedback Item Categories", path: "/v2/feedback-item-categories", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "feedback-item-statuses", label: "Feedback Item Statuses", path: "/v2/feedback-item-statuses", group: "Site Forms & Feedback", description: None, downloadable: false },
    EntityDef { id: "feedback-item-submission-types", label: "Feedback Submission Types", path: "/v2/feedback-item-submission-types", group: "Site Forms & Feedback", description: None, downloadable: false },
    // Files & attachments
    EntityDef { id: "attachments", label: "Attachments", path: "/v2/attachments", group: "Files & Attachments", description: Some("File records; enable \"download files\" to fetch the binaries"), downloadable: true },
    EntityDef { id: "attachment-types", label: "Attachment Types", path: "/v2/attachment-types", group: "Files & Attachments", description: None, downloadable: false },
    // Other
    EntityDef { id: "notifications", label: "Notifications", path: "/v2/notifications", group: "Other", description: None, downloadable: false },
    EntityDef { id: "notification-templates", label: "Notification Templates", path: "/v2/notification-templates", group: "Other", description: None, downloadable: false },
    EntityDef { id: "comments", label: "Comments (all)", path: "/v2/comments", group: "Other", description: Some("All comments; per-job comments are under Per-Job Details"), downloadable: false },
    EntityDef { id: "history-actions", label: "History Actions", path: "/v2/history-actions", group: "Other", description: None, downloadable: false },
    EntityDef { id: "powerbi-schema", label: "PowerBI Schema", path: "/v2/powerbi/schema", group: "Other", description: Some("Data model metadata (for reference)"), downloadable: false },
    // Reference
    EntityDef { id: "statuses", label: "Statuses", path: "/v2/statuses", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "perils", label: "Perils", path: "/v2/perils", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "catastrophe-codes", label: "Catastrophe Codes", path: "/v2/catastrophe-codes", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "licences", label: "Licences", path: "/v2/licences", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "licence-classifications", label: "Licence Classifications", path: "/v2/licence-classifications", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "custom-fields", label: "Custom Fields", path: "/v2/custom-fields", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "custom-field-options", label: "Custom Field Options", path: "/v2/custom-field-options", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "filter-types", label: "Filter Types", path: "/v2/filter-types", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "filter-tags", label: "Filter Tags", path: "/v2/filter-tags", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "workflows", label: "Workflows", path: "/v2/workflows", group: "Reference", description: None, downloadable: false },
    EntityDef { id: "units", label: "Units", path: "/v2/units", group: "Reference", description: None, downloadable: false },
];

/**
 * Per-job sub-resources. These endpoints either live under /jobs/{id}/ or
 * require a jobId filter, so they run once per job. Selecting any of these
 * requires the 'jobs' entity.
 */
pub const CHILDREN: &[ChildDef] = &[
    ChildDef {
        id: "job-notes",
        label: "Job Notes (Scribble Pad)",
        parent: "jobs",
        description: Some("GET /jobs/{id}/notes — one request per job"),
        single: true,
        request: |id| path_req(format!("/v2/jobs/{id}/notes")),
    },
    ChildDef {
        id: "job-reserve-transactions",
        label: "Job Reserve Transactions",
        parent: "jobs",
        description: None,
        single: false,
        request: |id| path_req(format!("/v2/jobs/{id}/reserve-transactions")),
    },
    ChildDef {
        id: "job-schedules",
        label: "Job Schedules",
        parent: "jobs",
        description: Some("GET /schedules/job/{id}"),
        single: false,
        request: |id| path_req(format!("/v2/schedules/job/{id}")),
    },
    ChildDef {
        id: "job-schedule-items",
        label: "Job Schedule Items",
        parent: "jobs",
        description: Some("GET /schedule-items/job/{id}"),
        single: false,
        request: |id| path_req(format!("/v2/schedule-items/job/{id}")),
    },
    ChildDef {
        id: "job-histories",
        label: "Job Histories",
        parent: "jobs",
        description: Some("GET /histories?q=jobId filter"),
        single: false,
        request: |id| ChildReq {
            path: "/v2/histories".into(),
            params: vec![("q".into(), q_req("jobId", id))],
        },
    },
    ChildDef {
        id: "job-status-histories",
        label: "Job Status Histories",
        parent: "jobs",
        description: None,
        single: false,
        request: |id| ChildReq {
            path: "/v2/status-histories".into(),
            params: vec![("q".into(), q_req("jobId", id))],
        },
    },
    ChildDef {
        id: "job-contact-relationships",
        label: "Job Contact Relationships",
        parent: "jobs",
        description: None,
        single: false,
        request: |id| ChildReq {
            path: "/v2/contact-relationships".into(),
            params: vec![("q".into(), q_req("jobId", id))],
        },
    },
    ChildDef {
        id: "job-comments",
        label: "Job Comments",
        parent: "jobs",
        description: Some("Comments where objectType='Job'"),
        single: false,
        request: |id| ChildReq {
            path: "/v2/comments".into(),
            params: vec![(
                "q".into(),
                format!("'objectType'.eq('Job'),'objectId'.eq('{id}')"),
            )],
        },
    },
];

pub fn catalog() -> Catalog {
    let mut groups = Vec::new();
    for e in ENTITIES {
        if !groups.contains(&e.group.to_string()) {
            groups.push(e.group.to_string());
        }
    }
    Catalog {
        entities: ENTITIES
            .iter()
            .map(|e| EntityInfo {
                id: e.id.into(),
                label: e.label.into(),
                group: e.group.into(),
                description: e.description.map(Into::into),
                downloadable: e.downloadable,
            })
            .collect(),
        children: CHILDREN
            .iter()
            .map(|c| ChildInfo {
                id: c.id.into(),
                label: c.label.into(),
                parent: c.parent.into(),
                description: c.description.map(Into::into),
            })
            .collect(),
        groups,
    }
}

pub fn entity_by_id(id: &str) -> Option<&'static EntityDef> {
    ENTITIES.iter().find(|e| e.id == id)
}

pub fn child_by_id(id: &str) -> Option<&'static ChildDef> {
    CHILDREN.iter().find(|c| c.id == id)
}

/// Extract a stable record id for dedupe/file naming.
pub fn record_id(rec: &serde_json::Value) -> Option<String> {
    let o = rec.as_object()?;
    for k in ["id", "uuid", "Id", "ID"] {
        if let Some(v) = o.get(k) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
            if let Some(n) = v.as_i64() {
                return Some(n.to_string());
            }
        }
    }
    None
}
