package provider

import (
	"strings"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type CuratedEntry struct {
	Item    *pluginv1.MediaItem
	Details *pluginv1.MediaDetails
}

var curatedEntries = []CuratedEntry{
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:872585",
			Title:     "Oppenheimer",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2023,
			PosterUrl: "https://image.tmdb.org/t/p/w500/8Gxv8gSFCU0XGDykEGv7zR1n2ua.jpg",
			Overview:  "The story of J. Robert Oppenheimer's role in the development of the atomic bomb during World War II.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "872585",
				ImdbId: "tt15398776",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:872585",
			Title:     "Oppenheimer",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2023,
			PosterUrl: "https://image.tmdb.org/t/p/w500/8Gxv8gSFCU0XGDykEGv7zR1n2ua.jpg",
			Overview:  "The story of J. Robert Oppenheimer's role in the development of the atomic bomb during World War II.",
			Genres:    []string{"Drama", "History"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "872585",
				ImdbId: "tt15398776",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:693134",
			Title:     "Dune: Part Two",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2024,
			PosterUrl: "https://image.tmdb.org/t/p/w500/1pdfLvkbY9ohJlCjQH2CZjjYVvJ.jpg",
			Overview:  "Follow the mythic journey of Paul Atreides as he unites with Chani and the Fremen while seeking revenge against the conspirators who destroyed his family.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "693134",
				ImdbId: "tt15239678",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:693134",
			Title:     "Dune: Part Two",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2024,
			PosterUrl: "https://image.tmdb.org/t/p/w500/1pdfLvkbY9ohJlCjQH2CZjjYVvJ.jpg",
			Overview:  "Follow the mythic journey of Paul Atreides as he unites with Chani and the Fremen while seeking revenge against the conspirators who destroyed his family.",
			Genres:    []string{"Science Fiction", "Adventure"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "693134",
				ImdbId: "tt15239678",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:157336",
			Title:     "Interstellar",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2014,
			PosterUrl: "https://image.tmdb.org/t/p/w500/gEU2QniE6E77NI6lCU6MxlNBvIx.jpg",
			Overview:  "The adventures of a group of explorers who make use of a newly discovered wormhole to surpass the limitations on human space travel and conquer the vast distances involved in an interstellar voyage.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "157336",
				ImdbId: "tt0816692",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:157336",
			Title:     "Interstellar",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2014,
			PosterUrl: "https://image.tmdb.org/t/p/w500/gEU2QniE6E77NI6lCU6MxlNBvIx.jpg",
			Overview:  "The adventures of a group of explorers who make use of a newly discovered wormhole to surpass the limitations on human space travel and conquer the vast distances involved in an interstellar voyage.",
			Genres:    []string{"Adventure", "Drama", "Science Fiction"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "157336",
				ImdbId: "tt0816692",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:27205",
			Title:     "Inception",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2010,
			PosterUrl: "https://image.tmdb.org/t/p/w500/edv5CZvWj09upOsy2Y6IwDhK8bt.jpg",
			Overview:  "Cobb, a skilled thief who steals corporate secrets through the use of dream-sharing technology, is given the inverse task of planting an idea into the mind of a C.E.O.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "27205",
				ImdbId: "tt1375666",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:27205",
			Title:     "Inception",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2010,
			PosterUrl: "https://image.tmdb.org/t/p/w500/edv5CZvWj09upOsy2Y6IwDhK8bt.jpg",
			Overview:  "Cobb, a skilled thief who steals corporate secrets through the use of dream-sharing technology, is given the inverse task of planting an idea into the mind of a C.E.O.",
			Genres:    []string{"Action", "Science Fiction", "Adventure"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "27205",
				ImdbId: "tt1375666",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:155",
			Title:     "The Dark Knight",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2008,
			PosterUrl: "https://image.tmdb.org/t/p/w500/qJ2tW6WMUDux911r6m7haRef0WH.jpg",
			Overview:  "Batman raises the stakes in his war on crime. With the help of Lt. Jim Gordon and District Attorney Harvey Dent, Batman sets out to dismantle the remaining criminal organizations that plague the streets.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "155",
				ImdbId: "tt0468569",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:155",
			Title:     "The Dark Knight",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2008,
			PosterUrl: "https://image.tmdb.org/t/p/w500/qJ2tW6WMUDux911r6m7haRef0WH.jpg",
			Overview:  "Batman raises the stakes in his war on crime. With the help of Lt. Jim Gordon and District Attorney Harvey Dent, Batman sets out to dismantle the remaining criminal organizations that plague the streets.",
			Genres:    []string{"Drama", "Action", "Crime", "Thriller"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "155",
				ImdbId: "tt0468569",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "tv:94605",
			Title:     "Arcane",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2021,
			PosterUrl: "https://image.tmdb.org/t/p/w500/fqldf2t8ztc9aiwn3k6mlX3tvRT.jpg",
			Overview:  "Amid the stark discord of twin cities Piltover and Zaun, two sisters fight on rival sides of a war between magic technologies and incompatible convictions.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "94605",
				ImdbId: "tt11126994",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "tv:94605",
			Title:     "Arcane",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2021,
			PosterUrl: "https://image.tmdb.org/t/p/w500/fqldf2t8ztc9aiwn3k6mlX3tvRT.jpg",
			Overview:  "Amid the stark discord of twin cities Piltover and Zaun, two sisters fight on rival sides of a war between magic technologies and incompatible convictions.",
			Genres:    []string{"Animation", "Sci-Fi & Fantasy", "Action & Adventure"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "94605",
				ImdbId: "tt11126994",
			},
			Seasons: []*pluginv1.Season{
				{
					SeasonNumber: 1,
					Title:        "Season 1",
					Episodes: []*pluginv1.Episode{
						{EpisodeNumber: 1, Title: "Welcome to the Playground", DurationSeconds: 2400},
						{EpisodeNumber: 2, Title: "Some Mysteries Are Better Left Unsolved", DurationSeconds: 2520},
						{EpisodeNumber: 3, Title: "The Base Violence Necessary for Change", DurationSeconds: 2640},
					},
				},
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "tv:1396",
			Title:     "Breaking Bad",
			Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
			Year:      2008,
			PosterUrl: "https://image.tmdb.org/t/p/w500/ggFHVNu6YYI5L9pCfOacjizRGt.jpg",
			Overview:  "Walter White, a New Mexico chemistry teacher, is diagnosed with Stage III cancer and given a prognosis of two years to live. He chooses to enter a dangerous world of drugs and crime.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "1396",
				ImdbId: "tt0903747",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "tv:1396",
			Title:     "Breaking Bad",
			Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
			Year:      2008,
			PosterUrl: "https://image.tmdb.org/t/p/w500/ggFHVNu6YYI5L9pCfOacjizRGt.jpg",
			Overview:  "Walter White, a New Mexico chemistry teacher, is diagnosed with Stage III cancer and given a prognosis of two years to live. He chooses to enter a dangerous world of drugs and crime.",
			Genres:    []string{"Drama", "Crime"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "1396",
				ImdbId: "tt0903747",
			},
			Seasons: []*pluginv1.Season{
				{
					SeasonNumber: 1,
					Title:        "Season 1",
					Episodes: []*pluginv1.Episode{
						{EpisodeNumber: 1, Title: "Pilot", DurationSeconds: 3480},
						{EpisodeNumber: 2, Title: "Cat's in the Bag...", DurationSeconds: 2880},
						{EpisodeNumber: 3, Title: "...And the Bag's in the River", DurationSeconds: 2880},
					},
				},
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "tv:1429",
			Title:     "Attack on Titan",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2013,
			PosterUrl: "https://image.tmdb.org/t/p/w500/hTP1DtLGFamjfu8WqjnuQdP1n4i.jpg",
			Overview:  "Several hundred years ago, humans were nearly exterminated by titans. A small percentage of humanity survived by barricading themselves in a city protected by extremely high walls.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "1429",
				ImdbId: "tt2560140",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "tv:1429",
			Title:     "Attack on Titan",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2013,
			PosterUrl: "https://image.tmdb.org/t/p/w500/hTP1DtLGFamjfu8WqjnuQdP1n4i.jpg",
			Overview:  "Several hundred years ago, humans were nearly exterminated by titans. A small percentage of humanity survived by barricading themselves in a city protected by extremely high walls.",
			Genres:    []string{"Animation", "Sci-Fi & Fantasy", "Action & Adventure"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "1429",
				ImdbId: "tt2560140",
			},
			Seasons: []*pluginv1.Season{
				{
					SeasonNumber: 1,
					Title:        "Season 1",
					Episodes: []*pluginv1.Episode{
						{EpisodeNumber: 1, Title: "To You, in 2000 Years: The Fall of Shiganshina, Part 1", DurationSeconds: 1440},
						{EpisodeNumber: 2, Title: "That Day: The Fall of Shiganshina, Part 2", DurationSeconds: 1440},
					},
				},
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:129",
			Title:     "Spirited Away",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2001,
			PosterUrl: "https://image.tmdb.org/t/p/w500/39wmItIWsg5sZMyRUHLkWBcuVCM.jpg",
			Overview:  "A young girl, Chihiro, becomes trapped in a strange new world of spirits. When her parents undergo a mysterious transformation, she must call upon the courage she never knew she had to free her family.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "129",
				ImdbId: "tt0245429",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:129",
			Title:     "Spirited Away",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2001,
			PosterUrl: "https://image.tmdb.org/t/p/w500/39wmItIWsg5sZMyRUHLkWBcuVCM.jpg",
			Overview:  "A young girl, Chihiro, becomes trapped in a strange new world of spirits. When her parents undergo a mysterious transformation, she must call upon the courage she never knew she had to free her family.",
			Genres:    []string{"Animation", "Family", "Fantasy"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "129",
				ImdbId: "tt0245429",
			},
		},
	},
}

func getFallbackSearch(query string) []*pluginv1.MediaItem {
	q := strings.ToLower(strings.TrimSpace(query))
	if q == "" || q == "popular" || q == "trending" || q == "all" || q == "batman" {
		var list []*pluginv1.MediaItem
		for _, e := range curatedEntries {
			list = append(list, e.Item)
		}
		return list
	}

	var matched []*pluginv1.MediaItem
	for _, e := range curatedEntries {
		if strings.Contains(strings.ToLower(e.Item.Title), q) || strings.Contains(strings.ToLower(e.Item.Overview), q) {
			matched = append(matched, e.Item)
		}
	}
	if len(matched) == 0 {
		// Return all popular items so screen is never blank
		for _, e := range curatedEntries {
			matched = append(matched, e.Item)
		}
	}
	return matched
}

func getFallbackDetails(mediaID string) *pluginv1.MediaDetails {
	for _, e := range curatedEntries {
		if e.Details.Id == mediaID {
			return e.Details
		}
	}
	// Default to first item if ID matches number
	for _, e := range curatedEntries {
		if strings.Contains(e.Details.Id, mediaID) || strings.Contains(mediaID, e.Details.ExternalIds.TmdbId) {
			return e.Details
		}
	}
	return nil
}
