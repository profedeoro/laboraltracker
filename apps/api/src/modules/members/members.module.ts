import { Module } from '@nestjs/common';
import { AuthModule } from '../auth/auth.module';
import { MembersController } from './members.controller';
import { MembersService } from './members.service';
import { MembersRepository } from './members.repository';

@Module({
  imports: [AuthModule], // PermissionGuard + TenantGuard (+ AuthRepository the TenantGuard uses)
  controllers: [MembersController],
  providers: [MembersService, MembersRepository],
})
export class MembersModule {}
