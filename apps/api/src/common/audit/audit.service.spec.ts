import { AuditService } from './audit.service';
import { AuditRepository } from './audit.repository';
import { AuditActions } from './audit-actions';

describe('AuditService', () => {
  it('builds a row with a ULID and serializes before/after into metadata', async () => {
    const insert = jest.fn().mockResolvedValue(undefined);
    const repo = { insert } as unknown as AuditRepository;
    const service = new AuditService(repo);

    await service.record({
      action: AuditActions.ProjectDeleted,
      companyId: 'c1',
      actorUserId: 'u1',
      entityType: 'Project',
      entityId: 'p1',
      before: { name: 'Old' },
      after: { name: 'Old', deletedAt: 'now' },
    });

    expect(insert).toHaveBeenCalledTimes(1);
    const [row, tx] = insert.mock.calls[0];
    expect(row.id).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/); // ULID
    expect(row.action).toBe('project.deleted');
    expect(row.companyId).toBe('c1');
    expect(row.entityType).toBe('Project');
    expect(row.metadata).toEqual({
      before: { name: 'Old' },
      after: { name: 'Old', deletedAt: 'now' },
    });
    expect(tx).toBeUndefined();
  });

  it('omits before/after from metadata when not provided, and forwards tx', async () => {
    const insert = jest.fn().mockResolvedValue(undefined);
    const repo = { insert } as unknown as AuditRepository;
    const service = new AuditService(repo);
    const fakeTx = {} as never;

    await service.record(
      { action: AuditActions.MemberInvited, companyId: 'c1', actorUserId: 'u1' },
      fakeTx,
    );

    const [row, tx] = insert.mock.calls[0];
    expect(row.metadata).toEqual({});
    expect(row.entityType).toBeNull();
    expect(tx).toBe(fakeTx);
  });
});
