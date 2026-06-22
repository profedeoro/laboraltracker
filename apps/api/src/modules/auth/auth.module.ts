import { Module } from '@nestjs/common';
import { JwtModule } from '@nestjs/jwt';
import { AuthController } from './auth.controller';
import { AuthService } from './auth.service';
import { AuthRepository } from './auth.repository';
import { TokenService } from './token.service';
import { AuthGuard } from '../../common/guards/auth.guard';

@Module({
  imports: [JwtModule.register({})],
  controllers: [AuthController],
  providers: [AuthService, AuthRepository, TokenService, AuthGuard],
  exports: [TokenService, AuthGuard],
})
export class AuthModule {}
