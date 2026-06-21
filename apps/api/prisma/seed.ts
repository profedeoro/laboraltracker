import { PrismaClient, RoleKey } from '@prisma/client';
import { hash } from '@node-rs/argon2';
import { ulid } from 'ulid';

const prisma = new PrismaClient();

// seed login (DEV/TEST ONLY):
//   admin@a.demo / employee@a.demo  (Empresa A)
//   admin@b.demo / employee@b.demo  (Empresa B)
//   password = DEV_SEED_PASSWORD (override con env SEED_PASSWORD)
const DEV_SEED_PASSWORD = process.env.SEED_PASSWORD ?? 'Password123!';

// Catálogo mínimo de permisos para Fase 1.
const PERMISSIONS = ['member.read', 'member.manage', 'report.view'];

// Qué permisos tiene cada rol (Fase 1; matriz doc 00 §2, recortada).
const ROLE_PERMISSIONS: Record<RoleKey, string[]> = {
  SUPER_ADMIN: [],
  COMPANY_ADMIN: ['member.read', 'member.manage', 'report.view'],
  MANAGER: ['member.read', 'report.view'],
  EMPLOYEE: [],
  CLIENT_VIEWER: ['report.view'],
};

async function main(): Promise<void> {
  const passwordHash = await hash(DEV_SEED_PASSWORD);

  // Permisos. Guardamos el id PERSISTIDO (retorno del upsert), no el ulid()
  // recién generado: en una re-corrida el upsert toma el path `update` y el
  // id real es el de la primera siembra, no el nuevo.
  const permIds = new Map<string, string>();
  for (const key of PERMISSIONS) {
    const perm = await prisma.permission.upsert({
      where: { key },
      update: {},
      create: { id: ulid(), key },
    });
    permIds.set(key, perm.id);
  }

  // Roles globales (companyId = null) + RolePermission
  const roleIds = new Map<RoleKey, string>();
  for (const key of Object.keys(ROLE_PERMISSIONS) as RoleKey[]) {
    const existing = await prisma.role.findFirst({
      where: { companyId: null, key },
    });
    const id = existing?.id ?? ulid();
    roleIds.set(key, id);
    if (!existing) {
      await prisma.role.create({ data: { id, key, name: key } });
    }
    for (const permKey of ROLE_PERMISSIONS[key]) {
      const permId = permIds.get(permKey)!;
      await prisma.rolePermission.upsert({
        where: { roleId_permissionId: { roleId: id, permissionId: permId } },
        update: {},
        create: { roleId: id, permissionId: permId },
      });
    }
  }

  // Empresas demo + usuarios + membresías
  const companies = [
    { slug: 'empresa-a', name: 'Empresa A', adminEmail: 'admin@a.demo', empEmail: 'employee@a.demo' },
    { slug: 'empresa-b', name: 'Empresa B', adminEmail: 'admin@b.demo', empEmail: 'employee@b.demo' },
  ];

  for (const c of companies) {
    const company = await prisma.company.upsert({
      where: { slug: c.slug },
      update: {},
      create: { id: ulid(), slug: c.slug, name: c.name },
    });

    for (const [email, name, roleKey] of [
      [c.adminEmail, `Admin ${c.name}`, RoleKey.COMPANY_ADMIN],
      [c.empEmail, `Employee ${c.name}`, RoleKey.EMPLOYEE],
    ] as const) {
      const user = await prisma.user.upsert({
        where: { email },
        update: {},
        create: { id: ulid(), email, name, passwordHash },
      });
      await prisma.companyMember.upsert({
        where: { userId_companyId: { userId: user.id, companyId: company.id } },
        update: {},
        create: {
          id: ulid(),
          userId: user.id,
          companyId: company.id,
          roleId: roleIds.get(roleKey)!,
        },
      });
    }
  }

  console.log('Seed done.');
}

main()
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(() => prisma.$disconnect());
