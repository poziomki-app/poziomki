CREATE TABLE public.event_place_polls (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL UNIQUE
        REFERENCES public.events(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE public.event_place_options (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id UUID NOT NULL
        REFERENCES public.event_place_polls(id) ON DELETE CASCADE,
    label VARCHAR(120) NOT NULL,
    latitude DOUBLE PRECISION,
    longitude DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX event_place_options_poll_id_idx
    ON public.event_place_options(poll_id);

CREATE TABLE public.event_place_votes (
    poll_id UUID NOT NULL
        REFERENCES public.event_place_polls(id) ON DELETE CASCADE,
    profile_id UUID NOT NULL
        REFERENCES public.profiles(id) ON DELETE CASCADE,
    option_id UUID NOT NULL
        REFERENCES public.event_place_options(id) ON DELETE CASCADE,
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, profile_id)
);

CREATE INDEX event_place_votes_option_id_idx
    ON public.event_place_votes(option_id);
