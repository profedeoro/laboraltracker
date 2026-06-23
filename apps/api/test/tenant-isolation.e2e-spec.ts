import { Test } from '@nestjs/testing';
import { INestApplication, ValidationPipe } from '@nestjs/common';
import request from 'supertest';
import { AppModule } from '../src/app.module';

describe('Cross-tenant isolation (e2e)', () => {
  let app: INestApplication;
  const base = '/api/v1';

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

  async function login(email: string, password = 'Password123!'): Promise<string> {
    const res = await request(app.getHttpServer())
      .post(`${base}/auth/login`)
      .send({ email, password })
      .expect(200);
    return res.body.accessToken as string;
  }

  it('rejects GET /members without a token (401)', async () => {
    await request(app.getHttpServer()).get(`${base}/members`).expect(401);
  });

  it('lets Empresa A admin see ONLY Empresa A members', async () => {
    const token = await login('admin@a.demo');
    const res = await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    const emails = (res.body as Array<{ email: string }>).map((m) => m.email).sort();
    expect(emails).toEqual(['admin@a.demo', 'employee@a.demo']);
  });

  it('lets Empresa B admin see ONLY Empresa B members', async () => {
    const token = await login('admin@b.demo');
    const res = await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(200);
    const emails = (res.body as Array<{ email: string }>).map((m) => m.email).sort();
    expect(emails).toEqual(['admin@b.demo', 'employee@b.demo']);
    // The decisive isolation assertion: B's token never surfaces A's data.
    expect(emails).not.toContain('admin@a.demo');
    expect(emails).not.toContain('employee@a.demo');
  });

  it('denies an EMPLOYEE without member.read (403)', async () => {
    const token = await login('employee@a.demo');
    await request(app.getHttpServer())
      .get(`${base}/members`)
      .set('Authorization', `Bearer ${token}`)
      .expect(403);
  });
});
