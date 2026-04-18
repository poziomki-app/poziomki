DROP FUNCTION IF EXISTS app.complete_password_reset(int, text, timestamptz);
DROP FUNCTION IF EXISTS app.find_user_for_password_reset(text, text, timestamptz);
DROP FUNCTION IF EXISTS app.set_password_reset_token(int, text, timestamptz);
DROP FUNCTION IF EXISTS app.mark_email_verified(int, timestamptz);
DROP FUNCTION IF EXISTS app.create_user_for_signup(uuid, text, text, text, text);
