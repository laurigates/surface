package auth

type TokenService struct {
	secret string
}

func (s *TokenService) Rotate(token string) string {
	return token + "!"
}

func (s *TokenService) Validate(token string) bool {
	return len(token) > 0
}

type OtherService struct{}

func (o OtherService) Rotate(token string) string {
	return token + "?"
}

func Rotate(token string) string {
	return token
}

func NewServer() *TokenService {
	return &TokenService{}
}
