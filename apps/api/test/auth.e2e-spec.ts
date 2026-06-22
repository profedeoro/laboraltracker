import { Test } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import request from 'supertest';
import { AppModule } from '../src/app.module';

describe('Auth flow (e2e)', () => {
  let app: INestApplication;

  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({ imports: [AppModule] }).compile();
    app = moduleRef.createNestApplication();
    app.setGlobalPrefix('api/v1');
    app.useGlobalPipes(
      new ValidationPipe({ whitelist: true, forbidNonWhitelisted: true, transform: true }),
    );
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  const base = '/api/v1';

  it('rejects bad credentials with 401', async () => {
    await request(app.getHttpServer())
      .post(`${base}/auth/login`)
      .send({ email: 'admin@a.demo', password: 'wrong' })
      .expect(401);
  });

  it('logs in, reads /me, rotates the refresh token, and logs out', async () => {
    // login
    const login = await request(app.getHttpServer())
      .post(`${base}/auth/login`)
      .send({ email: 'admin@a.demo', password: 'Password123!' })
      .expect(200);
    const { accessToken, refreshToken } = login.body;
    expect(accessToken).toBeDefined();
    expect(refreshToken).toContain('.');

    // /me
    const me = await request(app.getHttpServer())
      .get(`${base}/me`)
      .set('Authorization', `Bearer ${accessToken}`)
      .expect(200);
    expect(me.body.user.email).toBe('admin@a.demo');
    expect(me.body.memberships[0].roleKey).toBe('COMPANY_ADMIN');

    // refresh (rotation)
    const refreshed = await request(app.getHttpServer())
      .post(`${base}/auth/refresh`)
      .send({ refreshToken })
      .expect(200);
    expect(refreshed.body.refreshToken).not.toBe(refreshToken);

    // reuse of the OLD refresh token → 401 (it was rotated/revoked)
    await request(app.getHttpServer())
      .post(`${base}/auth/refresh`)
      .send({ refreshToken })
      .expect(401);

    // logout with the access token from the rotated session
    await request(app.getHttpServer())
      .post(`${base}/auth/logout`)
      .set('Authorization', `Bearer ${refreshed.body.accessToken}`)
      .expect(204);
  });

  it('rejects /me without a token (401)', async () => {
    await request(app.getHttpServer()).get(`${base}/me`).expect(401);
  });
});
