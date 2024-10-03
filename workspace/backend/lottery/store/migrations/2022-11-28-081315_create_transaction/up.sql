CREATE TABLE user_transaction (
    transaction_id VARCHAR NOT NULL,
    instruction_index SMALLINT NOT NULL,
    wallet VARCHAR NOT NULL,
    amount VARCHAR NOT NULL,
    mint VARCHAR NOT NULL,
    transaction_type VARCHAR NOT NULL,
    transaction_time TIMESTAMPTZ,
    sl_no SERIAL NOT NULL,
    PRIMARY KEY (transaction_id, instruction_index)
);

CREATE TABLE transaction_history (
    transaction_id VARCHAR PRIMARY KEY,
    sl_no SERIAL NOT NULL
);