import { JwtService } from '@nestjs/jwt';
import { ConfigService } from '@nestjs/config';
import { TokenService } from './token.service';

function makeService(): TokenService {
  const config = {
    get: (key: string) =>
      ({ JWT_SECRET: 'test-secret-at-least-32-characters-long', ACCESS_TTL: '15m' })[key],
  } as unknown as ConfigService;
  return new TokenService(new JwtService(), config);
}

describe('TokenService', () => {
  const svc = makeService();

  it('signs and verifies an access token round-trip', async () => {
    const token = await svc.signAccessToken({
      userId: 'u1',
      email: 'a@a.demo',
      companyId: 'c1',
      roleKey: 'COMPANY_ADMIN',
      permissions: ['member.read'],
      sessionId: 's1',
    });
    const claims = await svc.verifyAccessToken(token);
    expect(claims.sub).toBe('u1');
    expect(claims.sid).toBe('s1');
    expect(claims.permissions).toEqual(['member.read']);
  });

  it('rejects a tampered access token', async () => {
    await expect(svc.verifyAccessToken('not.a.jwt')).rejects.toThrow();
  });

  it('hashes and verifies a refresh secret, rejecting the wrong one', async () => {
    const secret = svc.generateRefreshSecret();
    const hash = await svc.hashRefreshSecret(secret);
    expect(await svc.verifyRefreshSecret(secret, hash)).toBe(true);
    expect(await svc.verifyRefreshSecret('wrong', hash)).toBe(false);
  });

  it('builds and parses a refresh token, returning null on malformed input', () => {
    const built = svc.buildRefreshToken('s1', 'abc');
    expect(svc.parseRefreshToken(built)).toEqual({ sessionId: 's1', secret: 'abc' });
    expect(svc.parseRefreshToken('nodot')).toBeNull();
    expect(svc.parseRefreshToken('a.b.c')).toBeNull();
  });
});
