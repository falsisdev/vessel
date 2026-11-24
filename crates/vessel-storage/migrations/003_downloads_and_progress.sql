-- Migration 003: persist stream label and user-reorderable queue_order on offline downloads.
ALTER TABLE downloads ADD COLUMN stream_label TEXT;
ALTER TABLE downloads ADD COLUMN queue_order INTEGER NOT NULL DEFAULT 0;
