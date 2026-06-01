-- Add up migration script here
CREATE TABLE conversations (
	id UUID PRIMARY KEY,
	kind TEXT NOT NULL CHECK (kind IN ('dm', 'group')),
	name TEXT,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE conversation_members (
	conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
	user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
	joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	last_read_at TIMESTAMPTZ,
	PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX idx_conversation_members_user ON conversation_members (user_id);

CREATE TABLE messages (
	id UUID PRIMARY KEY,
	conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
	sender_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
	body TEXT NOT NULL,
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_messages_conv_id_desc ON messages (conversation_id, id DESC);
