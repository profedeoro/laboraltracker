import { createParamDecorator, ExecutionContext } from '@nestjs/common';
import type { AuthCtx } from '../auth/auth-context';

export const CurrentUser = createParamDecorator(
  (_data: unknown, ctx: ExecutionContext): AuthCtx => {
    const request = ctx.switchToHttp().getRequest<{ authCtx: AuthCtx }>();
    return request.authCtx;
  },
);
