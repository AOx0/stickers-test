REMOVE SCOPE account;

DEFINE SCOPE account SESSION 24h
    SIGNUP (
        CREATE type::thing("user", string::trim($username))
        SET
            email = $email,
            pass = crypto::argon2::generate($password),
            first_name=$first_name,
            last_name=$last_name
    )
    SIGNIN (
        SELECT * FROM type::thing("user", string::trim($username)) 
         WHERE crypto::argon2::compare(pass, $password)
    )
;

REMOVE TABLE user;

DEFINE TABLE user SCHEMAFULL
    PERMISSIONS
        FOR select FULL
        FOR delete, create, update WHERE id = $auth.id
;

DEFINE FIELD id ON user 
    PERMISSIONS 
        FOR create, update, delete NONE
        FOR select FULL
        TYPE record(user)
;

-- Define first_name and last_name
DEFINE FIELD first_name ON TABLE user TYPE string;
DEFINE FIELD last_name ON TABLE user TYPE string;

-- 1. Solo es accessible para el dueño de la cuenta
-- 2. El email no puede ser nulo y debe ser valido
DEFINE FIELD email ON TABLE user TYPE string
    PERMISSIONS
        FOR select WHERE id = $auth.id
    ASSERT 
        $value != NONE AND string::is::email($value)
;

DEFINE INDEX userEmailIndex ON TABLE user COLUMNS email UNIQUE;

-- La contraseña puede ser actualizada por el usuario pero no vista
DEFINE FIELD pass ON TABLE user 
    PERMISSIONS 
        FOR select NONE TYPE string
;

DEFINE FIELD is_admin ON TABLE user 
    PERMISSIONS 
        FOR create, update, delete WHERE is_admin = true
        FOR select WHERE id = $auth.id AND is_admin = true
    TYPE bool
    DEFAULT false
;