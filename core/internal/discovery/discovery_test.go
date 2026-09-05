package discovery

import (
	"context"
	"testing"
)

func TestVectorCosineSimilarity(t *testing.T) {
	engine := NewAIEngine(nil, nil, nil, nil)

	v1 := engine.EmbedText("karanlık siberpunk distopya")
	v2 := engine.EmbedText("dark cyberpunk city neon")
	v3 := engine.EmbedText("kafa dağıtmalık eğlenceli komedi cozy")

	sim12 := v1.CosineSimilarity(v2)
	sim13 := v1.CosineSimilarity(v3)

	if sim12 <= sim13 {
		t.Fatalf("Expected cyberpunk texts to have higher similarity (got %v) than comedy (got %v)", sim12, sim13)
	}

	pct12 := MatchPercent(sim12)
	if pct12 < 60 {
		t.Fatalf("Expected high match percent for similar vibes, got %d%%", pct12)
	}
}

func TestTasteProfileBaseline(t *testing.T) {
	engine := NewAIEngine(nil, nil, nil, nil)
	profile, vec, err := engine.BuildTasteProfile(context.Background(), true)
	if err != nil {
		t.Fatalf("Failed to build baseline taste profile: %v", err)
	}
	if len(profile.ActiveTraitsTR) == 0 {
		t.Fatalf("Expected active traits in Turkish, got none")
	}
	if len(profile.ActiveTraitsEN) == 0 {
		t.Fatalf("Expected active traits in English, got none")
	}
	if profile.TasteAffinity["plot_twist"] == 0 {
		t.Fatalf("Expected taste affinity scores, got 0")
	}

	var sumSq float32
	for _, val := range vec {
		sumSq += val * val
	}
	if sumSq < 0.99 || sumSq > 1.01 {
		t.Fatalf("Expected normalized vector length close to 1.0, got %v", sumSq)
	}
}

func TestMoodPresets(t *testing.T) {
	engine := NewAIEngine(nil, nil, nil, nil)
	presets := engine.GetMoodPresets()
	if len(presets) < 4 {
		t.Fatalf("Expected at least 4 mood presets, got %d", len(presets))
	}
	for _, p := range presets {
		if p.ID == "" || p.TitleTR == "" || p.TitleEN == "" {
			t.Fatalf("Invalid preset: %+v", p)
		}
	}
}
