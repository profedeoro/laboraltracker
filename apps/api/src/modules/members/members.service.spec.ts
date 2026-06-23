import { MembersService } from './members.service';
import type { MembersRepository } from './members.repository';
import type { MemberDto } from './members.types';

const rows: MemberDto[] = [
  { id: 'm1', userId: 'u1', email: 'admin@a.demo', name: 'Admin A', roleKey: 'COMPANY_ADMIN', status: 'ACTIVE' },
];

describe('MembersService', () => {
  it('lists members for the given company id', async () => {
    const repo = { listByCompany: jest.fn().mockResolvedValue(rows) } as unknown as MembersRepository;
    const service = new MembersService(repo);
    const result = await service.list('c1');
    expect(repo.listByCompany).toHaveBeenCalledWith('c1');
    expect(result).toEqual(rows);
  });
});
