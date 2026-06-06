export function rotate(token: string): string;
export function rotate(token: string, force: boolean): string;
export function rotate(token: string, force?: boolean): string {
  return force ? token.toUpperCase() : token;
}

export class TokenService {
  rotate(token: string): string {
    return token + "!";
  }

  validate(token: string): boolean {
    return token.length > 0;
  }
}

export class OtherService {
  rotate(token: string): string {
    return token + "?";
  }
}

export const refresh = (token: string): string => {
  function inner(t: string): string {
    return t.trim();
  }
  return inner(token);
};

export const getResults = cache(
  unstable_cache(async (id: string): Promise<string> => {
    return id.trim();
  }),
);

export const loginSchema = z.object({ id: 1 });
