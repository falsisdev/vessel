package discovery

import (
	"context"
	"fmt"
	"math"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/cinema"
	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/domain/reading"
	"github.com/falsisdev/vessel/core/internal/service"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

const VectorDim = 64

// Semantic Vector Dimensions
const (
	DimAction          = 0
	DimAdventure       = 1
	DimDark            = 2
	DimSciFi           = 3
	DimCyberpunk       = 4
	DimFantasy         = 5
	DimMystery         = 6
	DimPsychological   = 7
	DimPlotTwist       = 8
	DimThriller        = 9
	DimHorror          = 10
	DimComedy          = 11
	DimDrama           = 12
	DimRomance         = 13
	DimSliceOfLife     = 14
	DimWholesome       = 15
	DimFastPaced       = 16
	DimAtmospheric     = 17
	DimPhilosophical   = 18
	DimCrime           = 19
	DimHistorical      = 20
	DimSpace           = 21
	DimPostApocalyptic = 22
	DimShonen          = 23
	DimSeinen          = 24
	DimIsekai          = 25
	DimRetro           = 26
	DimSupernatural    = 27
	DimAnimation       = 28
	DimDocumentary     = 29
	DimSports          = 30
	DimMusic           = 31
	DimMindBending     = 32
	DimSuspense        = 33
	DimDystopian       = 34
	DimMecha           = 35
	DimMagic           = 36
	DimDetective       = 37
	DimSatire          = 38
	DimMartialArts     = 39
	DimNoir            = 40
	DimTragedy         = 41
	DimEpic            = 42
	DimSurvival        = 43
	DimCozy            = 44
	DimHeist           = 45
	DimCult            = 46
	DimSuperheroes     = 47
	DimGaming          = 48
	DimMythology       = 49
	DimSteampunk       = 50
	DimTimeTravel      = 51
	DimFamily          = 52
	DimWar             = 53
	DimWestern         = 54
	DimPolitical       = 55
	DimParanormal      = 56
	DimGore            = 57
	DimRomCom          = 58
	DimEspionage       = 59
	DimVampires        = 60
	DimZombies         = 61
	DimCosmicHorror    = 62
	DimUrbanFantasy    = 63
)

type Vector [VectorDim]float32

type MoodPreset struct {
	ID          string   `json:"id"`
	Icon        string   `json:"icon"`
	TitleTR     string   `json:"title_tr"`
	TitleEN     string   `json:"title_en"`
	DescTR      string   `json:"desc_tr"`
	DescEN      string   `json:"desc_en"`
	TargetWords []string `json:"target_words"`
}

type TasteProfile struct {
	ActiveTraitsTR  []string       `json:"active_traits_tr"`
	ActiveTraitsEN  []string       `json:"active_traits_en"`
	TopKeywords     []string       `json:"top_keywords"`
	TotalItemsCount int            `json:"total_items_count"`
	TasteAffinity   map[string]int `json:"taste_affinity"`
	UpdatedAt       time.Time      `json:"updated_at"`
}

type RecommendationItem struct {
	ID         string   `json:"id"`
	ProviderID string   `json:"provider_id"`
	Title      string   `json:"title"`
	PosterURL  string   `json:"poster_url"`
	Domain     string   `json:"domain"`
	Type       string   `json:"type"`
	Year       int32    `json:"year,omitempty"`
	Rating     float32  `json:"rating,omitempty"`
	Genres     []string `json:"genres,omitempty"`
	Overview   string   `json:"overview,omitempty"`
	MatchScore int      `json:"match_score"`
	ReasonTR   string   `json:"reason_tr"`
	ReasonEN   string   `json:"reason_en"`
	VibeTags   []string `json:"vibe_tags"`
}

type AIEngine struct {
	librarySvc *service.LibraryService
	cinemaSvc  *service.CinemaService
	readingSvc *service.ReadingService
	catalogSvc *service.CatalogService

	lexicon map[string][]struct {
		dim    int
		weight float32
	}
	moodPresets []MoodPreset

	cachedTasteMu sync.RWMutex
	cachedProfile *TasteProfile
	cachedVector  Vector
	lastProfileAt time.Time
}

func NewAIEngine(
	libSvc *service.LibraryService,
	cinSvc *service.CinemaService,
	readSvc *service.ReadingService,
	catSvc *service.CatalogService,
) *AIEngine {
	engine := &AIEngine{
		librarySvc: libSvc,
		cinemaSvc:  cinSvc,
		readingSvc: readSvc,
		catalogSvc: catSvc,
		lexicon:    make(map[string][]struct {
			dim    int
			weight float32
		}),
	}

	engine.initLexicon()
	engine.initMoodPresets()
	return engine
}

func (e *AIEngine) addKeyword(word string, mappings ...struct {
	dim    int
	weight float32
}) {
	word = strings.ToLower(strings.TrimSpace(word))
	if word != "" {
		e.lexicon[word] = mappings
	}
}

func (e *AIEngine) initLexicon() {
	m := func(dim int, weight float32) struct {
		dim    int
		weight float32
	} {
		return struct {
			dim    int
			weight float32
		}{dim: dim, weight: weight}
	}

	// Plot Twist & Mind-Bending
	e.addKeyword("ters köşe", m(DimPlotTwist, 1.0), m(DimPsychological, 0.8), m(DimMindBending, 0.9))
	e.addKeyword("plot twist", m(DimPlotTwist, 1.0), m(DimPsychological, 0.8), m(DimMindBending, 0.9))
	e.addKeyword("twist", m(DimPlotTwist, 0.9), m(DimMystery, 0.5))
	e.addKeyword("beyin yakan", m(DimMindBending, 1.0), m(DimPsychological, 0.9), m(DimPhilosophical, 0.7))
	e.addKeyword("mind bending", m(DimMindBending, 1.0), m(DimPsychological, 0.9), m(DimPhilosophical, 0.7))
	e.addKeyword("mind-bending", m(DimMindBending, 1.0), m(DimPsychological, 0.9))
	e.addKeyword("akıl almaz", m(DimMindBending, 0.8), m(DimMystery, 0.6))
	e.addKeyword("psikolojik", m(DimPsychological, 1.0), m(DimDrama, 0.6), m(DimSuspense, 0.7))
	e.addKeyword("psychological", m(DimPsychological, 1.0), m(DimDrama, 0.6), m(DimSuspense, 0.7))

	// Dark, Gritty, Dystopian & Cyberpunk
	e.addKeyword("karanlık", m(DimDark, 1.0), m(DimAtmospheric, 0.7), m(DimNoir, 0.6))
	e.addKeyword("dark", m(DimDark, 1.0), m(DimAtmospheric, 0.7), m(DimNoir, 0.6))
	e.addKeyword("kasvetli", m(DimDark, 0.9), m(DimAtmospheric, 0.8))
	e.addKeyword("gritty", m(DimDark, 0.8), m(DimAction, 0.6), m(DimCrime, 0.7))
	e.addKeyword("siberpunk", m(DimCyberpunk, 1.0), m(DimSciFi, 0.9), m(DimDystopian, 0.8))
	e.addKeyword("cyberpunk", m(DimCyberpunk, 1.0), m(DimSciFi, 0.9), m(DimDystopian, 0.8))
	e.addKeyword("distopya", m(DimDystopian, 1.0), m(DimSciFi, 0.8), m(DimDark, 0.6))
	e.addKeyword("distopik", m(DimDystopian, 1.0), m(DimSciFi, 0.8), m(DimDark, 0.6))
	e.addKeyword("dystopian", m(DimDystopian, 1.0), m(DimSciFi, 0.8), m(DimDark, 0.6))
	e.addKeyword("dystopia", m(DimDystopian, 1.0), m(DimSciFi, 0.8))
	e.addKeyword("post-apocalyptic", m(DimPostApocalyptic, 1.0), m(DimSurvival, 0.9))
	e.addKeyword("kıyamet", m(DimPostApocalyptic, 0.9), m(DimSurvival, 0.8))

	// Sci-Fi, Space, Time Travel
	e.addKeyword("bilimkurgu", m(DimSciFi, 1.0), m(DimSpace, 0.5))
	e.addKeyword("bilim kurgu", m(DimSciFi, 1.0), m(DimSpace, 0.5))
	e.addKeyword("sci-fi", m(DimSciFi, 1.0), m(DimSpace, 0.5))
	e.addKeyword("scifi", m(DimSciFi, 1.0), m(DimSpace, 0.5))
	e.addKeyword("science fiction", m(DimSciFi, 1.0))
	e.addKeyword("uzay", m(DimSpace, 1.0), m(DimSciFi, 0.8))
	e.addKeyword("space", m(DimSpace, 1.0), m(DimSciFi, 0.8))
	e.addKeyword("zaman yolculuğu", m(DimTimeTravel, 1.0), m(DimSciFi, 0.8), m(DimMindBending, 0.7))
	e.addKeyword("time travel", m(DimTimeTravel, 1.0), m(DimSciFi, 0.8), m(DimMindBending, 0.7))
	e.addKeyword("mecha", m(DimMecha, 1.0), m(DimSciFi, 0.8), m(DimAction, 0.7))

	// Action, Adventure, Adrenaline
	e.addKeyword("aksiyon", m(DimAction, 1.0), m(DimFastPaced, 0.8))
	e.addKeyword("action", m(DimAction, 1.0), m(DimFastPaced, 0.8))
	e.addKeyword("adrenalin", m(DimFastPaced, 1.0), m(DimAction, 0.9))
	e.addKeyword("adrenaline", m(DimFastPaced, 1.0), m(DimAction, 0.9))
	e.addKeyword("macera", m(DimAdventure, 1.0), m(DimEpic, 0.6))
	e.addKeyword("adventure", m(DimAdventure, 1.0), m(DimEpic, 0.6))
	e.addKeyword("epik", m(DimEpic, 1.0), m(DimAction, 0.7), m(DimFantasy, 0.6))
	e.addKeyword("epic", m(DimEpic, 1.0), m(DimAction, 0.7), m(DimFantasy, 0.6))
	e.addKeyword("dövüş", m(DimMartialArts, 1.0), m(DimAction, 0.9))
	e.addKeyword("martial arts", m(DimMartialArts, 1.0), m(DimAction, 0.9))
	e.addKeyword("savaş", m(DimWar, 1.0), m(DimAction, 0.8), m(DimDrama, 0.6))
	e.addKeyword("war", m(DimWar, 1.0), m(DimAction, 0.8))

	// Mystery, Detective, Crime, Noir
	e.addKeyword("gizem", m(DimMystery, 1.0), m(DimSuspense, 0.8))
	e.addKeyword("mystery", m(DimMystery, 1.0), m(DimSuspense, 0.8))
	e.addKeyword("polisiye", m(DimDetective, 1.0), m(DimCrime, 0.9), m(DimMystery, 0.8))
	e.addKeyword("detective", m(DimDetective, 1.0), m(DimCrime, 0.9), m(DimMystery, 0.8))
	e.addKeyword("suç", m(DimCrime, 1.0), m(DimDrama, 0.6))
	e.addKeyword("crime", m(DimCrime, 1.0), m(DimDrama, 0.6))
	e.addKeyword("gerilim", m(DimThriller, 1.0), m(DimSuspense, 0.9))
	e.addKeyword("thriller", m(DimThriller, 1.0), m(DimSuspense, 0.9))
	e.addKeyword("suspense", m(DimSuspense, 1.0), m(DimMystery, 0.7))
	e.addKeyword("noir", m(DimNoir, 1.0), m(DimDark, 0.8), m(DimCrime, 0.8))
	e.addKeyword("soygun", m(DimHeist, 1.0), m(DimCrime, 0.8), m(DimFastPaced, 0.7))
	e.addKeyword("heist", m(DimHeist, 1.0), m(DimCrime, 0.8), m(DimFastPaced, 0.7))
	e.addKeyword("ajan", m(DimEspionage, 1.0), m(DimAction, 0.8), m(DimThriller, 0.7))
	e.addKeyword("espionage", m(DimEspionage, 1.0), m(DimThriller, 0.7))

	// Cozy, Wholesome, Comedy, Slice of Life
	e.addKeyword("kafa dağıtmalık", m(DimCozy, 1.0), m(DimComedy, 0.8), m(DimSliceOfLife, 0.7))
	e.addKeyword("huzurlu", m(DimWholesome, 1.0), m(DimCozy, 0.9), m(DimSliceOfLife, 0.8))
	e.addKeyword("cozy", m(DimCozy, 1.0), m(DimWholesome, 0.9), m(DimSliceOfLife, 0.8))
	e.addKeyword("wholesome", m(DimWholesome, 1.0), m(DimCozy, 0.8))
	e.addKeyword("komedi", m(DimComedy, 1.0), m(DimWholesome, 0.4))
	e.addKeyword("comedy", m(DimComedy, 1.0), m(DimWholesome, 0.4))
	e.addKeyword("eğlenceli", m(DimComedy, 0.8), m(DimWholesome, 0.7))
	e.addKeyword("funny", m(DimComedy, 0.9))
	e.addKeyword("chill", m(DimCozy, 1.0), m(DimSliceOfLife, 0.8))
	e.addKeyword("slice of life", m(DimSliceOfLife, 1.0), m(DimCozy, 0.8))
	e.addKeyword("romantik", m(DimRomance, 1.0), m(DimDrama, 0.5))
	e.addKeyword("romance", m(DimRomance, 1.0), m(DimDrama, 0.5))
	e.addKeyword("romcom", m(DimRomCom, 1.0), m(DimComedy, 0.8), m(DimRomance, 0.8))

	// Anime & Manga Subgenres
	e.addKeyword("anime", m(DimAnimation, 1.0))
	e.addKeyword("manga", m(DimAnimation, 0.8))
	e.addKeyword("manhwa", m(DimAnimation, 0.8), m(DimAction, 0.6))
	e.addKeyword("shonen", m(DimShonen, 1.0), m(DimAction, 0.8), m(DimAdventure, 0.7))
	e.addKeyword("şonen", m(DimShonen, 1.0), m(DimAction, 0.8), m(DimAdventure, 0.7))
	e.addKeyword("seinen", m(DimSeinen, 1.0), m(DimPsychological, 0.8), m(DimDark, 0.7))
	e.addKeyword("seynen", m(DimSeinen, 1.0), m(DimPsychological, 0.8), m(DimDark, 0.7))
	e.addKeyword("isekai", m(DimIsekai, 1.0), m(DimFantasy, 0.9), m(DimAdventure, 0.7))

	// Horror, Supernatural, Vampires
	e.addKeyword("korku", m(DimHorror, 1.0), m(DimDark, 0.8), m(DimSuspense, 0.7))
	e.addKeyword("horror", m(DimHorror, 1.0), m(DimDark, 0.8), m(DimSuspense, 0.7))
	e.addKeyword("doğaüstü", m(DimSupernatural, 1.0), m(DimFantasy, 0.7))
	e.addKeyword("supernatural", m(DimSupernatural, 1.0), m(DimFantasy, 0.7))
	e.addKeyword("paranormal", m(DimParanormal, 1.0), m(DimHorror, 0.6))
	e.addKeyword("zombi", m(DimZombies, 1.0), m(DimHorror, 0.8), m(DimSurvival, 0.8))
	e.addKeyword("zombie", m(DimZombies, 1.0), m(DimHorror, 0.8), m(DimSurvival, 0.8))
	e.addKeyword("vampir", m(DimVampires, 1.0), m(DimDark, 0.7), m(DimSupernatural, 0.7))
	e.addKeyword("vampire", m(DimVampires, 1.0), m(DimDark, 0.7), m(DimSupernatural, 0.7))

	// Retro & Nostalgia
	e.addKeyword("retro", m(DimRetro, 1.0))
	e.addKeyword("90s", m(DimRetro, 0.9), m(DimAtmospheric, 0.5))
	e.addKeyword("90'lar", m(DimRetro, 0.9), m(DimAtmospheric, 0.5))
	e.addKeyword("80s", m(DimRetro, 0.9))
	e.addKeyword("80'ler", m(DimRetro, 0.9))
	e.addKeyword("nostalji", m(DimRetro, 0.8), m(DimWholesome, 0.4))
	e.addKeyword("nostalgic", m(DimRetro, 0.8))
}

func (e *AIEngine) initMoodPresets() {
	e.moodPresets = []MoodPreset{
		{
			ID:          "mind_bending",
			Icon:        "🔮",
			TitleTR:     "Zihin Yakan & Ters Köşe",
			TitleEN:     "Mind-Bending & Plot Twists",
			DescTR:      "Kafanızı karıştıracak, tahmin edilemez sonlar ve psikolojik derinlik.",
			DescEN:      "Unpredictable twists, psychological depth, and reality-bending stories.",
			TargetWords: []string{"ters köşe", "plot twist", "beyin yakan", "psikolojik", "mind-bending", "gizem"},
		},
		{
			ID:          "dark_cyberpunk",
			Icon:        "⚡",
			TitleTR:     "Karanlık & Siberpunk",
			TitleEN:     "Dark & Cyberpunk Vibe",
			DescTR:      "Neon ışıklar altında kasvetli sokaklar, yüksek teknoloji ve distopya.",
			DescEN:      "Neon-soaked dystopias, high-tech gritty streets, and synthetic tones.",
			TargetWords: []string{"siberpunk", "cyberpunk", "distopik", "karanlık", "gritty", "bilim kurgu"},
		},
		{
			ID:          "cozy_wholesome",
			Icon:        "☕",
			TitleTR:     "Kafa Dağıtmalık & Huzurlu",
			TitleEN:     "Cozy, Chill & Wholesome",
			DescTR:      "Günün yorgunluğunu unutturan, iç ısıtan ve eğlenceli yapımlar.",
			DescEN:      "Warm, heartening, and feel-good stories to unwind and relax.",
			TargetWords: []string{"kafa dağıtmalık", "huzurlu", "cozy", "wholesome", "komedi", "chill"},
		},
		{
			ID:          "high_adrenaline",
			Icon:        "💥",
			TitleTR:     "Adrenalin & Epik Aksiyon",
			TitleEN:     "High Adrenaline & Epic Action",
			DescTR:      "Nefes kesen dövüş sahneleri, hızlı tempo ve büyük maceralar.",
			DescEN:      "Non-stop high-stakes combat, fast pacing, and epic journeys.",
			TargetWords: []string{"adrenalin", "aksiyon", "hızlı", "dövüş", "epik", "savaş"},
		},
		{
			ID:          "deep_mystery",
			Icon:        "🕵️",
			TitleTR:     "Derin Gizem & Polisiye",
			TitleEN:     "Deep Mystery & Noir Crime",
			DescTR:      "İpuçlarını birleştireceğiniz cinayetler, dedektifler ve karanlık sırlar.",
			DescEN:      "Investigative thrillers, gritty detective work, and atmospheric puzzles.",
			TargetWords: []string{"polisiye", "gizem", "dedektif", "suç", "noir", "gerilim"},
		},
		{
			ID:          "retro_classics",
			Icon:        "📼",
			TitleTR:     "Retro 80'ler & 90'lar",
			TitleEN:     "Retro 80s & 90s Classics",
			DescTR:      "Zamanın ötesinde klasiklerin nostaljik havası ve kült yapımlar.",
			DescEN:      "Timeless cult aesthetics, vintage vibes, and legendary storytelling.",
			TargetWords: []string{"retro", "90s", "80s", "nostalji", "kült", "klasik"},
		},
	}
}

// Vector Math
func (v Vector) Normalize() Vector {
	var sumSq float32
	for _, val := range v {
		sumSq += val * val
	}
	if sumSq == 0 {
		return v
	}
	norm := float32(math.Sqrt(float64(sumSq)))
	var res Vector
	for i, val := range v {
		res[i] = val / norm
	}
	return res
}

func (v Vector) CosineSimilarity(other Vector) float32 {
	var dot float32
	for i := 0; i < VectorDim; i++ {
		dot += v[i] * other[i]
	}
	return dot
}

func MatchPercent(sim float32) int {
	pct := int(((sim + 1.0) / 2.0) * 100)
	if pct < 15 {
		pct = 15
	}
	if pct > 99 {
		pct = 99
	}
	return pct
}

// EmbedText converts free-form text or metadata into a normalized semantic vector
func (e *AIEngine) EmbedText(text string) Vector {
	var v Vector
	lower := strings.ToLower(text)
	words := strings.FieldsFunc(lower, func(r rune) bool {
		return r == ' ' || r == ',' || r == '.' || r == '-' || r == ':' || r == '/' || r == ';' || r == '(' || r == ')'
	})

	// Check multi-word keys first
	for key, mappings := range e.lexicon {
		if strings.Contains(key, " ") && strings.Contains(lower, key) {
			for _, m := range mappings {
				v[m.dim] += m.weight * 1.5
			}
		}
	}

	// Check single words
	for _, word := range words {
		if mappings, ok := e.lexicon[word]; ok {
			for _, m := range mappings {
				v[m.dim] += m.weight
			}
		}
	}

	return v.Normalize()
}

// GetMoodPresets returns available discovery vibes
func (e *AIEngine) GetMoodPresets() []MoodPreset {
	return e.moodPresets
}

// BuildTasteProfile computes the user's live Taste DNA from SQLite progress & library
func (e *AIEngine) BuildTasteProfile(ctx context.Context, forceRefresh bool) (*TasteProfile, Vector, error) {
	e.cachedTasteMu.RLock()
	if !forceRefresh && e.cachedProfile != nil && time.Since(e.lastProfileAt) < 2*time.Minute {
		p := e.cachedProfile
		v := e.cachedVector
		e.cachedTasteMu.RUnlock()
		return p, v, nil
	}
	e.cachedTasteMu.RUnlock()

	var tasteVector Vector
	totalWeight := float32(0.0)
	affinityMap := make(map[string]int)
	itemsCount := 0

	var historyTitles []string

	// 1. Playback Progress
	if e.librarySvc != nil {
		if recents, err := e.librarySvc.ListRecentPlayback(ctx, 30); err == nil {
			for i, p := range recents {
				recencyWeight := float32(1.0) / float32(1.0+float64(i)*0.08)
				completionWeight := float32(0.6)
				if p.IsCompleted || p.ProgressPercent >= 80 {
					completionWeight = 1.0
				} else if p.ProgressPercent > 20 {
					completionWeight = p.ProgressPercent / 100.0
				}
				itemWeight := recencyWeight * completionWeight

				itemText := p.Title
				itemVec := e.EmbedText(itemText)
				for d := 0; d < VectorDim; d++ {
					tasteVector[d] += itemVec[d] * itemWeight
				}
				totalWeight += itemWeight
				itemsCount++
				if p.Title != "" {
					historyTitles = append(historyTitles, p.Title)
				}
			}
		}

		// 2. Reading Progress
		if recents, err := e.librarySvc.ListRecentReading(ctx, 30); err == nil {
			for i, r := range recents {
				recencyWeight := float32(1.0) / float32(1.0+float64(i)*0.08)
				itemWeight := recencyWeight * 0.95
				itemText := r.Title
				itemVec := e.EmbedText(itemText)
				for d := 0; d < VectorDim; d++ {
					tasteVector[d] += itemVec[d] * itemWeight
				}
				totalWeight += itemWeight
				itemsCount++
				if r.Title != "" {
					historyTitles = append(historyTitles, r.Title)
				}
			}
		}

		// 3. Library Items
		if libItems, _, err := e.librarySvc.ListItems(ctx, library.Filter{Limit: 20}); err == nil {
			for _, it := range libItems {
				itemWeight := float32(0.8)
				if it.Status == library.StatusCompleted || it.Status == library.StatusWatching || it.Status == library.StatusFavorite {
					itemWeight = 1.0
				}
				itemText := it.Title
				itemVec := e.EmbedText(itemText)
				for d := 0; d < VectorDim; d++ {
					tasteVector[d] += itemVec[d] * itemWeight
				}
				totalWeight += itemWeight
				itemsCount++
			}
		}
	}

	// Default baseline if user has fresh install
	if totalWeight == 0 {
		tasteVector[DimAction] = 0.8
		tasteVector[DimSciFi] = 0.7
		tasteVector[DimMystery] = 0.8
		tasteVector[DimPsychological] = 0.7
		tasteVector[DimPlotTwist] = 0.9
		tasteVector[DimMindBending] = 0.8
		tasteVector[DimDark] = 0.6
		tasteVector[DimComedy] = 0.5
	}

	normalizedTaste := tasteVector.Normalize()

	// Compute top traits
	type traitScore struct {
		nameTR string
		nameEN string
		score  float32
	}

	traits := []traitScore{
		{"Zihin Yakan & Ters Köşe", "Mind-Bending & Plot Twists", normalizedTaste[DimPlotTwist] + normalizedTaste[DimMindBending] + normalizedTaste[DimPsychological]},
		{"Karanlık Atmosfer & Siberpunk", "Dark Atmosphere & Cyberpunk", normalizedTaste[DimDark] + normalizedTaste[DimCyberpunk] + normalizedTaste[DimDystopian]},
		{"Derin Gizem & Polisiye", "Deep Mystery & Detective", normalizedTaste[DimMystery] + normalizedTaste[DimDetective] + normalizedTaste[DimNoir]},
		{"Epik Macera & Aksiyon", "Epic Adventure & Action", normalizedTaste[DimAction] + normalizedTaste[DimEpic] + normalizedTaste[DimAdventure]},
		{"Kafa Dağıtmalık & Huzurlu", "Cozy, Wholesome & Chill", normalizedTaste[DimWholesome] + normalizedTaste[DimCozy] + normalizedTaste[DimSliceOfLife]},
		{"Bilim Kurgu & Uzay", "Sci-Fi & Cosmic Space", normalizedTaste[DimSciFi] + normalizedTaste[DimSpace] + normalizedTaste[DimTimeTravel]},
		{"Korku & Gerilim", "Horror & Psychological Thriller", normalizedTaste[DimHorror] + normalizedTaste[DimThriller] + normalizedTaste[DimSuspense]},
		{"Sürükleyici Şonen / Seinen Anime", "Immersive Anime & Manga", normalizedTaste[DimShonen] + normalizedTaste[DimSeinen] + normalizedTaste[DimAnimation]},
	}

	sort.Slice(traits, func(i, j int) bool {
		return traits[i].score > traits[j].score
	})

	var traitsTR []string
	var traitsEN []string
	for i := 0; i < 3 && i < len(traits); i++ {
		traitsTR = append(traitsTR, traits[i].nameTR)
		traitsEN = append(traitsEN, traits[i].nameEN)
	}

	// Affinity percentages
	affinityMap["plot_twist"] = MatchPercent(normalizedTaste[DimPlotTwist])
	affinityMap["cyberpunk"] = MatchPercent(normalizedTaste[DimCyberpunk])
	affinityMap["mystery"] = MatchPercent(normalizedTaste[DimMystery])
	affinityMap["action"] = MatchPercent(normalizedTaste[DimAction])
	affinityMap["cozy"] = MatchPercent(normalizedTaste[DimCozy])
	affinityMap["scifi"] = MatchPercent(normalizedTaste[DimSciFi])

	profile := &TasteProfile{
		ActiveTraitsTR:  traitsTR,
		ActiveTraitsEN:  traitsEN,
		TopKeywords:     historyTitles,
		TotalItemsCount: itemsCount,
		TasteAffinity:   affinityMap,
		UpdatedAt:       time.Now().UTC(),
	}

	e.cachedTasteMu.Lock()
	e.cachedProfile = profile
	e.cachedVector = normalizedTaste
	e.lastProfileAt = time.Now()
	e.cachedTasteMu.Unlock()

	return profile, normalizedTaste, nil
}

// DiscoverByMood finds content matching a mood prompt or preset
func (e *AIEngine) DiscoverByMood(ctx context.Context, query string, moodKey string, limit int) ([]RecommendationItem, error) {
	if limit <= 0 {
		limit = 15
	}

	var targetVec Vector
	if moodKey != "" {
		for _, preset := range e.moodPresets {
			if preset.ID == moodKey {
				targetVec = e.EmbedText(strings.Join(preset.TargetWords, " "))
				break
			}
		}
	}

	if query != "" {
		qVec := e.EmbedText(query)
		for d := 0; d < VectorDim; d++ {
			targetVec[d] += qVec[d] * 1.5
		}
		targetVec = targetVec.Normalize()
	}

	// If neither query nor preset specified, use taste DNA
	if query == "" && moodKey == "" {
		_, tasteVec, _ := e.BuildTasteProfile(ctx, false)
		targetVec = tasteVec
	}

	return e.rankCatalogCandidates(ctx, targetVec, "", limit, query)
}

// GetTasteRecommendations returns personalized suggestions tailored to user's Taste DNA
func (e *AIEngine) GetTasteRecommendations(ctx context.Context, domain string, limit int) ([]RecommendationItem, error) {
	if limit <= 0 {
		limit = 12
	}

	_, tasteVec, err := e.BuildTasteProfile(ctx, false)
	if err != nil {
		return nil, err
	}

	return e.rankCatalogCandidates(ctx, tasteVec, domain, limit, "")
}

func (e *AIEngine) rankCatalogCandidates(ctx context.Context, targetVec Vector, domainFilter string, limit int, queryHint string) ([]RecommendationItem, error) {
	type candidate struct {
		item RecommendationItem
		sim  float32
	}

	var candidates []candidate
	var mu sync.Mutex
	var wg sync.WaitGroup

	// Fetch Cinema items
	if (domainFilter == "" || domainFilter == "cinema" || domainFilter == "all") && e.cinemaSvc != nil {
		wg.Add(1)
		go func() {
			defer wg.Done()
			searchTerms := []string{"popular", "top", "trending", "movie", "action", "mystery"}
			if queryHint != "" {
				searchTerms = append([]string{queryHint}, searchTerms...)
			}
			for _, term := range searchTerms[:3] {
				items, err := e.cinemaSvc.Search(ctx, term)
				if err != nil {
					continue
				}
				mu.Lock()
				for _, it := range items {
					cItem := e.toRecommendationFromCinema(&it)
					itemVec := e.EmbedText(cItem.Title + " " + cItem.Overview)
					sim := targetVec.CosineSimilarity(itemVec)
					cItem.MatchScore = MatchPercent(sim)
					cItem.ReasonTR = fmt.Sprintf("Zevk profiline göre %% %d atmosfer ve tema eşleşmesi.", cItem.MatchScore)
					cItem.ReasonEN = fmt.Sprintf("%d%% vibe match based on your taste profile.", cItem.MatchScore)
					candidates = append(candidates, candidate{item: cItem, sim: sim})
				}
				mu.Unlock()
				if len(items) > 0 {
					break
				}
			}
		}()
	}

	// Fetch Reading items
	if (domainFilter == "" || domainFilter == "reading" || domainFilter == "manga" || domainFilter == "all") && e.readingSvc != nil {
		wg.Add(1)
		go func() {
			defer wg.Done()
			items, err := e.readingSvc.Search(ctx, pluginv1.Domain_DOMAIN_MANGA, "popular")
			if err != nil {
				return
			}
			mu.Lock()
			for _, it := range items {
				rItem := e.toRecommendationFromReading(&it)
				itemVec := e.EmbedText(rItem.Title + " " + rItem.Overview)
				sim := targetVec.CosineSimilarity(itemVec)
				rItem.MatchScore = MatchPercent(sim)
				rItem.ReasonTR = fmt.Sprintf("Okuma geçmişine göre %% %d kurgusal uyum.", rItem.MatchScore)
				rItem.ReasonEN = fmt.Sprintf("%d%% match based on your reading history.", rItem.MatchScore)
				candidates = append(candidates, candidate{item: rItem, sim: sim})
			}
			mu.Unlock()
		}()
	}

	wg.Wait()

	// Deduplicate by ID
	seen := make(map[string]bool)
	var deduped []candidate
	for _, c := range candidates {
		if !seen[c.item.ID] && c.item.Title != "" {
			seen[c.item.ID] = true
			deduped = append(deduped, c)
		}
	}

	// Sort by highest cosine similarity
	sort.Slice(deduped, func(i, j int) bool {
		return deduped[i].sim > deduped[j].sim
	})

	var result []RecommendationItem
	for i := 0; i < len(deduped) && i < limit; i++ {
		result = append(result, deduped[i].item)
	}

	return result, nil
}

func (e *AIEngine) toRecommendationFromCinema(it *cinema.MediaItem) RecommendationItem {
	mType := "movie"
	if it.Type == cinema.MediaTypeSeries {
		mType = "series"
	} else if it.Type == cinema.MediaTypeAnime {
		mType = "anime"
	}

	return RecommendationItem{
		ID:         it.ID,
		ProviderID: it.ProviderID,
		Title:      it.Title,
		PosterURL:  it.PosterURL,
		Domain:     "cinema",
		Type:       mType,
		Year:       it.Year,
		Overview:   it.Overview,
	}
}

func (e *AIEngine) toRecommendationFromReading(it *reading.ReadingItem) RecommendationItem {
	mType := "manga"
	if it.Type == reading.ReadingTypeWebtoon {
		mType = "webtoon"
	} else if it.Type == reading.ReadingTypeBook {
		mType = "book"
	}

	return RecommendationItem{
		ID:         it.ID,
		ProviderID: it.ProviderID,
		Title:      it.Title,
		PosterURL:  it.PosterURL,
		Domain:     "reading",
		Type:       mType,
		Year:       it.Year,
		Overview:   it.Overview,
	}
}
