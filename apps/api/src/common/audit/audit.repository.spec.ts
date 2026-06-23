import { ulid } from 'ulid';
import { PrismaService } from '../../database/prisma.service';
import { AuditRepository } from './audit.repository';

describe('AuditRepository (integration, real DB)', () => {
  const prisma = new PrismaService();
  const repo = new AuditRepository(prisma);

  beforeAll(async () => {
    await prisma.$connect();
  });
  afterAll(async () => {
    await prisma.$disconnect();
  });

  it('inserts an append-only audit row', async () => {
    const id = ulid();
    await repo.insert({
      id,
      action: 'test.insert',
      companyId: null,
      actorUserId: null,
      entityType: null,
      entityId: null,
      metadata: { before: { a: 1 } },
    });

    const found = await prisma.auditLog.findUnique({ where: { id } });
    expect(found?.action).toBe('test.insert');
    expect(found?.metadata).toEqual({ before: { a: 1 } });

    await prisma.auditLog.delete({ where: { id } }); // test cleanup only
  });

  it('is atomic: an insert inside a rolled-back tx leaves no row', async () => {
    const id = ulid();
    await expect(
      prisma.$transaction(async (tx) => {
        await repo.insert(
          {
            id,
            action: 'test.rollback',
            companyId: null,
            actorUserId: null,
            entityType: null,
            entityId: null,
            metadata: {},
          },
          tx,
        );
        throw new Error('force rollback');
      }),
    ).rejects.toThrow('force rollback');

    const found = await prisma.auditLog.findUnique({ where: { id } });
    expect(found).toBeNull();
  });
});
