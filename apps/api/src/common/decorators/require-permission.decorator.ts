import { SetMetadata } from '@nestjs/common';

export const REQUIRE_PERMISSION_KEY = 'requirePermission';

/** Declares the Permission a route requires; read by PermissionGuard. */
export const RequirePermission = (permission: string) =>
  SetMetadata(REQUIRE_PERMISSION_KEY, permission);
