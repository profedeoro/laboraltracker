import { UnauthorizedException } from '@nestjs/common';
import { AuthService } from './auth.service';
import type { AuthRepository } from './auth.repository';
import type { TokenService } from './token.service';
import type { ResolvedMembership } from './auth.types';

const membership: ResolvedMembership = {
  companyId: 'c1',
  companyName: 'Empresa A',
  companySlug: 'empresa-a',
  roleKey: 'COMPANY_ADMIN',
  permissions: ['member.read'],
};

function makeRepo(over: Partial<jest.Mocked<AuthRepository>> = {}): jest.Mocked<AuthRepository> {
  return {
    findActiveUserByEmail: jest.fn(),
    findActiveUserById: jest.fn(),
    findActiveMemberships: jest.fn(),
    findMembership: jest.fn(),
    createSession: jest.fn().mockResolvedValue(undefined),
    findSessionById: jest.fn(),
    revokeSession: jest.fn().mockResolvedValue(undefined),
    revokeSessionChain: jest.fn().mockResolvedValue(undefined),
    ...over,
  } as unknown as jest.Mocked<AuthRepository>;
}

function makeTokens(): jest.Mocked<TokenService> {
  return {
    signAccessToken: jest.fn().mockResolvedValue('access.jwt'),
    verifyAccessToken: jest.fn(),
    generateRefreshSecret: jest.fn().mockReturnValue('secret'),
    hashRefreshSecret: jest.fn().mockResolvedValue('hashed'),
    verifyRefreshSecret: jest.fn(),
    buildRefreshToken: jest.fn().mockReturnValue('sid.secret'),
    parseRefreshToken: jest.fn(),
  } as unknown as jest.Mocked<TokenService>;
}

// ConfigService mock with a real `.get()` — AuthService calls config.get('REFRESH_TTL_DAYS', {infer:true}).
function makeConfig(refreshTtlDays = 30) {
  return {
    get: (key: string) => ({ REFRESH_TTL_DAYS: refreshTtlDays })[key],
  } as any;
}

describe('AuthService.login', () => {
  it('rejects an unknown email with 401', async () => {
    const repo = makeRepo({ findActiveUserByEmail: jest.fn().mockResolvedValue(null) });
    const svc = new AuthService(repo, makeTokens(), makeConfig());
    await expect(svc.login({ email: 'x@x.demo', password: 'p' })).rejects.toBeInstanceOf(
      UnauthorizedException,
    );
  });

  it('rejects a wrong password with 401 and never resolves memberships', async () => {
    const repo = makeRepo({
      findActiveUserByEmail: jest
        .fn()
        .mockResolvedValue({ id: 'u1', email: 'a@a.demo', name: 'A', passwordHash: 'h' }),
    });
    const tokens = makeTokens();
    const svc = new AuthService(repo, tokens, makeConfig());
    tokens.verifyRefreshSecret.mockResolvedValue(false); // not used; password check uses argon verify
    // password verification is delegated to TokenService.verifyRefreshSecret? No — see impl note.
    await expect(svc.login({ email: 'a@a.demo', password: 'bad' })).rejects.toBeInstanceOf(
      UnauthorizedException,
    );
    expect(repo.findActiveMemberships).not.toHaveBeenCalled();
  });

  it('rejects a user with no active membership with 401', async () => {
    const repo = makeRepo({
      findActiveUserByEmail: jest
        .fn()
        .mockResolvedValue({ id: 'u1', email: 'a@a.demo', name: 'A', passwordHash: 'h' }),
      findActiveMemberships: jest.fn().mockResolvedValue([]),
    });
    const svc = new AuthService(repo, makeTokens(), makeConfig());
    jest.spyOn(svc as any, 'verifyPassword').mockResolvedValue(true);
    await expect(svc.login({ email: 'a@a.demo', password: 'p' })).rejects.toBeInstanceOf(
      UnauthorizedException,
    );
  });

  it('issues tokens and creates a session on valid login', async () => {
    const repo = makeRepo({
      findActiveUserByEmail: jest
        .fn()
        .mockResolvedValue({ id: 'u1', email: 'a@a.demo', name: 'A', passwordHash: 'h' }),
      findActiveMemberships: jest.fn().mockResolvedValue([membership]),
    });
    const tokens = makeTokens();
    const svc = new AuthService(repo, tokens, makeConfig());
    jest.spyOn(svc as any, 'verifyPassword').mockResolvedValue(true);

    const result = await svc.login({ email: 'a@a.demo', password: 'p' });

    expect(result).toEqual({ accessToken: 'access.jwt', refreshToken: 'sid.secret' });
    expect(repo.createSession).toHaveBeenCalledTimes(1);
    expect(tokens.signAccessToken).toHaveBeenCalledWith(
      expect.objectContaining({ userId: 'u1', companyId: 'c1', roleKey: 'COMPANY_ADMIN' }),
    );
  });
});

describe('AuthService.refresh', () => {
  const validSession = {
    id: 's1',
    userId: 'u1',
    companyId: 'c1',
    refreshTokenHash: 'hashed',
    expiresAt: new Date(Date.now() + 86_400_000),
    revokedAt: null,
    userEmail: 'a@a.demo',
  };

  it('rejects a malformed refresh token with 401', async () => {
    const tokens = makeTokens();
    tokens.parseRefreshToken.mockReturnValue(null);
    const svc = new AuthService(makeRepo(), tokens, makeConfig());
    await expect(svc.refresh('garbage')).rejects.toBeInstanceOf(UnauthorizedException);
  });

  it('detects reuse of a revoked token and revokes the whole chain', async () => {
    const tokens = makeTokens();
    tokens.parseRefreshToken.mockReturnValue({ sessionId: 's1', secret: 'secret' });
    const repo = makeRepo({
      findSessionById: jest
        .fn()
        .mockResolvedValue({ ...validSession, revokedAt: new Date() }),
    });
    const svc = new AuthService(repo, tokens, makeConfig());
    await expect(svc.refresh('s1.secret')).rejects.toBeInstanceOf(UnauthorizedException);
    expect(repo.revokeSessionChain).toHaveBeenCalledWith('u1', 'c1');
  });

  it('rejects an expired session with 401', async () => {
    const tokens = makeTokens();
    tokens.parseRefreshToken.mockReturnValue({ sessionId: 's1', secret: 'secret' });
    tokens.verifyRefreshSecret.mockResolvedValue(true);
    const repo = makeRepo({
      findSessionById: jest
        .fn()
        .mockResolvedValue({ ...validSession, expiresAt: new Date(Date.now() - 1000) }),
    });
    const svc = new AuthService(repo, tokens, makeConfig());
    await expect(svc.refresh('s1.secret')).rejects.toBeInstanceOf(UnauthorizedException);
  });

  it('rejects a wrong secret with 401', async () => {
    const tokens = makeTokens();
    tokens.parseRefreshToken.mockReturnValue({ sessionId: 's1', secret: 'bad' });
    tokens.verifyRefreshSecret.mockResolvedValue(false);
    const repo = makeRepo({ findSessionById: jest.fn().mockResolvedValue(validSession) });
    const svc = new AuthService(repo, tokens, makeConfig());
    await expect(svc.refresh('s1.bad')).rejects.toBeInstanceOf(UnauthorizedException);
  });

  it('rotates on a valid refresh: revokes the old session and issues a new pair', async () => {
    const tokens = makeTokens();
    tokens.parseRefreshToken.mockReturnValue({ sessionId: 's1', secret: 'secret' });
    tokens.verifyRefreshSecret.mockResolvedValue(true);
    const repo = makeRepo({
      findSessionById: jest.fn().mockResolvedValue(validSession),
      findMembership: jest.fn().mockResolvedValue(membership),
    });
    const svc = new AuthService(repo, tokens, makeConfig());

    const result = await svc.refresh('s1.secret');

    expect(repo.revokeSession).toHaveBeenCalledWith('s1');
    expect(repo.createSession).toHaveBeenCalledTimes(1);
    expect(result).toEqual({ accessToken: 'access.jwt', refreshToken: 'sid.secret' });
  });
});
