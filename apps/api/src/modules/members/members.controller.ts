import { Controller, Get, UseGuards } from '@nestjs/common';
import { MembersService } from './members.service';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';
import { RequirePermission } from '../../common/decorators/require-permission.decorator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthCtx } from '../../common/auth/auth-context';
import type { MemberDto } from './members.types';

@Controller('members')
@UseGuards(PermissionGuard, TenantGuard)
export class MembersController {
  constructor(private readonly members: MembersService) {}

  @Get()
  @RequirePermission('member.read')
  list(@CurrentUser() user: AuthCtx): Promise<MemberDto[]> {
    return this.members.list(user.companyId);
  }
}
