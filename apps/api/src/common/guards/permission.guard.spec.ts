import { ExecutionContext, ForbiddenException } from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { PermissionGuard } from './permission.guard';
import type { AuthCtx } from '../auth/auth-context';

function makeContext(
  permissions: string[] | undefined,
): ExecutionContext {
  const authCtx: Partial<AuthCtx> | undefined =
    permissions === undefined ? undefined : { permissions };
  return {
    switchToHttp: () => ({ getRequest: () => ({ authCtx }) }),
    getHandler: () => 'handler',
    getClass: () => 'class',
  } as unknown as ExecutionContext;
}

function makeReflector(required: string | undefined): Reflector {
  return { getAllAndOverride: () => required } as unknown as Reflector;
}

describe('PermissionGuard', () => {
  it('allows when the route declares no required permission', () => {
    const guard = new PermissionGuard(makeReflector(undefined));
    expect(guard.canActivate(makeContext(['member.read']))).toBe(true);
  });

  it('allows when the token has the required permission', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(guard.canActivate(makeContext(['member.read', 'report.view']))).toBe(true);
  });

  it('denies (403) when the token lacks the required permission', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(() => guard.canActivate(makeContext([]))).toThrow(ForbiddenException);
  });

  it('denies (403) when there is no auth context', () => {
    const guard = new PermissionGuard(makeReflector('member.read'));
    expect(() => guard.canActivate(makeContext(undefined))).toThrow(ForbiddenException);
  });
});
