CREATE TABLE ticket_airdrop (
    wallet VARCHAR NOT NULL,
    epoch_index NUMERIC(20,0) NOT NULL, -- 20 digits, 0 decimals, to accommodate u64.
    num_sequences BIGINT NOT NULL,
    airdrop_id VARCHAR NOT NULL,
    PRIMARY KEY (wallet, epoch_index, airdrop_id)
);