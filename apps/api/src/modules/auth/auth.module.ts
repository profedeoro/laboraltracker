import { Module } from '@nestjs/common';
import { APP_GUARD } from '@nestjs/core';
import { JwtModule } from '@nestjs/jwt';
import { AuthController } from './auth.controller';
import { AuthService } from './auth.service';
import { AuthRepository } from './auth.repository';
import { TokenService } from './token.service';
import { AuthGuard } from '../../common/guards/auth.guard';
import { PermissionGuard } from '../../common/guards/permission.guard';
import { TenantGuard } from '../../common/guards/tenant.guard';

@Module({
  imports: [JwtModule.register({})],
  controllers: [AuthController],
  providers: [
    AuthService,
    AuthRepository,
    TokenService,
    AuthGuard,
    PermissionGuard,
    TenantGuard,
    { provide: APP_GUARD, useClass: AuthGuard },
  ],
  exports: [TokenService, AuthGuard, PermissionGuard, TenantGuard],
})
export class AuthModule {}
