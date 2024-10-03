CREATE TABLE faucet (
    wallet VARCHAR NOT NULL,
    amount VARCHAR NOT NULL,
    transaction_time TIMESTAMPTZ NOT NULL,
    transaction_id VARCHAR NOT NULL,
    PRIMARY KEY (wallet)
);
