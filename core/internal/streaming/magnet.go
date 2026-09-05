package streaming

import (
	"encoding/base32"
	"encoding/hex"
	"errors"
	"net/url"
	"strings"
)

var (
	ErrInvalidMagnetURI = errors.New("invalid magnet uri")
	ErrMissingInfoHash  = errors.New("missing infohash in magnet uri")
)

// Magnet represents a parsed magnet URI.
type Magnet struct {
	OriginalURI string
	InfoHash    string // 40-character lowercase hexadecimal
	DisplayName string
	Trackers    []string
}

// IsMagnetURI checks if the given string is a magnet URI.
func IsMagnetURI(uri string) bool {
	return strings.HasPrefix(strings.TrimSpace(uri), "magnet:?")
}

// ParseMagnet parses a magnet URI into a Magnet struct.
func ParseMagnet(rawURI string) (*Magnet, error) {
	trimmed := strings.TrimSpace(rawURI)
	if !IsMagnetURI(trimmed) {
		return nil, ErrInvalidMagnetURI
	}

	u, err := url.Parse(trimmed)
	if err != nil {
		return nil, ErrInvalidMagnetURI
	}

	q := u.Query()
	xtList := q["xt"]
	var infoHash string

	for _, xt := range xtList {
		if strings.HasPrefix(xt, "urn:btih:") {
			hashStr := strings.TrimPrefix(xt, "urn:btih:")
			// Could be 40-character hex or 32-character base32
			if len(hashStr) == 40 {
				infoHash = strings.ToLower(hashStr)
				break
			} else if len(hashStr) == 32 {
				// Base32 encoded
				decoded, err := base32.StdEncoding.DecodeString(strings.ToUpper(hashStr))
				if err == nil && len(decoded) == 20 {
					infoHash = hex.EncodeToString(decoded)
					break
				}
			}
		}
	}

	if infoHash == "" {
		return nil, ErrMissingInfoHash
	}

	dn := q.Get("dn")
	trackers := q["tr"]

	return &Magnet{
		OriginalURI: rawURI,
		InfoHash:    infoHash,
		DisplayName: dn,
		Trackers:    trackers,
	}, nil
}
