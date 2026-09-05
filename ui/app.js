// Vessel Modern Reactive UI Controller
// Connects to Vessel Core Gateway & IPC

const I18N_STRINGS = {
  en: {
    nav_cinema: "Cinema & TV",
    nav_reading: "Manga & Novels",
    nav_library: "My Library",
    nav_settings: "Settings",
    section_continue: "Continue Watching & Reading",
    section_discover: "Discover Media",
    pill_all: "All",
    pill_movies: "Movies",
    pill_series: "Series",
    pill_anime: "Anime",
    library_title: "Your Collection",
    status_watching: "In Progress",
    status_plan: "Plan to Watch",
    status_completed: "Completed",
    status_favorites: "Favorites",
    settings_title: "Platform Settings",
    settings_theme_title: "Theme Engine",
    settings_theme_desc: "Choose from built-in curated palettes or install community CSS themes.",
    settings_debrid_title: "Torrent & Debrid Streaming",
    settings_debrid_desc: "Connect Real-Debrid or TorBox for high-speed cloud torrent streaming.",
    settings_plugins_title: "Connected Plugins",
    settings_plugins_desc: "Installed catalog and media provider processes.",
    btn_save: "Save",
    btn_play: "Play",
    btn_resume: "Resume",
    btn_add_library: "+ Add to Library",
    btn_in_library: "✓ In Library",
    no_results: "No media items found. Try another query.",
    episodes: "Episodes",
    streams_loading: "Resolving highest quality stream...",
  },
  tr: {
    nav_cinema: "Sinema & Dizi",
    nav_reading: "Manga & Roman",
    nav_library: "Kütüphanem",
    nav_settings: "Ayarlar",
    section_continue: "İzlemeye & Okumaya Devam Et",
    section_discover: "Medya Keşfet",
    pill_all: "Tümü",
    pill_movies: "Filmler",
    pill_series: "Diziler",
    pill_anime: "Animeler",
    library_title: "Koleksiyonunuz",
    status_watching: "Devam Edenler",
    status_plan: "İzlenecekler",
    status_completed: "Tamamlananlar",
    status_favorites: "Favoriler",
    settings_title: "Platform Ayarları",
    settings_theme_title: "Tema Motoru",
    settings_theme_desc: "Dahili paletlerden seçin veya topluluk CSS temalarını uygulayın.",
    settings_debrid_title: "Torrent & Debrid Akış Motoru",
    settings_debrid_desc: "Yüksek hızlı bulut torrent akışı için Real-Debrid veya TorBox bağlayın.",
    settings_plugins_title: "Bağlı Eklentiler",
    settings_plugins_desc: "Yüklü katalog ve medya sağlayıcı süreçleri.",
    btn_save: "Kaydet",
    btn_play: "Oynat",
    btn_resume: "Devam Et",
    btn_add_library: "+ Kütüphaneye Ekle",
    btn_in_library: "✓ Kütüphanede",
    no_results: "Sonuç bulunamadı. Farklı bir arama deneyin.",
    episodes: "Bölümler",
    streams_loading: "En yüksek kaliteli akış çözümleniyor...",
  },
  de: {
    nav_cinema: "Kino & Serien",
    nav_reading: "Manga & Romane",
    nav_library: "Meine Bibliothek",
    nav_settings: "Einstellungen",
    section_continue: "Weiterschauen & Lesen",
    section_discover: "Medien entdecken",
    pill_all: "Alle",
    pill_movies: "Filme",
    pill_series: "Serien",
    pill_anime: "Anime",
    library_title: "Ihre Sammlung",
    status_watching: "Wird geschaut",
    status_plan: "Geplant",
    status_completed: "Abgeschlossen",
    status_favorites: "Favoriten",
    settings_title: "Plattformeinstellungen",
    settings_theme_title: "Theme-Engine",
    settings_theme_desc: "Wählen Sie vorgefertigte Paletten oder Community-Themes.",
    settings_debrid_title: "Torrent & Debrid-Streaming",
    settings_debrid_desc: "Real-Debrid oder TorBox für Highspeed-Streaming verbinden.",
    settings_plugins_title: "Verbundene Plugins",
    settings_plugins_desc: "Installierte Katalog- und Medienprozesse.",
    btn_save: "Speichern",
    btn_play: "Abspielen",
    btn_resume: "Fortsetzen",
    btn_add_library: "+ Zur Bibliothek",
    btn_in_library: "✓ In Bibliothek",
    no_results: "Keine Medien gefunden.",
    episodes: "Episoden",
    streams_loading: "Stream wird aufgelöst...",
  },
  fr: {
    nav_cinema: "Cinéma & Séries",
    nav_reading: "Manga & Romans",
    nav_library: "Ma Bibliothèque",
    nav_settings: "Paramètres",
    section_continue: "Reprendre la lecture",
    section_discover: "Découvrir",
    pill_all: "Tous",
    pill_movies: "Films",
    pill_series: "Séries",
    pill_anime: "Animés",
    library_title: "Votre Collection",
    status_watching: "En cours",
    status_plan: "À voir",
    status_completed: "Terminé",
    status_favorites: "Favoris",
    settings_title: "Paramètres du système",
    settings_theme_title: "Moteur de thèmes",
    settings_theme_desc: "Choisissez parmi les palettes intégrées ou installez des thèmes.",
    settings_debrid_title: "Streaming Torrent & Debrid",
    settings_debrid_desc: "Connectez Real-Debrid ou TorBox pour le streaming haute vitesse.",
    settings_plugins_title: "Extensions connectées",
    settings_plugins_desc: "Processus de catalogue et fournisseurs installés.",
    btn_save: "Enregistrer",
    btn_play: "Lire",
    btn_resume: "Reprendre",
    btn_add_library: "+ Ajouter",
    btn_in_library: "✓ Dans la biblio",
    no_results: "Aucun résultat trouvé.",
    episodes: "Épisodes",
    streams_loading: "Résolution du flux en cours...",
  },
  es: {
    nav_cinema: "Cine y Series",
    nav_reading: "Manga y Novelas",
    nav_library: "Mi Biblioteca",
    nav_settings: "Ajustes",
    section_continue: "Continuar viendo y leyendo",
    section_discover: "Descubrir",
    pill_all: "Todo",
    pill_movies: "Películas",
    pill_series: "Series",
    pill_anime: "Anime",
    library_title: "Tu Colección",
    status_watching: "Viendo",
    status_plan: "Pendiente",
    status_completed: "Completado",
    status_favorites: "Favoritos",
    settings_title: "Ajustes de la plataforma",
    settings_theme_title: "Motor de temas",
    settings_theme_desc: "Elige temas integrados o temas de la comunidad.",
    settings_debrid_title: "Streaming Torrent y Debrid",
    settings_debrid_desc: "Conecta Real-Debrid o TorBox para streaming en la nube.",
    settings_plugins_title: "Plugins conectados",
    settings_plugins_desc: "Procesos de catálogo instalados.",
    btn_save: "Guardar",
    btn_play: "Reproducir",
    btn_resume: "Reanudar",
    btn_add_library: "+ Añadir",
    btn_in_library: "✓ En biblioteca",
    no_results: "No se encontraron medios.",
    episodes: "Episodios",
    streams_loading: "Cargando stream...",
  },
  pt: {
    nav_cinema: "Cinema e Séries",
    nav_reading: "Mangás e Romances",
    nav_library: "Minha Biblioteca",
    nav_settings: "Configurações",
    section_continue: "Continuar Assistindo e Lendo",
    section_discover: "Descobrir",
    pill_all: "Tudo",
    pill_movies: "Filmes",
    pill_series: "Séries",
    pill_anime: "Anime",
    library_title: "Sua Coleção",
    status_watching: "Assistindo",
    status_plan: "Quero Assistir",
    status_completed: "Concluído",
    status_favorites: "Favoritos",
    settings_title: "Configurações",
    settings_theme_title: "Motor de temas",
    settings_theme_desc: "Escolha temas internos ou da comunidade.",
    settings_debrid_title: "Streaming Torrent e Debrid",
    settings_debrid_desc: "Conecte Real-Debrid ou TorBox.",
    settings_plugins_title: "Plugins Conectados",
    settings_plugins_desc: "Catálogos e provedores ativos.",
    btn_save: "Salvar",
    btn_play: "Reproduzir",
    btn_resume: "Continuar",
    btn_add_library: "+ Adicionar",
    btn_in_library: "✓ Na Biblioteca",
    no_results: "Nenhum resultado encontrado.",
    episodes: "Episódios",
    streams_loading: "Carregando transmissão...",
  },
  ru: {
    nav_cinema: "Кино и Сериалы",
    nav_reading: "Манга и Новеллы",
    nav_library: "Моя Библиотека",
    nav_settings: "Настройки",
    section_continue: "Продолжить просмотр и чтение",
    section_discover: "Обзор",
    pill_all: "Все",
    pill_movies: "Фильмы",
    pill_series: "Сериалы",
    pill_anime: "Аниме",
    library_title: "Ваша коллекция",
    status_watching: "Смотрю",
    status_plan: "В планах",
    status_completed: "Завершено",
    status_favorites: "Избранное",
    settings_title: "Настройки платформы",
    settings_theme_title: "Темы оформления",
    settings_theme_desc: "Встроенные темы и поддержка пользовательских стилей.",
    settings_debrid_title: "Торрент и Debrid стриминг",
    settings_debrid_desc: "Подключите Real-Debrid или TorBox.",
    settings_plugins_title: "Подключенные плагины",
    settings_plugins_desc: "Установленные каталоги и провайдеры.",
    btn_save: "Сохранить",
    btn_play: "Смотреть",
    btn_resume: "Продолжить",
    btn_add_library: "+ В библиотеку",
    btn_in_library: "✓ В библиотеке",
    no_results: "Ничего не найдено.",
    episodes: "Эпизоды",
    streams_loading: "Поиск источника...",
  },
  ja: {
    nav_cinema: "映画 & ドラマ",
    nav_reading: "マンガ & ノベル",
    nav_library: "マイライブラリ",
    nav_settings: "設定",
    section_continue: "続きから再生・読書",
    section_discover: "メディアを探す",
    pill_all: "すべて",
    pill_movies: "映画",
    pill_series: "ドラマ",
    pill_anime: "アニメ",
    library_title: "コレクション",
    status_watching: "視聴中",
    status_plan: "見たい",
    status_completed: "完了",
    status_favorites: "お気に入り",
    settings_title: "プラットフォーム設定",
    settings_theme_title: "テーマエンジン",
    settings_theme_desc: "組み込みテーマやカスタムCSSを適用します。",
    settings_debrid_title: "Torrent & Debrid ストリーミング",
    settings_debrid_desc: "Real-Debrid または TorBox を接続します。",
    settings_plugins_title: "接続済みプラグイン",
    settings_plugins_desc: "インストールされているメディアプロバイダー。",
    btn_save: "保存",
    btn_play: "再生",
    btn_resume: "再開",
    btn_add_library: "+ 追加",
    btn_in_library: "✓ 登録済み",
    no_results: "見つかりませんでした。",
    episodes: "エピソード",
    streams_loading: "ストリームを解決中...",
  },
  zh: {
    nav_cinema: "电影与剧集",
    nav_reading: "漫画与小说",
    nav_library: "我的媒体库",
    nav_settings: "设置",
    section_continue: "继续观看与阅读",
    section_discover: "探索媒体",
    pill_all: "全部",
    pill_movies: "电影",
    pill_series: "电视剧",
    pill_anime: "动漫",
    library_title: "你的收藏",
    status_watching: "正在观看",
    status_plan: "想看",
    status_completed: "已看完",
    status_favorites: "特别喜欢",
    settings_title: "平台设置",
    settings_theme_title: "主题引擎",
    settings_theme_desc: "挑选内置主题或载入社区 CSS 主题。",
    settings_debrid_title: "Torrent 与 Debrid 云播",
    settings_debrid_desc: "绑定 Real-Debrid 或 TorBox 高速播放。",
    settings_plugins_title: "已连接插件",
    settings_plugins_desc: "已加载的媒体数据源。",
    btn_save: "保存",
    btn_play: "播放",
    btn_resume: "继续",
    btn_add_library: "+ 收藏",
    btn_in_library: "✓ 已收藏",
    no_results: "未找到内容。",
    episodes: "剧集",
    streams_loading: "正在解析高清源...",
  },
  ar: {
    nav_cinema: "السينما والمسلسلات",
    nav_reading: "المانغا والروايات",
    nav_library: "مكتبتي",
    nav_settings: "الإعدادات",
    section_continue: "متابعة المشاهدة والقراءة",
    section_discover: "استكشاف الوسائط",
    pill_all: "الكل",
    pill_movies: "أفلام",
    pill_series: "مسلسلات",
    pill_anime: "أنمي",
    library_title: "مجموعتك",
    status_watching: "قيد المشاهدة",
    status_plan: "في خطة المشاهدة",
    status_completed: "مكتمل",
    status_favorites: "المفضلة",
    settings_title: "إعدادات المنصة",
    settings_theme_title: "محرك السمات",
    settings_theme_desc: "اختر من اللوحات المدمجة أو استخدم سمات مخصصة.",
    settings_debrid_title: "بث تورنت و Debrid",
    settings_debrid_desc: "اربط حساب Real-Debrid أو TorBox للبث السريع.",
    settings_plugins_title: "الإضافات المتصلة",
    settings_plugins_desc: "عمليات التزويد والكتالوج المثبتة.",
    btn_save: "حفظ",
    btn_play: "تشغيل",
    btn_resume: "استئناف",
    btn_add_library: "+ إلى المكتبة",
    btn_in_library: "✓ في المكتبة",
    no_results: "لم يتم العثور على نتائج.",
    episodes: "الحلقات",
    streams_loading: "جارٍ فك تشفير البث...",
  },
  fa: {
    nav_cinema: "سینما و سریال",
    nav_reading: "مانگا و رمان",
    nav_library: "کتابخانه من",
    nav_settings: "تنظیمات",
    section_continue: "ادامه تماشا و خواندن",
    section_discover: "کاوش رسانه",
    pill_all: "همه",
    pill_movies: "فیلم‌ها",
    pill_series: "سریال‌ها",
    pill_anime: "انیمه",
    library_title: "مجموعه شما",
    status_watching: "در حال تماشا",
    status_plan: "برنامه تماشا",
    status_completed: "تکمیل شده",
    status_favorites: "علاقه‌مندی‌ها",
    settings_title: "تنظیمات پلتفرم",
    settings_theme_title: "موتور تم",
    settings_theme_desc: "از پوسته‌های پیش‌فرض انتخاب کنید یا تم CSS دلخواه اعمال کنید.",
    settings_debrid_title: "استریم تورنت و دبریس",
    settings_debrid_desc: "حساب Real-Debrid یا TorBox را برای پخش پرسرعت متصل کنید.",
    settings_plugins_title: "پلاگین‌های متصل",
    settings_plugins_desc: "سرویس‌های کاتالوگ و رسانه فعال.",
    btn_save: "ذخیره",
    btn_play: "پخش",
    btn_resume: "ادامه",
    btn_add_library: "+ به کتابخانه",
    btn_in_library: "✓ در کتابخانه",
    no_results: "موردی پیدا نشد.",
    episodes: "قسمت‌ها",
    streams_loading: "در حال دریافت آدرس استریم...",
  },
};

class VesselApp {
  constructor() {
    this.currentRoute = "cinema";
    this.currentDomain = "cinema";
    this.currentFilter = "all";
    this.currentLocale = "system";
    this.activeTheme = null;
    this.searchDebounceTimer = null;
    this.currentPlaybackMedia = null;
    this.progressSyncInterval = null;

    this.init();
  }

  async init() {
    this.bindEvents();
    await this.loadLocalePreference();
    await this.checkCoreStatus();
    await this.loadActiveTheme();
    await this.loadResumeProgress();
    await this.loadInitialMedia();
    await this.loadDebridStatus();
  }

  bindEvents() {
    // Navigation routing
    document.querySelectorAll(".nav-item").forEach(btn => {
      btn.addEventListener("click", () => {
        const route = btn.dataset.route;
        this.switchRoute(route);
      });
    });

    // Domain / Filter pills
    document.querySelectorAll("#domain-pills .pill").forEach(pill => {
      pill.addEventListener("click", () => {
        document.querySelectorAll("#domain-pills .pill").forEach(p => p.classList.remove("active"));
        pill.classList.add("active");
        this.currentFilter = pill.dataset.filter;
        this.filterAndRenderMedia();
      });
    });

    // Search input
    const searchInput = document.getElementById("search-input");
    const clearBtn = document.getElementById("clear-search");

    searchInput.addEventListener("input", (e) => {
      const q = e.target.value.trim();
      clearBtn.classList.toggle("hidden", q === "");

      clearTimeout(this.searchDebounceTimer);
      this.searchDebounceTimer = setTimeout(() => {
        this.performSearch(q);
      }, 350);
    });

    clearBtn.addEventListener("click", () => {
      searchInput.value = "";
      clearBtn.classList.add("hidden");
      this.loadInitialMedia();
    });

    // Locale select
    document.getElementById("locale-select").addEventListener("change", (e) => {
      this.setLocale(e.target.value);
    });

    // Theme toggle button (switches between dark / light / midnight)
    document.getElementById("quick-theme-toggle").addEventListener("click", () => {
      this.cycleTheme();
    });

    // Modal close
    document.getElementById("modal-close-btn").addEventListener("click", () => {
      this.closeModal();
    });

    // Save Debrid buttons
    document.getElementById("save-rd-btn").addEventListener("click", () => {
      this.saveDebrid("realdebrid", document.getElementById("rd-api-key").value);
    });

    document.getElementById("save-tb-btn").addEventListener("click", () => {
      this.saveDebrid("torbox", document.getElementById("tb-api-key").value);
    });

    // Escape key closes modal
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") this.closeModal();
    });
  }

  // --- Localization (i18n) Engine ---
  async loadLocalePreference() {
    const saved = localStorage.getItem("vessel_locale") || "system";
    this.currentLocale = saved;
    document.getElementById("locale-select").value = saved;
    this.applyLocale(saved);
  }

  setLocale(code) {
    this.currentLocale = code;
    localStorage.setItem("vessel_locale", code);
    this.applyLocale(code);
  }

  applyLocale(code) {
    let target = code;
    if (code === "system") {
      const navLang = (navigator.language || "en").substring(0, 2).toLowerCase();
      target = I18N_STRINGS[navLang] ? navLang : "en";
    }

    const dict = I18N_STRINGS[target] || I18N_STRINGS.en;
    const isRTL = target === "ar" || target === "fa";

    document.documentElement.setAttribute("dir", isRTL ? "rtl" : "ltr");
    document.documentElement.setAttribute("lang", target);

    document.querySelectorAll("[data-i18n]").forEach(elem => {
      const key = elem.dataset.i18n;
      if (dict[key]) {
        elem.textContent = dict[key];
      }
    });

    const searchInput = document.getElementById("search-input");
    if (searchInput) {
      searchInput.placeholder = target === "tr"
        ? "Film, dizi, manga veya anime ara..."
        : "Search movies, series, manga, anime...";
    }
  }

  t(key) {
    let target = this.currentLocale;
    if (target === "system") {
      const navLang = (navigator.language || "en").substring(0, 2).toLowerCase();
      target = I18N_STRINGS[navLang] ? navLang : "en";
    }
    const dict = I18N_STRINGS[target] || I18N_STRINGS.en;
    return dict[key] || key;
  }

  // ---  Theme Engine ---
  async loadActiveTheme() {
    try {
      const res = await fetch("/api/theme/active");
      if (!res.ok) return;
      const data = await res.json();
      this.activeTheme = data;

      if (data.compiled_css) {
        document.getElementById("vessel-theme-vars").innerHTML = data.compiled_css;
      }
    } catch (e) {
      console.warn("Could not load theme:", e);
    }
    await this.renderThemeSelector();
  }

  async renderThemeSelector() {
    try {
      const res = await fetch("/api/themes");
      if (!res.ok) return;
      const data = await res.json();
      const container = document.getElementById("theme-options");
      if (!container) return;

      container.innerHTML = "";
      (data.themes || []).forEach(t => {
        const item = document.createElement("div");
        item.className = `theme-item ${this.activeTheme && this.activeTheme.theme && this.activeTheme.theme.id === t.id ? "active" : ""}`;

        item.innerHTML = `
          <div class="theme-palette">
            <span class="theme-color-swatch" style="background-color: ${this.getThemeColor(t.id, 0)}"></span>
            <span class="theme-color-swatch" style="background-color: ${this.getThemeColor(t.id, 1)}"></span>
            <span class="theme-color-swatch" style="background-color: ${this.getThemeColor(t.id, 2)}"></span>
          </div>
          <div class="theme-name">${t.name}</div>
        `;

        item.addEventListener("click", () => {
          this.switchTheme(t.id, t.active_variant || "dark");
        });

        container.appendChild(item);
      });
    } catch (e) {
      console.warn("Failed to render theme selector:", e);
    }
  }

  getThemeColor(id, idx) {
    const map = {
      "vessel-dark": ["#0f1117", "#1f2430", "#6366f1"],
      "vessel-light": ["#f8fafc", "#e2e8f0", "#4f46e5"],
      "midnight-oled": ["#000000", "#111111", "#a855f7"],
      "catppuccin": ["#1e1e2e", "#313244", "#cba6f7"],
      "nord": ["#2e3440", "#3b4252", "#88c0d0"],
      "dracula": ["#282a36", "#44475a", "#bd93f9"],
      "mangile-amber": ["#140f07", "#2b1e0f", "#f59e0b"],
    };
    return (map[id] && map[id][idx]) || "#4f46e5";
  }

  async switchTheme(themeID, variantID) {
    try {
      const res = await fetch("/api/theme/active", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ theme_id: themeID, variant_id: variantID })
      });
      if (res.ok) {
        await this.loadActiveTheme();
        this.showToast(`Theme switched to ${themeID}`);
      }
    } catch (e) {
      this.showToast("Failed to switch theme", "error");
    }
  }

  async cycleTheme() {
    const list = ["vessel-dark", "vessel-light", "midnight-oled", "dracula", "nord"];
    const current = this.activeTheme?.theme?.id || "vessel-dark";
    const nextIdx = (list.indexOf(current) + 1) % list.length;
    await this.switchTheme(list[nextIdx], "dark");
  }

  // --- Routing & Views ---
  switchRoute(route) {
    this.currentRoute = route;
    document.querySelectorAll(".nav-item").forEach(btn => {
      btn.classList.toggle("active", btn.dataset.route === route);
    });

    const mainView = document.getElementById("main-view");
    const resumeSection = document.getElementById("resume-section");
    const libraryView = document.getElementById("library-view");
    const settingsView = document.getElementById("settings-view");

    mainView.classList.add("hidden");
    libraryView.classList.add("hidden");
    settingsView.classList.add("hidden");

    if (route === "cinema") {
      this.currentDomain = "cinema";
      mainView.classList.remove("hidden");
      resumeSection.classList.remove("hidden");
      document.getElementById("view-title").textContent = this.t("section_discover");
      this.loadInitialMedia();
    } else if (route === "reading") {
      this.currentDomain = "reading";
      mainView.classList.remove("hidden");
      resumeSection.classList.remove("hidden");
      document.getElementById("view-title").textContent = this.t("nav_reading");
      this.loadInitialMedia();
    } else if (route === "library") {
      libraryView.classList.remove("hidden");
      resumeSection.classList.remove("hidden");
      this.loadLibraryItems("WATCHING");
    } else if (route === "settings") {
      settingsView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.loadDebridStatus();
      this.loadPlugins();
    }
  }

  // --- Media Catalog & Search ---
  async loadInitialMedia() {
    const query = this.currentDomain === "cinema" ? "Batman" : "Solo";
    await this.performSearch(query);
  }

  async performSearch(query) {
    if (!query) return;
    const grid = document.getElementById("media-grid");
    grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">Loading...</div>`;

    try {
      const domainNum = this.currentDomain === "cinema" ? 1 : 2;
      const res = await fetch(`/api/search?domain=${domainNum}&query=${encodeURIComponent(query)}`);
      if (!res.ok) throw new Error("Search failed");
      const data = await res.json();

      this.rawMediaItems = data.items || [];
      this.filterAndRenderMedia();
    } catch (e) {
      grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">${this.t("no_results")}</div>`;
    }
  }

  filterAndRenderMedia() {
    const grid = document.getElementById("media-grid");
    grid.innerHTML = "";

    let items = this.rawMediaItems || [];
    if (this.currentFilter !== "all") {
      items = items.filter(it => {
        const typeStr = this.mapMediaType(it.type).toLowerCase();
        return typeStr.includes(this.currentFilter);
      });
    }

    if (items.length === 0) {
      grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">${this.t("no_results")}</div>`;
      return;
    }

    items.forEach(item => {
      const card = document.createElement("div");
      card.className = "media-card";
      const poster = item.poster_url || "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400";
      const typeLabel = this.mapMediaType(item.type);

      card.innerHTML = `
        <div class="poster-wrapper">
          <img src="${poster}" alt="${item.title}" class="poster-img" loading="lazy" onerror="this.src='https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400'">
          <span class="card-badge">${typeLabel}</span>
        </div>
        <div class="card-details">
          <div class="card-title" title="${item.title}">${item.title}</div>
          <div class="card-meta">
            <span>${item.year || ""}</span>
            <span style="color: var(--v-accent-secondary)">★ 8.5</span>
          </div>
        </div>
      `;

      card.addEventListener("click", () => {
        this.openMediaModal(item);
      });

      grid.appendChild(card);
    });
  }

  mapMediaType(type) {
    const map = {
      1: "Movie",
      2: "Series",
      3: "Anime",
      4: "Manga",
      5: "Webtoon",
      6: "Webook",
      7: "Book",
    };
    return map[type] || "Media";
  }

  // --- Continue Watching / Reading Resume Progress ---
  async loadResumeProgress() {
    try {
      const res = await fetch("/api/progress/playback/recent?limit=5");
      if (!res.ok) return;
      const data = await res.json();
      const container = document.getElementById("resume-cards");
      if (!container) return;

      container.innerHTML = "";
      const items = data.items || [];
      if (items.length === 0) {
        document.getElementById("resume-section").classList.add("hidden");
        return;
      }
      document.getElementById("resume-section").classList.remove("hidden");

      items.forEach(p => {
        const card = document.createElement("div");
        card.className = "resume-card";
        const percent = Math.min(100, Math.round(p.progress_percent || 0));

        card.innerHTML = `
          <div class="resume-info">
            <div class="resume-title">${p.media_id}</div>
            <div class="resume-sub">S${p.season_number} E${p.episode_number} • ${percent}%</div>
            <div class="progress-bar-container">
              <div class="progress-bar-fill" style="width: ${percent}%;"></div>
            </div>
          </div>
        `;

        card.addEventListener("click", () => {
          this.openMediaModal({
            id: p.media_id,
            provider_id: p.provider_id,
            title: p.media_id,
            type: 2
          });
        });

        container.appendChild(card);
      });
    } catch (e) {
      console.warn("Failed to load resume progress:", e);
    }
  }

  // --- Media Modal & Video Player ---
  async openMediaModal(item) {
    const modal = document.getElementById("media-modal");
    const content = document.getElementById("modal-content");
    modal.classList.remove("hidden");
    content.innerHTML = `<div style="text-align: center; padding: 60px; color: var(--v-text-muted);">Loading details...</div>`;

    try {
      const domainNum = this.currentDomain === "cinema" ? 1 : 2;
      const res = await fetch(`/api/media?domain=${domainNum}&provider=${encodeURIComponent(item.provider_id || "")}&id=${encodeURIComponent(item.id)}`);
      const details = res.ok ? (await res.json()) : item;

      content.innerHTML = `
        <div class="player-container" id="player-mount">
          <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 16px;">
            <button class="btn btn-primary" id="start-stream-btn" style="padding: 12px 28px; font-size: 1.05rem;">
              ▶ ${this.t("btn_play")}
            </button>
            <span id="stream-status" style="font-size: 0.85rem; color: var(--v-text-muted);">Ready to stream</span>
          </div>
        </div>

        <div class="modal-details">
          <div class="modal-title-row">
            <h2>${details.title || item.title}</h2>
            <button class="btn btn-secondary" id="lib-toggle-btn">${this.t("btn_add_library")}</button>
          </div>
          <div class="modal-overview">${details.overview || "No synopsis available."}</div>

          ${details.seasons && details.seasons.length > 0 ? `
            <h3 style="margin-top: 20px; font-size: 1.1rem;">${this.t("episodes")}</h3>
            <div class="episode-list">
              ${details.seasons[0].episodes.map(ep => `
                <div class="episode-item" data-ep="${ep.episode_number}">
                  <span>Episode ${ep.episode_number}: ${ep.title}</span>
                  <button class="btn btn-secondary" style="padding: 4px 10px; font-size: 0.75rem;">Play</button>
                </div>
              `).join("")}
            </div>
          ` : ""}
        </div>
      `;

      // Start stream handler
      document.getElementById("start-stream-btn").addEventListener("click", () => {
        this.resolveAndPlayStream(item, 1, 1);
      });

      // Episode click handlers
      content.querySelectorAll(".episode-item").forEach(el => {
        el.addEventListener("click", () => {
          const epNum = parseInt(el.dataset.ep, 10);
          this.resolveAndPlayStream(item, 1, epNum);
        });
      });

      // Library toggle
      document.getElementById("lib-toggle-btn").addEventListener("click", () => {
        this.toggleLibraryItem(item);
      });

    } catch (e) {
      content.innerHTML = `<div style="text-align: center; padding: 40px; color: var(--v-status-error);">Failed to load metadata.</div>`;
    }
  }

  async resolveAndPlayStream(item, season, episode) {
    const statusText = document.getElementById("stream-status");
    if (statusText) statusText.textContent = this.t("streams_loading");

    try {
      // 1. Fetch available stream sources from provider
      const stRes = await fetch(`/api/streams?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&season=${season}&episode=${episode}`);
      let streamUrl = "";
      if (stRes.ok) {
        const stData = await stRes.json();
        if (stData.streams && stData.streams.length > 0) {
          streamUrl = stData.streams[0].url;
        }
      }

      // Fallback to sample or magnet
      if (!streamUrl) {
        streamUrl = "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=" + encodeURIComponent(item.title);
      }

      // 2. Resolve stream via Debrid / Local Proxy
      const resolveRes = await fetch("/api/stream/resolve", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          stream_url: streamUrl,
          title: item.title,
          season_number: season,
          episode_number: episode
        })
      });

      if (!resolveRes.ok) throw new Error("Stream resolution failed");
      const resolved = await resolveRes.json();
      const playbackUrl = resolved.stream.playback_url;

      // Mount native HTML5 video player
      const mount = document.getElementById("player-mount");
      mount.innerHTML = `
        <video id="vessel-video" controls autoplay playsinline style="width: 100%; height: 100%;">
          <source src="${playbackUrl}" type="video/mp4">
          Your browser does not support video playback.
        </video>
      `;

      const video = document.getElementById("vessel-video");
      this.trackPlaybackProgress(video, item, season, episode);

      this.showToast(`Playing via ${resolved.stream.provider} (${resolved.stream.quality || "HD"})`);
    } catch (e) {
      if (statusText) statusText.textContent = "Playback failed: " + e.message;
      this.showToast("Playback failed", "error");
    }
  }

  trackPlaybackProgress(video, item, season, episode) {
    clearInterval(this.progressSyncInterval);
    this.progressSyncInterval = setInterval(async () => {
      if (!video || video.paused) return;

      const current = video.currentTime;
      const duration = video.duration;
      if (!duration || duration <= 0) return;

      const percent = (current / duration) * 100;
      try {
        await fetch("/api/progress/playback", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            provider_id: item.provider_id || "cinema",
            media_id: item.id,
            domain: 1,
            season_number: season,
            episode_number: episode,
            current_position: current,
            total_duration: duration,
            progress_percent: percent,
            is_completed: percent >= 90
          })
        });
      } catch (e) {
        // silent progress sync failure
      }
    }, 5000);
  }

  closeModal() {
    clearInterval(this.progressSyncInterval);
    const modal = document.getElementById("media-modal");
    modal.classList.add("hidden");
    const mount = document.getElementById("player-mount");
    if (mount) mount.innerHTML = "";
    this.loadResumeProgress();
  }

  // --- Debrid API Configuration ---
  async loadDebridStatus() {
    try {
      const res = await fetch("/api/debrid/status");
      if (!res.ok) return;
      const data = await res.json();
      (data.accounts || []).forEach(acc => {
        const badge = document.getElementById(acc.provider === "realdebrid" ? "rd-status-badge" : "tb-status-badge");
        if (!badge) return;

        if (acc.is_premium) {
          badge.className = "account-badge premium";
          badge.textContent = `Active Premium (${acc.username || acc.email})`;
        } else if (acc.status === "disabled") {
          badge.className = "account-badge";
          badge.textContent = "Disabled";
        } else {
          badge.className = "account-badge";
          badge.textContent = acc.status;
        }
      });
    } catch (e) {
      console.warn("Failed to fetch debrid status:", e);
    }
  }

  async saveDebrid(provider, apiKey) {
    try {
      const res = await fetch("/api/debrid/configure", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider: provider,
          api_key: apiKey.trim(),
          enabled: apiKey.trim() !== ""
        })
      });

      const data = await res.json();
      if (data.success) {
        this.showToast(`${provider} credentials verified & saved!`);
        this.loadDebridStatus();
      } else {
        this.showToast(data.message || "Failed to authenticate", "error");
      }
    } catch (e) {
      this.showToast("Failed to connect to core", "error");
    }
  }

  // --- Plugins List ---
  async loadPlugins() {
    try {
      const res = await fetch("/api/plugins");
      if (!res.ok) return;
      const data = await res.json();
      const container = document.getElementById("plugin-list");
      if (!container) return;

      container.innerHTML = "";
      (data.plugins || []).forEach(p => {
        const row = document.createElement("div");
        row.style.cssText = "display: flex; justify-content: space-between; padding: 10px; border-bottom: 1px solid var(--v-border-subtle);";
        row.innerHTML = `
          <div>
            <strong>${p.name}</strong> <span style="font-size: 0.8rem; color: var(--v-text-muted);">${p.version}</span>
            <div style="font-size: 0.78rem; color: var(--v-text-secondary);">${p.description || ""}</div>
          </div>
          <span style="color: var(--v-status-success); font-weight: 600; font-size: 0.85rem;">Online</span>
        `;
        container.appendChild(row);
      });
    } catch (e) {
      console.warn("Failed to load plugins:", e);
    }
  }

  // --- Core Health Check ---
  async checkCoreStatus() {
    try {
      const res = await fetch("/api/ping");
      if (res.ok) {
        const data = await res.json();
        document.getElementById("core-status-text").textContent = `Core v${data.version || "1.0.0"}`;
      }
    } catch (e) {
      const ind = document.querySelector(".status-indicator");
      if (ind) ind.classList.remove("online");
      document.getElementById("core-status-text").textContent = "Core Offline";
    }
  }

  // --- Toast Notification ---
  showToast(message, type = "info") {
    const container = document.getElementById("toast-container");
    const toast = document.createElement("div");
    toast.className = `toast ${type}`;
    toast.textContent = message;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.opacity = "0";
      toast.style.transition = "opacity 0.3s ease";
      setTimeout(() => toast.remove(), 300);
    }, 3000);
  }
}

// Instantiate on DOM load
window.addEventListener("DOMContentLoaded", () => {
  window.app = new VesselApp();
});
