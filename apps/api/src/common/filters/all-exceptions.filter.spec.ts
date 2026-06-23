import { ArgumentsHost, ForbiddenException, HttpStatus } from '@nestjs/common';
import { Prisma } from '@prisma/client';
import { AllExceptionsFilter } from './all-exceptions.filter';

function mockHost(): { host: ArgumentsHost; status: jest.Mock; json: jest.Mock } {
  const json = jest.fn();
  const status = jest.fn().mockReturnValue({ json });
  const host = {
    switchToHttp: () => ({ getResponse: () => ({ status }) }),
  } as unknown as ArgumentsHost;
  return { host, status, json };
}

describe('AllExceptionsFilter', () => {
  const filter = new AllExceptionsFilter();

  it('maps Prisma P2002 (unique) to 409 CONFLICT', () => {
    const { host, status, json } = mockHost();
    const err = new Prisma.PrismaClientKnownRequestError('unique', {
      code: 'P2002',
      clientVersion: '6.0.0',
    });
    filter.catch(err, host);
    expect(status).toHaveBeenCalledWith(HttpStatus.CONFLICT);
    expect(json.mock.calls[0][0]).toMatchObject({
      success: false,
      error: { code: 'CONFLICT' },
    });
  });

  it('maps Prisma P2025 (not found) to 404', () => {
    const { host, status } = mockHost();
    const err = new Prisma.PrismaClientKnownRequestError('missing', {
      code: 'P2025',
      clientVersion: '6.0.0',
    });
    filter.catch(err, host);
    expect(status).toHaveBeenCalledWith(HttpStatus.NOT_FOUND);
  });

  it('respects an HttpException status (403)', () => {
    const { host, status, json } = mockHost();
    filter.catch(new ForbiddenException('nope'), host);
    expect(status).toHaveBeenCalledWith(HttpStatus.FORBIDDEN);
    expect(json.mock.calls[0][0].error.code).toBe('FORBIDDEN');
  });

  it('maps an unknown error to 500 without leaking the raw message', () => {
    const { host, status, json } = mockHost();
    filter.catch(new Error('SELECT * FROM secret leaked'), host);
    expect(status).toHaveBeenCalledWith(HttpStatus.INTERNAL_SERVER_ERROR);
    const body = json.mock.calls[0][0];
    expect(body.error.code).toBe('INTERNAL_ERROR');
    expect(JSON.stringify(body)).not.toContain('secret leaked');
  });
});
