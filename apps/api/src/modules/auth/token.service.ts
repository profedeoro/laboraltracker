import { Injectable } from '@nestjs/common';
import { JwtService } from '@nestjs/jwt';
import { ConfigService } from '@nestjs/config';
import { randomBytes } from 'node:crypto';
import { hash as argonHash, verify as argonVerify } from '@node-rs/argon2';
import type { Env } from '../../config/env.validation';
import type { AccessTokenClaims } from '../../common/auth/auth-context';

@Injectable()
export class TokenService {
  constructor(
    private readonly jwt: JwtService,
    private readonly config: ConfigService<Env, true>,
  ) {}

  async signAccessToken(p: {
    userId: string;
    email: string;
    companyId: string;
    roleKey: string;
    permissions: string[];
    sessionId: string;
  }): Promise<string> {
    const payload: AccessTokenClaims = {
      sub: p.userId,
      email: p.email,
      companyId: p.companyId,
      roleKey: p.roleKey,
      permissions: p.permissions,
      sid: p.sessionId,
    };
    return this.jwt.signAsync(payload, {
      secret: this.config.get('JWT_SECRET', { infer: true }),
      expiresIn: this.config.get('ACCESS_TTL', { infer: true }),
    });
  }

  async verifyAccessToken(token: string): Promise<AccessTokenClaims> {
    return this.jwt.verifyAsync<AccessTokenClaims>(token, {
      secret: this.config.get('JWT_SECRET', { infer: true }),
    });
  }

  generateRefreshSecret(): string {
    return randomBytes(32).toString('base64url');
  }

  async hashRefreshSecret(secret: string): Promise<string> {
    return argonHash(secret);
  }

  async verifyRefreshSecret(secret: string, hash: string): Promise<boolean> {
    try {
      return await argonVerify(hash, secret);
    } catch {
      return false;
    }
  }

  buildRefreshToken(sessionId: string, secret: string): string {
    return `${sessionId}.${secret}`;
  }

  parseRefreshToken(token: string): { sessionId: string; secret: string } | null {
    const parts = token.split('.');
    if (parts.length !== 2 || !parts[0] || !parts[1]) return null;
    return { sessionId: parts[0], secret: parts[1] };
  }
}
