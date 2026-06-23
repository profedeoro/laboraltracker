import { Injectable } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { ulid } from 'ulid';
import { AuditRepository } from './audit.repository';
import { AuditEntry } from './audit.types';

@Injectable()
export class AuditService {
  constructor(private readonly repo: AuditRepository) {}

  /**
   * Record a sensitive mutation. Pass the mutation's transaction client (`tx`) so
   * the audit row is atomic with the change it describes.
   */
  async record(entry: AuditEntry, tx?: Prisma.TransactionClient): Promise<void> {
    const metadata: Record<string, unknown> = {};
    if (entry.before !== undefined) metadata.before = entry.before;
    if (entry.after !== undefined) metadata.after = entry.after;

    await this.repo.insert(
      {
        id: ulid(),
        action: entry.action,
        companyId: entry.companyId,
        actorUserId: entry.actorUserId,
        entityType: entry.entityType ?? null,
        entityId: entry.entityId ?? null,
        metadata,
      },
      tx,
    );
  }
}
