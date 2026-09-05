package catalog

import (
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type CatalogType string

const (
	CatalogTypePopular   CatalogType = "popular"
	CatalogTypeFeatured  CatalogType = "featured"
	CatalogTypeTrending  CatalogType = "trending"
	CatalogTypeTopRated  CatalogType = "top_rated"
	CatalogTypeLatest    CatalogType = "latest"
)

type CatalogItem struct {
	ID          string             `json:"id"`
	ProviderID  string             `json:"provider_id"`
	Title       string             `json:"title"`
	Type        int                `json:"type"`
	TypeName    string             `json:"type_name"`
	Year        int32              `json:"year"`
	PosterURL   string             `json:"poster_url"`
	Overview    string             `json:"overview"`
	Rating      float32            `json:"rating"`
	Domain      pluginv1.Domain    `json:"domain"`
}

type CatalogRow struct {
	ID           string        `json:"id"`
	Title        string        `json:"title"`
	CatalogType  CatalogType   `json:"catalog_type"`
	Domain       pluginv1.Domain `json:"domain"`
	DomainName   string        `json:"domain_name"`
	ProviderID   string        `json:"provider_id"`
	ProviderName string        `json:"provider_name"`
	Items        []CatalogItem `json:"items"`
}
