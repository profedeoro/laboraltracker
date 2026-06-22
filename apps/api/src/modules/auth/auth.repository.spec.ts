import { PrismaService } from '../../database/prisma.service';
import { AuthRepository } from './auth.repository';

describe('AuthRepository (integration, seeded dev DB)', () => {
  const prisma = new PrismaService();
  const repo = new AuthRepository(prisma);

  beforeAll(async () => {
    await prisma.$connect();
  });
  afterAll(async () => {
    await prisma.$disconnect();
  });

  it('finds the seeded admin and resolves a COMPANY_ADMIN membership with permissions', async () => {
    const user = await repo.findActiveUserByEmail('admin@a.demo');
    expect(user).not.toBeNull();

    const memberships = await repo.findActiveMemberships(user!.id);
    expect(memberships).toHaveLength(1);
    expect(memberships[0].roleKey).toBe('COMPANY_ADMIN');
    expect(memberships[0].permissions).toEqual(
      expect.arrayContaining(['member.read', 'member.manage', 'report.view']),
    );
  });

  it('returns null for an unknown email', async () => {
    expect(await repo.findActiveUserByEmail('nobody@nowhere.demo')).toBeNull();
  });
});
