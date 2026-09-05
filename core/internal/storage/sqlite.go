package storage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/library"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	_ "modernc.org/sqlite"
)

var (
	ErrNotFound = errors.New("resource not found")
)

type SQLiteStorage struct {
	db *sql.DB
}

func NewSQLiteStorage(dsn string) (*SQLiteStorage, error) {
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to open sqlite database: %w", err)
	}

	db.SetMaxOpenConns(1) // SQLite single-writer safe default

	s := &SQLiteStorage{db: db}
	if err := s.migrate(context.Background()); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("failed to run sqlite migrations: %w", err)
	}

	return s, nil
}

func (s *SQLiteStorage) Close() error {
	if s.db != nil {
		return s.db.Close()
	}
	return nil
}

func (s *SQLiteStorage) migrate(ctx context.Context) error {
	queries := []string{
		`CREATE TABLE IF NOT EXISTS library_items (
			id TEXT PRIMARY KEY,
			provider_id TEXT NOT NULL,
			media_id TEXT NOT NULL,
			domain INTEGER NOT NULL,
			title TEXT NOT NULL,
			media_type INTEGER NOT NULL,
			poster_url TEXT,
			status TEXT NOT NULL,
			user_rating REAL DEFAULT 0,
			last_interacted_at DATETIME,
			created_at DATETIME,
			updated_at DATETIME,
			UNIQUE(provider_id, media_id)
		);`,
		`CREATE INDEX IF NOT EXISTS idx_library_domain_status ON library_items(domain, status);`,
		`CREATE INDEX IF NOT EXISTS idx_library_last_interacted ON library_items(last_interacted_at DESC);`,

		`CREATE TABLE IF NOT EXISTS playback_progress (
			id TEXT PRIMARY KEY,
			provider_id TEXT NOT NULL,
			media_id TEXT NOT NULL,
			domain INTEGER NOT NULL,
			season_number INTEGER NOT NULL DEFAULT 0,
			episode_number INTEGER NOT NULL DEFAULT 0,
			current_position REAL NOT NULL DEFAULT 0,
			total_duration REAL NOT NULL DEFAULT 0,
			progress_percent REAL NOT NULL DEFAULT 0,
			is_completed INTEGER NOT NULL DEFAULT 0,
			updated_at DATETIME,
			UNIQUE(provider_id, media_id, season_number, episode_number)
		);`,
		`CREATE INDEX IF NOT EXISTS idx_playback_updated ON playback_progress(updated_at DESC);`,

		`CREATE TABLE IF NOT EXISTS reading_progress (
			id TEXT PRIMARY KEY,
			provider_id TEXT NOT NULL,
			media_id TEXT NOT NULL,
			domain INTEGER NOT NULL,
			chapter_id TEXT NOT NULL,
			chapter_number REAL NOT NULL DEFAULT 0,
			current_page INTEGER NOT NULL DEFAULT 0,
			total_pages INTEGER NOT NULL DEFAULT 0,
			text_scroll_ratio REAL NOT NULL DEFAULT 0,
			is_completed INTEGER NOT NULL DEFAULT 0,
			updated_at DATETIME,
			UNIQUE(provider_id, media_id, chapter_id)
		);`,
		`CREATE INDEX IF NOT EXISTS idx_reading_updated ON reading_progress(updated_at DESC);`,
	}

	for _, q := range queries {
		if _, err := s.db.ExecContext(ctx, q); err != nil {
			return fmt.Errorf("migration query failed: %w (sql: %s)", err, q)
		}
	}

	return nil
}

func (s *SQLiteStorage) SaveLibraryItem(ctx context.Context, item *library.Item) error {
	now := time.Now().UTC()
	if item.CreatedAt.IsZero() {
		item.CreatedAt = now
	}
	item.UpdatedAt = now
	if item.LastInteractedAt.IsZero() {
		item.LastInteractedAt = now
	}
	if item.ID == "" {
		item.ID = fmt.Sprintf("%s:%s", item.ProviderID, item.MediaID)
	}

	query := `INSERT INTO library_items (
		id, provider_id, media_id, domain, title, media_type, poster_url,
		status, user_rating, last_interacted_at, created_at, updated_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(provider_id, media_id) DO UPDATE SET
		title = excluded.title,
		poster_url = excluded.poster_url,
		status = excluded.status,
		user_rating = excluded.user_rating,
		last_interacted_at = excluded.last_interacted_at,
		updated_at = excluded.updated_at;`

	_, err := s.db.ExecContext(ctx, query,
		item.ID,
		item.ProviderID,
		item.MediaID,
		int(item.Domain),
		item.Title,
		int(item.Type),
		item.PosterURL,
		string(item.Status),
		item.UserRating,
		item.LastInteractedAt,
		item.CreatedAt,
		item.UpdatedAt,
	)
	return err
}

func (s *SQLiteStorage) GetLibraryItem(ctx context.Context, providerID, mediaID string) (*library.Item, error) {
	query := `SELECT id, provider_id, media_id, domain, title, media_type, poster_url,
		status, user_rating, last_interacted_at, created_at, updated_at
		FROM library_items WHERE provider_id = ? AND media_id = ?;`

	row := s.db.QueryRowContext(ctx, query, providerID, mediaID)

	var item library.Item
	var dom, mType int
	var status string

	err := row.Scan(
		&item.ID,
		&item.ProviderID,
		&item.MediaID,
		&dom,
		&item.Title,
		&mType,
		&item.PosterURL,
		&status,
		&item.UserRating,
		&item.LastInteractedAt,
		&item.CreatedAt,
		&item.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, err
	}

	item.Domain = pluginv1.Domain(dom)
	item.Type = pluginv1.MediaType(mType)
	item.Status = library.Status(status)

	return &item, nil
}

func (s *SQLiteStorage) ListLibraryItems(ctx context.Context, filter library.Filter) ([]*library.Item, int, error) {
	var whereClauses []string
	var args []any

	if filter.Domain != pluginv1.Domain_DOMAIN_UNSPECIFIED {
		whereClauses = append(whereClauses, "domain = ?")
		args = append(args, int(filter.Domain))
	}
	if filter.Status != "" && filter.Status != library.StatusUnspecified {
		whereClauses = append(whereClauses, "status = ?")
		args = append(args, string(filter.Status))
	}

	whereSQL := ""
	if len(whereClauses) > 0 {
		whereSQL = " WHERE " + strings.Join(whereClauses, " AND ")
	}

	countQuery := "SELECT COUNT(*) FROM library_items" + whereSQL
	var totalCount int
	if err := s.db.QueryRowContext(ctx, countQuery, args...).Scan(&totalCount); err != nil {
		return nil, 0, err
	}

	limit := filter.Limit
	if limit <= 0 {
		limit = 50
	}
	offset := filter.Offset
	if offset < 0 {
		offset = 0
	}

	query := `SELECT id, provider_id, media_id, domain, title, media_type, poster_url,
		status, user_rating, last_interacted_at, created_at, updated_at
		FROM library_items` + whereSQL + ` ORDER BY last_interacted_at DESC LIMIT ? OFFSET ?;`

	queryArgs := append(args, limit, offset)
	rows, err := s.db.QueryContext(ctx, query, queryArgs...)
	if err != nil {
		return nil, 0, err
	}
	defer rows.Close()

	var items []*library.Item
	for rows.Next() {
		var item library.Item
		var dom, mType int
		var status string
		if err := rows.Scan(
			&item.ID,
			&item.ProviderID,
			&item.MediaID,
			&dom,
			&item.Title,
			&mType,
			&item.PosterURL,
			&status,
			&item.UserRating,
			&item.LastInteractedAt,
			&item.CreatedAt,
			&item.UpdatedAt,
		); err != nil {
			return nil, 0, err
		}
		item.Domain = pluginv1.Domain(dom)
		item.Type = pluginv1.MediaType(mType)
		item.Status = library.Status(status)
		items = append(items, &item)
	}

	return items, totalCount, rows.Err()
}

func (s *SQLiteStorage) DeleteLibraryItem(ctx context.Context, providerID, mediaID string) error {
	_, err := s.db.ExecContext(ctx, "DELETE FROM library_items WHERE provider_id = ? AND media_id = ?;", providerID, mediaID)
	return err
}

func (s *SQLiteStorage) SavePlaybackProgress(ctx context.Context, p *library.PlaybackProgress) error {
	p.UpdatedAt = time.Now().UTC()
	id := fmt.Sprintf("%s:%s:%d:%d", p.ProviderID, p.MediaID, p.SeasonNumber, p.EpisodeNumber)

	query := `INSERT INTO playback_progress (
		id, provider_id, media_id, domain, season_number, episode_number,
		current_position, total_duration, progress_percent, is_completed, updated_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(provider_id, media_id, season_number, episode_number) DO UPDATE SET
		current_position = excluded.current_position,
		total_duration = excluded.total_duration,
		progress_percent = excluded.progress_percent,
		is_completed = excluded.is_completed,
		updated_at = excluded.updated_at;`

	isCompletedInt := 0
	if p.IsCompleted {
		isCompletedInt = 1
	}

	_, err := s.db.ExecContext(ctx, query,
		id,
		p.ProviderID,
		p.MediaID,
		int(p.Domain),
		p.SeasonNumber,
		p.EpisodeNumber,
		p.CurrentPositionSeconds,
		p.TotalDurationSeconds,
		p.ProgressPercent,
		isCompletedInt,
		p.UpdatedAt,
	)
	return err
}

func (s *SQLiteStorage) GetPlaybackProgress(ctx context.Context, providerID, mediaID string, season, episode int32) (*library.PlaybackProgress, error) {
	query := `SELECT provider_id, media_id, domain, season_number, episode_number,
		current_position, total_duration, progress_percent, is_completed, updated_at
		FROM playback_progress WHERE provider_id = ? AND media_id = ? AND season_number = ? AND episode_number = ?;`

	row := s.db.QueryRowContext(ctx, query, providerID, mediaID, season, episode)

	var p library.PlaybackProgress
	var dom int
	var isCompletedInt int

	err := row.Scan(
		&p.ProviderID,
		&p.MediaID,
		&dom,
		&p.SeasonNumber,
		&p.EpisodeNumber,
		&p.CurrentPositionSeconds,
		&p.TotalDurationSeconds,
		&p.ProgressPercent,
		&isCompletedInt,
		&p.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, err
	}

	p.Domain = pluginv1.Domain(dom)
	p.IsCompleted = isCompletedInt == 1
	return &p, nil
}

func (s *SQLiteStorage) ListRecentPlaybackProgress(ctx context.Context, limit int) ([]*library.PlaybackProgress, error) {
	if limit <= 0 {
		limit = 20
	}

	query := `SELECT provider_id, media_id, domain, season_number, episode_number,
		current_position, total_duration, progress_percent, is_completed, updated_at
		FROM playback_progress ORDER BY updated_at DESC LIMIT ?;`

	rows, err := s.db.QueryContext(ctx, query, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var list []*library.PlaybackProgress
	for rows.Next() {
		var p library.PlaybackProgress
		var dom, isComp int
		if err := rows.Scan(
			&p.ProviderID,
			&p.MediaID,
			&dom,
			&p.SeasonNumber,
			&p.EpisodeNumber,
			&p.CurrentPositionSeconds,
			&p.TotalDurationSeconds,
			&p.ProgressPercent,
			&isComp,
			&p.UpdatedAt,
		); err != nil {
			return nil, err
		}
		p.Domain = pluginv1.Domain(dom)
		p.IsCompleted = isComp == 1
		list = append(list, &p)
	}

	return list, rows.Err()
}

func (s *SQLiteStorage) SaveReadingProgress(ctx context.Context, p *library.ReadingProgress) error {
	p.UpdatedAt = time.Now().UTC()
	id := fmt.Sprintf("%s:%s:%s", p.ProviderID, p.MediaID, p.ChapterID)

	query := `INSERT INTO reading_progress (
		id, provider_id, media_id, domain, chapter_id, chapter_number,
		current_page, total_pages, text_scroll_ratio, is_completed, updated_at
	) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(provider_id, media_id, chapter_id) DO UPDATE SET
		chapter_number = excluded.chapter_number,
		current_page = excluded.current_page,
		total_pages = excluded.total_pages,
		text_scroll_ratio = excluded.text_scroll_ratio,
		is_completed = excluded.is_completed,
		updated_at = excluded.updated_at;`

	isCompletedInt := 0
	if p.IsCompleted {
		isCompletedInt = 1
	}

	_, err := s.db.ExecContext(ctx, query,
		id,
		p.ProviderID,
		p.MediaID,
		int(p.Domain),
		p.ChapterID,
		p.ChapterNumber,
		p.CurrentPage,
		p.TotalPages,
		p.TextScrollRatio,
		isCompletedInt,
		p.UpdatedAt,
	)
	return err
}

func (s *SQLiteStorage) GetReadingProgress(ctx context.Context, providerID, mediaID, chapterID string) (*library.ReadingProgress, error) {
	query := `SELECT provider_id, media_id, domain, chapter_id, chapter_number,
		current_page, total_pages, text_scroll_ratio, is_completed, updated_at
		FROM reading_progress WHERE provider_id = ? AND media_id = ? AND chapter_id = ?;`

	row := s.db.QueryRowContext(ctx, query, providerID, mediaID, chapterID)

	var p library.ReadingProgress
	var dom, isComp int

	err := row.Scan(
		&p.ProviderID,
		&p.MediaID,
		&dom,
		&p.ChapterID,
		&p.ChapterNumber,
		&p.CurrentPage,
		&p.TotalPages,
		&p.TextScrollRatio,
		&isComp,
		&p.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, err
	}

	p.Domain = pluginv1.Domain(dom)
	p.IsCompleted = isComp == 1
	return &p, nil
}

func (s *SQLiteStorage) ListRecentReadingProgress(ctx context.Context, limit int) ([]*library.ReadingProgress, error) {
	if limit <= 0 {
		limit = 20
	}

	query := `SELECT provider_id, media_id, domain, chapter_id, chapter_number,
		current_page, total_pages, text_scroll_ratio, is_completed, updated_at
		FROM reading_progress ORDER BY updated_at DESC LIMIT ?;`

	rows, err := s.db.QueryContext(ctx, query, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var list []*library.ReadingProgress
	for rows.Next() {
		var p library.ReadingProgress
		var dom, isComp int
		if err := rows.Scan(
			&p.ProviderID,
			&p.MediaID,
			&dom,
			&p.ChapterID,
			&p.ChapterNumber,
			&p.CurrentPage,
			&p.TotalPages,
			&p.TextScrollRatio,
			&isComp,
			&p.UpdatedAt,
		); err != nil {
			return nil, err
		}
		p.Domain = pluginv1.Domain(dom)
		p.IsCompleted = isComp == 1
		list = append(list, &p)
	}

	return list, rows.Err()
}
