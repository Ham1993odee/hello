use crate::models::user::{NewUser, User};
use crate::schema::users;
use diesel::prelude::*;
use diesel::result::Error;

pub async fn register_user(
    conn: &mut PgConnection,
    username: &str,
    password: &str,
) -> Result<User, Error> {
    let password_hash = &hash_password(password).expect("Failed to hash password");
    let new_user = NewUser {
        username,
        password_hash,
    };

    let user = diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_returning())
        .get_result(conn);
    log::info!("{:?}", user);
    return user;
}

pub async fn login(conn: &mut PgConnection, username: &str, password: &str) -> Result<User, Error> {
    let user = users::table
        .filter(users::username.eq(username))
        .first::<User>(conn);

    match user {
        Ok(user) => {
            let is_password_correct = verify_password(&user.password_hash, password)
                .expect("Password verification failed");
            if is_password_correct {
                return Ok(user);
            } else {
                return Err(Error::NotFound);
            }
        }
        Err(error) => Err(error),
    }
}

fn hash_password(plain: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(plain, bcrypt::DEFAULT_COST)
}

fn verify_password(hash: &str, plain: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(plain, hash)
}

#[cfg(test)]
mod tests {
    use crate::database::test_db::{self, TestDb};

    use super::*;

    #[actix_rt::test]
    async fn test_register_user_success() {
        let db = TestDb::new();
        let mut conn = db.conn();

        let username = "testuser";
        let password = "password123";

        let result = register_user(&mut conn, username, password).await;
        println!("{:?}", result);
        assert!(
            result.is_ok(),
            "User registration failed when it should have succeeded"
        );

        let registered_user = result.unwrap();
        assert_eq!(registered_user.username, username);

        // Ensure the password is hashed
        let is_password_correct = verify_password(&registered_user.password_hash, password)
            .expect("Password verification failed");
        assert!(
            is_password_correct,
            "Password hashing or verification failed"
        );
    }

    #[actix_rt::test]
    async fn test_register_user_duplicate_username() {
        let db = TestDb::new();
        test_db::run_migrations(&mut db.conn());
        let mut conn = db.conn();

        let username = "testuser";
        let password = "password123";

        // First registration should succeed
        let first_result = register_user(&mut conn, username, password).await;
        assert!(first_result.is_ok(), "First user registration failed");

        // Second registration with the same username should fail
        let second_result = register_user(&mut conn, username, password).await;
        assert!(
            second_result.is_err(),
            "Second user registration succeeded when it should have failed due to duplicate username"
        );
    }

    #[actix_rt::test]
    async fn test_password_hashing() {
        let db = TestDb::new();
        test_db::run_migrations(&mut db.conn());

        let username = "testuser";
        let password = "password123";

        let result = register_user(&mut db.conn(), username, password).await;
        println!("RESULT: {:?}", result);
        assert!(result.is_ok(), "User registration failed");

        let registered_user = result.unwrap();

        // Ensure the password in the database is not the plain text password
        assert_ne!(
            registered_user.password_hash, password,
            "Password was not hashed properly"
        );

        // Verify that the hashed password matches the original password
        let is_password_correct = verify_password(&registered_user.password_hash, password)
            .expect("Password verification failed");
        assert!(is_password_correct, "Password verification failed");
    }
}