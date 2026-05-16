-- Account-delete needs to remove the user's DM conversations before the
-- final users-row delete; the conversations_user_*_id_fkey has no CASCADE,
-- so a leftover DM blocks the cascade. With FORCE RLS and no DELETE
-- policy on conversations, the diesel::delete in delete_user_data was
-- silently affecting zero rows.
--
-- This grants DELETE only on DMs where the caller is one of the two
-- pinned participants — event conversations remain governed by event
-- owner deletes (which cascade) and admin tooling.

CREATE POLICY conversations_dm_delete ON public.conversations
    FOR DELETE TO poziomki_api
    USING (
        kind = 'dm'
        AND (user_low_id = app.current_user_id() OR user_high_id = app.current_user_id())
    );
