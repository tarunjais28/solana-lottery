CREATE TABLE epoch
(
    pubkey varchar UNIQUE NOT NULL,
    epoch_index numeric(20,0) PRIMARY KEY, -- 20 digits, 0 decimals, to accommodate u64.
    epoch_status varchar not null,
    winning_combination smallint[6],
    yield_split_cfg jsonb not null,
    total_invested varchar,
    returns jsonb,
    started_at timestamptz not null,
    expected_end_at timestamptz not null,
    ended_at timestamptz,
    draw_enabled bool
);
