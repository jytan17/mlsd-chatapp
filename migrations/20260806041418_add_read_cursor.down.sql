ALTER TABLE conversation_members DROP COLUMN last_read_message_id;
ALTER TABLE conversation_members ADD COLUMN last_read_at TIMESTAMPTZ;
