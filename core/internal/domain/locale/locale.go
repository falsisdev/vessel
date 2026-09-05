package locale

import "strings"

const (
	DefaultLocale = "en"
	SystemLocale  = "system"
)

type LocaleInfo struct {
	Code       string
	Name       string
	NativeName string
	IsRTL      bool
}

var SupportedLocales = []LocaleInfo{
	{Code: "system", Name: "Cihaz Dili (System)", NativeName: "System", IsRTL: false},
	{Code: "en", Name: "English", NativeName: "English", IsRTL: false},
	{Code: "tr", Name: "Turkish", NativeName: "Türkçe", IsRTL: false},
	{Code: "de", Name: "German", NativeName: "Deutsch", IsRTL: false},
	{Code: "fr", Name: "French", NativeName: "Français", IsRTL: false},
	{Code: "es", Name: "Spanish", NativeName: "Español", IsRTL: false},
	{Code: "pt", Name: "Portuguese", NativeName: "Português", IsRTL: false},
	{Code: "ru", Name: "Russian", NativeName: "Русский", IsRTL: false},
	{Code: "ja", Name: "Japanese", NativeName: "日本語", IsRTL: false},
	{Code: "zh", Name: "Chinese", NativeName: "中文", IsRTL: false},
	{Code: "ar", Name: "Arabic", NativeName: "العربية", IsRTL: true},
	{Code: "fa", Name: "Persian", NativeName: "فارسی", IsRTL: true},
}

func ResolveLocale(code string) LocaleInfo {
	normalized := strings.ToLower(strings.TrimSpace(code))
	for _, l := range SupportedLocales {
		if strings.EqualFold(l.Code, normalized) {
			return l
		}
	}
	// Fallback to default (English)
	return SupportedLocales[1]
}
