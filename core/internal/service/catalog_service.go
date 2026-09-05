package service

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/catalog"
	"github.com/falsisdev/vessel/core/internal/plugin"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type CatalogService struct {
	manager *plugin.Manager
	timeout time.Duration
}

func NewCatalogService(manager *plugin.Manager, timeout time.Duration) *CatalogService {
	if timeout <= 0 {
		timeout = 5 * time.Second
	}
	return &CatalogService{
		manager: manager,
		timeout: timeout,
	}
}

type catalogQueryDef struct {
	id          string
	title       string
	catalogType catalog.CatalogType
	query       string
}

func (s *CatalogService) GetCatalogs(ctx context.Context, domain pluginv1.Domain) ([]catalog.CatalogRow, error) {
	var targetPlugins []plugin.Client
	if domain != pluginv1.Domain_DOMAIN_UNSPECIFIED {
		targetPlugins = s.manager.ListByDomainAndCapability(domain, pluginv1.Capability_CAPABILITY_SEARCH)
	} else {
		// All plugins that support search
		cinemaPlugins := s.manager.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_CINEMA, pluginv1.Capability_CAPABILITY_SEARCH)
		readingPlugins := s.manager.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_MANGA, pluginv1.Capability_CAPABILITY_SEARCH)
		livePlugins := s.manager.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_LIVE, pluginv1.Capability_CAPABILITY_SEARCH)
		iptvPlugins := s.manager.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_IPTV, pluginv1.Capability_CAPABILITY_SEARCH)

		seen := make(map[string]bool)
		for _, p := range append(append(append(cinemaPlugins, readingPlugins...), livePlugins...), iptvPlugins...) {
			if !seen[p.Manifest().Id] {
				seen[p.Manifest().Id] = true
				targetPlugins = append(targetPlugins, p)
			}
		}
	}

	if len(targetPlugins) == 0 {
		return []catalog.CatalogRow{}, nil
	}

	reqCtx, cancel := context.WithTimeout(ctx, s.timeout)
	defer cancel()

	var rows []catalog.CatalogRow
	var mu sync.Mutex
	var wg sync.WaitGroup

	for _, p := range targetPlugins {
		manifest := p.Manifest()
		pID := manifest.Id
		pName := manifest.Name
		pDomain := manifest.Domain

		var queries []catalogQueryDef
		if pDomain == pluginv1.Domain_DOMAIN_CINEMA {
			queries = []catalogQueryDef{
				{
					id:          fmt.Sprintf("%s-popular", pID),
					title:       fmt.Sprintf("%s • Popüler", pName),
					catalogType: catalog.CatalogTypePopular,
					query:       "popular",
				},
				{
					id:          fmt.Sprintf("%s-toprated", pID),
					title:       fmt.Sprintf("%s • En Çok Oy Alanlar", pName),
					catalogType: catalog.CatalogTypeTopRated,
					query:       "top_rated",
				},
				{
					id:          fmt.Sprintf("%s-featured", pID),
					title:       fmt.Sprintf("%s • Öne Çıkanlar", pName),
					catalogType: catalog.CatalogTypeFeatured,
					query:       "featured",
				},
			}
		} else if pDomain == pluginv1.Domain_DOMAIN_MANGA {
			queries = []catalogQueryDef{
				{
					id:          fmt.Sprintf("%s-popular", pID),
					title:       fmt.Sprintf("%s • Popüler İçerikler", pName),
					catalogType: catalog.CatalogTypePopular,
					query:       "popular",
				},
				{
					id:          fmt.Sprintf("%s-latest", pID),
					title:       fmt.Sprintf("%s • Son Oluşturulan İçerikler", pName),
					catalogType: catalog.CatalogTypeLatest,
					query:       "latest",
				},
			}
		} else if pDomain == pluginv1.Domain_DOMAIN_IPTV {
			queries = []catalogQueryDef{
				{
					id:          fmt.Sprintf("%s-popular", pID),
					title:       fmt.Sprintf("%s • Popüler Kanallar", pName),
					catalogType: catalog.CatalogTypePopular,
					query:       "popular",
				},
				{
					id:          fmt.Sprintf("%s-news", pID),
					title:       fmt.Sprintf("%s • Haber & Bilgi", pName),
					catalogType: catalog.CatalogTypeFeatured,
					query:       "news",
				},
				{
					id:          fmt.Sprintf("%s-sports", pID),
					title:       fmt.Sprintf("%s • Spor & Eğlence", pName),
					catalogType: catalog.CatalogTypeLatest,
					query:       "sports",
				},
			}
		} else {
			queries = []catalogQueryDef{
				{
					id:          fmt.Sprintf("%s-popular", pID),
					title:       fmt.Sprintf("%s • Yayınlar & Kanallar", pName),
					catalogType: catalog.CatalogTypePopular,
					query:       "popular",
				},
			}
		}

		for _, qDef := range queries {
			wg.Add(1)
			go func(client plugin.Client, q catalogQueryDef, providerID, providerName string, dom pluginv1.Domain) {
				defer wg.Done()

				resp, err := client.Search(reqCtx, q.query, 1)
				if err != nil || resp == nil || len(resp.Items) == 0 {
					return
				}

				var items []catalog.CatalogItem
				for _, it := range resp.Items {
					if it == nil {
						continue
					}
					typeName := "Media"
					switch it.Type {
					case pluginv1.MediaType_MEDIA_TYPE_MOVIE:
						typeName = "Movie"
					case pluginv1.MediaType_MEDIA_TYPE_SERIES:
						typeName = "Series"
					case pluginv1.MediaType_MEDIA_TYPE_ANIME:
						typeName = "Anime"
					case pluginv1.MediaType_MEDIA_TYPE_MANGA:
						typeName = "Manga"
					case pluginv1.MediaType_MEDIA_TYPE_WEBTOON:
						typeName = "Webtoon"
					case pluginv1.MediaType_MEDIA_TYPE_WEBOOK:
						typeName = "Novel"
					}

					poster := it.PosterUrl
					if poster == "" {
						poster = "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400"
					}

					items = append(items, catalog.CatalogItem{
						ID:          it.Id,
						ProviderID:  providerID,
						Title:       it.Title,
						Type:        int(it.Type),
						TypeName:    typeName,
						Year:        it.Year,
						PosterURL:   poster,
						Overview:    it.Overview,
						Rating:      8.5,
						Domain:      dom,
					})
				}

				if len(items) > 0 {
					domainName := "Cinema"
					switch dom {
					case pluginv1.Domain_DOMAIN_MANGA:
						domainName = "Manga"
					case pluginv1.Domain_DOMAIN_LIVE:
						domainName = "Live"
					case pluginv1.Domain_DOMAIN_IPTV:
						domainName = "IPTV"
					}

					row := catalog.CatalogRow{
						ID:           q.id,
						Title:        q.title,
						CatalogType:  q.catalogType,
						Domain:       dom,
						DomainName:   domainName,
						ProviderID:   providerID,
						ProviderName: providerName,
						Items:        items,
					}
					mu.Lock()
					rows = append(rows, row)
					mu.Unlock()
				}
			}(p, qDef, pID, pName, pDomain)
		}
	}

	wg.Wait()
	return rows, nil
}
