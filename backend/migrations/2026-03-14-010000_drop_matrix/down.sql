CREATE TABLE matrix_dm_rooms (
    id UUID PRIMARY KEY,
    user_low_pid UUID NOT NULL,
    user_high_pid UUID NOT NULL,
    room_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
