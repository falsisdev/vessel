-- Migration 002: persist title/poster/backdrop/episode_title on watch_progress for Continue Watching fallback.
ALTER TABLE watch_progress ADD COLUMN title TEXT;
ALTER TABLE watch_progress ADD COLUMN poster_url TEXT;
ALTER TABLE watch_progress ADD COLUMN backdrop_url TEXT;
ALTER TABLE watch_progress ADD COLUMN episode_title TEXT;
