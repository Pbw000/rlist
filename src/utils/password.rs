use rand::RngExt;

pub fn generate_random_password() -> String {
    let mut rng = rand::rng();
    const CHARS: &str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
    let password: String = (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARS.len());
            CHARS.chars().nth(idx).unwrap()
        })
        .collect();

    password
}
