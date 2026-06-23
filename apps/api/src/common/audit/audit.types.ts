export interface AuditEntry {
  action: string;
  companyId: string | null;
  actorUserId: string | null;
  entityType?: string;
  entityId?: string;
  before?: unknown;
  after?: unknown;
}

export interface AuditLogRow {
  id: string;
  action: string;
  companyId: string | null;
  actorUserId: string | null;
  entityType: string | null;
  entityId: string | null;
  metadata: Record<string, unknown>;
}
