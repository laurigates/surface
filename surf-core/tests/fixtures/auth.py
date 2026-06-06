def rotate(token):
    return token


def rotate(token, force):  # same name at module scope -> ambiguous without @N
    return token.upper() if force else token


class TokenService:
    def rotate(self, token):
        return token + "!"

    def validate(self, token):
        return len(token) > 0


class OtherService:
    def rotate(self, token):
        return token + "?"


def refresh(token):
    def inner(t):
        return t.strip()

    return inner(token)


@staticmethod
def cached():
    return 1
