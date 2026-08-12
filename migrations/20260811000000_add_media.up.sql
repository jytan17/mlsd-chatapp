CREATE TABLE media (
    id UUID PRIMARY KEY,
    uploader_id UUID NOT NULL REFERENCES users(id),
    content_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE messages ADD COLUMN media_id UUID REFERENCES media(id);
