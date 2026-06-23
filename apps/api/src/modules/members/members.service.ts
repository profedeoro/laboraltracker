import { Injectable } from '@nestjs/common';
import { MembersRepository } from './members.repository';
import type { MemberDto } from './members.types';

@Injectable()
export class MembersService {
  constructor(private readonly repo: MembersRepository) {}

  list(companyId: string): Promise<MemberDto[]> {
    return this.repo.listByCompany(companyId);
  }
}
