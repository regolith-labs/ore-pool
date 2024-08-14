-- create table
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'members') THEN
        CREATE TABLE members (
          address VARCHAR PRIMARY KEY,
          balance BIGINT NOT NULL,
          id BIGINT NOT NULL,
        );
    END IF;
END
$$;
