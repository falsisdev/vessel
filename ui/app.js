// Vessel Modern Reactive UI Controller - v2.1.0
// Next-Gen Media & Reading Platform

const I18N_STRINGS = {
  en: {
    nav_cinema: "Cinema",
    nav_reading: "Manga & E-Books",
    nav_live: "Live TV",
    nav_iptv: "IPTV",
    nav_library: "My Library",
    nav_plugins: "Plugins",
    nav_settings: "Settings",
    section_continue: "Continue Where You Left Off",
    section_discover: "Discover Media",
    search_placeholder: "Search movies, series, manga, anime, IPTV...",
    search_results_title: "Search Results",
    btn_back: "Back",
    btn_play: "Play Now",
    btn_read: "Read Now",
    btn_resume: "Resume",
    btn_add_library: "+ Add to Library",
    btn_in_library: "✓ In Library",
    btn_save: "Save",
    btn_install: "Install",
    btn_install_plugins: "Explore Plugins",
    btn_uninstall: "Uninstall",
    btn_enable: "Enable",
    btn_disable: "Disable",
    btn_active: "Active",
    btn_retry: "Retry",
    btn_watch_trailer: "Watch Trailer",
    cast: "Top Cast",
    directors: "Director",
    start_live: "Start Live Stream",
    pill_all: "All",
    pill_movies: "Movies",
    pill_series: "Series",
    pill_anime: "Anime",
    pill_manga: "Manga",
    pill_webtoon: "Webtoon",
    pill_novel: "Novels",
    pill_channels: "Channels",
    library_title: "Your Collection",
    status_watching: "In Progress",
    status_plan: "Plan to Watch",
    status_completed: "Completed",
    status_dropped: "Dropped",
    status_favorites: "Favorites",
    status_all: "All",
    iptv_guide_title: "Live TV Guide",
    iptv_guide_subtitle: "Browse global TV channels by country & category, watch live instantly.",
    settings_title: "Platform Settings",
    settings_theme_title: "Theme Engine",
    settings_theme_desc: "Choose from built-in curated palettes or install community CSS themes.",
    settings_debrid_title: "Torrent & Debrid Streaming",
    settings_debrid_desc: "Connect Real-Debrid or TorBox for high-speed cloud torrent streaming.",
    settings_plugins_title: "Connected Plugins",
    settings_plugins_desc: "Installed catalog and media provider processes.",
    plugins_title: "Plugin Hub",
    plugins_subtitle: "Manage installed providers or install new catalog and streaming extensions.",
    plugins_tab_installed: "Installed Extensions",
    plugins_tab_discover: "Add / Install Extension",
    plugins_install_url_title: "Install from URL or Repository",
    plugins_install_url_desc: "Paste a plugin manifest or repository URL to install.",
    plugins_install_local_title: "Install from Local Directory",
    plugins_install_local_desc: "Specify a local plugin binary or manifest path on your device.",
    plugins_curated_title: "Curated Directory",
    plugins_curated_desc: "Discover verified official extensions for Vessel.",
    plugin_builtin: "Built-in",
    plugin_external: "External Process",
    empty_domain_title: "No plugins active for this domain",
    empty_domain_desc: "Install or connect a plugin to browse and stream catalogs in this section.",
    no_results: "No media items found. Try another query.",
    resume_empty: "You haven't started watching or reading anything yet. Explore content below!",
    seasons: "Seasons",
    season: "Season",
    episodes: "Episodes",
    episode: "Episode",
    chapters: "Chapters",
    chapter: "Chapter",
    streams_title: "Available Streams & Torrents",
    streams_loading: "Searching available stream sources...",
    no_streams: "No streams currently found.",
    no_episode_streams: "No streams found for this episode.",
    synopsis: "Overview",
    theme_vessel_dark: "Vessel Dark (Default)",
    theme_vessel_light: "Vessel Light",
    theme_midnight_oled: "Midnight OLED",
    theme_catppuccin: "Catppuccin",
    theme_nord: "Nord",
    theme_dracula: "Dracula",
    theme_mangile: "Mangile Duman",
    theme_mangile_mauve: "Mangile Leylak",
    theme_mangile_stone: "Mangile Kaya",
    theme_mangile_zinc: "Mangile Çinko",
    theme_mangile_slate: "Mangile Arduvaz",
    theme_mangile_olive: "Mangile Zeytin",
    theme_mangile_taupe: "Mangile Boz",
    theme_mangile_gray: "Mangile Kır",
    theme_mangile_neutral: "Mangile Yavan",
    cat_popular_movies: "Popular Movies",
    cat_top_rated_movies: "Top Rated",
    cat_trending_movies: "Trending Now",
    cat_popular_contents: "Popular Content",
    cat_latest_contents: "Latest Releases",
    cat_popular_channels: "Popular Channels",
    cat_news_info: "News & Information",
    cat_sports_live: "Sports & Entertainment",
    status_offline: "Offline Downloads",
    lan_sync_title: "LAN Sync & Remote Control",
    lan_this_device: "This Device",
    lan_discovered_devices: "Discovered Devices",
    lan_remote_control: "Remote Control"
  },
  tr: {
    nav_cinema: "Sinema",
    nav_reading: "Manga & E-Kitap",
    nav_live: "Canlı Yayın",
    nav_iptv: "IPTV",
    nav_library: "Kütüphanem",
    nav_plugins: "Eklentiler",
    nav_settings: "Ayarlar",
    section_continue: "Kaldığın Yerden Devam Et",
    section_discover: "Medya Keşfet",
    search_placeholder: "Film, dizi, manga, anime, IPTV ara...",
    search_results_title: "Arama Sonuçları",
    btn_back: "Geri",
    btn_play: "Hemen İzle",
    btn_read: "Hemen Oku",
    btn_resume: "Devam Et",
    btn_add_library: "+ Kütüphaneye Ekle",
    btn_in_library: "✓ Kütüphanede",
    btn_save: "Kaydet",
    btn_install: "Yükle",
    btn_install_plugins: "Eklentileri Keşfet",
    btn_uninstall: "Kaldır",
    btn_enable: "Etkinleştir",
    btn_disable: "Devre Dışı",
    btn_active: "Aktif",
    btn_retry: "Tekrar Dene",
    btn_watch_trailer: "Fragman İzle",
    cast: "Oyuncular",
    directors: "Yönetmen",
    start_live: "Canlı Yayını Başlat",
    pill_all: "Tümü",
    pill_movies: "Filmler",
    pill_series: "Diziler",
    pill_anime: "Animeler",
    pill_manga: "Manga",
    pill_webtoon: "Webtoon",
    pill_novel: "Roman",
    pill_channels: "Kanallar",
    library_title: "Koleksiyonunuz",
    status_watching: "Devam Edenler",
    status_plan: "İzlenecekler",
    status_completed: "Tamamlananlar",
    status_dropped: "Bırakıldı",
    status_favorites: "Favoriler",
    status_all: "Tümü",
    iptv_guide_title: "Canlı TV / IPTV Rehberi",
    iptv_guide_subtitle: "Ülkelere ve kategorilere göre TV kanallarını keşfedin, anında canlı izleyin.",
    settings_title: "Platform Ayarları",
    settings_theme_title: "Tema Motoru",
    settings_theme_desc: "Dahili 15 zengin temadan birini seçin veya renkleri anında değiştirin.",
    settings_debrid_title: "Torrent & Debrid Akış Motoru",
    settings_debrid_desc: "Yüksek hızlı bulut torrent akışı için Real-Debrid veya TorBox bağlayın.",
    settings_plugins_title: "Bağlı Eklentiler",
    settings_plugins_desc: "Yüklü katalog ve medya sağlayıcı süreçleri.",
    plugins_title: "Eklenti Merkezi",
    plugins_subtitle: "Yüklü eklentileri yönetin veya yeni katalog ve akış uzantıları yükleyin.",
    plugins_tab_installed: "Yüklü Eklentiler",
    plugins_tab_discover: "Yeni Eklenti Ekle",
    plugins_install_url_title: "URL veya Depodan Yükle",
    plugins_install_url_desc: "Yüklemek için eklenti manifest veya Git depo URL'sini yapıştırın.",
    plugins_install_local_title: "Yerel Dizinden Yükle",
    plugins_install_local_desc: "Cihazınızdaki yerel eklenti ikili dosyasını veya manifest yolunu belirtin.",
    plugins_curated_title: "Önerilen Eklenti Kataloğu",
    plugins_curated_desc: "Vessel için doğrulanmış resmi eklentileri keşfedin.",
    plugin_builtin: "Dahili (Gömülü)",
    plugin_external: "Harici Süreç",
    empty_domain_title: "Bu alan için henüz aktif bir eklenti yok",
    empty_domain_desc: "Bu bölümde içerik keşfetmek ve akış izlemek için Eklentiler sayfasından eklenti bağlayın.",
    no_results: "Sonuç bulunamadı. Farklı bir arama deneyin.",
    resume_empty: "Henüz izlemeye veya okumaya başlamadınız. Aşağıdan içerikleri keşfedin!",
    seasons: "Sezonlar",
    season: "Sezon",
    episodes: "Bölümler",
    episode: "Bölüm",
    chapters: "Bölümler",
    chapter: "Bölüm",
    streams_title: "Mevcut Akış Kaynakları & Torrentler",
    streams_loading: "Kullanılabilir akış kaynakları aranıyor...",
    no_streams: "Şu anda uygun akış kaynağı bulunamadı.",
    no_episode_streams: "Bu bölüm için akış kaynağı bulunamadı.",
    synopsis: "Özet",
    theme_vessel_dark: "Vessel Dark (Varsayılan)",
    theme_vessel_light: "Vessel Light",
    theme_midnight_oled: "Midnight OLED",
    theme_catppuccin: "Catppuccin",
    theme_nord: "Nord",
    theme_dracula: "Dracula",
    theme_mangile: "Mangile Duman",
    theme_mangile_mauve: "Mangile Leylak",
    theme_mangile_stone: "Mangile Kaya",
    theme_mangile_zinc: "Mangile Çinko",
    theme_mangile_slate: "Mangile Arduvaz",
    theme_mangile_olive: "Mangile Zeytin",
    theme_mangile_taupe: "Mangile Boz",
    theme_mangile_gray: "Mangile Kır",
    theme_mangile_neutral: "Mangile Yavan",
    cat_popular_movies: "Popüler Filmler",
    cat_top_rated_movies: "En Çok Oy Alanlar",
    cat_trending_movies: "Trend Olanlar",
    cat_popular_contents: "Popüler İçerikler",
    cat_latest_contents: "Son Oluşturulan İçerikler",
    cat_popular_channels: "Popüler Kanallar",
    cat_news_info: "Haber & Bilgi",
    cat_sports_live: "Spor & Canlı",
    status_offline: "İndirilenler",
    lan_sync_title: "LAN Cihazları ve Uzaktan Kumanda",
    lan_this_device: "Bu Cihaz",
    lan_discovered_devices: "Ağdaki Cihazlar",
    lan_remote_control: "Uzaktan Kumanda"
  },
  de: {
    nav_cinema: "Kino",
    nav_reading: "Manga & E-Books",
    nav_live: "Live-TV",
    nav_iptv: "IPTV",
    nav_library: "Meine Bibliothek",
    nav_plugins: "Erweiterungen",
    nav_settings: "Einstellungen",
    section_continue: "Weitermachen, wo du aufgehört hast",
    section_discover: "Medien entdecken",
    search_placeholder: "Filme, Serien, Manga, Anime suchen...",
    search_results_title: "Suchergebnisse",
    btn_back: "Zurück",
    btn_play: "Jetzt ansehen",
    btn_read: "Jetzt lesen",
    btn_resume: "Fortsetzen",
    btn_add_library: "+ Zur Bibliothek",
    btn_in_library: "✓ In Bibliothek",
    btn_save: "Speichern",
    btn_install: "Installieren",
    btn_install_plugins: "Plugins entdecken",
    btn_uninstall: "Deinstallieren",
    btn_enable: "Aktivieren",
    btn_disable: "Deaktivieren",
    btn_active: "Aktiv",
    pill_all: "Alle",
    pill_movies: "Filme",
    pill_series: "Serien",
    pill_anime: "Anime",
    pill_manga: "Manga",
    pill_webtoon: "Webtoon",
    pill_novel: "Romane",
    pill_channels: "Sender",
    library_title: "Ihre Sammlung",
    status_all: "Alle",
    status_watching: "Wird geschaut",
    status_plan: "Geplant",
    status_completed: "Abgeschlossen",
    status_dropped: "Abgebrochen",
    status_favorites: "Favoriten",
    iptv_guide_title: "Live-TV-Programm",
    iptv_guide_subtitle: "Internationale und nationale Live-Streams",
    settings_title: "Plattformeinstellungen",
    settings_theme_title: "Theme-Engine",
    settings_theme_desc: "Wählen Sie vorgefertigte Paletten oder Community-Themes.",
    settings_debrid_title: "Torrent & Debrid-Streaming",
    settings_debrid_desc: "Real-Debrid oder TorBox für Highspeed-Streaming verbinden.",
    settings_plugins_title: "Verbundene Plugins",
    settings_plugins_desc: "Installierte Katalog- und Medienprozesse.",
    plugins_title: "Erweiterungs-Hub",
    plugins_subtitle: "Erweiterungen verwalten oder neue Katalog- und Streaming-Quellen installieren.",
    plugins_tab_installed: "Installierte Plugins",
    plugins_tab_discover: "Neues Plugin hinzufügen",
    plugins_install_url_title: "Über URL / Repository installieren",
    plugins_install_url_desc: "Manifest- oder Git-Repository-URL eingeben.",
    plugins_install_local_title: "Aus lokalem Verzeichnis installieren",
    plugins_install_local_desc: "Pfad zur lokalen Plugin-Datei angeben.",
    plugins_curated_title: "Empfohlene Erweiterungen",
    plugins_curated_desc: "Verifizierte offizielle und Community-Plugins.",
    plugin_builtin: "Integriert",
    plugin_external: "Externer Prozess",
    empty_domain_title: "Keine aktiven Plugins für diesen Bereich",
    empty_domain_desc: "Installieren Sie eine Erweiterung, um Inhalte für diesen Bereich zu streamen.",
    no_results: "Keine Medien gefunden.",
    resume_empty: "Noch keine Wiedergabe oder Lektüre gestartet.",
    seasons: "Staffeln",
    season: "Staffel",
    episodes: "Episoden",
    episode: "Episode",
    chapters: "Kapitel",
    chapter: "Kapitel",
    streams_title: "Verfügbare Streams & Torrents",
    streams_loading: "Stream wird aufgelöst...",
    no_streams: "Keine Streams gefunden.",
    synopsis: "Handlung",
    theme_mangile: "Dunkel Standard",
    theme_mangile_mauve: "Mauve Thema",
    theme_mangile_stone: "Stein Thema",
    theme_mangile_slate: "Schiefer Thema",
    theme_mangile_neutral: "Neutral Thema",
    theme_mangile_zinc: "Zink Thema",
    theme_mangile_sunset: "Sonnenuntergang",
    theme_mangile_desert: "Wüste Thema",
    theme_mangile_ice: "Eis Thema",
    theme_midnight_blue: "Mitternachtsblau",
    theme_ember: "Glutrot",
    theme_amethyst: "Amethyst Violett",
    theme_forest: "Waldgrün",
    theme_light_clean: "Klar Hell"
  },
  fr: {
    nav_cinema: "Cinéma",
    nav_reading: "Manga & E-Books",
    nav_live: "TV en direct",
    nav_iptv: "IPTV",
    nav_library: "Ma Bibliothèque",
    nav_plugins: "Extensions",
    nav_settings: "Paramètres",
    section_continue: "Reprendre là où vous vous êtes arrêté",
    section_discover: "Découvrir",
    search_placeholder: "Rechercher films, séries, mangas, animés...",
    search_results_title: "Résultats de recherche",
    btn_back: "Retour",
    btn_play: "Regarder",
    btn_read: "Lire",
    btn_resume: "Reprendre",
    btn_add_library: "+ Ajouter",
    btn_in_library: "✓ Dans la biblio",
    btn_save: "Enregistrer",
    btn_install: "Installer",
    btn_install_plugins: "Explorer les extensions",
    btn_uninstall: "Désinstaller",
    btn_enable: "Activer",
    btn_disable: "Désactiver",
    btn_active: "Actif",
    pill_all: "Tous",
    pill_movies: "Films",
    pill_series: "Séries",
    pill_anime: "Animés",
    pill_manga: "Manga",
    pill_webtoon: "Webtoon",
    pill_novel: "Romans",
    pill_channels: "Chaînes",
    library_title: "Votre Collection",
    status_all: "Tout",
    status_watching: "En cours",
    status_plan: "À voir",
    status_completed: "Terminé",
    status_dropped: "Abandonné",
    status_favorites: "Favoris",
    iptv_guide_title: "Guide TV en Direct",
    iptv_guide_subtitle: "Chaînes et flux télévisés du monde entier",
    settings_title: "Paramètres",
    settings_theme_title: "Moteur de thèmes",
    settings_theme_desc: "Choisissez parmi les palettes intégrées ou installez des thèmes.",
    settings_debrid_title: "Streaming Torrent & Debrid",
    settings_debrid_desc: "Connectez Real-Debrid ou TorBox.",
    settings_plugins_title: "Extensions connectées",
    settings_plugins_desc: "Processus de catalogue et fournisseurs installés.",
    plugins_title: "Centre d'extensions",
    plugins_subtitle: "Gérez les fournisseurs installés ou ajoutez de nouvelles sources.",
    plugins_tab_installed: "Extensions installées",
    plugins_tab_discover: "Ajouter une extension",
    plugins_install_url_title: "Installer depuis une URL",
    plugins_install_url_desc: "Collez l'URL d'un manifeste ou dépôt.",
    plugins_install_local_title: "Installer depuis un dossier local",
    plugins_install_local_desc: "Spécifiez le chemin vers le binaire ou dossier local.",
    plugins_curated_title: "Répertoire recommandé",
    plugins_curated_desc: "Extensions officielles et communautaires certifiées.",
    plugin_builtin: "Intégré",
    plugin_external: "Processus externe",
    empty_domain_title: "Aucune extension active pour ce domaine",
    empty_domain_desc: "Installez une extension pour parcourir et lire des médias dans cette section.",
    no_results: "Aucun résultat trouvé.",
    resume_empty: "Vous n'avez pas encore commencé de lecture ou visionnage.",
    seasons: "Saisons",
    season: "Saison",
    episodes: "Épisodes",
    episode: "Épisode",
    chapters: "Chapitres",
    chapter: "Chapitre",
    streams_title: "Flux & Torrents disponibles",
    streams_loading: "Recherche du meilleur flux...",
    no_streams: "Aucun flux trouvé.",
    synopsis: "Synopsis",
    theme_mangile: "Sombre par défaut",
    theme_mangile_mauve: "Thème Mauve",
    theme_mangile_stone: "Thème Pierre",
    theme_mangile_slate: "Thème Ardoise",
    theme_mangile_neutral: "Thème Neutre",
    theme_mangile_zinc: "Thème Zinc",
    theme_mangile_sunset: "Coucher de soleil",
    theme_mangile_desert: "Thème Désert",
    theme_mangile_ice: "Thème Glace",
    theme_midnight_blue: "Bleu Minuit",
    theme_ember: "Braise Chaude",
    theme_amethyst: "Améthyste",
    theme_forest: "Forêt Émeraude",
    theme_light_clean: "Clair Épuré"
  },
  es: {
    nav_cinema: "Cine",
    nav_reading: "Manga y Libros",
    nav_live: "TV en Vivo",
    nav_iptv: "IPTV",
    nav_library: "Mi Biblioteca",
    nav_plugins: "Extensiones",
    nav_settings: "Ajustes",
    section_continue: "Continuar donde lo dejaste",
    section_discover: "Descubrir",
    search_placeholder: "Buscar películas, series, manga, anime...",
    search_results_title: "Resultados de búsqueda",
    btn_back: "Volver",
    btn_play: "Ver ahora",
    btn_read: "Leer ahora",
    btn_resume: "Reanudar",
    btn_add_library: "+ Añadir",
    btn_in_library: "✓ En biblioteca",
    btn_save: "Guardar",
    btn_install: "Instalar",
    btn_install_plugins: "Explorar extensiones",
    btn_uninstall: "Desinstalar",
    btn_enable: "Activar",
    btn_disable: "Desactivar",
    btn_active: "Activo",
    pill_all: "Todo",
    pill_movies: "Películas",
    pill_series: "Series",
    pill_anime: "Anime",
    pill_manga: "Manga",
    pill_webtoon: "Webtoon",
    pill_novel: "Novelas",
    pill_channels: "Canales",
    library_title: "Tu Colección",
    status_all: "Todo",
    status_watching: "Viendo",
    status_plan: "Pendiente",
    status_completed: "Completado",
    status_dropped: "Abandonado",
    status_favorites: "Favoritos",
    iptv_guide_title: "Guía de TV en Vivo",
    iptv_guide_subtitle: "Transmisiones de canales de televisión del mundo",
    settings_title: "Ajustes",
    settings_theme_title: "Motor de temas",
    settings_theme_desc: "Elige temas integrados o de la comunidad.",
    settings_debrid_title: "Streaming Torrent y Debrid",
    settings_debrid_desc: "Conecta Real-Debrid o TorBox.",
    settings_plugins_title: "Plugins conectados",
    settings_plugins_desc: "Procesos de catálogo instalados.",
    plugins_title: "Centro de Extensiones",
    plugins_subtitle: "Gestiona proveedores instalados o añade nuevas fuentes.",
    plugins_tab_installed: "Extensiones instaladas",
    plugins_tab_discover: "Añadir extensión",
    plugins_install_url_title: "Instalar desde URL",
    plugins_install_url_desc: "Pega la URL de un manifiesto o repositorio.",
    plugins_install_local_title: "Instalar desde carpeta local",
    plugins_install_local_desc: "Ruta al ejecutable o manifiesto local.",
    plugins_curated_title: "Directorio Recomendado",
    plugins_curated_desc: "Extensiones verificadas para Vessel.",
    plugin_builtin: "Integrado",
    plugin_external: "Proceso externo",
    empty_domain_title: "No hay extensiones para esta categoría",
    empty_domain_desc: "Instala una extensión para descubrir y reproducir contenido aquí.",
    no_results: "No se encontraron medios.",
    resume_empty: "No has empezado a ver o leer nada todavía.",
    seasons: "Temporadas",
    season: "Temporada",
    episodes: "Episodios",
    episode: "Episodio",
    chapters: "Capítulos",
    chapter: "Capítulo",
    streams_title: "Fuentes y Torrents disponibles",
    streams_loading: "Cargando stream...",
    no_streams: "No se encontraron fuentes disponibles.",
    synopsis: "Sinopsis",
    theme_mangile: "Oscuro por defecto",
    theme_mangile_mauve: "Tema Malva",
    theme_mangile_stone: "Tema Piedra",
    theme_mangile_slate: "Tema Pizarra",
    theme_mangile_neutral: "Tema Neutro",
    theme_mangile_zinc: "Tema Cinc",
    theme_mangile_sunset: "Atardecer",
    theme_mangile_desert: "Desierto",
    theme_mangile_ice: "Hielo",
    theme_midnight_blue: "Azul Medianoche",
    theme_ember: "Brasa Cálida",
    theme_amethyst: "Amatista",
    theme_forest: "Bosque Esmeralda",
    theme_light_clean: "Claro Limpio"
  },
  pt: {
    nav_cinema: "Cinema",
    nav_reading: "Mangás e E-books",
    nav_live: "TV ao Vivo",
    nav_iptv: "IPTV",
    nav_library: "Minha Biblioteca",
    nav_plugins: "Extensões",
    nav_settings: "Configurações",
    section_continue: "Continuar de onde você parou",
    section_discover: "Descobrir",
    search_placeholder: "Buscar filmes, séries, mangás, animes...",
    search_results_title: "Resultados da busca",
    btn_back: "Voltar",
    btn_play: "Assistir agora",
    btn_read: "Ler agora",
    btn_resume: "Continuar",
    btn_add_library: "+ Adicionar",
    btn_in_library: "✓ Na Biblioteca",
    btn_save: "Salvar",
    btn_install: "Instalar",
    btn_install_plugins: "Explorar extensões",
    btn_uninstall: "Remover",
    btn_enable: "Ativar",
    btn_disable: "Desativar",
    btn_active: "Ativo",
    pill_all: "Tudo",
    pill_movies: "Filmes",
    pill_series: "Séries",
    pill_anime: "Anime",
    pill_manga: "Mangá",
    pill_webtoon: "Webtoon",
    pill_novel: "Livros/Novels",
    pill_channels: "Canais",
    library_title: "Sua Coleção",
    status_all: "Todos",
    status_watching: "Assistindo",
    status_plan: "Quero Assistir",
    status_completed: "Concluído",
    status_dropped: "Abandonado",
    status_favorites: "Favoritos",
    iptv_guide_title: "Guia de TV ao Vivo",
    iptv_guide_subtitle: "Canais e transmissões de TV ao vivo de todo o mundo",
    settings_title: "Configurações",
    settings_theme_title: "Motor de temas",
    settings_theme_desc: "Escolha temas internos ou da comunidade.",
    settings_debrid_title: "Streaming Torrent e Debrid",
    settings_debrid_desc: "Conecte Real-Debrid ou TorBox.",
    settings_plugins_title: "Plugins Conectados",
    settings_plugins_desc: "Catálogos e provedores ativos.",
    plugins_title: "Central de Extensões",
    plugins_subtitle: "Gerencie extensões instaladas ou adicione novos catálogos.",
    plugins_tab_installed: "Extensões Instaladas",
    plugins_tab_discover: "Adicionar Extensão",
    plugins_install_url_title: "Instalar via URL",
    plugins_install_url_desc: "Cole a URL de um manifesto ou repositório.",
    plugins_install_local_title: "Instalar de diretório local",
    plugins_install_local_desc: "Especifique o caminho do binário ou manifesto local.",
    plugins_curated_title: "Diretório Recomendado",
    plugins_curated_desc: "Extensões oficiais e da comunidade verificadas.",
    plugin_builtin: "Integrado",
    plugin_external: "Processo Externo",
    empty_domain_title: "Nenhuma extensão para esta categoria",
    empty_domain_desc: "Instale uma extensão para navegar e assistir conteúdos aqui.",
    no_results: "Nenhum resultado encontrado.",
    resume_empty: "Você ainda não começou a assistir ou ler nada.",
    seasons: "Temporadas",
    season: "Temporada",
    episodes: "Episódios",
    episode: "Episódio",
    chapters: "Capítulos",
    chapter: "Capítulo",
    streams_title: "Streams e Torrents disponíveis",
    streams_loading: "Carregando transmissão...",
    no_streams: "Nenhum stream encontrado.",
    synopsis: "Sinopse",
    theme_mangile: "Escuro Padrão",
    theme_mangile_mauve: "Tema Malva",
    theme_mangile_stone: "Tema Pedra",
    theme_mangile_slate: "Tema Ardósia",
    theme_mangile_neutral: "Tema Neutro",
    theme_mangile_zinc: "Tema Zinco",
    theme_mangile_sunset: "Pôr do Sol",
    theme_mangile_desert: "Deserto",
    theme_mangile_ice: "Gelo",
    theme_midnight_blue: "Azul Meia-Noite",
    theme_ember: "Brasa Quente",
    theme_amethyst: "Ametista",
    theme_forest: "Floresta Esmeralda",
    theme_light_clean: "Claro Limpo"
  },
  ru: {
    nav_cinema: "Кино",
    nav_reading: "Манга и Электронные книги",
    nav_live: "ТВ онлайн",
    nav_iptv: "IPTV",
    nav_library: "Моя Библиотека",
    nav_plugins: "Плагины",
    nav_settings: "Настройки",
    section_continue: "Продолжить с места остановки",
    section_discover: "Обзор",
    search_placeholder: "Поиск фильмов, сериалов, манги, аниме...",
    search_results_title: "Результаты поиска",
    btn_back: "Назад",
    btn_play: "Смотреть",
    btn_read: "Читать",
    btn_resume: "Продолжить",
    btn_add_library: "+ В библиотеку",
    btn_in_library: "✓ В библиотеке",
    btn_save: "Сохранить",
    btn_install: "Установить",
    btn_install_plugins: "Каталог плагинов",
    btn_uninstall: "Удалить",
    btn_enable: "Включить",
    btn_disable: "Выключить",
    btn_active: "Активен",
    pill_all: "Все",
    pill_movies: "Фильмы",
    pill_series: "Сериалы",
    pill_anime: "Аниме",
    pill_manga: "Манга",
    pill_webtoon: "Вебтун",
    pill_novel: "Новеллы",
    pill_channels: "Каналы",
    library_title: "Ваша коллекция",
    status_all: "Все",
    status_watching: "Смотрю",
    status_plan: "В планах",
    status_completed: "Завершено",
    status_dropped: "Брошено",
    status_favorites: "Избранное",
    iptv_guide_title: "Телегид прямых трансляций",
    iptv_guide_subtitle: "Мировые телеканалы и онлайн-трансляции",
    settings_title: "Настройки платформы",
    settings_theme_title: "Темы оформления",
    settings_theme_desc: "Встроенные темы и поддержка пользовательских стилей.",
    settings_debrid_title: "Торрент и Debrid стриминг",
    settings_debrid_desc: "Подключите Real-Debrid или TorBox.",
    settings_plugins_title: "Подключенные плагины",
    settings_plugins_desc: "Установленные каталоги и провайдеры.",
    plugins_title: "Центр плагинов",
    plugins_subtitle: "Управление установленными расширениями и поиск новых источников.",
    plugins_tab_installed: "Установленные",
    plugins_tab_discover: "Добавить плагин",
    plugins_install_url_title: "Установка по URL",
    plugins_install_url_desc: "Вставьте ссылку на манифест или репозиторий.",
    plugins_install_local_title: "Установка из папки",
    plugins_install_local_desc: "Укажите локальный путь к плагину на устройстве.",
    plugins_curated_title: "Каталог проверенных плагинов",
    plugins_curated_desc: "Официальные и проверенные расширения сообщества.",
    plugin_builtin: "Встроенный",
    plugin_external: "Внешний процесс",
    empty_domain_title: "Нет активных плагинов для этого раздела",
    empty_domain_desc: "Установите плагин, чтобы получить доступ к медиа в этом разделе.",
    no_results: "Ничего не найдено.",
    resume_empty: "Вы пока ничего не начали смотреть или читать.",
    seasons: "Сезоны",
    season: "Сезон",
    episodes: "Эпизоды",
    episode: "Эпизод",
    chapters: "Главы",
    chapter: "Глава",
    streams_title: "Доступные потоки и торренты",
    streams_loading: "Поиск лучшего источника...",
    no_streams: "Источников пока нет.",
    synopsis: "Описание",
    theme_mangile: "Темная по умолчанию",
    theme_mangile_mauve: "Лиловая тема",
    theme_mangile_stone: "Каменная тема",
    theme_mangile_slate: "Сланцевая тема",
    theme_mangile_neutral: "Нейтральная тема",
    theme_mangile_zinc: "Цинковая тема",
    theme_mangile_sunset: "Закат",
    theme_mangile_desert: "Пустыня",
    theme_mangile_ice: "Ледяная тема",
    theme_midnight_blue: "Полуночный синий",
    theme_ember: "Теплый уголь",
    theme_amethyst: "Аметист",
    theme_forest: "Изумрудный лес",
    theme_light_clean: "Чистая светлая"
  },
  ja: {
    nav_cinema: "映画",
    nav_reading: "マンガ & 電子書籍",
    nav_live: "ライブ配信",
    nav_iptv: "IPTV",
    nav_library: "マイライブラリ",
    nav_plugins: "プラグイン",
    nav_settings: "設定",
    section_continue: "続きから再開",
    section_discover: "メディアを探す",
    search_placeholder: "映画、ドラマ、マンガ、アニメを検索...",
    search_results_title: "検索結果",
    btn_back: "戻る",
    btn_play: "今すぐ視聴",
    btn_read: "今すぐ読む",
    btn_resume: "再開",
    btn_add_library: "+ ライブラリに追加",
    btn_in_library: "✓ 登録済み",
    btn_save: "保存",
    btn_install: "インストール",
    btn_install_plugins: "プラグインを探す",
    btn_uninstall: "削除",
    btn_enable: "有効化",
    btn_disable: "無効化",
    btn_active: "稼働中",
    pill_all: "すべて",
    pill_movies: "映画",
    pill_series: "ドラマ",
    pill_anime: "アニメ",
    pill_manga: "マンガ",
    pill_webtoon: "ウェブトゥーン",
    pill_novel: "ノベル",
    pill_channels: "チャンネル",
    library_title: "コレクション",
    status_all: "すべて",
    status_watching: "視聴中",
    status_plan: "見たい",
    status_completed: "完了",
    status_dropped: "中断",
    status_favorites: "お気に入り",
    iptv_guide_title: "ライブTV番組表",
    iptv_guide_subtitle: "世界中のテレビチャンネルとライブ配信",
    settings_title: "プラットフォーム設定",
    settings_theme_title: "テーマエンジン",
    settings_theme_desc: "組み込みテーマやカスタムCSSを適用します。",
    settings_debrid_title: "Torrent & Debrid ストリーミング",
    settings_debrid_desc: "Real-Debrid または TorBox を接続します。",
    settings_plugins_title: "接続済みプラグイン",
    settings_plugins_desc: "インストールされているメディアプロバイダー。",
    plugins_title: "プラグインハブ",
    plugins_subtitle: "プラグインの管理や新しいソースのインストール。",
    plugins_tab_installed: "インストール済み",
    plugins_tab_discover: "プラグインを追加",
    plugins_install_url_title: "URLからインストール",
    plugins_install_url_desc: "マニフェストURLまたはGitリポジトリを入力。",
    plugins_install_local_title: "ローカルからインストール",
    plugins_install_local_desc: "ローカルのバイナリまたはパスを指定。",
    plugins_curated_title: "おすすめプラグイン",
    plugins_curated_desc: "公式およびコミュニティで検証されたプラグイン。",
    plugin_builtin: "内蔵",
    plugin_external: "外部プロセス",
    empty_domain_title: "このジャンルのプラグインがありません",
    empty_domain_desc: "プラグインをインストールしてコンテンツを楽しみましょう。",
    no_results: "見つかりませんでした。",
    resume_empty: "まだ再生や読書を始めていません。",
    seasons: "シーズン",
    season: "シーズン",
    episodes: "エピソード",
    episode: "エピソード",
    chapters: "チャプター",
    chapter: "話",
    streams_title: "利用可能なストリーム",
    streams_loading: "ストリームを解決中...",
    no_streams: "ストリームが見つかりません。",
    synopsis: "あらすじ",
    theme_mangile: "標準ダーク",
    theme_mangile_mauve: "モーヴ テーマ",
    theme_mangile_stone: "ストーン テーマ",
    theme_mangile_slate: "スレート テーマ",
    theme_mangile_neutral: "ニュートラル テーマ",
    theme_mangile_zinc: "ジンク テーマ",
    theme_mangile_sunset: "サンセット",
    theme_mangile_desert: "デザート",
    theme_mangile_ice: "アイス",
    theme_midnight_blue: "ミッドナイトブルー",
    theme_ember: "ウォームアンバー",
    theme_amethyst: "アメジストパープル",
    theme_forest: "フォレストエメラルド",
    theme_light_clean: "クリーンライト"
  },
  zh: {
    nav_cinema: "电影",
    nav_reading: "漫画与电子书",
    nav_live: "电视直播",
    nav_iptv: "IPTV",
    nav_library: "我的媒体库",
    nav_plugins: "插件中心",
    nav_settings: "系统设置",
    section_continue: "从上次离开的地方继续",
    section_discover: "探索媒体",
    search_placeholder: "搜索电影、电视剧、漫画、动漫...",
    search_results_title: "搜索结果",
    btn_back: "返回",
    btn_play: "立即播放",
    btn_read: "立即阅读",
    btn_resume: "继续",
    btn_add_library: "+ 收藏到媒体库",
    btn_in_library: "✓ 已收藏",
    btn_save: "保存",
    btn_install: "安装",
    btn_install_plugins: "探索插件",
    btn_uninstall: "卸载",
    btn_enable: "启用",
    btn_disable: "禁用",
    btn_active: "运行中",
    pill_all: "全部",
    pill_movies: "电影",
    pill_series: "电视剧",
    pill_anime: "动漫",
    pill_manga: "漫画",
    pill_webtoon: "条漫",
    pill_novel: "小说",
    pill_channels: "频道",
    library_title: "你的收藏",
    status_all: "全部",
    status_watching: "正在观看",
    status_plan: "想看",
    status_completed: "已看完",
    status_dropped: "已放弃",
    status_favorites: "特别喜欢",
    iptv_guide_title: "电视直播节目表",
    iptv_guide_subtitle: "全球电视频道与网络在线直播",
    settings_title: "平台设置",
    settings_theme_title: "主题引擎",
    settings_theme_desc: "挑选内置主题或载入社区 CSS 主题。",
    settings_debrid_title: "Torrent 与 Debrid 云播",
    settings_debrid_desc: "绑定 Real-Debrid 或 TorBox 高速播放。",
    settings_plugins_title: "已连接插件",
    settings_plugins_desc: "已加载的媒体数据源。",
    plugins_title: "扩展插件中心",
    plugins_subtitle: "管理已安装的提供商或添加新的媒体数据源。",
    plugins_tab_installed: "已安装扩展",
    plugins_tab_discover: "安装新扩展",
    plugins_install_url_title: "通过 URL 或代码库安装",
    plugins_install_url_desc: "输入扩展清单或仓库地址。",
    plugins_install_local_title: "通过本地路径安装",
    plugins_install_local_desc: "输入本地插件可执行文件或文件夹路径。",
    plugins_curated_title: "官方推荐扩展",
    plugins_curated_desc: "经过验证的安全扩展集合。",
    plugin_builtin: "内置程序",
    plugin_external: "独立进程",
    empty_domain_title: "此分类下暂无可用插件",
    empty_domain_desc: "安装对应的扩展程序以解锁此分类的海量内容。",
    no_results: "未找到内容。",
    resume_empty: "尚未开始任何观看或阅读。",
    seasons: "季数",
    season: "季",
    episodes: "集数",
    episode: "集",
    chapters: "章节",
    chapter: "话",
    streams_title: "可用播放源与种子",
    streams_loading: "正在解析高清源...",
    no_streams: "暂无可用播放源。",
    synopsis: "剧情简介",
    theme_mangile: "默认暗黑",
    theme_mangile_mauve: "锦葵紫",
    theme_mangile_stone: "冷石灰",
    theme_mangile_slate: "石板青",
    theme_mangile_neutral: "纯中性",
    theme_mangile_zinc: "锌暗灰",
    theme_mangile_sunset: "落日晚霞",
    theme_mangile_desert: "暖黄沙漠",
    theme_mangile_ice: "冰蓝极境",
    theme_midnight_blue: "午夜深蓝",
    theme_ember: "余烬暖金",
    theme_amethyst: "紫水晶",
    theme_forest: "翡翠森林",
    theme_light_clean: "极简素白"
  },
  ar: {
    nav_cinema: "السينما",
    nav_reading: "المانغا والكتب الإلكترونية",
    nav_live: "البث المباشر",
    nav_iptv: "IPTV",
    nav_library: "مكتبتي",
    nav_plugins: "الإضافات",
    nav_settings: "الإعدادات",
    section_continue: "المتابعة من حيث توقفت",
    section_discover: "استكشاف الوسائط",
    search_placeholder: "ابحث عن أفلام، مسلسلات، مانغا، أنمي...",
    search_results_title: "نتائج البحث",
    btn_back: "رجوع",
    btn_play: "شاهد الآن",
    btn_read: "اقرأ الآن",
    btn_resume: "استئناف",
    btn_add_library: "+ إضافة إلى المكتبة",
    btn_in_library: "✓ في المكتبة",
    btn_save: "حفظ",
    btn_install: "تثبيت",
    btn_install_plugins: "استكشاف الإضافات",
    btn_uninstall: "إلغاء التثبيت",
    btn_enable: "تفعيل",
    btn_disable: "تعطيل",
    btn_active: "نشط",
    pill_all: "الكل",
    pill_movies: "أفلام",
    pill_series: "مسلسلات",
    pill_anime: "أنمي",
    pill_manga: "مانغا",
    pill_webtoon: "ويبتون",
    pill_novel: "روايات",
    pill_channels: "قنوات",
    library_title: "مجموعتك",
    status_all: "الكل",
    status_watching: "قيد المشاهدة",
    status_plan: "في خطة المشاهدة",
    status_completed: "مكتمل",
    status_dropped: "متروك",
    status_favorites: "المفضلة",
    iptv_guide_title: "دليل القنوات التلفزيونية الحية",
    iptv_guide_subtitle: "بثوث وقنوات تلفزيونية مباشرة من حول العالم",
    settings_title: "إعدادات المنصة",
    settings_theme_title: "محرك السمات",
    settings_theme_desc: "اختر من اللوحات المدمجة أو استخدم سمات مخصصة.",
    settings_debrid_title: "بث تورنت و Debrid",
    settings_debrid_desc: "اربط حساب Real-Debrid أو TorBox للبث السريع.",
    settings_plugins_title: "الإضافات المتصلة",
    settings_plugins_desc: "عمليات التزويد والكتالوج المثبتة.",
    plugins_title: "مركز الإضافات",
    plugins_subtitle: "إدارة الإضافات المثبتة أو تثبيت مصادر كتالوج وبث جديدة.",
    plugins_tab_installed: "الإضافات المثبتة",
    plugins_tab_discover: "إضافة ملحق جديد",
    plugins_install_url_title: "تثبيت من رابط URL",
    plugins_install_url_desc: "الصق رابط ملف المانيفست أو المستودع للتثبيت.",
    plugins_install_local_title: "تثبيت من مسار محلي",
    plugins_install_local_desc: "حدد مسار الملف التنفيذي أو المانيفست على جهازك.",
    plugins_curated_title: "الدليل المعتمد",
    plugins_curated_desc: "إضافات رسمية وموثوقة من مجتمع Vessel.",
    plugin_builtin: "مدمج داخلي",
    plugin_external: "عملية خارجية",
    empty_domain_title: "لا توجد إضافات نشطة لهذا القسم",
    empty_domain_desc: "قم بتثبيت إضافة لعرض وبث المحتوى في هذا القسم.",
    no_results: "لم يتم العثور على نتائج.",
    resume_empty: "لم تبدأ بمشاهدة أو قراءة أي شيء بعد.",
    seasons: "المواسم",
    season: "الموسم",
    episodes: "الحلقات",
    episode: "الحلقة",
    chapters: "الفصول",
    chapter: "الفصل",
    streams_title: "البث والتورنت المتاح",
    streams_loading: "جارٍ فك تشفير البث...",
    no_streams: "لا توجد مصادر بث متاحة حالياً.",
    synopsis: "نبذة عن القصة",
    theme_mangile: "داكن افتراضي",
    theme_mangile_mauve: "سمة الموف",
    theme_mangile_stone: "سمة الحجر",
    theme_mangile_slate: "سمة الصخر",
    theme_mangile_neutral: "سمة محايدة",
    theme_mangile_zinc: "سمة الزنك",
    theme_mangile_sunset: "سمة الغروب",
    theme_mangile_desert: "سمة الصحراء",
    theme_mangile_ice: "سمة الجليد",
    theme_midnight_blue: "أزرق منتصف الليل",
    theme_ember: "جمر دافئ",
    theme_amethyst: "جمشت بنفسجي",
    theme_forest: "غابة الزمرد",
    theme_light_clean: "فاتح نقي"
  },
  fa: {
    nav_cinema: "سینما",
    nav_reading: "مانگا و کتاب الکترونیکی",
    nav_live: "پخش زنده",
    nav_iptv: "IPTV",
    nav_library: "کتابخانه من",
    nav_plugins: "پلاگین‌ها",
    nav_settings: "تنظیمات",
    section_continue: "ادامه از جایی که متوقف شدید",
    section_discover: "کاوش رسانه",
    search_placeholder: "جستجوی فیلم، سریال، مانگا، انیمه...",
    search_results_title: "نتایج جستجو",
    btn_back: "بازگشت",
    btn_play: "پخش فوری",
    btn_read: "مطالعه فوری",
    btn_resume: "ادامه",
    btn_add_library: "+ به کتابخانه",
    btn_in_library: "✓ در کتابخانه",
    btn_save: "ذخیره",
    btn_install: "نصب",
    btn_install_plugins: "کاوش افزونه‌ها",
    btn_uninstall: "حذف",
    btn_enable: "فعال‌سازی",
    btn_disable: "غیرفعال‌سازی",
    btn_active: "فعال",
    pill_all: "همه",
    pill_movies: "فیلم‌ها",
    pill_series: "سریال‌ها",
    pill_anime: "انیمه",
    pill_manga: "مانگا",
    pill_webtoon: "وبتون",
    pill_novel: "رمان",
    pill_channels: "کانال‌ها",
    library_title: "مجموعه شما",
    status_all: "همه",
    status_watching: "در حال تماشا",
    status_plan: "برنامه تماشا",
    status_completed: "تکمیل شده",
    status_dropped: "رها شده",
    status_favorites: "علاقه‌مندی‌ها",
    iptv_guide_title: "راهنمای شبکه‌های زنده تلویزیونی",
    iptv_guide_subtitle: "شبکه‌های تلویزیونی زنده سراسر جهان",
    settings_title: "تنظیمات پلتفرم",
    settings_theme_title: "موتور تم",
    settings_theme_desc: "از پوسته‌های پیش‌فرض انتخاب کنید یا تم CSS دلخواه اعمال کنید.",
    settings_debrid_title: "استریم تورنت و دبریس",
    settings_debrid_desc: "حساب Real-Debrid یا TorBox را برای پخش پرسرعت متصل کنید.",
    settings_plugins_title: "پلاگین‌های متصل",
    settings_plugins_desc: "سرویس‌های کاتالوگ و رسانه فعال.",
    plugins_title: "مرکز افزونه‌ها",
    plugins_subtitle: "مدیریت افزونه‌های نصب‌شده یا افزودن منابع کاتالوگ جدید.",
    plugins_tab_installed: "افزونه‌های نصب‌شده",
    plugins_tab_discover: "نصب افزونه جدید",
    plugins_install_url_title: "نصب از طریق آدرس اینترنتی",
    plugins_install_url_desc: "آدرس فایل مانیفست یا مخزن را وارد کنید.",
    plugins_install_local_title: "نصب از پوشه محلی",
    plugins_install_local_desc: "مسیر فایل اجرایی یا مانیفست محلی را مشخص کنید.",
    plugins_curated_title: "دایرکتوری برگزیده",
    plugins_curated_desc: "افزونه‌های رسمی و تاییدشده برای Vessel.",
    plugin_builtin: "داخلی",
    plugin_external: "فرآیند خارجی",
    empty_domain_title: "هیچ افزونه فعالی برای این بخش وجود ندارد",
    empty_domain_desc: "برای مشاهده و پخش محتوا در این بخش، یک افزونه اضافه کنید.",
    no_results: "موردی پیدا نشد.",
    resume_empty: "هنوز پخش یا مطالعه‌ای را شروع نکرده‌اید.",
    seasons: "فصل‌ها",
    season: "فصل",
    episodes: "قسمت‌ها",
    episode: "قسمت",
    chapters: "فصل‌ها/بخش‌ها",
    chapter: "بخش",
    streams_title: "منابع پخش و تورنت موجود",
    streams_loading: "در حال دریافت آدرس استریم...",
    no_streams: "در حال حاضر هیچ استریمی یافت نشد.",
    synopsis: "خلاصه داستان",
    theme_mangile: "تیره پیش‌فرض",
    theme_mangile_mauve: "تم بنفش ملایم",
    theme_mangile_stone: "تم سنگ",
    theme_mangile_slate: "تم تخته‌سنگ",
    theme_mangile_neutral: "تم خنثی",
    theme_mangile_zinc: "تم زینک",
    theme_mangile_sunset: "تم غروب",
    theme_mangile_desert: "تم کویر",
    theme_mangile_ice: "تم یخ",
    theme_midnight_blue: "آبی نیمه‌شب",
    theme_ember: "اخگر گرم",
    theme_amethyst: "یاقوت ارغوانی",
    theme_forest: "زمرد جنگل",
    theme_light_clean: "روشن شفاف"
  },
  az: {
    nav_cinema: "Kino",
    nav_reading: "Manqa & E-Kitab",
    nav_live: "Canlı Yayım",
    nav_iptv: "IPTV",
    nav_library: "Kitabxanam",
    nav_plugins: "Qoşmalar",
    nav_settings: "Tənzimləmələr",
    section_continue: "Qaldığınız Yerdən Davam Edin",
    section_discover: "Kəşf Edin",
    search_placeholder: "Film, serial, manqa, anime, IPTV axtar...",
    search_results_title: "Axtarış Nəticələri",
    btn_back: "Geri",
    btn_play: "İzlə",
    btn_read: "Oxu",
    btn_resume: "Davam Et",
    btn_add_library: "+ Kitabxanaya Əlavə Et",
    btn_in_library: "✓ Kitabxanadadır",
    btn_save: "Yadda Saxla",
    btn_install: "Quraşdır",
    btn_install_plugins: "Qoşmaları Kəşf Et",
    btn_uninstall: "Sil",
    btn_enable: "Aktivləşdir",
    btn_disable: "Deaktiv Et",
    btn_active: "Aktiv",
    btn_retry: "Yenidən Yoxla",
    btn_watch_trailer: "Treylerə Bax",
    cast: "Aktyorlar",
    directors: "Rejissor",
    start_live: "Canlı Yayımı Başlat",
    pill_all: "Hamısı",
    pill_movies: "Filmlər",
    pill_series: "Seriallar",
    pill_anime: "Animelər",
    pill_manga: "Manqa",
    pill_webtoon: "Vebtun",
    pill_novel: "Romanlar",
    pill_channels: "Kanallar",
    library_title: "Kolleksiyanız",
    status_watching: "İzlənir / Oxunur",
    status_plan: "Daha Sonra",
    status_completed: "Tamamlandı",
    status_dropped: "Dayandırıldı",
    status_favorites: "Sevimlilər",
    status_all: "Hamısı",
    iptv_guide_title: "Canlı TV / IPTV Bələdçisi",
    iptv_guide_subtitle: "Ölkələrə və kateqoriyalara görə TV kanallarını kəşf edin, anında canlı izləyin.",
    settings_title: "Platforma Tənzimləmələri",
    settings_theme_title: "Mövzu Mühərriki",
    settings_theme_desc: "15 zəngin mövzudan birini seçin və ya rəngləri fərdiləşdirin.",
    settings_debrid_title: "Torrent & Debrid Yayım Mühərriki",
    settings_debrid_desc: "Yüksək sürətli bulud torrent axını üçün Real-Debrid və ya TorBox qoşun.",
    settings_plugins_title: "Qoşulmuş Əlavələr",
    settings_plugins_desc: "Quraşdırılmış kataloq və media təminatçı prosesləri.",
    plugins_title: "Qoşma Mərkəzi",
    plugins_subtitle: "Quraşdırılmış qoşmaları idarə edin və ya yeni kataloq mənbələri əlavə edin.",
    plugins_tab_installed: "Quraşdırılmış Qoşmalar",
    plugins_tab_discover: "Yeni Qoşma Əlavə Et",
    plugins_install_url_title: "URL və ya Repozitoriyadan Quraşdır",
    plugins_install_url_desc: "Quraşdırmaq üçün manifest və ya Git repozitoriya linkini yapışdırın.",
    plugins_install_local_title: "Lokal Qovluqdan Quraşdır",
    plugins_install_local_desc: "Cihazınızdakı lokal qoşma faylını və ya manifest yolunu qeyd edin.",
    plugins_curated_title: "Tövsiyə Edilən Qoşmalar",
    plugins_curated_desc: "Vessel üçün təsdiqlənmiş rəsmi qoşmaları kəşf edin.",
    plugin_builtin: "Daxili (Gömülü)",
    plugin_external: "Xarici Proses",
    empty_domain_title: "Bu bölmə üçün aktiv qoşma yoxdur",
    empty_domain_desc: "Bu bölmədə məzmun kəşf etmək üçün Qoşmalar səhifəsindən qoşma əlavə edin.",
    no_results: "Nəticə tapılmadı.",
    resume_empty: "Hələ heç nə izləməyə və ya oxumağa başlamamısınız.",
    seasons: "Mövsümlər",
    season: "Mövsüm",
    episodes: "Seriyalar",
    episode: "Seriya",
    chapters: "Fəsillər",
    chapter: "Fəsil",
    streams_title: "Mövcud Yayım Mənbələri & Torrentlər",
    streams_loading: "Yayım mənbələri axtarılır...",
    no_streams: "Yayım mənbəyi tapılmadı.",
    no_episode_streams: "Bu seriya üçün yayım mənbəyi tapılmadı.",
    synopsis: "Xülasə"
  }
};

const PLUGIN_I18N = {
  "com.vessel.cinema.cinemasis": {
    en: { name: "Cinemasis", desc: "Official TMDB-based cinema catalog and metadata provider" },
    tr: { name: "Cinemasis", desc: "Resmi TMDB tabanlı sinema kataloğu ve medya sağlayıcısı" },
    az: { name: "Cinemasis", desc: "Rəsmi TMDB əsaslı film və serial kataloqu təminatçısı" },
    de: { name: "Cinemasis", desc: "Offizieller TMDB-basierter Kinokatalog und Metadaten-Anbieter" },
    fr: { name: "Cinemasis", desc: "Fournisseur officiel de catalogue et métadonnées cinéma basé sur TMDB" },
    es: { name: "Cinemasis", desc: "Proveedor oficial de catálogo de cine y metadatos basado en TMDB" },
    ru: { name: "Cinemasis", desc: "Официальный поставщик каталога фильмов и сериалов на базе TMDB" },
    ja: { name: "Cinemasis", desc: "TMDBベースの公式映画・ドラマカタログおよびメタデータプロバイダー" },
    zh: { name: "Cinemasis", desc: "基于 TMDB 的官方影视目录与元数据提供程序" },
    ar: { name: "Cinemasis", desc: "مزود كتالوج وبيانات الأفلام والمسلسلات الرسمي المستند إلى TMDB" },
    pt: { name: "Cinemasis", desc: "Provedor oficial de catálogo de cinema e metadados baseado no TMDB" },
    fa: { name: "Cinemasis", desc: "ارائه‌دهنده رسمی کاتالوگ و فراداده سینما بر پایه TMDB" },
  },
  "com.vessel.reading.mangile": {
    en: { name: "Mangile", desc: "Official embedded reading provider for Manga, Webtoon, and Webook powered by Mangile" },
    tr: { name: "Mangile", desc: "Mangile destekli Manga, Webtoon ve Webook için resmi gömülü okuma sağlayıcısı" },
    az: { name: "Mangile", desc: "Manqa, Vebtun və Vebbuk üçün rəsmi daxili oxuma təminatçısı" },
    de: { name: "Mangile", desc: "Offizieller Lese-Anbieter für Manga, Webtoon und Webook unterstützt von Mangile" },
    fr: { name: "Mangile", desc: "Fournisseur officiel de lecture pour Manga, Webtoon et Webook propulsé par Mangile" },
    es: { name: "Mangile", desc: "Proveedor oficial de lectura de Manga, Webtoon y Webook impulsado por Mangile" },
    ru: { name: "Mangile", desc: "Официальный провайдер чтения манги, вебтунов и веббуков на базе Mangile" },
    ja: { name: "Mangile", desc: "Mangile搭載のマンガ・ウェブトゥーン・ウェブブック公式リーダープロバイダー" },
    zh: { name: "Mangile", desc: "由 Mangile 驱动的官方漫画、网络条漫及网络小说阅读提供程序" },
    ar: { name: "Mangile", desc: "مزود قراءة المانجا والويبتون والكتب الإلكترونية الرسمي المدعوم من Mangile" },
    pt: { name: "Mangile", desc: "Provedor oficial de leitura para Mangá, Webtoon e Webook desenvolvido por Mangile" },
    fa: { name: "Mangile", desc: "ارائه‌دهنده رسمی خواندن مانگا، وبتون و رمان تحت قدرت Mangile" },
  },
  "com.vessel.iptv": {
    en: { name: "IPTV", desc: "Open-source live television channels and streams worldwide (iptv-org)" },
    tr: { name: "IPTV", desc: "Dünya genelinden açık TV yayınları ve kanalları (iptv-org)" },
    az: { name: "IPTV", desc: "Dünya miqyasında açıq canlı televiziya kanalları və yayımları (iptv-org)" },
    de: { name: "IPTV", desc: "Open-Source Live-Fernsehsender und Streams weltweit (iptv-org)" },
    fr: { name: "IPTV", desc: "Chaînes de télévision en direct et flux mondiaux open-source (iptv-org)" },
    es: { name: "IPTV", desc: "Canales de televisión en vivo y transmisiones mundiales de código abierto (iptv-org)" },
    ru: { name: "IPTV", desc: "Открытые прямые трансляции телевизионных каналов со всего мира (iptv-org)" },
    ja: { name: "IPTV", desc: "世界中のオープンソースライブTVチャンネルおよびストリーム（iptv-org）" },
    zh: { name: "IPTV", desc: "全球开源电视频道与在线直播流（iptv-org）" },
    ar: { name: "IPTV", desc: "قنوات تلفزيونية حية وبثوث مفتوحة المصدر من جميع أنحاء العالم (iptv-org)" },
    pt: { name: "IPTV", desc: "Canais de televisão aberta e transmissões ao vivo do mundo todo (iptv-org)" },
    fa: { name: "IPTV", desc: "شبکه‌های تلویزیونی زنده متن‌باز و پخش اینترنتی از سراسر جهان (iptv-org)" },
  }
};

class VesselApp {
  constructor() {
    this.currentRoute = "cinema";
    this.currentDomain = "cinema"; // "cinema" | "reading" | "live" | "iptv"
    this.currentFilter = "all";
    this.activeTheme = null;
    this.currentLocale = "system";
    this.currentDetailsItem = null;
    this.searchDebounceTimer = null;
    this.previousRoute = "cinema";
  }

  async init() {
    await this.loadLocalePreference();
    await this.loadActiveTheme();
    this.bindEvents();
    this.switchRoute("cinema");
  }

  // --- Event Binding ---
  bindEvents() {
    // Navigation routing
    document.querySelectorAll(".nav-item").forEach(btn => {
      btn.addEventListener("click", () => {
        const route = btn.dataset.route;
        this.switchRoute(route);
      });
    });

    // Sidebar collapse toggle
    const toggleBtn = document.getElementById("sidebar-toggle-btn");
    const sidebar = document.getElementById("app-sidebar");
    if (toggleBtn && sidebar) {
      const savedCollapsed = localStorage.getItem("vessel_sidebar_collapsed") === "true";
      if (savedCollapsed) sidebar.classList.add("collapsed");

      toggleBtn.addEventListener("click", () => {
        sidebar.classList.toggle("collapsed");
        localStorage.setItem("vessel_sidebar_collapsed", sidebar.classList.contains("collapsed"));
      });
    }

    // Search input (Full-page multi-domain search)
    const searchInput = document.getElementById("search-input");
    const clearBtn = document.getElementById("clear-search");

    searchInput.addEventListener("input", (e) => {
      const q = e.target.value.trim();
      clearBtn.classList.toggle("hidden", q === "");

      clearTimeout(this.searchDebounceTimer);
      this.searchDebounceTimer = setTimeout(() => {
        if (q === "") {
          this.closeSearchView();
        } else {
          this.openSearchView(q);
        }
      }, 350);
    });

    searchInput.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        const q = searchInput.value.trim();
        if (q) {
          clearTimeout(this.searchDebounceTimer);
          this.openSearchView(q);
        }
      } else if (e.key === "Escape") {
        this.closeSearchView();
      }
    });

    clearBtn.addEventListener("click", () => {
      searchInput.value = "";
      clearBtn.classList.add("hidden");
      this.closeSearchView();
    });

    // Search back button
    document.getElementById("search-back-btn")?.addEventListener("click", () => {
      this.closeSearchView();
    });

    // Search domain filter buttons
    document.querySelectorAll("#search-domain-filters .tab").forEach(tab => {
      tab.addEventListener("click", () => {
        document.querySelectorAll("#search-domain-filters .tab").forEach(t => t.classList.remove("active"));
        tab.classList.add("active");
        this.searchDomainFilter = tab.dataset.searchDomain || "all";
        this.renderSearchResults();
      });
    });

    // Global keyboard shortcuts
    document.addEventListener("keydown", (e) => {
      if (e.key === "/" && document.activeElement !== searchInput) {
        e.preventDefault();
        searchInput.focus();
      } else if (e.key === "Escape") {
        const searchView = document.getElementById("search-view");
        const detailsView = document.getElementById("details-view");
        if (searchView && !searchView.classList.contains("hidden")) {
          this.closeSearchView();
        } else if (detailsView && !detailsView.classList.contains("hidden")) {
          this.closeDetailsView();
        }
      }
    });

    // Details Back Button
    document.getElementById("details-back-btn").addEventListener("click", () => {
      this.closeDetailsView();
    });

    // Empty domain button -> switch to plugins
    document.getElementById("empty-domain-btn").addEventListener("click", () => {
      this.switchRoute("plugins");
    });

    // Locale select
    document.getElementById("locale-select").addEventListener("change", (e) => {
      this.setLocale(e.target.value);
    });

    // Quick theme toggle
    document.getElementById("quick-theme-toggle").addEventListener("click", () => {
      this.cycleTheme();
    });

    // Plugins Tabs
    document.querySelectorAll(".plugins-nav-tabs .tab").forEach(tab => {
      tab.addEventListener("click", () => {
        document.querySelectorAll(".plugins-nav-tabs .tab").forEach(t => t.classList.remove("active"));
        tab.classList.add("active");
        const tabKey = tab.dataset.tab;
        document.getElementById("tab-content-installed").classList.toggle("hidden", tabKey !== "installed");
        document.getElementById("tab-content-discover").classList.toggle("hidden", tabKey !== "discover");
      });
    });

    // Plugin Install Forms
    document.getElementById("plugin-install-url-btn")?.addEventListener("click", () => {
      const input = document.getElementById("plugin-install-url-input");
      if (input && input.value.trim()) {
        this.installPlugin({ url: input.value.trim() });
        input.value = "";
      }
    });

    document.getElementById("plugin-install-path-btn")?.addEventListener("click", () => {
      const input = document.getElementById("plugin-install-path-input");
      if (input && input.value.trim()) {
        this.installPlugin({ path: input.value.trim() });
        input.value = "";
      }
    });

    // Library Tabs
    document.querySelectorAll(".library-tabs .tab").forEach(tab => {
      tab.addEventListener("click", () => {
        document.querySelectorAll(".library-tabs .tab").forEach(t => t.classList.remove("active"));
        tab.classList.add("active");
        this.loadLibraryItems(tab.dataset.status);
      });
    });

    // Debrid save buttons
    document.getElementById("save-rd-btn")?.addEventListener("click", () => {
      this.saveDebrid("realdebrid", document.getElementById("rd-api-key").value);
    });
    document.getElementById("save-tb-btn")?.addEventListener("click", () => {
      this.saveDebrid("torbox", document.getElementById("tb-api-key").value);
    });

    // Multi-Device LAN Sync & Remote Control
    this.setupLanSync();
  }

  // --- Localization (i18n) Engine ---
  async loadLocalePreference() {
    const saved = localStorage.getItem("vessel_locale") || "system";
    this.currentLocale = saved;
    const select = document.getElementById("locale-select");
    if (select) select.value = saved;
    this.applyLocale(saved);
  }

  setLocale(code) {
    this.currentLocale = code;
    localStorage.setItem("vessel_locale", code);
    this.applyLocale(code);
    this.renderCurrentView();
  }

  getEffectiveLocale() {
    if (this.currentLocale === "system") {
      const navLang = (navigator.language || "en").substring(0, 2).toLowerCase();
      return I18N_STRINGS[navLang] ? navLang : "en";
    }
    return I18N_STRINGS[this.currentLocale] ? this.currentLocale : "en";
  }

  applyLocale(code) {
    const target = this.getEffectiveLocale();
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

    // Update nav-item tooltips for collapsed sidebar
    document.querySelectorAll(".nav-item").forEach(btn => {
      const span = btn.querySelector("[data-i18n]");
      if (span && dict[span.dataset.i18n]) {
        btn.setAttribute("data-tooltip", dict[span.dataset.i18n]);
      }
    });

    // Localized search placeholder
    const searchInput = document.getElementById("search-input");
    if (searchInput) {
      searchInput.placeholder = dict.search_placeholder || "Search movies, series, manga, anime...";
    }
  }

  t(key) {
    const target = this.getEffectiveLocale();
    const dict = I18N_STRINGS[target] || I18N_STRINGS.en;
    return dict[key] || I18N_STRINGS.en[key] || key;
  }

  getPluginLocalizedInfo(pluginId, fallbackName = "", fallbackDesc = "") {
    const loc = this.getEffectiveLocale();
    const pDict = PLUGIN_I18N[pluginId];
    if (!pDict) return { title: fallbackName, description: fallbackDesc };
    const exact = pDict[loc] || pDict["en"];
    return {
      title: exact?.name || fallbackName,
      description: exact?.desc || pDict["en"]?.desc || fallbackDesc
    };
  }

  formatTime(seconds) {
    if (isNaN(seconds) || seconds < 0) return "00:00";
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);
    if (h > 0) {
      return `${h}:${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    }
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }

  // --- Theme Engine ---
  async loadActiveTheme() {
    try {
      const res = await fetch("/api/theme/active");
      if (!res.ok) return;
      const data = await res.json();
      this.activeTheme = data;

      if (data.compiled_css) {
        document.getElementById("vessel-theme-vars").innerHTML = data.compiled_css;
      }
      this.updateBranding(data);
    } catch (e) {
      console.warn("Could not load theme:", e);
    }
    await this.renderThemeSelector();
  }

  updateBranding(themeData) {
    const isDark = themeData?.is_dark !== false;
    const logoImg = document.getElementById("brand-logo");
    const faviconLink = document.getElementById("app-favicon");

    if (logoImg) {
      logoImg.src = isDark ? "/assets/logo-light.png" : "/assets/logo-dark.png";
    }
    if (faviconLink) {
      faviconLink.href = isDark ? "/assets/favicon.png" : "/assets/favicon-dark.png";
    }
  }

  async renderThemeSelector() {
    const container = document.getElementById("theme-options");
    if (!container) return;

    try {
      const res = await fetch("/api/themes");
      if (!res.ok) return;
      const data = await res.json();
      const themes = data.themes || (Array.isArray(data) ? data : []);

      container.innerHTML = "";
      themes.forEach(t => {
        const isCurrent = this.activeTheme && (this.activeTheme.theme?.id === t.id || this.activeTheme.id === t.id);
        const item = document.createElement("div");
        item.className = `theme-item ${isCurrent ? "active" : ""}`;

        // Localized theme name
        const transKey = "theme_" + t.id.replace(/-/g, "_");
        const displayName = this.t(transKey) !== transKey ? this.t(transKey) : (t.display_name || t.name);

        const tokens = t.variants?.[0]?.tokens || {};
        const primaryColor = tokens["accent-primary"] || this.getThemeColor(t.id, "primary");
        const surfaceColor = tokens["bg-surface"] || this.getThemeColor(t.id, "surface");
        const baseColor = tokens["bg-base"] || this.getThemeColor(t.id, "base");

        item.innerHTML = `
          <div class="theme-palette">
            <div class="theme-color-swatch" style="background: ${baseColor}"></div>
            <div class="theme-color-swatch" style="background: ${surfaceColor}"></div>
            <div class="theme-color-swatch" style="background: ${primaryColor}"></div>
          </div>
          <div class="theme-name">${displayName}</div>
        `;

        item.addEventListener("click", () => {
          this.applyTheme(t.id);
        });

        container.appendChild(item);
      });
    } catch (e) {
      console.warn("Could not render theme options:", e);
    }
  }

  getThemeColor(themeId, type) {
    const swatches = {
      "vessel-dark": { base: "#0b0f17", surface: "#131a26", primary: "#6366f1" },
      "vessel-light": { base: "#f8fafc", surface: "#ffffff", primary: "#4f46e5" },
      "midnight-oled": { base: "#000000", surface: "#050505", primary: "#06b6d4" },
      "catppuccin": { base: "#1e1e2e", surface: "#181825", primary: "#cba6f7" },
      "nord": { base: "#2e3440", surface: "#3b4252", primary: "#88c0d0" },
      "dracula": { base: "#282a36", surface: "#343746", primary: "#ff79c6" },
      "mangile": { base: "#0c1017", surface: "#131a24", primary: "#ffffff" },
      "mangile-mauve": { base: "#131118", surface: "#1a1722", primary: "#ffffff" },
      "mangile-stone": { base: "#141210", surface: "#1c1917", primary: "#ffffff" },
      "mangile-zinc": { base: "#09090b", surface: "#141417", primary: "#ffffff" },
      "mangile-slate": { base: "#0b1120", surface: "#141d2f", primary: "#ffffff" },
      "mangile-olive": { base: "#0f120e", surface: "#161c15", primary: "#ffffff" },
      "mangile-taupe": { base: "#141211", surface: "#1d1a19", primary: "#ffffff" },
      "mangile-gray": { base: "#111827", surface: "#1f2937", primary: "#ffffff" },
      "mangile-neutral": { base: "#0a0a0a", surface: "#171717", primary: "#ffffff" },
    };
    return swatches[themeId]?.[type] || (type === "primary" ? "#ffffff" : "#171b24");
  }

  async applyTheme(themeId) {
    try {
      const res = await fetch("/api/theme/active", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ theme_id: themeId })
      });
      if (!res.ok) throw new Error("Failed to change theme");
      const data = await res.json();
      this.activeTheme = data;

      if (data.compiled_css) {
        document.getElementById("vessel-theme-vars").innerHTML = data.compiled_css;
      }
      this.updateBranding(data);
      this.renderThemeSelector();
      this.showToast(`Applied theme: ${data.display_name || themeId}`);
    } catch (e) {
      this.showToast(`Error applying theme: ${e.message}`, "error");
    }
  }

  async cycleTheme() {
    const list = ["mangile", "mangile-mauve", "mangile-stone", "midnight-blue", "light-clean"];
    const curr = this.activeTheme?.id || "mangile";
    const nextIdx = (list.indexOf(curr) + 1) % list.length;
    await this.applyTheme(list[nextIdx]);
  }

  // --- Routing & Domain Navigation ---
  switchRoute(route) {
    if (this.currentRoute !== "details") {
      this.previousRoute = this.currentRoute;
    }
    this.currentRoute = route;

    document.querySelectorAll(".nav-item").forEach(btn => {
      btn.classList.toggle("active", btn.dataset.route === route);
    });

    const mainView = document.getElementById("main-view");
    const resumeSection = document.getElementById("resume-section");
    const detailsView = document.getElementById("details-view");
    const searchView = document.getElementById("search-view");
    const iptvView = document.getElementById("iptv-view");
    const libraryView = document.getElementById("library-view");
    const pluginsView = document.getElementById("plugins-view");
    const settingsView = document.getElementById("settings-view");

    // Hide all views first
    mainView.classList.add("hidden");
    if (searchView) searchView.classList.add("hidden");
    if (iptvView) iptvView.classList.add("hidden");
    detailsView.classList.add("hidden");
    libraryView.classList.add("hidden");
    pluginsView.classList.add("hidden");
    settingsView.classList.add("hidden");

    if (route === "iptv") {
      this.currentDomain = "iptv";
      if (iptvView) iptvView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.renderIPTVGuide();
    } else if (route === "cinema" || route === "reading" || route === "live") {
      this.currentDomain = route;
      mainView.classList.remove("hidden");
      resumeSection.classList.remove("hidden");

      // Update Header Title based on domain
      const titleMap = {
        cinema: this.t("nav_cinema"),
        reading: this.t("nav_reading"),
        live: this.t("nav_live"),
      };
      document.getElementById("view-title").textContent = titleMap[route] || this.t("section_discover");

      this.updatePillsForDomain(route);
      this.loadResumeProgress();
      this.loadCatalogsForDomain(route);
    } else if (route === "library") {
      libraryView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.loadLibraryItems("ALL");
    } else if (route === "plugins") {
      pluginsView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.loadPluginsView();
    } else if (route === "settings") {
      settingsView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.loadDebridStatus();
      this.renderThemeSelector();
    }
  }

  // --- Live TV / IPTV Guide Controller ---
  async renderIPTVGuide() {
    if (!this.iptvState) {
      this.iptvState = {
        country: "ALL",
        category: "ALL",
        search: "",
        channels: [],
        activeChannel: null,
      };
      this.bindIPTVControls();
    }
    await this.fetchIPTVChannels();
    this.renderIPTVChannelsList();
  }

  bindIPTVControls() {
    // Country filter pills
    document.querySelectorAll("#iptv-country-filters .filter-pill").forEach(btn => {
      btn.addEventListener("click", async () => {
        document.querySelectorAll("#iptv-country-filters .filter-pill").forEach(b => b.classList.remove("active"));
        btn.classList.add("active");
        this.iptvState.country = btn.dataset.country || "ALL";
        await this.fetchIPTVChannels();
        this.renderIPTVChannelsList();
      });
    });

    // Category filter tabs
    document.querySelectorAll("#iptv-category-filters .tab").forEach(tab => {
      tab.addEventListener("click", () => {
        document.querySelectorAll("#iptv-category-filters .tab").forEach(t => t.classList.remove("active"));
        tab.classList.add("active");
        this.iptvState.category = tab.dataset.category || "ALL";
        this.renderIPTVChannelsList();
      });
    });

    // Search input
    const searchInput = document.getElementById("iptv-search-input");
    if (searchInput) {
      searchInput.addEventListener("input", (e) => {
        this.iptvState.search = e.target.value.trim().toLowerCase();
        this.renderIPTVChannelsList();
      });
    }

    // Fullscreen button on IPTV player
    const fsBtn = document.getElementById("iptv-player-fullscreen-btn");
    if (fsBtn) {
      fsBtn.addEventListener("click", () => {
        const vid = document.getElementById("iptv-video-player");
        if (!vid) return;
        if (document.fullscreenElement) {
          document.exitFullscreen?.().catch(() => {});
        } else {
          vid.requestFullscreen?.().catch(() => {});
        }
      });
    }
  }

  getChannelLogoFallback(name) {
    const clean = (name || "TV").replace(/[^a-zA-Z0-9]/g, "").substring(0, 4).toUpperCase() || "TV";
    return `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="%231e293b"/><text x="50%" y="54%" dominant-baseline="middle" text-anchor="middle" fill="%2338bdf8" font-size="14" font-weight="900" font-family="system-ui, sans-serif">${clean}</text></svg>`;
  }

  async fetchIPTVChannels() {
    const listElem = document.getElementById("iptv-channels-list");
    if (listElem) {
      listElem.innerHTML = `
        <div style="text-align: center; padding: 40px; color: var(--v-text-muted);">
          <div style="font-size: 1.5rem; animation: pulse-online 1.5s infinite; margin-bottom: 8px;">⏳</div>
          <div>${this.t("streams_loading") || "Kanallar yükleniyor..."}</div>
        </div>
      `;
    }

    try {
      let query = "popular";
      if (this.iptvState.country !== "ALL") {
        query = `country:${this.iptvState.country}`;
      }
      const res = await fetch(`/api/search?domain=7&query=${encodeURIComponent(query)}&q=${encodeURIComponent(query)}`);
      if (!res.ok) throw new Error("Could not fetch IPTV channels");
      const data = await res.json();
      this.iptvState.channels = data.items || [];
    } catch (e) {
      if (listElem) listElem.innerHTML = `<div style="padding: 24px; color: var(--v-status-error); text-align: center;">${e.message}</div>`;
    }
  }

  renderIPTVChannelsList() {
    const listElem = document.getElementById("iptv-channels-list");
    if (!listElem) return;

    let list = this.iptvState.channels || [];
    const cat = this.iptvState.category;
    const search = this.iptvState.search;

    if (cat !== "ALL") {
      list = list.filter(ch => {
        const extra = ch.extra || (ch.external_ids ? ch.external_ids.extra : {}) || {};
        const chCat = (extra.category || ch.overview || "").toLowerCase();
        return chCat.includes(cat.toLowerCase());
      });
    }

    if (search) {
      list = list.filter(ch => {
        return (ch.title || "").toLowerCase().includes(search) ||
          (ch.overview || "").toLowerCase().includes(search);
      });
    }

    listElem.innerHTML = "";
    if (list.length === 0) {
      listElem.innerHTML = `
        <div style="text-align: center; padding: 40px; color: var(--v-text-muted);">
          Kriterlere uygun TV kanalı bulunamadı.
        </div>
      `;
      return;
    }

    list.forEach(ch => {
      const extra = ch.extra || (ch.external_ids ? ch.external_ids.extra : {}) || {};
      const country = extra.country || "TV";
      const category = extra.category || "General";
      const quality = extra.quality || "HD";
      const isActive = this.iptvState.activeChannel && this.iptvState.activeChannel.id === ch.id;

      const logoFallback = this.getChannelLogoFallback(ch.title);
      const logoSrc = ch.poster_url || logoFallback;

      const item = document.createElement("div");
      item.className = `iptv-channel-card ${isActive ? "active" : ""}`;
      item.innerHTML = `
        <img class="iptv-channel-logo" src="${logoSrc}" alt="${ch.title}" onerror="this.onerror=null; this.src='${logoFallback}';">
        <div class="iptv-channel-meta">
          <div class="iptv-channel-name">${ch.title}</div>
          <div class="iptv-channel-sub">
            <span class="iptv-tag">${country}</span>
            <span class="iptv-tag">${category}</span>
            <span class="iptv-tag">${quality}</span>
          </div>
        </div>
      `;

      item.addEventListener("click", () => {
        this.playIPTVChannel(ch);
      });

      listElem.appendChild(item);
    });
  }

  async playIPTVChannel(channel) {
    this.iptvState.activeChannel = channel;
    this.renderIPTVChannelsList();

    const titleElem = document.getElementById("iptv-active-title");
    const tagsElem = document.getElementById("iptv-active-tags");
    const emptyElem = document.getElementById("iptv-player-empty");
    const overlay = document.getElementById("iptv-player-overlay");
    const video = document.getElementById("iptv-video-player");

    if (titleElem) titleElem.textContent = channel.title;
    const extra = channel.extra || (channel.external_ids ? channel.external_ids.extra : {}) || {};
    if (tagsElem) {
      tagsElem.innerHTML = `
        <span class="iptv-tag">${extra.country || "TV"}</span>
        <span class="iptv-tag">${extra.category || "General"}</span>
        <span class="iptv-tag">${extra.quality || "HD"}</span>
      `;
    }

    if (emptyElem) emptyElem.classList.add("hidden");
    if (overlay) overlay.classList.remove("hidden");
    const loadText = document.getElementById("iptv-loading-text");
    if (loadText) loadText.textContent = `${channel.title} yayını bağlanıyor...`;

    let streamUrl = extra.stream_url;
    if (!streamUrl) {
      try {
        const res = await fetch(`/api/streams?provider=com.vessel.iptv&media=${encodeURIComponent(channel.id)}`);
        if (res.ok) {
          const data = await res.json();
          if (data.streams && data.streams.length > 0) {
            streamUrl = data.streams[0].url;
          }
        }
      } catch (e) {
        console.warn("Could not fetch streams for IPTV channel:", e);
      }
    }

    if (!streamUrl) {
      if (overlay) overlay.classList.remove("hidden");
      if (loadText) loadText.textContent = "Bu kanal için aktif akış adresi bulunamadı.";
      return;
    }

    if (window.Hls && window.Hls.isSupported() && (streamUrl.includes(".m3u8") || streamUrl.includes("hls") || !streamUrl.endsWith(".mp4"))) {
      if (this.iptvHlsInstance) {
        this.iptvHlsInstance.destroy();
        this.iptvHlsInstance = null;
      }
      const hls = new window.Hls({ enableWorker: true });
      this.iptvHlsInstance = hls;
      hls.loadSource(streamUrl);
      hls.attachMedia(video);
      hls.on(window.Hls.Events.MANIFEST_PARSED, () => {
        if (overlay) overlay.classList.add("hidden");
        video.play().catch(e => console.log("Autoplay:", e));
      });
      hls.on(window.Hls.Events.ERROR, (event, data) => {
        if (data.fatal) {
          if (overlay) overlay.classList.remove("hidden");
          if (loadText) loadText.textContent = "Yayın bağlantısı kurulamadı. Lütfen başka bir kanal seçin.";
        }
      });
    } else {
      video.src = streamUrl;
      video.play().catch(e => console.log("Autoplay:", e));
      video.onloadeddata = () => { if (overlay) overlay.classList.add("hidden"); };
      video.onerror = () => {
        if (overlay) overlay.classList.remove("hidden");
        if (loadText) loadText.textContent = "Yayın oynatılamadı.";
      };
    }
  }

  // --- Vessel Custom Engineered Video Player Modal ---
  openVesselPlayer(streamUrl, meta = {}) {
    const modal = document.getElementById("vessel-player-modal");
    const video = document.getElementById("vessel-video-element");
    const titleElem = document.getElementById("vessel-hud-title");
    const subElem = document.getElementById("vessel-hud-subtitle");
    const hud = document.getElementById("vessel-player-hud");
    const scrubberContainer = document.getElementById("vessel-scrubber-container");
    const scrubberTrack = scrubberContainer.querySelector(".vessel-scrubber-track");
    const scrubberProgress = document.getElementById("vessel-scrubber-progress");
    const scrubberBuffer = document.getElementById("vessel-scrubber-buffer");
    const scrubberThumb = document.getElementById("vessel-scrubber-thumb");
    const scrubberTooltip = document.getElementById("vessel-scrubber-tooltip");
    const curTimeElem = document.getElementById("vessel-current-time");
    const durTimeElem = document.getElementById("vessel-duration-time");
    const playBtn = document.getElementById("vessel-play-btn");
    const playIcon = document.getElementById("vessel-play-icon");
    const pauseIcon = document.getElementById("vessel-pause-icon");
    const centerPlay = document.getElementById("vessel-center-play");
    const centerSpinner = document.getElementById("vessel-center-spinner");
    const rewindBtn = document.getElementById("vessel-rewind-btn");
    const forwardBtn = document.getElementById("vessel-forward-btn");
    const volumeBtn = document.getElementById("vessel-volume-btn");
    const volHigh = document.getElementById("vessel-vol-high");
    const volMuted = document.getElementById("vessel-vol-muted");
    const volSlider = document.getElementById("vessel-volume-slider");
    const audioBtn = document.getElementById("vessel-audio-btn");
    const audioMenu = document.getElementById("vessel-audio-menu");
    const subBtn = document.getElementById("vessel-sub-btn");
    const subMenu = document.getElementById("vessel-sub-menu");
    const qualityBtn = document.getElementById("vessel-quality-btn");
    const qualityMenu = document.getElementById("vessel-quality-menu");
    const speedBtn = document.getElementById("vessel-speed-btn");
    const speedMenu = document.getElementById("vessel-speed-menu");
    const fsBtn = document.getElementById("vessel-fullscreen-btn");
    const fsEnter = document.getElementById("vessel-fs-enter");
    const fsExit = document.getElementById("vessel-fs-exit");
    const pipBtn = document.getElementById("vessel-pip-btn");
    const closeBtn = document.getElementById("vessel-close-player-btn");

    const item = meta.item || {};
    const title = item.title || meta.title || "Cinema";
    const season = meta.season || 0;
    const episode = meta.episode || 0;
    if (titleElem) titleElem.textContent = title;
    if (subElem) subElem.textContent = season > 0 ? `${this.t("season")} ${season} • ${this.t("episode")} ${episode}` : (item.year || "");

    modal.classList.remove("hidden");
    if (centerSpinner) centerSpinner.classList.remove("hidden");

    const swarmBadge = document.getElementById("vessel-swarm-badge");
    if (this.torrentPollInterval) {
      clearInterval(this.torrentPollInterval);
      this.torrentPollInterval = null;
    }

    if (streamUrl && (streamUrl.includes("/stream/torrent") || streamUrl.startsWith("magnet:"))) {
      if (swarmBadge) {
        swarmBadge.classList.remove("hidden");
        swarmBadge.innerHTML = `🧲 P2P Swarm: Bağlantı kuruluyor...`;
      }
      const pollSwarm = async () => {
        try {
          const res = await fetch(`/api/torrent/status?uri=${encodeURIComponent(streamUrl)}`);
          if (res.ok) {
            const data = await res.json();
            if (swarmBadge) {
              const speedMB = ((data.download_speed || 0) / (1024 * 1024)).toFixed(1);
              const progressPct = Math.round((data.progress || 0) * 100);
              swarmBadge.innerHTML = `🧲 P2P: <strong>${data.peers || 0} peers</strong> • ${speedMB} MB/s • ${progressPct}% buffered`;
            }
          }
        } catch (e) {}
      };
      pollSwarm();
      this.torrentPollInterval = setInterval(pollSwarm, 1500);
    } else {
      if (swarmBadge) swarmBadge.classList.add("hidden");
    }

    if (this.vesselHlsInstance) {
      this.vesselHlsInstance.destroy();
      this.vesselHlsInstance = null;
    }

    const setPlayState = (playing) => {
      if (playIcon) playIcon.classList.toggle("hidden", playing);
      if (pauseIcon) pauseIcon.classList.toggle("hidden", !playing);
      if (centerPlay) centerPlay.classList.toggle("hidden", playing);
    };

    // YouTube Stream Detection & Responsive Embed
    const extractYT = (u) => {
      if (!u) return null;
      const m = u.match(/(?:youtu\.be\/|youtube\.com\/(?:watch\?v=|embed\/|live\/|v\/))([a-zA-Z0-9_-]{11})/);
      return m ? m[1] : null;
    };
    const ytId = extractYT(streamUrl);

    let existingYt = document.getElementById("vessel-yt-frame");
    if (existingYt) existingYt.remove();

    if (ytId) {
      const ytFrame = document.createElement("iframe");
      ytFrame.id = "vessel-yt-frame";
      ytFrame.style.cssText = "position:absolute; top:0; left:0; width:100%; height:100%; border:none; z-index:1;";
      ytFrame.allow = "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share";
      ytFrame.allowFullscreen = true;
      ytFrame.src = `https://www.youtube-nocookie.com/embed/${ytId}?autoplay=1&enablejsapi=1&rel=0`;
      modal.insertBefore(ytFrame, modal.firstChild);

      video.classList.add("hidden");
      if (centerSpinner) centerSpinner.classList.add("hidden");
      if (centerPlay) centerPlay.classList.add("hidden");
      if (scrubberContainer) scrubberContainer.style.opacity = "0.2";
    } else {
      video.classList.remove("hidden");
      if (scrubberContainer) scrubberContainer.style.opacity = "1";

      const isHls = streamUrl.includes(".m3u8") || streamUrl.includes("/hls") || meta.format === 1;
      if (isHls && window.Hls && window.Hls.isSupported()) {
        const hls = new window.Hls({ enableWorker: true });
        this.vesselHlsInstance = hls;
        hls.loadSource(streamUrl);
        hls.attachMedia(video);

        hls.on(window.Hls.Events.MANIFEST_PARSED, () => {
          if (centerSpinner) centerSpinner.classList.add("hidden");
          video.play().catch(e => console.log("Autoplay:", e));
          setPlayState(true);

        // Audio Tracks
        if (hls.audioTracks && hls.audioTracks.length > 0 && audioMenu) {
          audioMenu.innerHTML = hls.audioTracks.map((tr, i) => `
            <button class="hud-menu-item ${hls.audioTrack === i ? "active" : ""}" data-track="${i}">
              ${tr.name || tr.lang || `Track ${i + 1}`}
            </button>
          `).join("");
          audioMenu.querySelectorAll(".hud-menu-item").forEach(itemBtn => {
            itemBtn.onclick = (e) => {
              e.stopPropagation();
              const idx = parseInt(itemBtn.dataset.track);
              hls.audioTrack = idx;
              document.getElementById("vessel-audio-label").textContent = itemBtn.textContent.trim();
              audioMenu.querySelectorAll(".hud-menu-item").forEach(b => b.classList.remove("active"));
              itemBtn.classList.add("active");
              audioMenu.classList.add("hidden");
            };
          });
        }

        // Subtitles Tracks
        if (subMenu) {
          let subs = (hls.subtitleTracks || []).map((tr, i) => ({ id: i, label: tr.name || tr.lang || `Sub ${i + 1}`, type: "hls" }));
          if (meta.subtitles && meta.subtitles.length > 0) {
            meta.subtitles.forEach((s, idx) => {
              subs.push({ id: `ext-${idx}`, label: s.title || s.language || `Ext ${idx + 1}`, url: s.url, type: "ext" });
            });
          }

          subMenu.innerHTML = `<button class="hud-menu-item ${hls.subtitleTrack === -1 ? "active" : ""}" data-sub="-1">Off (Kapalı)</button>` +
            subs.map(s => `<button class="hud-menu-item" data-sub="${s.id}">${s.label}</button>`).join("");

          subMenu.querySelectorAll(".hud-menu-item").forEach(itemBtn => {
            itemBtn.onclick = (e) => {
              e.stopPropagation();
              const sid = itemBtn.dataset.sub;
              if (sid === "-1") {
                hls.subtitleTrack = -1;
                document.getElementById("vessel-sub-label").textContent = "Subs";
              } else if (sid.startsWith("ext-")) {
                const extSub = subs.find(s => String(s.id) === sid);
                if (extSub && extSub.url) {
                  const track = document.createElement("track");
                  track.kind = "subtitles";
                  track.label = extSub.label;
                  track.src = extSub.url;
                  track.default = true;
                  video.appendChild(track);
                }
                document.getElementById("vessel-sub-label").textContent = itemBtn.textContent.trim();
              } else {
                hls.subtitleTrack = parseInt(sid);
                document.getElementById("vessel-sub-label").textContent = itemBtn.textContent.trim();
              }
              subMenu.querySelectorAll(".hud-menu-item").forEach(b => b.classList.remove("active"));
              itemBtn.classList.add("active");
              subMenu.classList.add("hidden");
            };
          });
        }

        // Quality Levels
        if (hls.levels && hls.levels.length > 0 && qualityMenu) {
          qualityMenu.innerHTML = `<button class="hud-menu-item active" data-level="-1">Auto</button>` +
            hls.levels.map((lvl, idx) => `<button class="hud-menu-item" data-level="${idx}">${lvl.height ? lvl.height + "p" : `Level ${idx}`}</button>`).join("");
          qualityMenu.querySelectorAll(".hud-menu-item").forEach(itemBtn => {
            itemBtn.onclick = (e) => {
              e.stopPropagation();
              const l = parseInt(itemBtn.dataset.level);
              hls.currentLevel = l;
              document.getElementById("vessel-quality-label").textContent = itemBtn.textContent.trim();
              qualityMenu.querySelectorAll(".hud-menu-item").forEach(b => b.classList.remove("active"));
              itemBtn.classList.add("active");
              qualityMenu.classList.add("hidden");
            };
          });
        }
      });

      hls.on(window.Hls.Events.WAITING, () => { if (centerSpinner) centerSpinner.classList.remove("hidden"); });
      hls.on(window.Hls.Events.PLAYING, () => { if (centerSpinner) centerSpinner.classList.add("hidden"); });
    } else {
      video.src = streamUrl;
      video.play().catch(e => console.log("Autoplay:", e));
      video.onloadeddata = () => {
        if (centerSpinner) centerSpinner.classList.add("hidden");
        setPlayState(true);
      };
    }
  }

    // HUD Auto-hide
    let hudTimeout = null;
    const showHUD = () => {
      hud.classList.remove("inactive");
      modal.style.cursor = "default";
      clearTimeout(hudTimeout);
      hudTimeout = setTimeout(() => {
        if (!video.paused && !document.querySelector(".hud-dropdown-menu:not(.hidden)")) {
          hud.classList.add("inactive");
          modal.style.cursor = "none";
        }
      }, 3000);
    };
    modal.onmousemove = showHUD;
    modal.onclick = showHUD;

    // Scrubber
    video.ontimeupdate = () => {
      if (!video.duration) return;
      const pct = (video.currentTime / video.duration) * 100;
      if (scrubberProgress) scrubberProgress.style.width = `${pct}%`;
      if (scrubberThumb) scrubberThumb.style.left = `${pct}%`;
      if (curTimeElem) curTimeElem.textContent = this.formatTime(video.currentTime);
      if (durTimeElem) durTimeElem.textContent = this.formatTime(video.duration);

      if (video.buffered.length > 0 && scrubberBuffer) {
        const bufEnd = video.buffered.end(video.buffered.length - 1);
        scrubberBuffer.style.width = `${(bufEnd / video.duration) * 100}%`;
      }

      if (video.currentTime > 4 && Math.floor(video.currentTime) % 5 === 0) {
        this.savePlaybackProgressPing(item, season, episode, video.currentTime, video.duration);
      }
    };

    const seekScrubber = (e) => {
      const rect = scrubberTrack.getBoundingClientRect();
      const pos = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      video.currentTime = pos * (video.duration || 0);
    };

    let isScrubbing = false;
    if (scrubberContainer) {
      scrubberContainer.onmousedown = (e) => {
        isScrubbing = true;
        seekScrubber(e);
      };
      window.onmousemove = (e) => {
        if (isScrubbing) seekScrubber(e);
      };
      window.onmouseup = () => { isScrubbing = false; };

      scrubberContainer.onmousemove = (e) => {
        const rect = scrubberTrack.getBoundingClientRect();
        const pos = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
        if (scrubberTooltip) {
          scrubberTooltip.classList.remove("hidden");
          scrubberTooltip.style.left = `${pos * 100}%`;
          scrubberTooltip.textContent = this.formatTime(pos * (video.duration || 0));
        }
      };
      scrubberContainer.onmouseleave = () => {
        if (scrubberTooltip) scrubberTooltip.classList.add("hidden");
      };
    }

    // Play/Pause
    const togglePlay = () => {
      if (video.paused) {
        video.play();
        setPlayState(true);
      } else {
        video.pause();
        setPlayState(false);
      }
    };
    if (playBtn) playBtn.onclick = (e) => { e.stopPropagation(); togglePlay(); };
    if (centerPlay) centerPlay.onclick = (e) => { e.stopPropagation(); togglePlay(); };
    video.onclick = (e) => { e.stopPropagation(); togglePlay(); };

    // Rewind / Forward
    if (rewindBtn) rewindBtn.onclick = (e) => { e.stopPropagation(); video.currentTime = Math.max(0, video.currentTime - 10); };
    if (forwardBtn) forwardBtn.onclick = (e) => { e.stopPropagation(); video.currentTime = Math.min(video.duration || 0, video.currentTime + 10); };

    // Volume
    if (volSlider) {
      volSlider.oninput = (e) => {
        video.volume = parseFloat(e.target.value);
        video.muted = video.volume === 0;
        if (volHigh) volHigh.classList.toggle("hidden", video.muted);
        if (volMuted) volMuted.classList.toggle("hidden", !video.muted);
      };
    }
    if (volumeBtn) {
      volumeBtn.onclick = (e) => {
        e.stopPropagation();
        video.muted = !video.muted;
        if (volHigh) volHigh.classList.toggle("hidden", video.muted);
        if (volMuted) volMuted.classList.toggle("hidden", !video.muted);
        if (volSlider) volSlider.value = video.muted ? 0 : video.volume;
      };
    }

    // Dropdown popovers
    const toggleMenu = (menu) => {
      if (!menu) return;
      const isClosed = menu.classList.contains("hidden");
      document.querySelectorAll(".hud-dropdown-menu").forEach(m => m.classList.add("hidden"));
      if (isClosed) menu.classList.remove("hidden");
    };
    if (audioBtn) audioBtn.onclick = (e) => { e.stopPropagation(); toggleMenu(audioMenu); };
    if (subBtn) subBtn.onclick = (e) => { e.stopPropagation(); toggleMenu(subMenu); };
    if (qualityBtn) qualityBtn.onclick = (e) => { e.stopPropagation(); toggleMenu(qualityMenu); };
    if (speedBtn) speedBtn.onclick = (e) => { e.stopPropagation(); toggleMenu(speedMenu); };

    // Speed options
    if (speedMenu) {
      speedMenu.querySelectorAll(".hud-menu-item").forEach(itemBtn => {
        itemBtn.onclick = (e) => {
          e.stopPropagation();
          const spd = parseFloat(itemBtn.dataset.speed);
          video.playbackRate = spd;
          document.getElementById("vessel-speed-label").textContent = `${spd}x`;
          speedMenu.querySelectorAll(".hud-menu-item").forEach(b => b.classList.remove("active"));
          itemBtn.classList.add("active");
          speedMenu.classList.add("hidden");
        };
      });
    }

    // Fullscreen
    const toggleFullscreen = () => {
      if (document.fullscreenElement) {
        document.exitFullscreen?.().catch(() => {});
        if (fsEnter) fsEnter.classList.remove("hidden");
        if (fsExit) fsExit.classList.add("hidden");
      } else {
        modal.requestFullscreen?.().catch(() => {});
        if (fsEnter) fsEnter.classList.add("hidden");
        if (fsExit) fsExit.classList.remove("hidden");
      }
    };
    if (fsBtn) fsBtn.onclick = (e) => { e.stopPropagation(); toggleFullscreen(); };
    video.ondblclick = (e) => { e.stopPropagation(); toggleFullscreen(); };

    // PiP
    if (pipBtn) {
      pipBtn.onclick = (e) => {
        e.stopPropagation();
        if (document.pictureInPictureElement) {
          document.exitPictureInPicture?.().catch(() => {});
        } else {
          video.requestPictureInPicture?.().catch(() => {});
        }
      };
    }

    // Close Player
    const closePlayer = () => {
      this.savePlaybackProgressPing(item, season, episode, video.currentTime, video.duration);
      video.pause();
      video.src = "";
      video.classList.remove("hidden");
      const ytF = document.getElementById("vessel-yt-frame");
      if (ytF) ytF.remove();
      if (scrubberContainer) scrubberContainer.style.opacity = "1";
      if (this.vesselHlsInstance) {
        this.vesselHlsInstance.destroy();
        this.vesselHlsInstance = null;
      }
      if (document.fullscreenElement) {
        document.exitFullscreen?.().catch(() => {});
      }
      if (this.torrentPollInterval) {
        clearInterval(this.torrentPollInterval);
        this.torrentPollInterval = null;
      }
      const swarmBadge = document.getElementById("vessel-swarm-badge");
      if (swarmBadge) swarmBadge.classList.add("hidden");
      modal.classList.add("hidden");
      window.removeEventListener("keydown", keyHandler);
      this.loadResumeProgress();
    };
    if (closeBtn) closeBtn.onclick = (e) => { e.stopPropagation(); closePlayer(); };

    // Keyboard Shortcuts
    const keyHandler = (e) => {
      if (modal.classList.contains("hidden")) return;
      if (e.key === " " || e.key === "k") {
        e.preventDefault();
        togglePlay();
      } else if (e.key === "ArrowLeft" || e.key === "j") {
        e.preventDefault();
        video.currentTime = Math.max(0, video.currentTime - 10);
      } else if (e.key === "ArrowRight" || e.key === "l") {
        e.preventDefault();
        video.currentTime = Math.min(video.duration || 0, video.currentTime + 10);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        video.volume = Math.min(1, video.volume + 0.1);
        if (volSlider) volSlider.value = video.volume;
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        video.volume = Math.max(0, video.volume - 0.1);
        if (volSlider) volSlider.value = video.volume;
      } else if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        toggleFullscreen();
      } else if (e.key === "m" || e.key === "M") {
        e.preventDefault();
        video.muted = !video.muted;
        if (volHigh) volHigh.classList.toggle("hidden", video.muted);
        if (volMuted) volMuted.classList.toggle("hidden", !video.muted);
      } else if (e.key === "Escape") {
        if (!document.fullscreenElement) {
          closePlayer();
        }
      }
    };
    window.addEventListener("keydown", keyHandler);
  }

  savePlaybackProgressPing(item, season, episode, currentTime, duration) {
    if (!item || !item.id) return;
    const title = item.title || "";
    const poster = item.poster_url || "";
    fetch("/api/progress/playback", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        provider_id: item.provider_id || "com.vessel.cinema.cinemasis",
        media_id: item.id,
        title: title,
        poster_url: poster,
        domain: 1,
        season_number: season,
        episode_number: episode,
        current_position: currentTime,
        total_duration: duration || 0,
        progress_percent: duration > 0 ? (currentTime / duration) * 100 : 0,
        is_completed: duration > 0 && (currentTime / duration) >= 0.9
      })
    }).catch(() => {});
  }


  renderCurrentView() {
    if (this.currentRoute === "cinema" || this.currentRoute === "reading" || this.currentRoute === "live" || this.currentRoute === "iptv") {
      this.switchRoute(this.currentRoute);
    } else if (this.currentRoute === "library") {
      this.loadLibraryItems("WATCHING");
    } else if (this.currentRoute === "plugins") {
      this.loadPluginsView();
    } else if (this.currentRoute === "settings") {
      this.renderThemeSelector();
    }
  }

  updatePillsForDomain(domain) {
    const container = document.getElementById("domain-pills");
    if (!container) return;

    if (domain === "reading") {
      container.innerHTML = `
        <button class="pill active" data-filter="all">${this.t("pill_all")}</button>
        <button class="pill" data-filter="manga">${this.t("pill_manga")}</button>
        <button class="pill" data-filter="webtoon">${this.t("pill_webtoon")}</button>
        <button class="pill" data-filter="novel">${this.t("pill_novel")}</button>
      `;
    } else if (domain === "cinema") {
      container.innerHTML = `
        <button class="pill active" data-filter="all">${this.t("pill_all")}</button>
        <button class="pill" data-filter="movie">${this.t("pill_movies")}</button>
        <button class="pill" data-filter="series">${this.t("pill_series")}</button>
        <button class="pill" data-filter="anime">${this.t("pill_anime")}</button>
      `;
    } else if (domain === "live" || domain === "iptv") {
      container.innerHTML = `
        <button class="pill active" data-filter="all">${this.t("pill_all")}</button>
        <button class="pill" data-filter="channels">${this.t("pill_channels")}</button>
      `;
    } else {
      container.innerHTML = "";
    }

    this.currentFilter = "all";
    container.querySelectorAll(".pill").forEach(pill => {
      pill.addEventListener("click", () => {
        container.querySelectorAll(".pill").forEach(p => p.classList.remove("active"));
        pill.classList.add("active");
        this.currentFilter = pill.dataset.filter;
        this.filterSwiperItems();
      });
    });
  }

  localizeCatalogTitle(catRow) {
    if (!catRow) return "Catalog";
    const type = catRow.catalog_type || "";
    const id = catRow.id || "";
    const dom = catRow.domain || (this.currentDomain === "reading" ? 2 : (this.currentDomain === "iptv" ? 7 : 1));
    const provName = catRow.provider_name || "Vessel";

    let label = "";
    if (dom === 1 || dom === "cinema") {
      if (type === "popular" || id.includes("-popular")) label = this.t("cat_popular_movies");
      else if (type === "top_rated" || id.includes("-toprated")) label = this.t("cat_top_rated_movies");
      else if (type === "featured" || type === "trending" || id.includes("-featured")) label = this.t("cat_trending_movies");
    } else if (dom === 2 || dom === "reading" || dom === 4 || dom === 5 || dom === 6) {
      if (type === "popular" || id.includes("-popular")) label = this.t("cat_popular_contents");
      else if (type === "latest" || id.includes("-latest")) label = this.t("cat_latest_contents");
    } else if (dom === 7 || dom === "iptv") {
      if (type === "popular" || id.includes("-popular")) label = this.t("cat_popular_channels");
      else if (id.includes("-news")) label = this.t("cat_news_info");
      else if (id.includes("-sports")) label = this.t("cat_sports_live");
    }

    if (!label) {
      label = catRow.title || "Catalog";
      if (label.includes(" • ")) {
        label = label.split(" • ")[1];
      }
    }

    return `${provName} • ${label}`;
  }

  // --- Modern Streaming Swipers / Catalog Rows ---
  async loadCatalogsForDomain(domain) {
    const container = document.getElementById("catalogs-container");
    const emptyNotice = document.getElementById("empty-domain-notice");
    container.innerHTML = `
      <div style="text-align: center; padding: 60px; color: var(--v-text-muted);">
        <div style="font-size: 1.8rem; margin-bottom: 12px; animation: pulse-online 1.5s infinite;">⏳</div>
        <div>Loading catalogs...</div>
      </div>
    `;

    try {
      const res = await fetch(`/api/catalogs?domain=${encodeURIComponent(domain)}`);
      if (!res.ok) throw new Error("Failed to load catalogs");
      const data = await res.json();
      const catalogs = data.catalogs || [];

      // Sort catalogs by user-defined plugin priority order if saved
      const rawOrder = localStorage.getItem("vessel_plugin_order");
      if (rawOrder) {
        try {
          const pluginOrder = JSON.parse(rawOrder);
          if (Array.isArray(pluginOrder) && pluginOrder.length > 0) {
            catalogs.sort((a, b) => {
              const idxA = pluginOrder.indexOf(a.provider_id);
              const idxB = pluginOrder.indexOf(b.provider_id);
              const wA = idxA === -1 ? 999 : idxA;
              const wB = idxB === -1 ? 999 : idxB;
              return wA - wB;
            });
          }
        } catch (_) {}
      }

      container.innerHTML = "";

      if (catalogs.length === 0) {
        emptyNotice.classList.remove("hidden");
        document.getElementById("empty-domain-title").textContent = this.t("empty_domain_title");
        document.getElementById("empty-domain-desc").textContent = this.t("empty_domain_desc");
        return;
      }

      emptyNotice.classList.add("hidden");
      this.currentCatalogs = catalogs;

      catalogs.forEach((catRow, idx) => {
        const rowElem = document.createElement("div");
        rowElem.className = "catalog-row";
        rowElem.id = `catalog-row-${idx}`;

        rowElem.innerHTML = `
          <div class="catalog-header">
            <div class="catalog-title-group">
              <h3 class="catalog-title">${this.localizeCatalogTitle(catRow)}</h3>
              <span class="catalog-provider-badge">${catRow.provider_name || "Vessel"}</span>
            </div>
            <div class="catalog-nav-controls">
              <button class="swiper-nav-btn swiper-prev" data-target="swiper-${idx}" title="Previous">
                <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2.5" fill="none">
                  <polyline points="15 18 9 12 15 6"></polyline>
                </svg>
              </button>
              <button class="swiper-nav-btn swiper-next" data-target="swiper-${idx}" title="Next">
                <svg viewBox="0 0 24 24" width="16" height="16" stroke="currentColor" stroke-width="2.5" fill="none">
                  <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
              </button>
            </div>
          </div>
          <div class="catalog-swiper-container">
            <div class="catalog-swiper" id="swiper-${idx}">
              <!-- Cards dynamically inserted -->
            </div>
          </div>
        `;

        const swiper = rowElem.querySelector(`#swiper-${idx}`);
        (catRow.items || []).forEach(item => {
          const card = this.createMediaCard(item);
          swiper.appendChild(card);
        });

        // Setup smooth scroll controls
        const prevBtn = rowElem.querySelector(".swiper-prev");
        const nextBtn = rowElem.querySelector(".swiper-next");

        prevBtn.addEventListener("click", () => {
          swiper.scrollBy({ left: -420, behavior: "smooth" });
        });
        nextBtn.addEventListener("click", () => {
          swiper.scrollBy({ left: 420, behavior: "smooth" });
        });

        container.appendChild(rowElem);
      });
    } catch (e) {
      container.innerHTML = `
        <div style="text-align: center; padding: 50px; color: var(--v-status-error);">
          ${e.message}
        </div>
      `;
    }
  }

  filterSwiperItems() {
    if (!this.currentCatalogs) return;
    const filter = this.currentFilter.toLowerCase();

    document.querySelectorAll(".catalog-swiper .media-card").forEach(card => {
      if (filter === "all") {
        card.style.display = "";
      } else {
        const typeStr = (card.dataset.type || "").toLowerCase();
        card.style.display = typeStr.includes(filter) ? "" : "none";
      }
    });
  }

  // --- Dedicated Full-Page Multi-Domain Search System ---
  async openSearchView(query) {
    if (!query) return;
    this.currentSearchQuery = query;
    this.searchDomainFilter = this.searchDomainFilter || "all";

    const searchView = document.getElementById("search-view");
    const mainView = document.getElementById("main-view");
    const resumeSection = document.getElementById("resume-section");
    const libraryView = document.getElementById("library-view");
    const pluginsView = document.getElementById("plugins-view");
    const settingsView = document.getElementById("settings-view");
    const detailsView = document.getElementById("details-view");

    if (mainView) mainView.classList.add("hidden");
    if (resumeSection) resumeSection.classList.add("hidden");
    if (libraryView) libraryView.classList.add("hidden");
    if (pluginsView) pluginsView.classList.add("hidden");
    if (settingsView) settingsView.classList.add("hidden");
    if (detailsView) detailsView.classList.add("hidden");
    if (searchView) searchView.classList.remove("hidden");

    window.scrollTo({ top: 0, behavior: "smooth" });

    const queryTitle = document.getElementById("search-view-query-title");
    const countEl = document.getElementById("search-view-count");
    const grid = document.getElementById("search-full-grid");
    const emptyNotice = document.getElementById("search-empty-notice");

    if (queryTitle) queryTitle.textContent = `${this.t("search_results_title")}: "${query}"`;
    if (countEl) countEl.textContent = "Tüm aktif eklentiler taranıyor (Sinema, Manga, IPTV)...";
    if (emptyNotice) emptyNotice.classList.add("hidden");

    if (grid) {
      grid.innerHTML = `
        <div style="grid-column: 1/-1; text-align: center; padding: 60px; color: var(--v-text-muted);">
          <div style="font-size: 2rem; margin-bottom: 12px; animation: pulse-online 1.5s infinite;">⏳</div>
          <div style="font-size: 1rem;">Tüm sağlayıcılarda aranıyor...</div>
        </div>
      `;
    }

    try {
      // Query multi-domain search concurrently across all active providers (Cinema, Reading, IPTV)
      const res = await fetch(`/api/search?domain=all&query=${encodeURIComponent(query)}`);
      if (!res.ok) throw new Error("Arama sorgusu başarısız oldu");
      const data = await res.json();
      this.allSearchResults = data.items || [];
      const rawOrder = localStorage.getItem("vessel_plugin_order");
      if (rawOrder) {
        try {
          const pluginOrder = JSON.parse(rawOrder);
          if (Array.isArray(pluginOrder) && pluginOrder.length > 0) {
            this.allSearchResults.sort((a, b) => {
              const idxA = pluginOrder.indexOf(a.provider_id);
              const idxB = pluginOrder.indexOf(b.provider_id);
              const wA = idxA === -1 ? 999 : idxA;
              const wB = idxB === -1 ? 999 : idxB;
              return wA - wB;
            });
          }
        } catch (_) {}
      }
      this.renderSearchResults();
    } catch (e) {
      if (grid) grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-status-error); padding: 40px;">${e.message}</div>`;
      if (countEl) countEl.textContent = "Arama hatası";
    }
  }

  renderSearchResults() {
    const grid = document.getElementById("search-full-grid");
    const countEl = document.getElementById("search-view-count");
    const emptyNotice = document.getElementById("search-empty-notice");
    if (!grid) return;

    const all = this.allSearchResults || [];
    const filter = this.searchDomainFilter || "all";

    let filtered = all;
    if (filter === "cinema") {
      filtered = all.filter(it => it.domain === 1 || it.type === 1 || it.type === 2 || it.type === 3 || it.type_name === "Movie" || it.type_name === "Series" || it.type_name === "Anime");
    } else if (filter === "reading") {
      filtered = all.filter(it => it.domain === 2 || it.type === 4 || it.type === 5 || it.type === 6 || it.type_name === "Manga" || it.type_name === "Webtoon" || it.type_name === "Novel");
    } else if (filter === "iptv") {
      filtered = all.filter(it => it.domain === 7 || it.type === 7 || it.type_name === "IPTV" || (it.id && (it.id.startsWith("tr-") || it.id.startsWith("intl-"))));
    }

    grid.innerHTML = "";
    if (countEl) {
      countEl.textContent = `${filtered.length} sonuç bulundu (${all.length} toplam)`;
    }

    if (filtered.length === 0) {
      if (emptyNotice) emptyNotice.classList.remove("hidden");
      return;
    }

    if (emptyNotice) emptyNotice.classList.add("hidden");
    filtered.forEach(it => {
      const card = this.createMediaCard(it);
      grid.appendChild(card);
    });
  }

  closeSearchView() {
    const searchView = document.getElementById("search-view");
    if (searchView) searchView.classList.add("hidden");
    const searchInput = document.getElementById("search-input");
    const clearBtn = document.getElementById("clear-search");
    if (searchInput) searchInput.value = "";
    if (clearBtn) clearBtn.classList.add("hidden");

    this.switchRoute(this.currentRoute || "cinema");
  }

  // --- Media Card Component ---
  createMediaCard(item) {
    const card = document.createElement("div");
    card.className = "media-card";

    // Guaranteed robust property extraction: no undefined values
    let rawTitle = item.title || item.Title || item.name || item.Name || "";
    if (!rawTitle || rawTitle === "undefined") {
      rawTitle = "Unknown Title";
    }

    let poster = item.poster_url || item.PosterURL || "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400";
    if (poster === "undefined") {
      poster = "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400";
    }

    const typeVal = item.type !== undefined ? item.type : (item.Type !== undefined ? item.Type : 1);
    const typeLabel = item.type_name || this.mapMediaType(typeVal);
    const isIPTV = typeVal === 7 || item.domain === 7 || item.type_name === "IPTV" || (item.id && (item.id.startsWith("tr-") || item.id.startsWith("intl-")));
    const isReading = !isIPTV && (this.currentDomain === "reading" || (typeVal >= 4 && typeVal <= 6));
    const year = item.year || item.Year || (isReading || isIPTV ? "" : "2024");
    const rating = item.rating || (isIPTV ? "HD" : 8.5);

    card.dataset.type = typeLabel;

    const overlayIcon = isIPTV ? "📡" : (isReading ? "📖" : "▶");

    card.innerHTML = `
      <div class="poster-wrapper">
        <img src="${poster}" alt="${rawTitle}" class="poster-img" loading="lazy" style="${isIPTV ? 'object-fit: contain; background: #000; padding: 12px;' : ''}" onerror="this.src='https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400'">
        <span class="card-badge" style="${isIPTV ? 'background: rgba(239, 68, 68, 0.9); color: #fff;' : ''}">${typeLabel}</span>
        <div class="poster-overlay-btn">${overlayIcon}</div>
      </div>
      <div class="card-details">
        <div class="card-title" title="${rawTitle}">${rawTitle}</div>
        <div class="card-meta">
          <span>${year || (isIPTV ? "Canlı TV" : "")}</span>
          <span class="rating-badge">${isIPTV ? "🔴 CANLI" : `★ ${rating}`}</span>
        </div>
      </div>
    `;

    card.addEventListener("click", () => {
      this.openDetailsView({
        id: item.id || item.ID,
        provider_id: item.provider_id || item.ProviderID,
        title: rawTitle,
        poster_url: poster,
        year: year,
        type: typeVal,
        type_name: typeLabel,
        domain: item.domain || (isIPTV ? 7 : (isReading ? 2 : 1)),
        overview: item.overview || item.Overview || "",
        external_ids: item.external_ids || item.ExternalIDs
      });
    });

    return card;
  }

  mapMediaType(type) {
    const map = {
      1: "Movie",
      2: "Series",
      3: "Anime",
      4: "Manga",
      5: "Webtoon",
      6: "Novel",
      7: "IPTV"
    };
    return map[type] || "Media";
  }

  // --- Full-Page Details View (NO MODALS / POPUPS!) ---
  async openDetailsView(item) {
    this.currentDetailsItem = item;
    const detailsView = document.getElementById("details-view");
    const content = document.getElementById("details-content");

    // Hide other views
    document.getElementById("main-view").classList.add("hidden");
    document.getElementById("resume-section").classList.add("hidden");
    document.getElementById("library-view").classList.add("hidden");
    document.getElementById("plugins-view").classList.add("hidden");
    document.getElementById("settings-view").classList.add("hidden");
    const searchView = document.getElementById("search-view");
    if (searchView) searchView.classList.add("hidden");

    detailsView.classList.remove("hidden");
    window.scrollTo({ top: 0, behavior: "smooth" });

    content.innerHTML = `
      <div style="text-align: center; padding: 100px; color: var(--v-text-muted);">
        <div style="font-size: 2.2rem; margin-bottom: 12px; animation: pulse-online 1.5s infinite;">⏳</div>
        <div>Loading details...</div>
      </div>
    `;

    try {
      const isIPTV = item.domain === 7 || item.type === 7 || item.type_name === "IPTV" || (item.id && (item.id.startsWith("tr-") || item.id.startsWith("intl-"))) || (item.provider_id === "com.vessel.iptv");
      const isReading = !isIPTV && (this.currentDomain === "reading" || (item.type >= 4 && item.type <= 6));

      let domainNum = 1;
      if (isIPTV) domainNum = 7;
      else if (isReading) domainNum = 2;

      const res = await fetch(`/api/media?domain=${domainNum}&provider=${encodeURIComponent(item.provider_id || "")}&id=${encodeURIComponent(item.id)}`);
      const details = res.ok ? (await res.json()) : item;

      if (isIPTV) {
        this.renderIPTVDetails(item, details);
      } else if (isReading) {
        this.renderReadingDetails(item, details);
      } else {
        this.renderCinemaDetails(item, details);
      }
    } catch (e) {
      content.innerHTML = `<div style="text-align: center; padding: 60px; color: var(--v-status-error);">Failed to load metadata: ${e.message}</div>`;
    }
  }

  closeDetailsView() {
    document.getElementById("details-view").classList.add("hidden");
    const video = document.getElementById("vessel-video-player");
    if (video) {
      video.pause();
      video.src = "";
    }
    if (this.currentHlsInstance) {
      this.currentHlsInstance.destroy();
      this.currentHlsInstance = null;
    }
    const trailerBox = document.getElementById("inline-trailer-box");
    const iframe = document.getElementById("trailer-iframe");
    if (trailerBox && iframe) {
      iframe.src = "";
      trailerBox.classList.add("hidden");
    }
    this.switchRoute(this.previousRoute || "cinema");
  }

  async setupDetailsLibraryControl(item, details) {
    const select = document.getElementById("details-lib-status");
    if (!select) return;

    const provider = item.provider_id || item.ProviderID || "";
    const mediaId = item.id || item.ID || "";

    try {
      const res = await fetch("/api/library?status=ALL");
      if (res.ok) {
        const data = await res.json();
        const match = (data.items || []).find(it => it.media_id === mediaId && (!provider || it.provider_id === provider));
        if (match && match.status) {
          select.value = match.status;
        }
      }
    } catch (e) {
      console.warn("Could not check library status:", e);
    }

    select.onchange = async () => {
      const chosenStatus = select.value;
      if (chosenStatus === "NONE") {
        try {
          await fetch(`/api/library?provider=${encodeURIComponent(provider)}&media=${encodeURIComponent(mediaId)}`, {
            method: "DELETE"
          });
          this.showToast("Koleksiyondan kaldırıldı", "info");
        } catch (err) {
          this.showToast(err.message, "error");
        }
      } else {
        try {
          const isReading = this.currentDomain === "reading" || item.type >= 4;
          const res = await fetch("/api/library", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              provider_id: provider,
              media_id: mediaId,
              domain: isReading ? 2 : (item.type === 7 ? 3 : 1),
              title: details.title || item.title,
              type: item.type || 1,
              poster_url: details.poster_url || item.poster_url,
              status: chosenStatus,
              user_rating: 8.5
            })
          });
          if (!res.ok) throw new Error("Koleksiyon güncellenemedi");
          this.showToast("Koleksiyon güncellendi!");
        } catch (err) {
          this.showToast(err.message, "error");
        }
      }
    };
  }

  renderCinemaDetails(item, details) {
    const content = document.getElementById("details-content");
    const title = details.title || item.title || "Unknown";
    const poster = details.poster_url || item.poster_url;
    const year = details.year || item.year || 2024;
    const genres = details.genres || ["Cinema", "Drama"];

    const extra = (details.external_ids && details.external_ids.extra) || (item.external_ids && item.external_ids.extra) || {};
    const backdrop = extra.backdrop_url || details.backdrop_url || poster;
    const tagline = extra.tagline || "";
    const rating = extra.rating ? extra.rating : (details.vote_average ? details.vote_average.toFixed(1) : "8.5");
    const voteCount = extra.vote_count ? `(${Number(extra.vote_count).toLocaleString()} oy)` : "";
    const runtime = extra.runtime ? `${extra.runtime} dk` : "";
    const status = extra.status || "";
    const directors = extra.directors || "";
    const trailerKey = extra.trailer_key || "";

    let castList = [];
    if (extra.cast_json) {
      try {
        castList = JSON.parse(extra.cast_json);
      } catch (e) {
        console.warn("Failed to parse cast_json", e);
      }
    }

    let seasons = [];
    if (extra.seasons_json) {
      try {
        seasons = JSON.parse(extra.seasons_json);
      } catch (e) {
        console.warn("Failed to parse seasons_json", e);
      }
    }
    if (!seasons || seasons.length === 0) {
      seasons = details.seasons || [];
    }

    const isSeries = seasons.length > 0 || item.type === 2 || item.type === 3 || details.type === 2 || details.type === 3;

    content.innerHTML = `
      <div class="details-hero">
        <div class="details-hero-backdrop" style="background-image: url('${backdrop}')"></div>
        <div class="details-poster-col">
          <img class="details-poster-img" src="${poster}" alt="${title}">
          <div class="details-lib-box">
            <select id="details-lib-status" class="details-lib-select">
              <option value="NONE">➕ ${this.t("btn_add_library") || "Koleksiyona Ekle"}</option>
              <option value="WATCHING">▶ ${this.t("status_watching") || "İzleniyor"}</option>
              <option value="PLAN_TO_WATCH">🕒 ${this.t("status_plan") || "Daha Sonra"}</option>
              <option value="COMPLETED">✓ ${this.t("status_completed") || "Tamamlandı"}</option>
              <option value="DROPPED">⏹ ${this.t("status_dropped") || "Bırakıldı"}</option>
              <option value="FAVORITE">★ ${this.t("status_favorites") || "Favoriler"}</option>
            </select>
          </div>
        </div>
        <div class="details-info-col">
          <div class="details-meta-tags">
            <span class="badge-subtle" style="background: rgba(var(--v-accent-primary-rgb, 255,255,255), 0.15); color: var(--v-accent-primary); font-weight: 700;">
              ${this.mapMediaType(item.type)}
            </span>
            <span class="badge-subtle">★ ${rating} ${voteCount}</span>
            <span class="badge-subtle">${year}</span>
            ${runtime ? `<span class="badge-subtle">${runtime}</span>` : ""}
            ${status ? `<span class="badge-subtle">${status}</span>` : ""}
            ${genres.map(g => `<span class="badge-subtle">${g}</span>`).join("")}
          </div>
          <h1 class="details-title">${title}</h1>
          ${tagline ? `<p class="details-tagline" style="font-style: italic; color: var(--v-text-muted); font-size: 1.05rem; margin-bottom: 10px;">"${tagline}"</p>` : ""}
          <p class="details-overview">${details.overview || item.overview || "No overview available."}</p>
          ${directors ? `<div class="details-crew" style="margin-top: 10px; font-size: 0.9rem; color: var(--v-text-secondary);"><span class="crew-item"><strong>${this.t("directors")}:</strong> ${directors}</span></div>` : ""}
          <div class="details-actions" style="margin-top: 18px; display: flex; gap: 12px; flex-wrap: wrap;">
            <button class="btn btn-primary" id="details-watch-now-btn" style="font-size: 1rem; padding: 10px 24px;">
              ▶ ${this.t("btn_play")}
            </button>
            ${trailerKey ? `
              <button class="btn btn-secondary" id="details-trailer-btn" style="font-size: 1rem; padding: 10px 20px;">
                🎬 ${this.t("btn_watch_trailer")}
              </button>
            ` : ""}
          </div>
        </div>
      </div>

      <div class="details-subsections">
        <!-- Embedded Trailer Container -->
        <div id="inline-trailer-box" class="details-section-box hidden">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
            <h3 class="details-section-title" style="margin: 0;">🎬 ${this.t("btn_watch_trailer")}</h3>
            <button class="btn btn-secondary" id="close-trailer-btn" style="padding: 4px 12px;">✕ Kapat</button>
          </div>
          <div style="position: relative; padding-bottom: 56.25%; height: 0; overflow: hidden; border-radius: 8px; background: #000;">
            <iframe id="trailer-iframe" style="position: absolute; top: 0; left: 0; width: 100%; height: 100%; border: 0;" allow="accelerometer; autoplay; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>
          </div>
        </div>

        <!-- Interactive Player Container -->
        <div id="inline-player-box" class="details-section-box hidden">
          <div class="player-container">
            <video id="vessel-video-player" controls autoplay style="width: 100%; height: 500px; max-height: 70vh; background: #000; border-radius: 8px;"></video>
          </div>
        </div>

        <!-- Cast Carousel -->
        ${castList.length > 0 ? `
          <div class="details-section-box">
            <h3 class="details-section-title">${this.t("cast")}</h3>
            <div class="cast-carousel">
              ${castList.map(c => `
                <div class="cast-card">
                  <img class="cast-avatar" src="${c.profile_url || '/assets/vessel_primary.png'}" alt="${c.name}" onerror="this.src='/assets/vessel_primary.png'">
                  <div class="cast-name" title="${c.name}">${c.name}</div>
                  <div class="cast-character" title="${c.character}">${c.character || ""}</div>
                </div>
              `).join("")}
            </div>
          </div>
        ` : ""}

        <!-- Episodes List for Series -->
        ${isSeries && seasons.length > 0 ? `
          <div class="details-section-box">
            <h3 class="details-section-title">${this.t("episodes")}</h3>
            <div class="season-tabs" id="details-season-tabs">
              ${seasons.map((s, idx) => `
                <button class="tab ${idx === 0 ? "active" : ""}" data-season-idx="${idx}">
                  ${this.t("season")} ${s.season_number || (idx + 1)}
                </button>
              `).join("")}
            </div>
            <div class="episodes-detailed-list" id="details-episodes-grid">
              <!-- Rendered for current season -->
            </div>
          </div>
        ` : ""}

        <!-- Stream Sources / Debrid Section -->
        <div class="details-section-box">
          <h3 class="details-section-title">${this.t("streams_title")}</h3>
          <div class="streams-grid" id="details-streams-grid">
            <div style="color: var(--v-text-muted); padding: 12px;">${this.t("streams_loading")}</div>
          </div>
        </div>
      </div>
    `;

    // Library status controller
    this.setupDetailsLibraryControl(item, details);

    // Trailer toggle
    if (trailerKey) {
      document.getElementById("details-trailer-btn")?.addEventListener("click", () => {
        const trailerBox = document.getElementById("inline-trailer-box");
        const iframe = document.getElementById("trailer-iframe");
        if (trailerBox && iframe) {
          iframe.src = `https://www.youtube-nocookie.com/embed/${trailerKey}?autoplay=1`;
          trailerBox.classList.remove("hidden");
          trailerBox.scrollIntoView({ behavior: "smooth" });
        }
      });

      document.getElementById("close-trailer-btn")?.addEventListener("click", () => {
        const trailerBox = document.getElementById("inline-trailer-box");
        const iframe = document.getElementById("trailer-iframe");
        if (trailerBox && iframe) {
          iframe.src = "";
          trailerBox.classList.add("hidden");
        }
      });
    }

    // Default season & episode
    this.selectedSeason = isSeries && seasons.length > 0 ? (seasons[0].season_number || 1) : 1;
    this.selectedEpisode = isSeries && seasons.length > 0 && seasons[0].episodes?.length > 0 ? (seasons[0].episodes[0].episode_number || 1) : 1;

    // Watch Now button
    document.getElementById("details-watch-now-btn").addEventListener("click", () => {
      this.playMediaStream(item, this.selectedSeason, this.selectedEpisode);
    });

    // If series, setup season tabs & detailed episode cards
    if (isSeries && seasons.length > 0) {
      const renderSeasonEpisodes = (sIdx) => {
        const grid = document.getElementById("details-episodes-grid");
        const curSeason = seasons[sIdx];
        if (!curSeason || !grid) return;
        const eps = curSeason.episodes || [];

        grid.innerHTML = eps.length > 0 ? eps.map((ep, epIdx) => {
          const epNum = ep.episode_number || (epIdx + 1);
          const epTitle = ep.title || `Bölüm ${epNum}`;
          const still = ep.still_url || poster;
          const airDate = ep.air_date ? `Yayın: ${ep.air_date}` : "";
          const dur = ep.duration_seconds ? `${Math.round(ep.duration_seconds / 60)} dk` : (runtime ? runtime : "");
          const epOverview = ep.overview || "Bu bölüm için henüz detaylı özet girilmedi.";
          const isActive = curSeason.season_number === this.selectedSeason && epNum === this.selectedEpisode;

          return `
            <div class="episode-card-detailed ${isActive ? 'active' : ''}" data-ep="${epNum}" data-season="${curSeason.season_number || (sIdx + 1)}">
              <div class="episode-thumb-wrap">
                <img src="${still}" alt="${epTitle}" class="episode-thumb-img" loading="lazy" onerror="this.src='${poster}'">
                ${dur ? `<div class="episode-thumb-badge">${dur}</div>` : ""}
              </div>
              <div class="episode-info-wrap">
                <div class="episode-title-row">
                  <div class="episode-title-text">${epNum}. ${epTitle}</div>
                  ${airDate ? `<span class="episode-air-date">${airDate}</span>` : ""}
                </div>
                <div class="episode-synopsis">${epOverview}</div>
              </div>
            </div>
          `;
        }).join("") : `<div style="padding: 20px; color: var(--v-text-muted);">No episodes found.</div>`;

        grid.querySelectorAll(".episode-card-detailed").forEach(card => {
          card.addEventListener("click", () => {
            grid.querySelectorAll(".episode-card-detailed").forEach(c => c.classList.remove("active"));
            card.classList.add("active");
            const epNum = parseInt(card.dataset.ep, 10);
            const sNum = parseInt(card.dataset.season, 10);
            this.selectedSeason = sNum;
            this.selectedEpisode = epNum;
            this.loadStreamsList(item, sNum, epNum);
          });
        });
      };

      renderSeasonEpisodes(0);

      document.querySelectorAll("#details-season-tabs .tab").forEach(tab => {
        tab.addEventListener("click", () => {
          document.querySelectorAll("#details-season-tabs .tab").forEach(t => t.classList.remove("active"));
          tab.classList.add("active");
          const sIdx = parseInt(tab.dataset.seasonIdx, 10);
          this.selectedSeason = seasons[sIdx]?.season_number || (sIdx + 1);
          renderSeasonEpisodes(sIdx);
        });
      });
    }

    // Load available streams for initial season & episode
    this.loadStreamsList(item, this.selectedSeason, this.selectedEpisode);
  }

  renderIPTVDetails(item, details) {
    const content = document.getElementById("details-content");
    const title = details.title || item.title || "IPTV Kanalı";
    const poster = details.poster_url || item.poster_url || "/assets/vessel_primary.png";
    const extra = (details.external_ids && details.external_ids.extra) || (item.external_ids && item.external_ids.extra) || {};
    const category = extra.category || "Canlı TV";
    const country = extra.country || "Global";
    const quality = extra.quality || "HD";
    const streamURL = extra.stream_url || "";

    content.innerHTML = `
      <div class="details-hero">
        <div class="details-hero-backdrop" style="background-image: url('${poster}')"></div>
        <div class="details-poster-col">
          <img class="details-poster-img" src="${poster}" alt="${title}" style="object-fit: contain; background: #000; padding: 12px;">
          <div class="details-lib-box">
            <select id="details-lib-status" class="details-lib-select">
              <option value="NONE">➕ ${this.t("btn_add_library") || "Koleksiyona Ekle"}</option>
              <option value="WATCHING">▶ ${this.t("status_watching") || "İzleniyor"}</option>
              <option value="PLAN_TO_WATCH">🕒 ${this.t("status_plan") || "Daha Sonra"}</option>
              <option value="COMPLETED">✓ ${this.t("status_completed") || "Tamamlandı"}</option>
              <option value="DROPPED">⏹ ${this.t("status_dropped") || "Bırakıldı"}</option>
              <option value="FAVORITE">★ ${this.t("status_favorites") || "Favoriler"}</option>
            </select>
          </div>
        </div>
        <div class="details-info-col">
          <div class="details-meta-tags">
            <span class="badge-subtle" style="background: rgba(239, 68, 68, 0.18); color: #ef4444; font-weight: 700;">
              🔴 CANLI IPTV
            </span>
            <span class="badge-subtle">${quality}</span>
            <span class="badge-subtle">${country}</span>
            <span class="badge-subtle">${category}</span>
          </div>
          <h1 class="details-title">${title}</h1>
          <p class="details-overview">${details.overview || item.overview || `${title} canlı televizyon yayını.`}</p>
          <div class="details-actions">
            <button class="btn btn-primary" id="details-watch-live-btn" style="font-size: 1rem; padding: 10px 24px;">
              ▶ ${this.t("start_live")}
            </button>
          </div>
        </div>
      </div>

      <div class="details-subsections">
        <div class="details-section-box">
          <h3 class="details-section-title">${this.t("streams_title")}</h3>
          <div class="streams-grid" id="details-streams-grid">
            <div class="stream-card-row">
              <div>
                <span style="font-weight: 700; margin-right: 8px;">${quality}</span>
                <span>${title} (Doğrudan HLS Canlı Akış)</span>
              </div>
              <button class="btn btn-primary stream-play-btn" style="padding: 6px 16px; font-size: 0.85rem;">▶ ${this.t("start_live")}</button>
            </div>
          </div>
        </div>
      </div>
    `;

    this.setupDetailsLibraryControl(item, details);

    const playHandler = () => {
      this.playDirectHlsStream(streamURL || `/api/streams?provider=com.vessel.iptv&media=${encodeURIComponent(item.id)}`, title, item);
    };

    document.getElementById("details-watch-live-btn").addEventListener("click", playHandler);
    const streamBtn = content.querySelector(".stream-play-btn");
    if (streamBtn) streamBtn.addEventListener("click", playHandler);
  }

  async playDirectHlsStream(streamUrl, title, item = null) {
    let finalUrl = streamUrl;
    if (streamUrl.startsWith("/api/streams")) {
      try {
        const res = await fetch(streamUrl);
        if (res.ok) {
          const data = await res.json();
          if (data.streams && data.streams.length > 0) {
            finalUrl = data.streams[0].url;
          }
        }
      } catch (e) {
        console.warn("Could not fetch stream url:", e);
      }
    }

    this.openVesselPlayer(finalUrl, {
      item: item || { title, poster_url: "/assets/vessel_primary.png" },
      title,
      season: 0,
      episode: 0,
      subtitles: []
    });
  }

  renderReadingDetails(item, details) {
    const content = document.getElementById("details-content");
    const title = details.title || item.title || "Unknown";
    const poster = details.poster_url || item.poster_url;
    const chapters = details.chapters || [];
    const genres = details.genres || ["Manga"];

    const firstCh = chapters.length > 0 ? chapters[0] : null;
    const firstChNum = (firstCh && firstCh.chapter_number !== undefined && firstCh.chapter_number !== null) ? firstCh.chapter_number : 0;
    const firstChId = firstCh ? (firstCh.id || "") : "";

    content.innerHTML = `
      <div class="details-hero">
        <div class="details-hero-backdrop" style="background-image: url('${poster}')"></div>
        <div class="details-poster-col">
          <img class="details-poster-img" src="${poster}" alt="${title}">
          <div class="details-lib-box">
            <select id="details-lib-status" class="details-lib-select">
              <option value="NONE">➕ ${this.t("btn_add_library") || "Koleksiyona Ekle"}</option>
              <option value="WATCHING">▶ ${this.t("status_watching") || "Okunuyor"}</option>
              <option value="PLAN_TO_WATCH">🕒 ${this.t("status_plan") || "Daha Sonra"}</option>
              <option value="COMPLETED">✓ ${this.t("status_completed") || "Tamamlandı"}</option>
              <option value="DROPPED">⏹ ${this.t("status_dropped") || "Bırakıldı"}</option>
              <option value="FAVORITE">★ ${this.t("status_favorites") || "Favoriler"}</option>
            </select>
          </div>
        </div>
        <div class="details-info-col">
          <div class="details-meta-tags">
            <span class="badge-subtle" style="background: rgba(var(--v-accent-primary-rgb, 255,255,255), 0.15); color: var(--v-accent-primary); font-weight: 700;">
              ${this.mapMediaType(item.type)}
            </span>
            <span class="badge-subtle">★ 8.5</span>
            ${genres.map(g => `<span class="badge-subtle">${g}</span>`).join("")}
          </div>
          <h1 class="details-title">${title}</h1>
          <p class="details-overview">${details.overview || item.overview || "No synopsis available."}</p>
          <div class="details-actions">
            ${chapters.length > 0 ? `
              <button class="btn btn-primary" id="details-read-now-btn" style="font-size: 1rem; padding: 10px 24px;">
                📖 ${this.t("btn_read")} ${this.t("chapter")} ${firstChNum}
              </button>
            ` : ""}
          </div>
        </div>
      </div>

      <div class="details-subsections">
        <!-- Interactive Reader Box (Embedded right in page when reading) -->
        <div id="inline-reader-box" class="details-section-box hidden" style="padding: 0; overflow: hidden; background: #0b0d13; border: 1px solid var(--v-border-subtle); border-radius: var(--v-radius-lg);">
          <div class="reader-container" id="inline-reader-container" style="position: relative;">
            <!-- Reader canvas/pages injected here -->
          </div>
        </div>

        <div class="details-section-box">
          <h3 class="details-section-title">${this.t("chapters")} (${chapters.length})</h3>
          <div class="episodes-grid" style="max-height: 480px; overflow-y: auto;">
            ${chapters.length > 0 ? chapters.map(ch => {
              const chNum = (ch.chapter_number !== undefined && ch.chapter_number !== null) ? ch.chapter_number : 0;
              const chId = ch.id || "";
              return `
                <div class="chapter-row" data-ch="${chNum}" data-chid="${chId}" style="margin-bottom: 8px; cursor: pointer; display: flex; justify-content: space-between; align-items: center;">
                  <div>
                    <span style="font-weight: 600;">${this.t("chapter")} ${chNum}</span>
                    ${ch.title ? `<span style="color: var(--v-text-muted); margin-left: 8px;">- ${ch.title}</span>` : ""}
                  </div>
                  <div style="display: flex; gap: 8px; align-items: center;">
                    <button class="btn btn-secondary chapter-dl-action-btn" data-ch="${chNum}" data-chid="${chId}" data-title="${ch.title || ''}" title="Download for Offline" style="padding: 5px 10px; font-size: 0.8rem;">📥</button>
                    <button class="btn btn-secondary" style="padding: 5px 14px; font-size: 0.8rem;">${this.t("btn_read")}</button>
                  </div>
                </div>
              `;
            }).join("") : `<div style="text-align: center; color: var(--v-text-muted); padding: 24px;">No chapters found.</div>`}
          </div>
        </div>
      </div>
    `;

    this.setupDetailsLibraryControl(item, details);

    const readBtn = document.getElementById("details-read-now-btn");
    if (readBtn && chapters.length > 0) {
      readBtn.addEventListener("click", () => {
        this.openInlineChapter(item, details, firstChNum, firstChId, chapters);
      });
    }

    content.querySelectorAll(".chapter-row").forEach(row => {
      row.addEventListener("click", () => {
        const chNum = parseFloat(row.dataset.ch);
        const chId = row.dataset.chid || "";
        this.openInlineChapter(item, details, chNum, chId, chapters);
      });
    });

    content.querySelectorAll(".chapter-dl-action-btn").forEach(btn => {
      btn.addEventListener("click", async (e) => {
        e.stopPropagation();
        const chNum = parseFloat(btn.dataset.ch);
        const chId = btn.dataset.chid || "";
        const chTitle = btn.dataset.title || "";
        btn.textContent = "⏳";
        try {
          const res = await fetch("/api/reading/download", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              provider_id: item.provider_id || "com.vessel.reading.mangile",
              media_id: item.id,
              media_title: title,
              poster_url: poster,
              chapter_id: chId,
              chapter_num: chNum,
              title: chTitle
            })
          });
          if (!res.ok) throw new Error("Download request failed");
          btn.textContent = "✓";
          this.showToast(`Bölüm ${chNum} indirme kuyruğuna alındı / Download started`);
        } catch (err) {
          btn.textContent = "📥";
          this.showToast(err.message, "error");
        }
      });
    });
  }

  async openInlineChapter(item, details, chapterNum, chapterId, chapters) {
    const readerBox = document.getElementById("inline-reader-box");
    const container = document.getElementById("inline-reader-container");
    readerBox.classList.remove("hidden");
    readerBox.scrollIntoView({ behavior: "smooth" });

    container.innerHTML = `
      <div style="text-align: center; padding: 60px 20px; color: var(--v-text-muted);">
        <div style="font-size: 2rem; animation: pulse-online 1.5s infinite; margin-bottom: 12px;">📖</div>
        <div>${this.t("loading") || "Yükleniyor..."} ${this.t("chapter")} ${chapterNum}...</div>
      </div>
    `;

    try {
      const provider = item.provider_id || "com.vessel.reading.mangile";
      let url;
      if (item.isOffline) {
        url = `/api/reading/offline/content?provider=${encodeURIComponent(provider)}&media=${encodeURIComponent(item.id)}&chapter=${encodeURIComponent(chapterId)}`;
      } else {
        url = `/api/chapter?provider=${encodeURIComponent(provider)}&media=${encodeURIComponent(item.id)}&chapter_num=${chapterNum}`;
        if (chapterId) {
          url += `&chapter=${encodeURIComponent(chapterId)}`;
        }
      }
      const res = await fetch(url);
      if (!res.ok) {
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error || "Could not fetch chapter content");
      }
      const content = await res.json();

      const chList = chapters || details.chapters || [];
      const currentIndex = chList.findIndex(c => {
        if (chapterId && c.id && c.id === chapterId) return true;
        return c.chapter_number === chapterNum;
      });
      const prevCh = currentIndex > 0 ? chList[currentIndex - 1] : null;
      const nextCh = (currentIndex >= 0 && currentIndex < chList.length - 1) ? chList[currentIndex + 1] : null;

      const pages = content.pages || [];
      const hasPages = pages.length > 0;
      const hasText = !!content.text_content;
      const totalPages = hasPages ? pages.length : 1;

      // Manga Reader Local State (Mangile-inspired)
      let readerMode = localStorage.getItem("vessel_reader_mode") || "webtoon"; // "webtoon" | "paged" | "continuous"
      let zoomLevel = 100; // 60 to 160%
      let brightnessLevel = 100; // 30 to 100%
      let pagedIndex = 0;
      let isAutoScrolling = false;
      let autoScrollSpeed = 1; // 1, 2, 3
      let autoScrollAnimId = null;

      const titleStr = details.title || item.title || "Manga";

      let html = `
        <div class="reader-progress-bar-wrap">
          <div class="reader-progress-bar-fill" id="reader-progress-fill" style="width: 0%;"></div>
        </div>

        <div class="reader-toolbar">
          <div class="reader-tool-group">
            <span style="font-weight: 700; font-size: 0.9rem; color: var(--v-text-primary); white-space: nowrap;">
              ${titleStr}
            </span>
            ${chList.length > 1 ? `
              <select id="reader-chapter-dropdown" class="select-field" style="padding: 4px 10px; font-size: 0.82rem; border-radius: 6px; background: var(--v-bg-elevated); color: var(--v-text-primary); border: 1px solid var(--v-border-subtle); max-width: 180px;">
                ${chList.map(c => {
                  const cNum = (c.chapter_number !== undefined && c.chapter_number !== null) ? c.chapter_number : 0;
                  const cId = c.id || "";
                  const isSelected = (chapterId && cId === chapterId) || cNum === chapterNum;
                  return `<option value="${cNum}" data-chid="${cId}" ${isSelected ? "selected" : ""}>${this.t("chapter")} ${cNum}${c.title ? ` - ${c.title}` : ""}</option>`;
                }).join("")}
              </select>
            ` : `<span class="badge-subtle">${this.t("chapter")} ${chapterNum}</span>`}

            ${hasPages ? `
              <select id="reader-mode-select" class="select-field" style="padding: 4px 8px; font-size: 0.82rem; border-radius: 6px; background: var(--v-bg-elevated); color: var(--v-text-primary); border: 1px solid var(--v-border-subtle);">
                <option value="webtoon" ${readerMode === "webtoon" ? "selected" : ""}>📜 Webtoon</option>
                <option value="paged" ${readerMode === "paged" ? "selected" : ""}>📖 Tek Sayfa</option>
                <option value="continuous" ${readerMode === "continuous" ? "selected" : ""}>↔ Sürekli</option>
              </select>
            ` : ""}
          </div>

          <div class="reader-tool-group">
            ${hasPages ? `
              <div style="display: flex; align-items: center; gap: 3px; background: var(--v-bg-elevated); border: 1px solid var(--v-border-subtle); border-radius: 6px; padding: 2px 4px;">
                <button class="btn btn-secondary" id="reader-zoom-dec" style="padding: 2px 7px; font-size: 0.8rem; border: none;">-</button>
                <span id="reader-zoom-text" style="font-size: 0.75rem; min-width: 38px; text-align: center;">100%</span>
                <button class="btn btn-secondary" id="reader-zoom-inc" style="padding: 2px 7px; font-size: 0.8rem; border: none;">+</button>
              </div>

              <div style="display: flex; align-items: center; gap: 4px;" title="Parlaklık / Gece Modu">
                <span style="font-size: 0.85rem;">🌙</span>
                <input type="range" id="reader-brightness-slider" min="30" max="100" value="100" style="width: 60px; accent-color: var(--v-accent-primary); cursor: pointer;">
              </div>

              <div style="display: flex; align-items: center; gap: 3px;">
                <button class="btn btn-secondary" id="reader-autoscroll-btn" style="padding: 4px 10px; font-size: 0.8rem;" title="Otomatik Kaydır (Boşluk tuşu)">
                  ▶ Kaydır
                </button>
                <select id="reader-autoscroll-speed" class="select-field" style="padding: 4px 6px; font-size: 0.8rem; border-radius: 6px; background: var(--v-bg-elevated); color: var(--v-text-primary); border: 1px solid var(--v-border-subtle);">
                  <option value="1">1x</option>
                  <option value="2">2x</option>
                  <option value="3">3x</option>
                </select>
              </div>
            ` : ""}

            ${hasText ? `
              <div class="btn-group" style="display: flex; gap: 4px;">
                <button class="btn btn-secondary" id="reader-font-dec" style="padding: 4px 8px; font-size: 0.8rem;" title="Yazıyı Küçült">A-</button>
                <button class="btn btn-secondary" id="reader-font-inc" style="padding: 4px 8px; font-size: 0.8rem;" title="Yazıyı Büyüt">A+</button>
              </div>
            ` : ""}

            <button class="btn btn-secondary" id="reader-fullscreen-btn" style="padding: 4px 10px; font-size: 0.8rem;" title="Tam Ekran (F)">
              ⛶ Tam Ekran
            </button>
            <button class="btn btn-secondary" id="close-reader-btn" style="padding: 4px 12px; font-size: 0.8rem;">
              ✕ ${this.t("close") || "Kapat"}
            </button>
          </div>
        </div>

        <div id="reader-viewport" style="position: relative; width: 100%; min-height: 70vh; overflow: hidden;">
          <div id="reader-body-render" style="transition: filter 0.2s ease;"></div>
        </div>

        <div class="reader-footer" style="display: flex; justify-content: space-between; align-items: center; padding: 18px 24px; border-top: 1px solid var(--v-border-subtle); background: var(--v-bg-surface); flex-wrap: wrap; gap: 10px;">
          <button class="btn btn-secondary" id="reader-prev-btn" ${prevCh ? "" : "disabled"} style="padding: 8px 18px;">
            ← ${this.t("chapter")} ${prevCh ? (prevCh.chapter_number !== undefined ? prevCh.chapter_number : "") : ""}
          </button>
          <div style="font-size: 0.85rem; color: var(--v-text-muted);" id="reader-page-indicator">
            ${this.t("chapter")} ${chapterNum}
          </div>
          <button class="btn btn-primary" id="reader-next-btn" ${nextCh ? "" : "disabled"} style="padding: 8px 18px;">
            ${this.t("chapter")} ${nextCh ? (nextCh.chapter_number !== undefined ? nextCh.chapter_number : "") : ""} →
          </button>
        </div>
      `;

      container.innerHTML = html;

      const bodyRender = document.getElementById("reader-body-render");
      const progressFill = document.getElementById("reader-progress-fill");
      const pageIndicator = document.getElementById("reader-page-indicator");

      // Render content according to mode
      const renderReaderMode = () => {
        if (hasPages) {
          if (readerMode === "paged") {
            const curP = pages[pagedIndex] || pages[0];
            const pNum = curP.page_number || (pagedIndex + 1);
            bodyRender.innerHTML = `
              <div class="reader-pages-container mode-paged">
                <div class="reader-paged-wrap" style="transform: scale(${zoomLevel / 100}); transform-origin: top center; transition: transform 0.15s ease;">
                  <div class="reader-tap-zone reader-tap-left" id="reader-tap-left" title="Önceki Sayfa (Sol)"></div>
                  <img src="${curP.url}" alt="Sayfa ${pNum}" class="reader-page-img" style="max-height: 85vh; max-width: 90vw; object-fit: contain; border-radius: 6px; box-shadow: 0 10px 30px rgba(0,0,0,0.5);">
                  <div class="reader-tap-zone reader-tap-right" id="reader-tap-right" title="Sonraki Sayfa (Sağ)"></div>
                </div>
              </div>
            `;
            if (pageIndicator) pageIndicator.textContent = `Sayfa ${pNum} / ${totalPages} (${Math.round((pNum / totalPages) * 100)}%)`;
            if (progressFill) progressFill.style.width = `${(pNum / totalPages) * 100}%`;

            document.getElementById("reader-tap-left")?.addEventListener("click", () => {
              if (pagedIndex > 0) {
                pagedIndex--;
                renderReaderMode();
              } else if (prevCh) {
                const cNum = (prevCh.chapter_number !== undefined && prevCh.chapter_number !== null) ? prevCh.chapter_number : 0;
                this.openInlineChapter(item, details, cNum, prevCh.id || "", chList);
              }
            });

            document.getElementById("reader-tap-right")?.addEventListener("click", () => {
              if (pagedIndex < pages.length - 1) {
                pagedIndex++;
                renderReaderMode();
              } else if (nextCh) {
                const cNum = (nextCh.chapter_number !== undefined && nextCh.chapter_number !== null) ? nextCh.chapter_number : 0;
                this.openInlineChapter(item, details, cNum, nextCh.id || "", chList);
              }
            });
          } else {
            // Webtoon or Continuous mode
            const isWebtoon = readerMode === "webtoon";
            const maxWidthPx = Math.round((isWebtoon ? 820 : 880) * (zoomLevel / 100));
            bodyRender.innerHTML = `
              <div class="reader-pages-container ${isWebtoon ? 'mode-webtoon' : 'mode-continuous'}" style="max-width: ${maxWidthPx}px; transition: max-width 0.2s ease;">
                ${pages.map((p, pIdx) => `
                  <div class="reader-page-item" style="width: 100%; text-align: center;">
                    <img src="${p.url}" alt="Sayfa ${p.page_number || pIdx + 1}" class="reader-page-img" loading="lazy">
                    ${!isWebtoon ? `<div style="font-size: 0.75rem; color: var(--v-text-muted); margin-top: 6px;">${p.page_number || pIdx + 1} / ${totalPages}</div>` : ""}
                  </div>
                `).join("")}
              </div>
            `;
          }
        } else if (hasText) {
          const paragraphs = content.text_content.split(/\n\n+/).map(p => {
            const trimmed = p.trim();
            if (!trimmed) return "";
            const imgMatch = trimmed.match(/^!\[(.*?)\]\((https?:\/\/[^\s)]+)\)$/);
            if (imgMatch) {
              return `<figure class="reader-illustration" style="text-align: center; margin: 24px 0;"><img src="${imgMatch[2]}" alt="${imgMatch[1]}" style="max-width: 100%; max-height: 70vh; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.3);"><figcaption style="font-size: 0.85rem; color: var(--v-text-muted); margin-top: 6px; font-style: italic;">${imgMatch[1]}</figcaption></figure>`;
            }
            return `<p>${trimmed}</p>`;
          }).join("");

          bodyRender.innerHTML = `
            <div class="reader-text-content" id="reader-text-body" style="max-width: 800px; margin: 0 auto; padding: 24px 16px;">
              <h2 style="font-size: 1.6rem; font-weight: 800; margin-bottom: 24px; text-align: center;">${content.title || `${this.t("chapter")} ${chapterNum}`}</h2>
              ${paragraphs}
            </div>
          `;
        }
      };

      renderReaderMode();

      // Scroll progress tracking for Webtoon and Continuous
      const updateScrollProgress = () => {
        if (readerMode === "paged") return;
        const rect = bodyRender.getBoundingClientRect();
        const totalH = rect.height;
        const windowH = window.innerHeight;
        const scrolled = -rect.top;
        if (totalH <= windowH) {
          if (progressFill) progressFill.style.width = "100%";
          return;
        }
        const pct = Math.max(0, Math.min(100, Math.round((scrolled / (totalH - windowH)) * 100)));
        if (progressFill) progressFill.style.width = `${pct}%`;
        if (pageIndicator) pageIndicator.textContent = `${this.t("chapter")} ${chapterNum} (%${pct})`;
      };
      window.addEventListener("scroll", updateScrollProgress, { passive: true });
      readerBox.addEventListener("scroll", updateScrollProgress, { passive: true });

      // Brightness Slider
      const brightSlider = document.getElementById("reader-brightness-slider");
      if (brightSlider) {
        brightSlider.addEventListener("input", (e) => {
          brightnessLevel = parseInt(e.target.value, 10);
          bodyRender.style.filter = `brightness(${brightnessLevel}%)`;
        });
      }

      // Zoom In / Out
      const zoomText = document.getElementById("reader-zoom-text");
      document.getElementById("reader-zoom-dec")?.addEventListener("click", () => {
        if (zoomLevel > 60) {
          zoomLevel -= 15;
          if (zoomText) zoomText.textContent = `${zoomLevel}%`;
          renderReaderMode();
        }
      });
      document.getElementById("reader-zoom-inc")?.addEventListener("click", () => {
        if (zoomLevel < 180) {
          zoomLevel += 15;
          if (zoomText) zoomText.textContent = `${zoomLevel}%`;
          renderReaderMode();
        }
      });

      // Reading Mode Selector
      const modeSelect = document.getElementById("reader-mode-select");
      if (modeSelect) {
        modeSelect.addEventListener("change", (e) => {
          readerMode = e.target.value;
          localStorage.setItem("vessel_reader_mode", readerMode);
          renderReaderMode();
          updateScrollProgress();
        });
      }

      // Fullscreen Toggle
      const fsBtn = document.getElementById("reader-fullscreen-btn");
      const toggleReaderFS = () => {
        const isFS = document.fullscreenElement || readerBox.classList.contains("reader-box-fullscreen");
        if (isFS) {
          if (document.exitFullscreen) document.exitFullscreen().catch(() => {});
          readerBox.classList.remove("reader-box-fullscreen");
          if (fsBtn) fsBtn.textContent = "⛶ Tam Ekran";
        } else {
          if (readerBox.requestFullscreen) {
            readerBox.requestFullscreen().catch(() => {
              readerBox.classList.add("reader-box-fullscreen");
            });
          } else {
            readerBox.classList.add("reader-box-fullscreen");
          }
          if (fsBtn) fsBtn.textContent = "✕ Çıkış";
        }
      };
      if (fsBtn) fsBtn.addEventListener("click", toggleReaderFS);

      // Auto-scroll loop
      const autoScrollBtn = document.getElementById("reader-autoscroll-btn");
      const autoScrollSpeedSelect = document.getElementById("reader-autoscroll-speed");
      if (autoScrollSpeedSelect) {
        autoScrollSpeedSelect.addEventListener("change", (e) => {
          autoScrollSpeed = parseFloat(e.target.value);
        });
      }

      const stopAutoScroll = () => {
        isAutoScrolling = false;
        if (autoScrollAnimId) cancelAnimationFrame(autoScrollAnimId);
        if (autoScrollBtn) autoScrollBtn.textContent = "▶ Kaydır";
      };

      const stepAutoScroll = () => {
        if (!isAutoScrolling) return;
        const scrollTarget = readerBox.classList.contains("reader-box-fullscreen") ? readerBox : window;
        const speedMultiplier = autoScrollSpeed === 1 ? 1.2 : (autoScrollSpeed === 2 ? 2.5 : 4.5);
        if (scrollTarget === window) {
          window.scrollBy(0, speedMultiplier);
          if (window.innerHeight + window.scrollY >= document.body.offsetHeight - 50) {
            stopAutoScroll();
            return;
          }
        } else {
          readerBox.scrollTop += speedMultiplier;
          if (readerBox.scrollTop + readerBox.clientHeight >= readerBox.scrollHeight - 50) {
            stopAutoScroll();
            return;
          }
        }
        autoScrollAnimId = requestAnimationFrame(stepAutoScroll);
      };

      const toggleAutoScroll = () => {
        isAutoScrolling = !isAutoScrolling;
        if (isAutoScrolling) {
          if (autoScrollBtn) autoScrollBtn.textContent = "⏸ Durdur";
          stepAutoScroll();
        } else {
          stopAutoScroll();
        }
      };
      if (autoScrollBtn) autoScrollBtn.addEventListener("click", toggleAutoScroll);

      // Chapter Dropdown
      const dropdown = document.getElementById("reader-chapter-dropdown");
      if (dropdown) {
        dropdown.addEventListener("change", (e) => {
          stopAutoScroll();
          const opt = e.target.selectedOptions[0];
          const cNum = parseFloat(e.target.value);
          const cId = opt ? opt.dataset.chid : "";
          this.openInlineChapter(item, details, cNum, cId, chList);
        });
      }

      // Prev & Next Buttons
      const prevBtn = document.getElementById("reader-prev-btn");
      if (prevBtn && prevCh) {
        prevBtn.addEventListener("click", () => {
          stopAutoScroll();
          const cNum = (prevCh.chapter_number !== undefined && prevCh.chapter_number !== null) ? prevCh.chapter_number : 0;
          this.openInlineChapter(item, details, cNum, prevCh.id || "", chList);
        });
      }

      const nextBtn = document.getElementById("reader-next-btn");
      if (nextBtn && nextCh) {
        nextBtn.addEventListener("click", () => {
          stopAutoScroll();
          const cNum = (nextCh.chapter_number !== undefined && nextCh.chapter_number !== null) ? nextCh.chapter_number : 0;
          this.openInlineChapter(item, details, cNum, nextCh.id || "", chList);
        });
      }

      // Close Button
      const closeReader = () => {
        stopAutoScroll();
        if (document.fullscreenElement) {
          document.exitFullscreen?.().catch(() => {});
        }
        readerBox.classList.remove("reader-box-fullscreen");
        readerBox.classList.add("hidden");
        window.removeEventListener("scroll", updateScrollProgress);
        window.removeEventListener("keydown", readerKeyHandler);
        this.loadResumeProgress();
      };
      document.getElementById("close-reader-btn")?.addEventListener("click", closeReader);

      // Keyboard Controls
      const readerKeyHandler = (e) => {
        if (readerBox.classList.contains("hidden")) return;
        if (e.key === "f" || e.key === "F") {
          e.preventDefault();
          toggleReaderFS();
        } else if (e.key === " " && hasPages && readerMode !== "paged") {
          e.preventDefault();
          toggleAutoScroll();
        } else if (readerMode === "paged") {
          if (e.key === "ArrowLeft" || e.key === "a") {
            if (pagedIndex > 0) { pagedIndex--; renderReaderMode(); }
          } else if (e.key === "ArrowRight" || e.key === "d") {
            if (pagedIndex < pages.length - 1) { pagedIndex++; renderReaderMode(); }
          }
        } else if (e.key === "Escape") {
          if (document.fullscreenElement || readerBox.classList.contains("reader-box-fullscreen")) {
            toggleReaderFS();
          } else {
            closeReader();
          }
        }
      };
      window.addEventListener("keydown", readerKeyHandler);

      // Save reading progress ping with title and poster
      await fetch("/api/progress/reading", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider_id: provider,
          media_id: item.id,
          domain: 2,
          title: details.title || item.title || "",
          poster_url: details.poster_url || item.poster_url || "",
          chapter_id: content.chapter_id || chapterId || `ch-${chapterNum}`,
          chapter_number: chapterNum,
          current_page: 1,
          total_pages: totalPages,
          is_completed: false
        })
      }).catch(() => {});

    } catch (e) {
      container.innerHTML = `
        <div style="text-align: center; padding: 40px; color: var(--v-status-error);">
          <h4>Failed to load chapter</h4>
          <p style="margin-top: 8px; color: var(--v-text-muted);">${e.message}</p>
          <button class="btn btn-secondary" id="retry-chapter-btn" style="margin-top: 16px;">Yeniden Dene</button>
        </div>
      `;
      document.getElementById("retry-chapter-btn")?.addEventListener("click", () => {
        this.openInlineChapter(item, details, chapterNum, chapterId, chapters);
      });
    }
  }

  async playMediaStream(item, season, episode, streamObj = null) {
    try {
      let targetStream = streamObj;
      if (!targetStream) {
        this.showToast("Akış kaynakları taranıyor...", "info");
        const res = await fetch(`/api/streams?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&season=${season}&episode=${episode}`);
        if (!res.ok) throw new Error("Could not fetch stream sources");
        const data = await res.json();
        const streams = data.streams || [];

        if (streams.length === 0) {
          this.showToast("Bu içerik için aktif akış kaynağı bulunamadı.", "warning");
          return;
        }
        targetStream = streams[0];
      }

      // Resolve stream
      let streamUrl = targetStream.url;
      try {
        const resolveRes = await fetch("/api/stream/resolve", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ url: targetStream.url, media_id: item.id, season, episode })
        });
        if (resolveRes.ok) {
          const resolved = await resolveRes.json();
          if (resolved.url) streamUrl = resolved.url;
        }
      } catch (err) {
        console.warn("Stream resolve error:", err);
      }

      this.openVesselPlayer(streamUrl, {
        item,
        season,
        episode,
        subtitles: targetStream.subtitles || []
      });
    } catch (e) {
      this.showToast(`Playback error: ${e.message}`, "error");
    }
  }

  async loadStreamsList(item, season, episode) {
    const grid = document.getElementById("details-streams-grid");
    if (!grid) return;

    grid.innerHTML = `
      <div style="padding: 20px; color: var(--v-text-muted); display: flex; align-items: center; gap: 10px;">
        <div style="font-size: 1.2rem; animation: pulse-online 1.5s infinite;">⏳</div>
        <div>${this.t("streams_loading")} (Sezon ${season}, Bölüm ${episode})</div>
      </div>
    `;

    try {
      const res = await fetch(`/api/streams?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&season=${season}&episode=${episode}`);
      if (!res.ok) throw new Error("Could not load streams");
      const data = await res.json();
      const streams = data.streams || [];

      if (streams.length === 0) {
        grid.innerHTML = `
          <div class="streams-empty-banner">
            <div class="empty-banner-icon">📡</div>
            <div style="flex: 1;">
              <div class="empty-banner-title">${this.t("no_episode_streams")}</div>
              <div class="empty-banner-desc">Sezon ${season}, Bölüm ${episode} için şu anda aktif eşler veya akış sağlayıcısı bulunamadı. Torrent / Debrid bağlantılarınızı kontrol edebilir veya yeniden deneyebilirsiniz.</div>
            </div>
            <button class="btn btn-secondary" id="retry-streams-btn" style="white-space: nowrap;">
              🔄 ${this.t("btn_retry")}
            </button>
          </div>
        `;
        document.getElementById("retry-streams-btn")?.addEventListener("click", () => {
          this.loadStreamsList(item, season, episode);
        });
        return;
      }

      grid.innerHTML = streams.map((st, idx) => `
        <div class="stream-card-row">
          <div>
            <span style="font-weight: 700; margin-right: 8px;">${st.quality || "1080p"}</span>
            <span>${st.title || "Stream Source"}</span>
          </div>
          <button class="btn btn-secondary stream-play-btn" data-idx="${idx}" style="padding: 4px 14px; font-size: 0.8rem;">▶ Stream</button>
        </div>
      `).join("");

      grid.querySelectorAll(".stream-play-btn").forEach(btn => {
        btn.addEventListener("click", () => {
          const idx = parseInt(btn.dataset.idx, 10);
          this.playMediaStream(item, season, episode, streams[idx]);
        });
      });
    } catch (e) {
      grid.innerHTML = `
        <div class="streams-empty-banner">
          <div class="empty-banner-icon">📡</div>
          <div style="flex: 1;">
            <div class="empty-banner-title">${this.t("no_episode_streams")}</div>
            <div class="empty-banner-desc">Hata: ${e.message}</div>
          </div>
          <button class="btn btn-secondary" id="retry-streams-btn" style="white-space: nowrap;">
            🔄 ${this.t("btn_retry")}
          </button>
        </div>
      `;
      document.getElementById("retry-streams-btn")?.addEventListener("click", () => {
        this.loadStreamsList(item, season, episode);
      });
    }
  }

  // --- Dedicated Plugins View ---
  async loadPluginsView() {
    const installedList = document.getElementById("installed-plugins-list");
    const curatedList = document.getElementById("curated-plugins-list");

    installedList.innerHTML = `<div style="grid-column: 1/-1; text-align: center; padding: 40px; color: var(--v-text-muted);">⏳ Loading plugins...</div>`;

    try {
      // 1. Fetch Installed Plugins
      const instRes = await fetch("/api/plugins");
      const instData = instRes.ok ? await instRes.json() : { plugins: [] };
      const installed = instData.plugins || [];

      // Sort installed according to custom priority
      const rawOrder = localStorage.getItem("vessel_plugin_order");
      if (rawOrder) {
        try {
          const pOrder = JSON.parse(rawOrder);
          installed.sort((a, b) => {
            const idxA = pOrder.indexOf(a.id);
            const idxB = pOrder.indexOf(b.id);
            return (idxA === -1 ? 999 : idxA) - (idxB === -1 ? 999 : idxB);
          });
        } catch (_) {}
      }

      installedList.innerHTML = "";
      if (installed.length === 0) {
        installedList.innerHTML = `<div style="grid-column: 1/-1; text-align: center; padding: 30px; color: var(--v-text-muted);">No plugins installed.</div>`;
      } else {
        installed.forEach((p, idx) => {
          const card = document.createElement("div");
          card.className = "plugin-card";
          const isEnabled = p.enabled !== false;
          const loc = this.getPluginLocalizedInfo(p.id, p.name || p.id, p.description || "");
          const langDisplay = p.language_display || (p.id.includes("mangile") ? "🇹🇷 TR" : "🌐 Universal");

          card.innerHTML = `
            <div class="plugin-card-header">
              <div>
                <h4 class="plugin-card-title">${loc.title}</h4>
                <div style="font-size: 0.8rem; color: var(--v-text-muted); margin-top: 4px;">v${p.version || "1.0.0"} • ${p.author || "Vessel"}</div>
              </div>
              <div style="display: flex; gap: 6px; align-items: center;">
                <span class="badge-subtle" style="background: rgba(56, 189, 248, 0.12); color: var(--v-accent); font-size: 0.75rem;">
                  ${langDisplay}
                </span>
                <span class="badge-subtle" style="background: ${isEnabled ? "rgba(var(--v-status-success-rgb, 16, 185, 129), 0.15)" : "rgba(156, 163, 175, 0.15)"}; color: ${isEnabled ? "var(--v-status-success)" : "var(--v-text-muted)"};">
                  ${isEnabled ? this.t("btn_active") : "Devre Dışı"}
                </span>
              </div>
            </div>
            <p class="plugin-card-desc">${loc.description}</p>
            <div class="plugin-card-footer">
              <span style="font-size: 0.8rem; color: var(--v-text-muted);">${p.is_builtin ? this.t("plugin_builtin") : this.t("plugin_external")}</span>
              <div style="display: flex; align-items: center; gap: 8px;">
                <div class="plugin-reorder-group" style="display: inline-flex; gap: 4px;">
                  <button class="btn btn-secondary reorder-up-btn" title="Önceliği Artır (Yukarı Taşı)" style="padding: 3px 8px; font-size: 0.75rem;">⬆</button>
                  <button class="btn btn-secondary reorder-down-btn" title="Önceliği Azalt (Aşağı Taşı)" style="padding: 3px 8px; font-size: 0.75rem;">⬇</button>
                </div>
                <button class="btn btn-secondary toggle-plugin-btn" style="font-size: 0.8rem; padding: 4px 12px;">
                  ${isEnabled ? this.t("btn_disable") : this.t("btn_enable")}
                </button>
              </div>
            </div>
          `;

          const upBtn = card.querySelector(".reorder-up-btn");
          const downBtn = card.querySelector(".reorder-down-btn");

          upBtn.addEventListener("click", () => {
            if (idx === 0) return;
            const newOrder = installed.map(item => item.id);
            const temp = newOrder[idx];
            newOrder[idx] = newOrder[idx - 1];
            newOrder[idx - 1] = temp;
            localStorage.setItem("vessel_plugin_order", JSON.stringify(newOrder));
            this.showToast("Eklenti önceliği artırıldı", "info");
            this.loadPluginsView();
          });

          downBtn.addEventListener("click", () => {
            if (idx === installed.length - 1) return;
            const newOrder = installed.map(item => item.id);
            const temp = newOrder[idx];
            newOrder[idx] = newOrder[idx + 1];
            newOrder[idx + 1] = temp;
            localStorage.setItem("vessel_plugin_order", JSON.stringify(newOrder));
            this.showToast("Eklenti önceliği azaltıldı", "info");
            this.loadPluginsView();
          });

          const toggleBtn = card.querySelector(".toggle-plugin-btn");
          toggleBtn.addEventListener("click", async () => {
            await this.togglePlugin(p.id, !isEnabled);
          });

          installedList.appendChild(card);
        });
      }

      // 2. Fetch Curated Directory
      const curRes = await fetch("/api/plugins/available");
      const curData = curRes.ok ? await curRes.json() : { plugins: [] };
      const available = curData.plugins || [];

      curatedList.innerHTML = "";
      available.forEach(p => {
        const card = document.createElement("div");
        card.className = "plugin-card";
        const isInst = p.installed;
        const loc = this.getPluginLocalizedInfo(p.id, p.name, p.description);
        const langDisplay = p.language_display || (p.id.includes("mangile") ? "🇹🇷 TR" : "🌐 Universal");

        card.innerHTML = `
          <div class="plugin-card-header">
            <div>
              <h4 class="plugin-card-title">${loc.title}</h4>
              <div style="font-size: 0.8rem; color: var(--v-text-muted); margin-top: 4px;">v${p.version} • ${p.author}</div>
            </div>
            <div style="display: flex; gap: 6px; align-items: center;">
              <span class="badge-subtle" style="background: rgba(56, 189, 248, 0.12); color: var(--v-accent); font-size: 0.75rem;">
                ${langDisplay}
              </span>
              <span class="badge-subtle">${(p.domain || "MEDIA").toUpperCase()}</span>
            </div>
          </div>
          <p class="plugin-card-desc">${loc.description}</p>
          <div class="plugin-card-footer">
            <span style="font-size: 0.8rem; color: var(--v-text-muted);">${p.is_builtin ? this.t("plugin_builtin") : "Community"}</span>
            <button class="btn ${isInst ? "btn-secondary" : "btn-primary"} install-action-btn" style="font-size: 0.8rem; padding: 6px 14px;">
              ${isInst ? "✓ Installed" : this.t("btn_install")}
            </button>
          </div>
        `;

        const actionBtn = card.querySelector(".install-action-btn");
        if (!isInst) {
          actionBtn.addEventListener("click", () => {
            this.installPlugin({ id: p.id });
          });
        }

        curatedList.appendChild(card);
      });
    } catch (e) {
      installedList.innerHTML = `<div style="grid-column: 1/-1; color: var(--v-status-error);">${e.message}</div>`;
    }
  }

  async installPlugin(payload) {
    try {
      const res = await fetch("/api/plugins/install", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });
      if (!res.ok) throw new Error("Failed to install plugin");
      const data = await res.json();
      this.showToast(data.message || "Plugin installed successfully!");
      this.loadPluginsView();
    } catch (e) {
      this.showToast(`Installation error: ${e.message}`, "error");
    }
  }

  async togglePlugin(id, enabled) {
    try {
      const res = await fetch("/api/plugins/toggle", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, enabled })
      });
      if (!res.ok) throw new Error("Eklenti durumu değiştirilemedi");
      const data = await res.json();
      this.showToast(enabled ? "Eklenti etkinleştirildi" : "Eklenti devre dışı bırakıldı");
      this.loadPluginsView();
    } catch (e) {
      this.showToast(`Hata: ${e.message}`, "error");
    }
  }

  // --- Resume Progress ---
  async loadResumeProgress() {
    const container = document.getElementById("resume-cards");
    const resumeSection = document.getElementById("resume-section");

    try {
      const isReading = this.currentDomain === "reading";
      const endpoint = isReading ? "/api/progress/reading/recent?limit=8" : "/api/progress/playback/recent?limit=8";
      const res = await fetch(endpoint);
      if (!res.ok) return;
      const data = await res.json();
      const items = data.items || [];

      container.innerHTML = "";
      if (items.length === 0) {
        container.innerHTML = `
          <div class="resume-empty">
            <span style="font-size: 1.25rem;">⏳</span>
            <span>${this.t("resume_empty")}</span>
          </div>
        `;
        return;
      }

      items.forEach(p => {
        const card = document.createElement("div");
        card.className = "resume-card";
        const rawId = p.media_id || p.MediaID || "";
        let title = p.title || p.Title || "";
        let poster = p.poster_url || p.PosterURL || "";

        if (!title || title === rawId) {
          if (rawId.includes("-") && !rawId.match(/^[0-9a-f]{8}-/i)) {
            title = rawId.split("-").map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
          } else {
            title = isReading ? "Reading Progress" : "Media Progress";
          }
        }

        const fallbackThumb = `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="200" height="130" viewBox="0 0 200 130"><rect width="200" height="130" fill="%231e293b"/><text x="50%" y="54%" dominant-baseline="middle" text-anchor="middle" fill="%2364748b" font-size="28" font-weight="bold">${isReading ? '📖' : '🎬'}</text></svg>`;
        const posterSrc = poster || fallbackThumb;

        if (isReading) {
          const chNum = p.chapter_number || p.ChapterNumber || 1;
          const currPage = p.current_page || p.CurrentPage || 1;
          const totPages = p.total_pages || p.TotalPages || 1;
          const percent = totPages > 0 ? Math.min(100, Math.round((currPage / totPages) * 100)) : 0;

          card.innerHTML = `
            <div class="resume-thumb-wrap" style="position: relative; width: 100%; height: 130px; overflow: hidden; background: var(--v-bg-elevated);">
              <img class="resume-thumb" src="${posterSrc}" alt="${title}" onerror="this.onerror=null; this.src='${fallbackThumb}';">
              <button class="resume-delete-btn" title="Kaldır / Sil" data-del="true">✕</button>
            </div>
            <div class="resume-info">
              <div class="resume-title" title="${title}">${title}</div>
              <div class="resume-sub">${this.t("chapter")} ${chNum} • ${currPage}/${totPages} (%${percent})</div>
              <div class="progress-bar-container">
                <div class="progress-bar-fill" style="width: ${percent}%;"></div>
              </div>
            </div>
          `;
          card.addEventListener("click", () => {
            this.openDetailsView({ id: p.media_id, provider_id: p.provider_id, title, poster_url: posterSrc, type: 4 });
          });
        } else {
          const percent = Math.min(100, Math.round(p.progress_percent || p.ProgressPercent || 0));
          const sNum = p.season_number || p.SeasonNumber || 1;
          const epNum = p.episode_number || p.EpisodeNumber || 1;

          card.innerHTML = `
            <div class="resume-thumb-wrap" style="position: relative; width: 100%; height: 130px; overflow: hidden; background: var(--v-bg-elevated);">
              <img class="resume-thumb" src="${posterSrc}" alt="${title}" onerror="this.onerror=null; this.src='${fallbackThumb}';">
              <button class="resume-delete-btn" title="Kaldır / Sil" data-del="true">✕</button>
            </div>
            <div class="resume-info">
              <div class="resume-title" title="${title}">${title}</div>
              <div class="resume-sub">${this.t("season")} ${sNum} • ${this.t("episode")} ${epNum} (%${percent})</div>
              <div class="progress-bar-container">
                <div class="progress-bar-fill" style="width: ${percent}%;"></div>
              </div>
            </div>
          `;
          card.addEventListener("click", () => {
            this.openDetailsView({ id: p.media_id, provider_id: p.provider_id, title, poster_url: posterSrc, type: 1 });
          });
        }

        // Auto-enrich title & poster from media endpoint if not available in SQLite
        if (!p.title || !p.poster_url) {
          fetch(`/api/media?provider=${encodeURIComponent(p.provider_id || "")}&id=${encodeURIComponent(rawId)}&domain=${isReading ? 2 : 1}`)
            .then(r => r.ok ? r.json() : null)
            .then(d => {
              if (d && (d.title || d.poster_url)) {
                if (d.title) {
                  const tEl = card.querySelector(".resume-title");
                  if (tEl) tEl.textContent = d.title;
                }
                if (d.poster_url) {
                  const pEl = card.querySelector(".resume-thumb");
                  if (pEl) pEl.src = d.poster_url;
                }
              }
            })
            .catch(() => {});
        }

        const delBtn = card.querySelector(".resume-delete-btn");
        if (delBtn) {
          delBtn.addEventListener("click", async (e) => {
            e.stopPropagation();
            try {
              if (isReading) {
                const chId = p.chapter_id || p.ChapterID || "";
                await fetch(`/api/progress/reading?provider=${encodeURIComponent(p.provider_id || "")}&media=${encodeURIComponent(p.media_id || "")}&chapter=${encodeURIComponent(chId)}`, {
                  method: "DELETE"
                });
              } else {
                const sNum = p.season_number || p.SeasonNumber || 1;
                const epNum = p.episode_number || p.EpisodeNumber || 1;
                await fetch(`/api/progress/playback?provider=${encodeURIComponent(p.provider_id || "")}&media=${encodeURIComponent(p.media_id || "")}&season=${sNum}&episode=${epNum}`, {
                  method: "DELETE"
                });
              }
              this.showToast("İlerleme kaldırıldı", "info");
              this.loadResumeProgress();
            } catch (err) {
              this.showToast(err.message, "error");
            }
          });
        }

        container.appendChild(card);
      });
    } catch (e) {
      console.warn("Could not load resume items:", e);
    }
  }

  // --- Library System ---
  async loadLibraryItems(status) {
    const grid = document.getElementById("library-grid");
    if (status === "OFFLINE") {
      return this.loadOfflineDownloads(grid);
    }
    grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">⏳ Loading collection...</div>`;

    try {
      const res = await fetch(`/api/library?status=${status || "ALL"}`);
      if (!res.ok) throw new Error("Failed to load library");
      const data = await res.json();
      const items = data.items || [];

      grid.innerHTML = "";
      if (items.length === 0) {
        grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 50px;">Bu sekmede henüz içerik bulunmuyor.</div>`;
        return;
      }

      items.forEach(it => {
        const card = document.createElement("div");
        card.className = "media-card";
        const title = it.title || "Unknown";
        const poster = it.poster_url || "/assets/vessel_primary.png";
        const typeLabel = this.mapMediaType(it.type || 1);
        const curStatus = it.status || "WATCHING";

        card.innerHTML = `
          <div class="poster-wrapper">
            <img src="${poster}" alt="${title}" class="poster-img" loading="lazy" onerror="this.src='/assets/vessel_primary.png'">
            <span class="card-badge">${typeLabel}</span>
            <div class="poster-overlay-btn">▶</div>
          </div>
          <div class="card-details">
            <div class="card-title" title="${title}">${title}</div>
            <div class="library-card-actions">
              <select class="library-item-status-select" data-media="${it.media_id}" data-provider="${it.provider_id}">
                <option value="WATCHING" ${curStatus === "WATCHING" ? "selected" : ""}>▶ ${this.t("status_watching") || "İzleniyor"}</option>
                <option value="PLAN_TO_WATCH" ${curStatus === "PLAN_TO_WATCH" ? "selected" : ""}>🕒 ${this.t("status_plan") || "Daha Sonra"}</option>
                <option value="COMPLETED" ${curStatus === "COMPLETED" ? "selected" : ""}>✓ ${this.t("status_completed") || "Tamamlandı"}</option>
                <option value="DROPPED" ${curStatus === "DROPPED" ? "selected" : ""}>⏹ ${this.t("status_dropped") || "Bırakıldı"}</option>
                <option value="FAVORITE" ${curStatus === "FAVORITE" ? "selected" : ""}>★ ${this.t("status_favorites") || "Favoriler"}</option>
              </select>
              <button class="library-item-remove-btn" title="Koleksiyondan Kaldır">🗑</button>
            </div>
          </div>
        `;

        card.querySelector(".poster-wrapper").addEventListener("click", () => {
          this.openDetailsView({ id: it.media_id, provider_id: it.provider_id, title, poster_url: poster, type: it.type });
        });

        const statusSelect = card.querySelector(".library-item-status-select");
        statusSelect.addEventListener("click", (e) => e.stopPropagation());
        statusSelect.addEventListener("change", async (e) => {
          e.stopPropagation();
          const newStatus = statusSelect.value;
          try {
            await fetch("/api/library", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                provider_id: it.provider_id,
                media_id: it.media_id,
                domain: it.domain || 1,
                title: it.title,
                type: it.type || 1,
                poster_url: it.poster_url,
                status: newStatus,
                user_rating: it.user_rating || 8.5
              })
            });
            this.showToast(`Durum güncellendi: ${newStatus}`);
            if (status !== "ALL" && status !== newStatus) {
              this.loadLibraryItems(status);
            }
          } catch (err) {
            this.showToast(err.message, "error");
          }
        });

        const removeBtn = card.querySelector(".library-item-remove-btn");
        removeBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            await fetch(`/api/library?provider=${encodeURIComponent(it.provider_id || "")}&media=${encodeURIComponent(it.media_id || "")}`, {
              method: "DELETE"
            });
            this.showToast("Koleksiyondan kaldırıldı", "info");
            card.remove();
          } catch (err) {
            this.showToast(err.message, "error");
          }
        });

        grid.appendChild(card);
      });
    } catch (e) {
      grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-status-error); padding: 30px;">${e.message}</div>`;
    }
  }

  async toggleLibrary(item, details) {
    try {
      const isReading = this.currentDomain === "reading" || item.type >= 4;
      const res = await fetch("/api/library", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider_id: item.provider_id,
          media_id: item.id,
          domain: isReading ? 2 : 1,
          title: details.title || item.title,
          type: item.type || 1,
          poster_url: details.poster_url || item.poster_url,
          status: "WATCHING",
          user_rating: 8.5
        })
      });
      if (!res.ok) throw new Error("Failed to save to library");
      this.showToast("Saved to your collection!");
      const libBtn = document.getElementById("details-lib-btn");
      if (libBtn) libBtn.textContent = this.t("btn_in_library");
    } catch (e) {
      this.showToast(e.message, "error");
    }
  }

  // --- Debrid Settings ---
  async loadDebridStatus() {
    try {
      const res = await fetch("/api/debrid/status");
      if (!res.ok) return;
      const data = await res.json();

      const rdBadge = document.getElementById("rd-status-badge");
      if (rdBadge) {
        rdBadge.textContent = data.realdebrid_configured ? "✓ Connected & Active" : "Not configured";
        rdBadge.className = `account-badge ${data.realdebrid_configured ? "premium" : ""}`;
      }

      const tbBadge = document.getElementById("tb-status-badge");
      if (tbBadge) {
        tbBadge.textContent = data.torbox_configured ? "✓ Connected & Active" : "Not configured";
        tbBadge.className = `account-badge ${data.torbox_configured ? "premium" : ""}`;
      }
    } catch (e) {
      console.warn("Could not check debrid status:", e);
    }
  }

  async saveDebrid(service, apiKey) {
    if (!apiKey) return;
    try {
      const res = await fetch("/api/debrid/configure", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ service, api_key: apiKey })
      });
      if (!res.ok) throw new Error("Configuration failed");
      this.showToast(`Configured ${service} successfully!`);
      this.loadDebridStatus();
    } catch (e) {
      this.showToast(`Error: ${e.message}`, "error");
    }
  }

  // --- Offline Manga & E-Book Library System ---
  async loadOfflineDownloads(grid) {
    grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">⏳ İndirilenler yükleniyor / Loading offline collection...</div>`;
    try {
      const res = await fetch("/api/reading/downloads");
      if (!res.ok) throw new Error("Could not fetch offline downloads");
      const data = await res.json();
      const downloads = data.downloads || [];

      grid.innerHTML = "";
      if (downloads.length === 0) {
        grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 50px;">Henüz indirilmiş çevrimdışı bölüm bulunmuyor.<br><span style="font-size: 0.85rem;">Manga & E-Kitap bölümlerinin yanındaki 📥 butonuna tıklayarak istediğiniz bölümleri çevrimdışı okumak için indirebilirsiniz.</span></div>`;
        return;
      }

      downloads.forEach(dl => {
        const card = document.createElement("div");
        card.className = "media-card";
        const title = dl.media_title || dl.media_id || "Manga";
        const chTitle = dl.title || `Bölüm ${dl.chapter_number}`;
        const poster = dl.poster_url || "/assets/vessel_primary.png";
        const mb = ((dl.total_bytes || 0) / (1024 * 1024)).toFixed(1);

        card.innerHTML = `
          <div class="poster-wrapper">
            <img src="${poster}" alt="${title}" class="poster-img" loading="lazy" onerror="this.src='/assets/vessel_primary.png'">
            <span class="card-badge" style="background: rgba(16, 185, 129, 0.9); color: #fff;">📥 Çevrimdışı</span>
            <div class="poster-overlay-btn">📖</div>
          </div>
          <div class="card-details">
            <div class="card-title" title="${title}">${title}</div>
            <div style="font-size: 0.82rem; color: var(--v-text-muted); margin-bottom: 4px;">${chTitle}</div>
            <div style="font-size: 0.75rem; color: var(--v-accent-primary); font-weight: 600;">${dl.downloaded_pages || dl.total_pages}/${dl.total_pages || 0} Sayfa (${mb} MB)</div>
            <div class="library-card-actions" style="margin-top: 8px; display: flex; gap: 6px;">
              <button class="btn btn-primary offline-read-btn" style="padding: 5px 12px; font-size: 0.78rem; flex: 1;">📖 Oku</button>
              <button class="library-item-remove-btn offline-delete-btn" title="İndirmeyi Cihazdan Sil">🗑</button>
            </div>
          </div>
        `;

        card.querySelector(".offline-read-btn").addEventListener("click", () => {
          this.openOfflineReader(dl);
        });
        card.querySelector(".poster-wrapper").addEventListener("click", () => {
          this.openOfflineReader(dl);
        });

        const delBtn = card.querySelector(".offline-delete-btn");
        delBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            const delRes = await fetch(`/api/reading/download?provider=${encodeURIComponent(dl.provider_id || "com.vessel.reading.mangile")}&media=${encodeURIComponent(dl.media_id || "")}&chapter=${encodeURIComponent(dl.chapter_id || "")}`, {
              method: "DELETE"
            });
            if (!delRes.ok) throw new Error("Delete failed");
            this.showToast("İndirme silindi / Download removed", "info");
            card.remove();
          } catch (err) {
            this.showToast(err.message, "error");
          }
        });

        grid.appendChild(card);
      });
    } catch (err) {
      grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: #ef4444; padding: 40px;">Hata: ${err.message}</div>`;
    }
  }

  async openOfflineReader(dl) {
    const item = {
      id: dl.media_id,
      provider_id: dl.provider_id || "com.vessel.reading.mangile",
      title: dl.media_title || dl.media_id,
      poster_url: dl.poster_url,
      type: 4,
      isOffline: true
    };
    const details = {
      title: item.title,
      poster_url: item.poster_url,
      chapters: [{ id: dl.chapter_id, chapter_number: dl.chapter_number, title: dl.title }]
    };
    this.openDetailsView(item);
    setTimeout(() => {
      this.openInlineChapter(item, details, dl.chapter_number, dl.chapter_id, details.chapters);
    }, 120);
  }

  // --- Multi-Device LAN Sync & Remote Control ---
  setupLanSync() {
    const lanToggle = document.getElementById("quick-lan-toggle");
    const lanModal = document.getElementById("lan-sync-modal");
    const lanClose = document.getElementById("lan-modal-close");
    const refreshBtn = document.getElementById("lan-refresh-btn");

    if (lanToggle && lanModal) {
      lanToggle.addEventListener("click", () => {
        lanModal.classList.remove("hidden");
        this.refreshLanDevices();
      });
    }

    if (lanClose && lanModal) {
      lanClose.addEventListener("click", () => {
        lanModal.classList.add("hidden");
      });
    }

    if (refreshBtn) {
      refreshBtn.addEventListener("click", () => {
        this.refreshLanDevices();
      });
    }

    // Remote control buttons
    document.querySelectorAll(".lan-remote-btn").forEach(btn => {
      btn.addEventListener("click", async () => {
        const action = btn.dataset.action;
        const payload = btn.dataset.payload || "";
        await this.sendLanRemoteCommand(action, payload);
      });
    });

    // Start background poll for remote control commands
    this.startLanCommandPolling();
  }

  async refreshLanDevices() {
    const listEl = document.getElementById("lan-devices-list");
    const localNameEl = document.getElementById("lan-local-name");
    const localPortEl = document.getElementById("lan-local-port");
    if (!listEl) return;

    listEl.innerHTML = `<div style="text-align: center; color: var(--v-text-muted); padding: 16px;">Scanning LAN for Vessel instances...</div>`;

    try {
      const res = await fetch("/api/sync/devices");
      if (!res.ok) throw new Error("Could not fetch LAN devices");
      const data = await res.json();

      if (localNameEl && data.device_name) localNameEl.textContent = data.device_name;
      if (localPortEl && data.port) localPortEl.textContent = `Port ${data.port}`;

      const devices = data.devices || [];
      const remoteDevices = devices.filter(d => d.device_id !== data.device_id);

      listEl.innerHTML = "";
      if (remoteDevices.length === 0) {
        listEl.innerHTML = `<div style="text-align: center; color: var(--v-text-muted); padding: 16px;">Yerel ağda başka aktif Vessel cihazı bulunamadı.<br><span style="font-size: 0.75rem;">(Diğer bilgisayar, tablet veya telefonunuzda Vessel'ı açtığınızda otomatik eşleşir)</span></div>`;
        return;
      }

      remoteDevices.forEach(dev => {
        const row = document.createElement("div");
        row.className = "lan-device-row";
        const isSelected = this.selectedLanTargetId === dev.device_id;
        if (isSelected) row.style.borderColor = "var(--v-accent-primary)";

        row.innerHTML = `
          <div>
            <div style="font-weight: 600; font-size: 0.9rem;">${dev.device_name || "Vessel Node"}</div>
            <div style="font-size: 0.75rem; color: var(--v-text-muted);">${dev.ip_address || "LAN"}:${dev.port || 8080}</div>
          </div>
          <button class="btn btn-secondary" style="padding: 4px 10px; font-size: 0.75rem;">
            ${isSelected ? "✓ Seçildi" : "Seç"}
          </button>
        `;

        row.addEventListener("click", () => {
          this.selectedLanTargetId = dev.device_id;
          const label = document.getElementById("lan-selected-target-label");
          if (label) label.textContent = `Hedef: ${dev.device_name} (${dev.ip_address})`;
          this.refreshLanDevices();
        });

        listEl.appendChild(row);
      });
    } catch (e) {
      listEl.innerHTML = `<div style="text-align: center; color: #ef4444; padding: 16px;">${e.message}</div>`;
    }
  }

  async sendLanRemoteCommand(action, payload = "") {
    if (!this.selectedLanTargetId) {
      this.showToast("Lütfen önce listeden kontrol edilecek bir cihaz seçin.", "error");
      return;
    }

    if (action === "cast_current") {
      const video = document.getElementById("vessel-video-element");
      if (video && video.src && !video.classList.contains("hidden")) {
        action = "load";
        payload = video.src;
      } else {
        this.showToast("Şu an oynatılan aktif bir video bulunmuyor.", "error");
        return;
      }
    }

    try {
      const res = await fetch("/api/sync/remote", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          target_id: this.selectedLanTargetId,
          action: action,
          payload: payload
        })
      });
      if (!res.ok) throw new Error("Remote command failed");
      this.showToast(`Komut iletildi: ${action}`);
    } catch (err) {
      this.showToast(err.message, "error");
    }
  }

  startLanCommandPolling() {
    setInterval(async () => {
      try {
        const res = await fetch("/api/sync/poll");
        if (!res.ok) return;
        const data = await res.json();
        const commands = data.commands || [];
        for (const cmd of commands) {
          this.executeRemoteCommand(cmd);
        }
      } catch (e) {}
    }, 4000);
  }

  executeRemoteCommand(cmd) {
    const video = document.getElementById("vessel-video-element");
    this.showToast(`📡 Uzaktan kumanda: ${cmd.action}`, "info");

    switch (cmd.action) {
      case "play":
        if (video) video.play();
        break;
      case "pause":
        if (video) video.pause();
        break;
      case "seek":
        if (video) {
          const delta = parseFloat(cmd.payload) || 0;
          video.currentTime = Math.max(0, Math.min(video.duration || 0, video.currentTime + delta));
        }
        break;
      case "volume":
        if (video) {
          const vol = parseFloat(cmd.payload) || 1;
          video.volume = Math.max(0, Math.min(1, vol));
        }
        break;
      case "load":
        if (cmd.payload) {
          this.openVesselPlayer(cmd.payload, { title: "Casted Stream from LAN" });
        }
        break;
    }
  }

  // --- Toast Notification ---
  showToast(message, type = "info") {
    const container = document.getElementById("toast-container");
    if (!container) return;

    const toast = document.createElement("div");
    toast.className = `toast toast-${type}`;
    const icon = type === "error" ? "⚠️" : (type === "warning" ? "🔔" : "✓");
    toast.innerHTML = `<span>${icon}</span><span>${message}</span>`;

    container.appendChild(toast);
    setTimeout(() => {
      toast.style.opacity = "0";
      setTimeout(() => toast.remove(), 250);
    }, 3200);
  }
}

// Global bootstrap
window.addEventListener("DOMContentLoaded", () => {
  window.vessel = new VesselApp();
  window.vessel.init();
});
