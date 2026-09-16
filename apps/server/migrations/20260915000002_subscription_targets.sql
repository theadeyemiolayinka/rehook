-- Store the target identifier each agent wants deliveries routed to for
-- each subscribed endpoint. The agent declares this in its Subscribe
-- message so the server can auto-dispatch newly captured events.

ALTER TABLE agent_subscriptions ADD COLUMN target_id TEXT NOT NULL DEFAULT '';
