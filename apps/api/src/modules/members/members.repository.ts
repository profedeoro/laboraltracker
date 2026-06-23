import { Injectable } from '@nestjs/common';
import { PrismaService } from '../../database/prisma.service';
import type { MemberDto } from './members.types';

@Injectable()
export class MembersRepository {
  constructor(private readonly prisma: PrismaService) {}

  /** Lists the active (non-deleted) members of a single company. */
  async listByCompany(companyId: string): Promise<MemberDto[]> {
    const members = await this.prisma.companyMember.findMany({
      where: { companyId, status: 'ACTIVE', deletedAt: null },
      include: {
        user: { select: { id: true, email: true, name: true } },
        role: { select: { key: true } },
      },
      orderBy: { createdAt: 'asc' },
    });
    return members.map((m) => ({
      id: m.id,
      userId: m.user.id,
      email: m.user.email,
      name: m.user.name,
      roleKey: m.role.key,
      status: m.status,
    }));
  }
}
