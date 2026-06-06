pub struct TokenService {
    pub secret: String,
}

impl TokenService {
    pub fn rotate(&self, token: &str) -> String {
        let helper = |t: &str| t.to_string();
        helper(token)
    }

    pub fn validate(&self, token: &str) -> bool {
        !token.is_empty()
    }
}

pub mod refresh {
    pub fn rotate(token: &str) -> String {
        token.to_string()
    }

    pub fn nested() -> u8 {
        fn deep() -> u8 {
            7
        }
        deep()
    }
}

pub fn rotate(token: &str) -> String {
    token.to_string()
}
