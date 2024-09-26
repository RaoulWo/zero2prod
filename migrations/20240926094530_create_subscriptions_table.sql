-- Create subscriptions table
CREATE TABLE subscriptions(
  id UUID PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  name TEXT NOT NULL,
  subscribed_at TIMESTAMPTZ NOT NULL
);
