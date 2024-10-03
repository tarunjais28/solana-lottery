CREATE TABLE ticket
(
    wallet VARCHAR NOT NULL,
    epoch_index NUMERIC(20,0) NOT NULL, -- 20 digits, 0 decimals, to accommodate u64.
    arweave_url VARCHAR (255),
    balance VARCHAR NOT NULL,
    price VARCHAR NOT NULL,
    risq_id TEXT DEFAULT NULL,
    PRIMARY KEY (wallet, epoch_index),
    UNIQUE (wallet, epoch_index)
);

CREATE TABLE sequences
(
    wallet VARCHAR NOT NULL,
    epoch_index NUMERIC(20,0) NOT NULL, -- 20 digits, 0 decimals, to accommodate u64.
    _1 SMALLINT NOT NULL,
    _2 SMALLINT NOT NULL,
    _3 SMALLINT NOT NULL,
    _4 SMALLINT NOT NULL,
    _5 SMALLINT NOT NULL,
    _6 SMALLINT NOT NULL,
    sequence_type VARCHAR NOT NULL,
    CONSTRAINT fk_sequences_ticket
        FOREIGN KEY (wallet, epoch_index)
        REFERENCES ticket (wallet, epoch_index)
        ON DELETE CASCADE
);

CREATE INDEX idx_wallet_epoch_index ON ticket (wallet, epoch_index);
CREATE INDEX idx_epoch_index_4_prefix ON sequences (epoch_index, _1, _2, _3, _4);
