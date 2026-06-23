import { ExecutionContext, ForbiddenException } from '@nestjs/common';
import { TenantGuard } from './tenant.guard';
import type { AuthRepository } from '../../modules/auth/auth.repository';
import type { AuthCtx } from '../auth/auth-context';

function makeContext(authCtx: Partial<AuthCtx> | undefined): ExecutionContext {
  return {
    switchToHttp: () => ({ getRequest: () => ({ authCtx }) }),
  } as unknown as ExecutionContext;
}

const membership = {
  companyId: 'c1',
  companyName: 'Empresa A',
  companySlug: 'empresa-a',
  roleKey: 'COMPANY_ADMIN',
  permissions: ['member.read'],
};

describe('TenantGuard', () => {
  it('allows when the user has an active membership in the token company', async () => {
    const repo = { findMembership: jest.fn().mockResolvedValue(membership) } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(
      guard.canActivate(makeContext({ userId: 'u1', companyId: 'c1' })),
    ).resolves.toBe(true);
    expect((repo.findMembership as jest.Mock)).toHaveBeenCalledWith('u1', 'c1');
  });

  it('denies (403) when there is no active membership', async () => {
    const repo = { findMembership: jest.fn().mockResolvedValue(null) } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(
      guard.canActivate(makeContext({ userId: 'u1', companyId: 'c-other' })),
    ).rejects.toBeInstanceOf(ForbiddenException);
  });

  it('denies (403) when there is no auth context', async () => {
    const repo = { findMembership: jest.fn() } as unknown as AuthRepository;
    const guard = new TenantGuard(repo);
    await expect(guard.canActivate(makeContext(undefined))).rejects.toBeInstanceOf(
      ForbiddenException,
    );
    expect((repo.findMembership as jest.Mock)).not.toHaveBeenCalled();
  });
});
