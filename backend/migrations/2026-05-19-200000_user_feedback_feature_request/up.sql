-- Add a second free-text column to capture feature requests separately
-- from the general "what works / what doesn't" message. Lets the
-- Zostaw opinię dialog ask both questions without conflating them in
-- triage email + dashboard.

ALTER TABLE public.user_feedback
    ADD COLUMN feature_request TEXT;
