CREATE TABLE stake_update (
    id UUID NOT NULL,
    wallet VARCHAR NOT NULL,
    amount VARCHAR NOT NULL,
    state VARCHAR NOT NULL,
    type VARCHAR NOT NULL,
    currency VARCHAR NOT NULL,
    mint VARCHAR NOT NULL,
    transaction_id VARCHAR,
    PRIMARY KEY (id)
);
