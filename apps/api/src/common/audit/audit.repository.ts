import { Injectable } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { PrismaService } from '../../database/prisma.service';
import { AuditLogRow } from './audit.types';

@Injectable()
export class AuditRepository {
  constructor(private readonly prisma: PrismaService) {}

  /**
   * Append-only insert. Uses the caller's transaction client when given, so the
   * audit row and the mutation commit (or roll back) together.
   */
  async insert(row: AuditLogRow, tx?: Prisma.TransactionClient): Promise<void> {
    const db = tx ?? this.prisma;
    await db.auditLog.create({
      data: {
        id: row.id,
        action: row.action,
        companyId: row.companyId,
        actorUserId: row.actorUserId,
        entityType: row.entityType,
        entityId: row.entityId,
        metadata: row.metadata as Prisma.InputJsonValue,
      },
    });
  }
}
