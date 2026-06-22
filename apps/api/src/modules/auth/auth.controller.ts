import { Body, Controller, Get, Post, HttpCode } from '@nestjs/common';
import { AuthService } from './auth.service';
import { LoginDto } from './dto/login.dto';
import { RefreshDto } from './dto/refresh.dto';
import { SwitchCompanyDto } from './dto/switch-company.dto';
import { Public } from '../../common/decorators/public.decorator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthCtx } from '../../common/auth/auth-context';

@Controller()
export class AuthController {
  constructor(private readonly auth: AuthService) {}

  @Public()
  @Post('auth/login')
  @HttpCode(200)
  login(@Body() dto: LoginDto) {
    return this.auth.login(dto);
  }

  @Public()
  @Post('auth/refresh')
  @HttpCode(200)
  refresh(@Body() dto: RefreshDto) {
    return this.auth.refresh(dto.refreshToken);
  }

  @Post('auth/logout')
  @HttpCode(204)
  async logout(@CurrentUser() user: AuthCtx): Promise<void> {
    await this.auth.logout(user.sessionId);
  }

  @Post('auth/switch-company')
  @HttpCode(200)
  switchCompany(@CurrentUser() user: AuthCtx, @Body() dto: SwitchCompanyDto) {
    return this.auth.switchCompany(user.userId, user.email, dto.companyId);
  }

  @Get('me')
  me(@CurrentUser() user: AuthCtx) {
    return this.auth.me(user.userId, user.companyId);
  }
}
