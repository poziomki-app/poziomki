ALTER TABLE public.events
    ADD COLUMN visibility VARCHAR(20) NOT NULL DEFAULT 'public';

CREATE INDEX events_visibility_idx ON public.events(visibility);
