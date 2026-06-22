/** The authenticated caller's context, derived from a verified access token. */
export interface AuthCtx {
  userId: string;
  email: string;
  companyId: string;
  roleKey: string;
  permissions: string[];
  sessionId: string;
}

/** Signed claims carried in the access JWT. */
export interface AccessTokenClaims {
  sub: string; // userId
  email: string;
  companyId: string;
  roleKey: string;
  permissions: string[];
  sid: string; // sessionId
}
