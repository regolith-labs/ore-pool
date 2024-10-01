-- create members table
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'members') THEN
        CREATE TABLE members (
          address VARCHAR PRIMARY KEY,
          id BIGINT NOT NULL,
          authority VARCHAR NOT NULL,
          pool_address VARCHAR NOT NULL,
          total_balance BIGINT NOT NULL,
          is_approved BOOLEAN NOT NULL,
          is_kyc BOOLEAN NOT NULL,
          is_synced BOOLEAN NOT NULL,
          CONSTRAINT unique_member_id UNIQUE (id)
        );
    END IF;
END
$$;

-- create stakers table
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'stakers') THEN
        CREATE TABLE stakers (
          address VARCHAR PRIMARY KEY, -- address of share account
          member_id BIGINT NOT NULL,
          mint VARCHAR NOT NULL, -- the mint of the boost account
          FOREIGN KEY (member_id) REFERENCES members(id)
        );
    END IF;
END
$$;
