import { Injectable } from '@nestjs/common';
import { PrismaService } from '../../database/prisma.service';
import type { ResolvedMembership, SessionRecord, PublicUser } from './auth.types';

@Injectable()
export class AuthRepository {
  constructor(private readonly prisma: PrismaService) {}

  async findActiveUserByEmail(
    email: string,
  ): Promise<{ id: string; email: string; name: string; passwordHash: string } | null> {
    const user = await this.prisma.user.findFirst({
      where: { email, status: 'ACTIVE', deletedAt: null },
      select: { id: true, email: true, name: true, passwordHash: true },
    });
    return user;
  }

  async findActiveUserById(id: string): Promise<PublicUser | null> {
    return this.prisma.user.findFirst({
      where: { id, status: 'ACTIVE', deletedAt: null },
      select: { id: true, email: true, name: true },
    });
  }

  async findActiveMemberships(userId: string): Promise<ResolvedMembership[]> {
    const members = await this.prisma.companyMember.findMany({
      where: { userId, status: 'ACTIVE', deletedAt: null, company: { deletedAt: null } },
      include: {
        company: { select: { id: true, name: true, slug: true } },
        role: { include: { permissions: { include: { permission: true } } } },
      },
    });
    return members.map((m) => this.toResolved(m));
  }

  async findMembership(userId: string, companyId: string): Promise<ResolvedMembership | null> {
    const m = await this.prisma.companyMember.findFirst({
      where: { userId, companyId, status: 'ACTIVE', deletedAt: null, company: { deletedAt: null } },
      include: {
        company: { select: { id: true, name: true, slug: true } },
        role: { include: { permissions: { include: { permission: true } } } },
      },
    });
    return m ? this.toResolved(m) : null;
  }

  async createSession(p: {
    id: string;
    userId: string;
    companyId: string;
    refreshTokenHash: string;
    expiresAt: Date;
  }): Promise<void> {
    await this.prisma.session.create({ data: p });
  }

  async findSessionById(id: string): Promise<SessionRecord | null> {
    const s = await this.prisma.session.findUnique({
      where: { id },
      include: { user: { select: { email: true } } },
    });
    if (!s) return null;
    return {
      id: s.id,
      userId: s.userId,
      companyId: s.companyId,
      refreshTokenHash: s.refreshTokenHash,
      expiresAt: s.expiresAt,
      revokedAt: s.revokedAt,
      userEmail: s.user.email,
    };
  }

  async revokeSession(id: string): Promise<void> {
    await this.prisma.session.updateMany({
      where: { id, revokedAt: null },
      data: { revokedAt: new Date() },
    });
  }

  async revokeSessionChain(userId: string, companyId: string): Promise<void> {
    await this.prisma.session.updateMany({
      where: { userId, companyId, revokedAt: null },
      data: { revokedAt: new Date() },
    });
  }

  private toResolved(m: {
    company: { id: string; name: string; slug: string };
    role: { key: string; permissions: Array<{ permission: { key: string } }> };
  }): ResolvedMembership {
    return {
      companyId: m.company.id,
      companyName: m.company.name,
      companySlug: m.company.slug,
      roleKey: m.role.key,
      permissions: m.role.permissions.map((rp) => rp.permission.key),
    };
  }
}
