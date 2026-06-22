import {
  Injectable,
  UnauthorizedException,
  ForbiddenException,
  NotFoundException,
} from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { verify as argonVerify } from '@node-rs/argon2';
import { ulid } from 'ulid';
import type { Env } from '../../config/env.validation';
import { AuthRepository } from './auth.repository';
import { TokenService } from './token.service';
import type { AuthTokens, ResolvedMembership, MeResponse } from './auth.types';

@Injectable()
export class AuthService {
  constructor(
    private readonly repo: AuthRepository,
    private readonly tokens: TokenService,
    private readonly config: ConfigService<Env, true>,
  ) {}

  async login(dto: { email: string; password: string }): Promise<AuthTokens> {
    const user = await this.repo.findActiveUserByEmail(dto.email);
    if (!user) throw new UnauthorizedException('Invalid credentials');

    const ok = await this.verifyPassword(dto.password, user.passwordHash);
    if (!ok) throw new UnauthorizedException('Invalid credentials');

    const memberships = await this.repo.findActiveMemberships(user.id);
    if (memberships.length === 0) throw new UnauthorizedException('Invalid credentials');

    // Single membership → that one. Multiple → the first; switch-company changes it.
    const active = memberships[0];
    return this.issueTokens({ userId: user.id, email: user.email }, active);
  }

  /** Mints an access+refresh pair and persists a new Session for the given membership. */
  protected async issueTokens(
    user: { userId: string; email: string },
    m: ResolvedMembership,
  ): Promise<AuthTokens> {
    const sessionId = ulid();
    const secret = this.tokens.generateRefreshSecret();
    const refreshTokenHash = await this.tokens.hashRefreshSecret(secret);
    const days = this.config.get('REFRESH_TTL_DAYS', { infer: true });
    const expiresAt = new Date(Date.now() + days * 86_400_000);

    await this.repo.createSession({
      id: sessionId,
      userId: user.userId,
      companyId: m.companyId,
      refreshTokenHash,
      expiresAt,
    });

    const accessToken = await this.tokens.signAccessToken({
      userId: user.userId,
      email: user.email,
      companyId: m.companyId,
      roleKey: m.roleKey,
      permissions: m.permissions,
      sessionId,
    });
    const refreshToken = this.tokens.buildRefreshToken(sessionId, secret);
    return { accessToken, refreshToken };
  }

  async refresh(refreshToken: string): Promise<AuthTokens> {
    const parsed = this.tokens.parseRefreshToken(refreshToken);
    if (!parsed) throw new UnauthorizedException('Invalid refresh token');

    const session = await this.repo.findSessionById(parsed.sessionId);
    if (!session) throw new UnauthorizedException('Invalid refresh token');

    // Reuse of an already-rotated token → token theft signal → kill the chain.
    if (session.revokedAt) {
      await this.repo.revokeSessionChain(session.userId, session.companyId);
      throw new UnauthorizedException('Invalid refresh token');
    }

    if (session.expiresAt.getTime() < Date.now()) {
      throw new UnauthorizedException('Invalid refresh token');
    }

    const ok = await this.tokens.verifyRefreshSecret(parsed.secret, session.refreshTokenHash);
    if (!ok) throw new UnauthorizedException('Invalid refresh token');

    // Membership may have been revoked since issuance.
    const membership = await this.repo.findMembership(session.userId, session.companyId);
    if (!membership) throw new UnauthorizedException('Invalid refresh token');

    // Rotate: revoke the presented session, issue a fresh pair for the SAME company.
    await this.repo.revokeSession(session.id);
    return this.issueTokens({ userId: session.userId, email: session.userEmail }, membership);
  }

  async switchCompany(userId: string, email: string, companyId: string): Promise<AuthTokens> {
    const membership = await this.repo.findMembership(userId, companyId);
    if (!membership) throw new ForbiddenException('No active membership in target company');
    // The previous company's session stays valid — simultaneous multi-company allowed.
    return this.issueTokens({ userId, email }, membership);
  }

  async me(userId: string, activeCompanyId: string): Promise<MeResponse> {
    const user = await this.repo.findActiveUserById(userId);
    if (!user) throw new NotFoundException('User not found');
    const memberships = await this.repo.findActiveMemberships(userId);
    return {
      user,
      activeCompanyId,
      memberships: memberships.map((m) => ({
        companyId: m.companyId,
        companyName: m.companyName,
        roleKey: m.roleKey,
      })),
    };
  }

  async logout(sessionId: string): Promise<void> {
    await this.repo.revokeSession(sessionId);
  }

  protected async verifyPassword(plain: string, hash: string): Promise<boolean> {
    try {
      return await argonVerify(hash, plain);
    } catch {
      return false;
    }
  }
}
