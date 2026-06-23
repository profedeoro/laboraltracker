/**
 * Single source of truth for the Phase 2 audit vocabulary. Later slices reference
 * these constants instead of scattering magic strings. Only sensitive mutations
 * (deletes, role/status/settings changes) are audited — proportion by sensitivity.
 */
export const AuditActions = {
  ProjectDeleted: 'project.deleted',
  TaskDeleted: 'task.deleted',
  MemberInvited: 'member.invited',
  MemberRoleChanged: 'member.role_changed',
  MemberStatusChanged: 'member.status_changed',
  TeamDeleted: 'team.deleted',
  TeamManagerChanged: 'team.manager_changed',
  CompanyUpdated: 'company.updated',
  CompanyCreated: 'company.created',
  CompanyPlanChanged: 'company.plan_changed',
  CompanyStatusChanged: 'company.status_changed',
} as const;

export type AuditAction = (typeof AuditActions)[keyof typeof AuditActions];
