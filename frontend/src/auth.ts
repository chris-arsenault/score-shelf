import { CognitoUserPool, CognitoUser, AuthenticationDetails } from "amazon-cognito-identity-js";
import type { CognitoUserSession, IAuthenticationCallback } from "amazon-cognito-identity-js";
import { config } from "./config";

export const MFA_ENROLLMENT_URL = "https://mail.ahara.io";

export type SoftwareTokenMfaChallenge = {
  kind: "softwareTokenMfa";
  username: string;
  submitCode: (code: string) => Promise<CognitoUserSession>;
};

export type MfaSetupRedirect = {
  username: string;
  enrollmentUrl: string;
};

export type SignInResult =
  | {
      status: "signedIn";
      username: string;
      session: CognitoUserSession;
    }
  | {
      status: "softwareTokenMfaRequired";
      challenge: SoftwareTokenMfaChallenge;
    }
  | {
      status: "mfaSetupRequired";
      redirect: MfaSetupRedirect;
    };

const getUserPool = () => {
  if (!config.cognitoUserPoolId || !config.cognitoClientId) {
    throw new Error("Missing Cognito configuration");
  }
  return new CognitoUserPool({
    UserPoolId: config.cognitoUserPoolId,
    ClientId: config.cognitoClientId,
  });
};

const getCurrentUser = () => {
  try {
    return getUserPool().getCurrentUser();
  } catch {
    return null;
  }
};

const normalizeError = (error: unknown) => {
  if (error instanceof Error) return error;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return new Error(error.message);
  }
  return new Error(String(error));
};

const readOtpCode = (rawCode: string) => {
  const code = rawCode.replace(/\s+/g, "");
  if (!/^\d{6}$/.test(code)) {
    throw new Error("Enter the 6-digit authenticator code.");
  }
  return code;
};

const completeSoftwareTokenMfa = (
  user: CognitoUser,
  rawCode: string
): Promise<CognitoUserSession> => {
  return new Promise((resolve, reject) => {
    let code: string;
    try {
      code = readOtpCode(rawCode);
    } catch (error) {
      reject(normalizeError(error));
      return;
    }
    user.sendMFACode(
      code,
      {
        onSuccess: resolve,
        onFailure: (error: unknown) => reject(normalizeError(error)),
      },
      "SOFTWARE_TOKEN_MFA"
    );
  });
};

const createSoftwareTokenMfaChallenge = (
  user: CognitoUser,
  username: string
): SoftwareTokenMfaChallenge => ({
  kind: "softwareTokenMfa",
  username,
  submitCode: (code) => completeSoftwareTokenMfa(user, code),
});

const createMfaSetupRedirect = (username: string): MfaSetupRedirect => ({
  username,
  enrollmentUrl: MFA_ENROLLMENT_URL,
});

const unsupportedSmsMfa = () =>
  new Error("SMS MFA is not supported. Use an authenticator app code.");

type AuthCallbackConfig = {
  user: CognitoUser;
  username: string;
  resolve: (result: SignInResult) => void;
  reject: (error: Error) => void;
};

const createAuthCallbacks = ({
  user,
  username,
  resolve,
  reject,
}: AuthCallbackConfig): IAuthenticationCallback => {
  const onSuccess = (session: CognitoUserSession) =>
    resolve({ status: "signedIn", username, session });
  const onFailure = (error: unknown) => reject(normalizeError(error));
  const resolveTotpChallenge = () =>
    resolve({
      status: "softwareTokenMfaRequired",
      challenge: createSoftwareTokenMfaChallenge(user, username),
    });

  return {
    onSuccess,
    onFailure,
    newPasswordRequired: () => reject(new Error("Password reset is required before signing in.")),
    mfaRequired: () => reject(unsupportedSmsMfa()),
    totpRequired: resolveTotpChallenge,
    customChallenge: () => reject(new Error("Custom auth challenges are not supported.")),
    mfaSetup: () =>
      resolve({
        status: "mfaSetupRequired",
        redirect: createMfaSetupRedirect(username),
      }),
    selectMFAType: () => {
      user.sendMFASelectionAnswer("SOFTWARE_TOKEN_MFA", {
        onSuccess,
        onFailure,
        mfaRequired: () => reject(unsupportedSmsMfa()),
        totpRequired: resolveTotpChallenge,
      });
    },
  };
};

export const signIn = (username: string, password: string): Promise<SignInResult> => {
  const authenticationDetails = new AuthenticationDetails({
    Username: username,
    Password: password,
  });

  const user = new CognitoUser({
    Username: username,
    Pool: getUserPool(),
  });

  return new Promise((resolve, reject) => {
    user.authenticateUser(
      authenticationDetails,
      createAuthCallbacks({
        user,
        username,
        resolve,
        reject,
      })
    );
  });
};

export const signOut = () => {
  const user = getCurrentUser();
  user?.signOut();
};

/** The Cognito access token for API calls, or null when signed out. */
export const getAccessToken = async (): Promise<string | null> => {
  const session = await getSession();
  return session?.getAccessToken().getJwtToken() ?? null;
};

export const getSession = (): Promise<CognitoUserSession | null> => {
  const user = getCurrentUser();
  if (!user) {
    return Promise.resolve(null);
  }
  return new Promise((resolve) => {
    user.getSession((error: Error | null, session: CognitoUserSession | null) => {
      if (error || !session) {
        resolve(null);
        return;
      }
      resolve(session);
    });
  });
};
