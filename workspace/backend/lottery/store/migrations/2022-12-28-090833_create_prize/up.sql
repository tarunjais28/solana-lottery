CREATE TABLE prize(
    wallet VARCHAR NOT NULL,
    epoch_index NUMERIC(20,0) NOT NULL, -- 20 digits, 0 decimals, to accommodate u64.
    tier SMALLINT NOT NULL,
    page INT NOT NULL,
    winner_index INT NOT NULL,
    amount VARCHAR NOT NULL,
    claimable BOOLEAN NOT NULL,
    claimed BOOLEAN NOT NULL,
    PRIMARY KEY (wallet, epoch_index, page, winner_index)
);
