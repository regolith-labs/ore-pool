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
          webhook BOOLEAN NOT NULL, -- whether or not the address has been added to the webhook
          FOREIGN KEY (member_id) REFERENCES members(id)
        );
    END IF;
END
$$;

-- create total-rewards table
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'total_rewards') THEN
        CREATE TABLE total_rewards (
            address VARCHAR PRIMARY KEY, -- unique address for the total rewards account
            pool VARCHAR NOT NULL, -- pool associated with total rewards
            miner_rewards BIGINT NOT NULL, -- total rewards for miners
            staker_rewards BIGINT NOT NULL, -- total rewards for stakers
            operator_rewards BIGINT NOT NULL, -- total rewards for the operator
            is_synced BOOLEAN NOT NULL -- whether the total rewards account is synced
        );
    END IF;
END
$$;

-- create share-rewards table
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'share_rewards') THEN
        CREATE TABLE share_rewards (
            address VARCHAR PRIMARY KEY, -- unique address for the share rewards account
            pool VARCHAR NOT NULL, -- pool associated with share rewards
            mint VARCHAR NOT NULL, -- mint associated with the share account
            rewards BIGINT NOT NULL, -- total rewards for share holders
            is_synced BOOLEAN NOT NULL -- whether the share rewards account is synced
        );
    END IF;
END
$$;
