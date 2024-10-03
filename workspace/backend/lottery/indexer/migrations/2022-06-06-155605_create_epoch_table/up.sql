-- An entry is created here after an epoch is sent to Risq
CREATE TABLE epoch
(
    index NUMERIC(20,0) NOT NULL, -- 20 digits, 0 decimals, to accommodate u64.
    draw_info_sent_to_risq_at TIMESTAMPTZ,
    tickets_generated_at TIMESTAMPTZ,
    PRIMARY KEY (index)
);

