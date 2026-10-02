// Hash a password exactly as yagami's signup does, for resetting one by hand:
//
//   cargo run -p yagami --bin hash-password
//   UPDATE accounts SET password_hash = '<output>' WHERE lower(username) = lower('<name>');
//
// Reads the password as one line from stdin and prints its argon2id PHC string. Must stay in step
// with account::hash_password.

use std::io::stdin;

use argon2::{Argon2, password_hash::PasswordHasher};

fn main() {
    let mut password = String::new();
    stdin()
        .read_line(&mut password)
        .expect("failed to read stdin");
    let password = password.trim_end_matches(['\r', '\n']);
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("OS CSPRNG unavailable");
    println!("{hash}");
}
