import {
  CanActivate,
  ExecutionContext,
  ForbiddenException,
  Injectable,
} from '@nestjs/common';
import { AuthRepository } from '../../modules/auth/auth.repository';
import type { AuthCtx } from '../auth/auth-context';

@Injectable()
export class TenantGuard implements CanActivate {
  constructor(private readonly repo: AuthRepository) {}

  async canActivate(context: ExecutionContext): Promise<boolean> {
    const request = context
      .switchToHttp()
      .getRequest<{ authCtx?: AuthCtx }>();
    const ctx = request.authCtx;
    if (!ctx) throw new ForbiddenException('No auth context');

    // Re-validate against the DB: the membership may have been revoked within
    // the access-token TTL. The token's companyId is signed but not live.
    const membership = await this.repo.findMembership(ctx.userId, ctx.companyId);
    if (!membership) {
      throw new ForbiddenException('No active membership in company');
    }
    return true;
  }
}
