CREATE TABLE IF NOT EXISTS media (
  hash_id VARCHAR(128) NOT NULL PRIMARY KEY,
  extension VARCHAR(16) NOT NULL,
  has_thumbnail BOOLEAN NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  comment TEXT NULL,
  uploaded TIMESTAMPTZ NOT NULL
);