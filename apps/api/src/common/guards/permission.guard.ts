import {
  CanActivate,
  ExecutionContext,
  ForbiddenException,
  Injectable,
} from '@nestjs/common';
import { Reflector } from '@nestjs/core';
import { REQUIRE_PERMISSION_KEY } from '../decorators/require-permission.decorator';
import type { AuthCtx } from '../auth/auth-context';

@Injectable()
export class PermissionGuard implements CanActivate {
  constructor(private readonly reflector: Reflector) {}

  canActivate(context: ExecutionContext): boolean {
    const required = this.reflector.getAllAndOverride<string | undefined>(
      REQUIRE_PERMISSION_KEY,
      [context.getHandler(), context.getClass()],
    );
    // No permission declared on this route → nothing to enforce.
    if (!required) return true;

    const request = context
      .switchToHttp()
      .getRequest<{ authCtx?: AuthCtx }>();
    const permissions = request.authCtx?.permissions;
    if (!permissions || !permissions.includes(required)) {
      throw new ForbiddenException('Missing required permission');
    }
    return true;
  }
}
