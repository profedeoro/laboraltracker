export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
}

export interface ResolvedMembership {
  companyId: string;
  companyName: string;
  companySlug: string;
  roleKey: string;
  permissions: string[];
}

export interface SessionRecord {
  id: string;
  userId: string;
  companyId: string;
  refreshTokenHash: string;
  expiresAt: Date;
  revokedAt: Date | null;
  userEmail: string;
}

export interface PublicUser {
  id: string;
  email: string;
  name: string;
}

export interface MeResponse {
  user: PublicUser;
  activeCompanyId: string;
  memberships: Array<{ companyId: string; companyName: string; roleKey: string }>;
}
