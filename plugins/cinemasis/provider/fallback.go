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
	{
		Item: &pluginv1.MediaItem{
			Id:        "tv:13916",
			Title:     "Death Note",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2006,
			PosterUrl: "https://image.tmdb.org/t/p/w500/t7q9vvdBfy7E5C0fU0rE9C88Ika.jpg",
			Overview:  "Light Yagami finds a notebook with deadly power, leading into a genius psychological battle of wits with detective L, packed with mind-bending plot twists and intense mystery.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "13916",
				ImdbId: "tt0877057",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "tv:13916",
			Title:     "Death Note",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2006,
			PosterUrl: "https://image.tmdb.org/t/p/w500/t7q9vvdBfy7E5C0fU0rE9C88Ika.jpg",
			Overview:  "Light Yagami finds a notebook with deadly power, leading into a genius psychological battle of wits with detective L, packed with mind-bending plot twists and intense mystery.",
			Genres:    []string{"Animation", "Mystery", "Psychological", "Thriller"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "13916",
				ImdbId: "tt0877057",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "tv:42635",
			Title:     "Steins;Gate",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2011,
			PosterUrl: "https://image.tmdb.org/t/p/w500/5ibLbbLq6d6aMh7P7z5vE2kK6r6.jpg",
			Overview:  "Self-proclaimed mad scientist Rintaro Okabe accidentally discovers time-travel. A dark, mind-bending psychological thriller with time loops, conspiracy, and unexpected plot twists.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "42635",
				ImdbId: "tt1910272",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "tv:42635",
			Title:     "Steins;Gate",
			Type:      pluginv1.MediaType_MEDIA_TYPE_ANIME,
			Year:      2011,
			PosterUrl: "https://image.tmdb.org/t/p/w500/5ibLbbLq6d6aMh7P7z5vE2kK6r6.jpg",
			Overview:  "Self-proclaimed mad scientist Rintaro Okabe accidentally discovers time-travel. A dark, mind-bending psychological thriller with time loops, conspiracy, and unexpected plot twists.",
			Genres:    []string{"Animation", "Sci-Fi", "Psychological", "Thriller"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "42635",
				ImdbId: "tt1910272",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:278",
			Title:     "The Shawshank Redemption",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1994,
			PosterUrl: "https://image.tmdb.org/t/p/w500/9cqNxx0GxF0bflZmeSMuL5tnGzr.jpg",
			Overview:  "Imprisoned in the 1940s for the double murder of his wife and her lover, upstanding banker Andy Dufresne begins a new life at the Shawshank prison.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "278",
				ImdbId: "tt0111161",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:278",
			Title:     "The Shawshank Redemption",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1994,
			PosterUrl: "https://image.tmdb.org/t/p/w500/9cqNxx0GxF0bflZmeSMuL5tnGzr.jpg",
			Overview:  "Imprisoned in the 1940s for the double murder of his wife and her lover, upstanding banker Andy Dufresne begins a new life at the Shawshank prison.",
			Genres:    []string{"Drama", "Crime"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "278",
				ImdbId: "tt0111161",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:238",
			Title:     "The Godfather",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1972,
			PosterUrl: "https://image.tmdb.org/t/p/w500/3bhkrj58Vtu7enYsRolD1fZdja1.jpg",
			Overview:  "Spanning the years 1945 to 1955, a chronicle of the fictional Italian-American Corleone crime family. When organized crime family patriarch, Vito Corleone barely survives an attempt on his life, his youngest son, Michael steps in to take care of the would-be killers.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "238",
				ImdbId: "tt0068646",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:238",
			Title:     "The Godfather",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1972,
			PosterUrl: "https://image.tmdb.org/t/p/w500/3bhkrj58Vtu7enYsRolD1fZdja1.jpg",
			Overview:  "Spanning the years 1945 to 1955, a chronicle of the fictional Italian-American Corleone crime family.",
			Genres:    []string{"Drama", "Crime"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "238",
				ImdbId: "tt0068646",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:680",
			Title:     "Pulp Fiction",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1994,
			PosterUrl: "https://image.tmdb.org/t/p/w500/vQWk5YBFWF4bZaofAbv0tShwBvQ.jpg",
			Overview:  "A burger-loving hit man, his philosophical partner, a drug-addled gangster's moll and a washed-up boxer converge in this sprawling, comedic crime caper.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "680",
				ImdbId: "tt0110912",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:680",
			Title:     "Pulp Fiction",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1994,
			PosterUrl: "https://image.tmdb.org/t/p/w500/vQWk5YBFWF4bZaofAbv0tShwBvQ.jpg",
			Overview:  "A burger-loving hit man, his philosophical partner, a drug-addled gangster's moll and a washed-up boxer converge.",
			Genres:    []string{"Thriller", "Crime"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "680",
				ImdbId: "tt0110912",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:68718",
			Title:     "Django Unchained",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2012,
			PosterUrl: "https://image.tmdb.org/t/p/w500/7oWY8vd27zHzGh9V6R1aNiZmuqq.jpg",
			Overview:  "With the help of a German bounty-hunter, a freed slave sets out to rescue his wife from a brutal Mississippi plantation owner.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "68718",
				ImdbId: "tt1853728",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:68718",
			Title:     "Django Unchained",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2012,
			PosterUrl: "https://image.tmdb.org/t/p/w500/7oWY8vd27zHzGh9V6R1aNiZmuqq.jpg",
			Overview:  "With the help of a German bounty-hunter, a freed slave sets out to rescue his wife from a brutal Mississippi plantation owner in the Wild West.",
			Genres:    []string{"Drama", "Western", "Action"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "68718",
				ImdbId: "tt1853728",
			},
		},
	},
	{
		Item: &pluginv1.MediaItem{
			Id:        "movie:429",
			Title:     "The Good, the Bad and the Ugly",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1966,
			PosterUrl: "https://image.tmdb.org/t/p/w500/bX2xnavhMYjWDoZp1VM6VnU1xwe.jpg",
			Overview:  "While the Civil War rages between the Union and a Confederacy, three gunslingers and cowboy outlaws race to find a fortune in buried Confederate gold.",
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "429",
				ImdbId: "tt0060196",
			},
		},
		Details: &pluginv1.MediaDetails{
			Id:        "movie:429",
			Title:     "The Good, the Bad and the Ugly",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      1966,
			PosterUrl: "https://image.tmdb.org/t/p/w500/bX2xnavhMYjWDoZp1VM6VnU1xwe.jpg",
			Overview:  "While the Civil War rages, three cowboy gunslingers search for buried gold in Sergio Leone's definitive western.",
			Genres:    []string{"Western", "Adventure"},
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: "429",
				ImdbId: "tt0060196",
			},
		},
	},
}

func getFallbackSearch(query string) []*pluginv1.MediaItem {
	q := strings.ToLower(strings.TrimSpace(query))

	// Western / Cowboy queries
	if strings.Contains(q, "western") || strings.Contains(q, "kovboy") || strings.Contains(q, "cowboy") || strings.Contains(q, "django") {
		var list []*pluginv1.MediaItem
		for _, e := range curatedEntries {
			if e.Item.Id == "movie:68718" || e.Item.Id == "movie:429" {
				list = append(list, e.Item)
			}
		}
		if len(list) > 0 {
			return list
		}
	}

	// Psychological / Plot Twist / Anime queries
	if strings.Contains(q, "death note") || strings.Contains(q, "steins") || strings.Contains(q, "ters köşe") || strings.Contains(q, "plot twist") || strings.Contains(q, "psychological") || strings.Contains(q, "mind bending") || strings.Contains(q, "zihin yakan") {
		var list []*pluginv1.MediaItem
		for _, e := range curatedEntries {
			if e.Item.Id == "tv:13916" || e.Item.Id == "tv:42635" || e.Item.Id == "tv:1429" || e.Item.Id == "movie:99" {
				list = append(list, e.Item)
			}
		}
		if len(list) > 0 {
			return list
		}
	}

	// Top rated queries
	if q == "top_rated" || q == "toprated" || q == "en çok oy alanlar" {
		var list []*pluginv1.MediaItem
		// Shawshank, Godfather, Pulp Fiction, Spirited Away, Dark Knight
		for _, e := range curatedEntries {
			if e.Item.Id == "movie:278" || e.Item.Id == "movie:238" || e.Item.Id == "movie:680" || e.Item.Id == "movie:129" || e.Item.Id == "movie:155" {
				list = append(list, e.Item)
			}
		}
		return list
	}

	// Popular / Trending queries
	if q == "" || q == "popular" || q == "trending" || q == "all" || q == "featured" {
		var list []*pluginv1.MediaItem
		// Oppenheimer, Dune 2, Spider-man, Batman, Cyberpunk
		for _, e := range curatedEntries {
			if e.Item.Id == "movie:872585" || e.Item.Id == "movie:693134" || e.Item.Id == "movie:569094" || e.Item.Id == "movie:414906" || e.Item.Id == "series:105248" || e.Item.Id == "series:1399" {
				list = append(list, e.Item)
			}
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
