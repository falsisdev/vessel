// Vessel Modern Reactive UI Controller - v2.1.0
// Next-Gen Media & Reading Platform

const I18N_STRINGS = {
  en: {
    nav_cinema: "Cinema",
    nav_reading: "Manga & Comics",
    nav_live: "Live TV",
    nav_iptv: "IPTV",
    nav_library: "My Library",
    nav_plugins: "Plugins",
    nav_settings: "Settings",
    section_continue: "Continue Where You Left Off",
    section_discover: "Discover Media",
    search_placeholder: "Search movies, series, manga, anime...",
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
    status_favorites: "Favorites",
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
    plugins_curated_desc: "Discover verified official and community extensions for Vessel.",
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
    streams_loading: "Resolving highest quality stream...",
    no_streams: "No streams currently found.",
    synopsis: "Overview",
    theme_mangile: "Default Dark",
    theme_mangile_mauve: "Mauve Theme",
    theme_mangile_stone: "Stone Theme",
    theme_mangile_slate: "Slate Theme",
    theme_mangile_neutral: "Neutral Theme",
    theme_mangile_zinc: "Zinc Theme",
    theme_mangile_sunset: "Sunset Theme",
    theme_mangile_desert: "Desert Theme",
    theme_mangile_ice: "Ice Theme",
    theme_midnight_blue: "Midnight Blue",
    theme_ember: "Ember Warm",
    theme_amethyst: "Amethyst Purple",
    theme_forest: "Forest Emerald",
    theme_light_clean: "Light Clean"
  },
  tr: {
    nav_cinema: "Sinema",
    nav_reading: "Manga & Çizgi Roman",
    nav_live: "Canlı Yayın",
    nav_iptv: "IPTV",
    nav_library: "Kütüphanem",
    nav_plugins: "Eklentiler",
    nav_settings: "Ayarlar",
    section_continue: "Kaldığın Yerden Devam Et",
    section_discover: "Medya Keşfet",
    search_placeholder: "Film, dizi, manga, anime ara...",
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
    status_favorites: "Favoriler",
    settings_title: "Platform Ayarları",
    settings_theme_title: "Tema Motoru",
    settings_theme_desc: "Dahili nötr paletlerden seçin veya topluluk CSS temalarını uygulayın.",
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
    plugins_curated_desc: "Vessel için doğrulanmış resmi ve topluluk eklentilerini keşfedin.",
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
    streams_loading: "En yüksek kaliteli akış çözümleniyor...",
    no_streams: "Şu anda uygun akış kaynağı bulunamadı.",
    synopsis: "Özet",
    theme_mangile: "Varsayılan Koyu",
    theme_mangile_mauve: "Leylak Teması",
    theme_mangile_stone: "Taş Teması",
    theme_mangile_slate: "Kayrak Teması",
    theme_mangile_neutral: "Nötr Teması",
    theme_mangile_zinc: "Çinko Teması",
    theme_mangile_sunset: "Gün Batımı Teması",
    theme_mangile_desert: "Çöl Teması",
    theme_mangile_ice: "Buz Teması",
    theme_midnight_blue: "Gece Mavisi",
    theme_ember: "Sıcak Kor",
    theme_amethyst: "Ametist Moru",
    theme_forest: "Zümrüt Orman",
    theme_light_clean: "Temiz Açık"
  },
  de: {
    nav_cinema: "Kino",
    nav_reading: "Manga & Comics",
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
    nav_reading: "Manga & Bandes dessinées",
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
    status_watching: "En cours",
    status_plan: "À voir",
    status_completed: "Terminé",
    status_favorites: "Favoris",
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
    nav_reading: "Manga y Cómics",
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
    status_watching: "Viendo",
    status_plan: "Pendiente",
    status_completed: "Completado",
    status_favorites: "Favoritos",
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
    nav_reading: "Mangás e HQs",
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
    nav_reading: "Манга и Комиксы",
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
    nav_reading: "マンガ & コミック",
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
    nav_reading: "漫画与绘本",
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
    nav_reading: "المانغا والقصص",
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
    nav_reading: "مانگا و کمیک",
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

    // Search input
    const searchInput = document.getElementById("search-input");
    const clearBtn = document.getElementById("clear-search");

    searchInput.addEventListener("input", (e) => {
      const q = e.target.value.trim();
      clearBtn.classList.toggle("hidden", q === "");

      clearTimeout(this.searchDebounceTimer);
      this.searchDebounceTimer = setTimeout(() => {
        if (q === "") {
          this.clearSearch();
        } else {
          this.performSearch(q);
        }
      }, 350);
    });

    clearBtn.addEventListener("click", () => {
      searchInput.value = "";
      clearBtn.classList.add("hidden");
      this.clearSearch();
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
      const themes = await res.json();

      container.innerHTML = "";
      themes.forEach(t => {
        const item = document.createElement("div");
        item.className = `theme-item ${this.activeTheme && this.activeTheme.id === t.id ? "active" : ""}`;

        // Localized theme name
        const transKey = "theme_" + t.id.replace(/-/g, "_");
        const displayName = this.t(transKey) !== transKey ? this.t(transKey) : (t.display_name || t.name);

        const primaryColor = this.getThemeColor(t.id, "primary");
        const surfaceColor = this.getThemeColor(t.id, "surface");
        const baseColor = this.getThemeColor(t.id, "base");

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
      "mangile": { base: "#09090b", surface: "#18181b", primary: "#ffffff" },
      "mangile-mauve": { base: "#120e16", surface: "#1e1824", primary: "#ffffff" },
      "mangile-stone": { base: "#0c0a09", surface: "#1c1917", primary: "#ffffff" },
      "mangile-slate": { base: "#020617", surface: "#0f172a", primary: "#ffffff" },
      "mangile-neutral": { base: "#0a0a0a", surface: "#171717", primary: "#ffffff" },
      "mangile-zinc": { base: "#09090b", surface: "#18181b", primary: "#ffffff" },
      "mangile-sunset": { base: "#160b12", surface: "#271420", primary: "#ffffff" },
      "mangile-desert": { base: "#140e0b", surface: "#261b15", primary: "#ffffff" },
      "mangile-ice": { base: "#06131a", surface: "#0e2430", primary: "#ffffff" },
      "midnight-blue": { base: "#0b0f19", surface: "#111827", primary: "#38bdf8" },
      "ember": { base: "#18100e", surface: "#241613", primary: "#f97316" },
      "amethyst": { base: "#140e1b", surface: "#1f152b", primary: "#a855f7" },
      "forest": { base: "#0d1712", surface: "#13231c", primary: "#10b981" },
      "light-clean": { base: "#f8fafc", surface: "#ffffff", primary: "#0f172a" },
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
    const libraryView = document.getElementById("library-view");
    const pluginsView = document.getElementById("plugins-view");
    const settingsView = document.getElementById("settings-view");

    // Hide all views first
    mainView.classList.add("hidden");
    detailsView.classList.add("hidden");
    libraryView.classList.add("hidden");
    pluginsView.classList.add("hidden");
    settingsView.classList.add("hidden");

    if (route === "cinema" || route === "reading" || route === "live" || route === "iptv") {
      this.currentDomain = route;
      mainView.classList.remove("hidden");
      resumeSection.classList.remove("hidden");

      // Update Header Title based on domain
      const titleMap = {
        cinema: this.t("nav_cinema"),
        reading: this.t("nav_reading"),
        live: this.t("nav_live"),
        iptv: this.t("nav_iptv"),
      };
      document.getElementById("view-title").textContent = titleMap[route] || this.t("section_discover");

      this.updatePillsForDomain(route);
      this.loadResumeProgress();
      this.loadCatalogsForDomain(route);
    } else if (route === "library") {
      libraryView.classList.remove("hidden");
      resumeSection.classList.add("hidden");
      this.loadLibraryItems("WATCHING");
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
              <h3 class="catalog-title">${catRow.title}</h3>
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

  // --- Search System ---
  async performSearch(query) {
    const searchSection = document.getElementById("search-results-section");
    const grid = document.getElementById("search-media-grid");
    const catalogsContainer = document.getElementById("catalogs-container");
    const resumeSection = document.getElementById("resume-section");

    searchSection.classList.remove("hidden");
    catalogsContainer.classList.add("hidden");
    resumeSection.classList.add("hidden");

    grid.innerHTML = `
      <div style="grid-column: 1/-1; text-align: center; padding: 40px; color: var(--v-text-muted);">
        <div style="font-size: 1.5rem; margin-bottom: 8px;">⏳</div>
        <div>Searching...</div>
      </div>
    `;

    try {
      const domainNum = this.currentDomain === "reading" ? 2 : 1;
      const res = await fetch(`/api/search?domain=${domainNum}&query=${encodeURIComponent(query)}`);
      if (!res.ok) throw new Error("Search query failed");
      const data = await res.json();
      const items = data.items || [];

      grid.innerHTML = "";
      if (items.length === 0) {
        grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; padding: 40px; color: var(--v-text-muted);">${this.t("no_results")}</div>`;
        return;
      }

      items.forEach(it => {
        grid.appendChild(this.createMediaCard(it));
      });
    } catch (e) {
      grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-status-error); padding: 30px;">${e.message}</div>`;
    }
  }

  clearSearch() {
    document.getElementById("search-results-section").classList.add("hidden");
    document.getElementById("catalogs-container").classList.remove("hidden");
    document.getElementById("resume-section").classList.remove("hidden");
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
    const year = item.year || item.Year || (typeVal >= 4 ? "" : "2024");
    const rating = item.rating || 8.5;
    const isReading = this.currentDomain === "reading" || typeVal >= 4;

    card.dataset.type = typeLabel;

    card.innerHTML = `
      <div class="poster-wrapper">
        <img src="${poster}" alt="${rawTitle}" class="poster-img" loading="lazy" onerror="this.src='https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=400'">
        <span class="card-badge">${typeLabel}</span>
        <div class="poster-overlay-btn">${isReading ? "📖" : "▶"}</div>
      </div>
      <div class="card-details">
        <div class="card-title" title="${rawTitle}">${rawTitle}</div>
        <div class="card-meta">
          <span>${year}</span>
          <span class="rating-badge">★ ${rating}</span>
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
        overview: item.overview || item.Overview || ""
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
      7: "Book"
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

    detailsView.classList.remove("hidden");
    window.scrollTo({ top: 0, behavior: "smooth" });

    content.innerHTML = `
      <div style="text-align: center; padding: 100px; color: var(--v-text-muted);">
        <div style="font-size: 2.2rem; margin-bottom: 12px; animation: pulse-online 1.5s infinite;">⏳</div>
        <div>Loading details...</div>
      </div>
    `;

    try {
      const isReading = this.currentDomain === "reading" || item.type >= 4;
      const domainNum = isReading ? 2 : 1;
      const res = await fetch(`/api/media?domain=${domainNum}&provider=${encodeURIComponent(item.provider_id || "")}&id=${encodeURIComponent(item.id)}`);
      const details = res.ok ? (await res.json()) : item;

      if (isReading) {
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
    this.switchRoute(this.previousRoute || "cinema");
  }

  renderCinemaDetails(item, details) {
    const content = document.getElementById("details-content");
    const title = details.title || item.title || "Unknown";
    const poster = details.poster_url || item.poster_url;
    const year = details.year || item.year || 2024;
    const genres = details.genres || ["Cinema", "Drama"];
    const seasons = details.seasons || [];
    const isSeries = seasons.length > 0 || item.type === 2 || item.type === 3;

    content.innerHTML = `
      <div class="details-hero">
        <div class="details-hero-backdrop" style="background-image: url('${poster}')"></div>
        <div class="details-poster-col">
          <img class="details-poster-img" src="${poster}" alt="${title}">
          <button class="btn btn-secondary" id="details-lib-btn" style="width: 100%;">
            ${this.t("btn_add_library")}
          </button>
        </div>
        <div class="details-info-col">
          <div class="details-meta-tags">
            <span class="badge-subtle" style="background: rgba(var(--v-accent-primary-rgb, 255,255,255), 0.15); color: var(--v-accent-primary); font-weight: 700;">
              ${this.mapMediaType(item.type)}
            </span>
            <span class="badge-subtle">★ 8.5</span>
            <span class="badge-subtle">${year}</span>
            ${genres.map(g => `<span class="badge-subtle">${g}</span>`).join("")}
          </div>
          <h1 class="details-title">${title}</h1>
          <p class="details-overview">${details.overview || item.overview || "No overview available."}</p>
          <div class="details-actions">
            <button class="btn btn-primary" id="details-watch-now-btn" style="font-size: 1rem; padding: 10px 24px;">
              ▶ ${this.t("btn_play")}
            </button>
          </div>
        </div>
      </div>

      <div class="details-subsections">
        <!-- Interactive Player Container (embedded directly on page when played) -->
        <div id="inline-player-box" class="details-section-box hidden">
          <div class="player-container">
            <video id="vessel-video-player" controls autoplay style="width: 100%; height: 100%; border-radius: 8px;"></video>
          </div>
        </div>

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
            <div class="episodes-grid" id="details-episodes-grid">
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

    // Library toggle action
    document.getElementById("details-lib-btn").addEventListener("click", () => {
      this.toggleLibrary(item, details);
    });

    // Watch Now button
    document.getElementById("details-watch-now-btn").addEventListener("click", () => {
      this.playMediaStream(item, 1, 1);
    });

    // If series, setup season tabs
    if (isSeries && seasons.length > 0) {
      const renderSeasonEpisodes = (sIdx) => {
        const grid = document.getElementById("details-episodes-grid");
        const curSeason = seasons[sIdx];
        if (!curSeason || !grid) return;
        const eps = curSeason.episodes || [];

        grid.innerHTML = eps.length > 0 ? eps.map(ep => `
          <div class="episode-card" data-ep="${ep.episode_number || 1}">
            <div class="episode-card-header">
              <span>${this.t("episode")} ${ep.episode_number || 1}: ${ep.title || "Episode " + (ep.episode_number || 1)}</span>
              <button class="btn btn-secondary" style="padding: 4px 12px; font-size: 0.8rem;">▶</button>
            </div>
            <div class="episode-card-overview">${ep.overview || "No episode synopsis available."}</div>
          </div>
        `).join("") : `<div style="padding: 20px; color: var(--v-text-muted);">No episodes found.</div>`;

        grid.querySelectorAll(".episode-card").forEach(c => {
          c.addEventListener("click", () => {
            const epNum = parseInt(c.dataset.ep, 10);
            this.playMediaStream(item, curSeason.season_number || (sIdx + 1), epNum);
          });
        });
      };

      renderSeasonEpisodes(0);

      document.querySelectorAll("#details-season-tabs .tab").forEach(tab => {
        tab.addEventListener("click", () => {
          document.querySelectorAll("#details-season-tabs .tab").forEach(t => t.classList.remove("active"));
          tab.classList.add("active");
          renderSeasonEpisodes(parseInt(tab.dataset.seasonIdx, 10));
        });
      });
    }

    // Load available streams
    this.loadStreamsList(item, 1, 1);
  }

  renderReadingDetails(item, details) {
    const content = document.getElementById("details-content");
    const title = details.title || item.title || "Unknown";
    const poster = details.poster_url || item.poster_url;
    const chapters = details.chapters || [];
    const genres = details.genres || ["Manga"];

    content.innerHTML = `
      <div class="details-hero">
        <div class="details-hero-backdrop" style="background-image: url('${poster}')"></div>
        <div class="details-poster-col">
          <img class="details-poster-img" src="${poster}" alt="${title}">
          <button class="btn btn-secondary" id="details-lib-btn" style="width: 100%;">
            ${this.t("btn_add_library")}
          </button>
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
                📖 ${this.t("btn_read")} ${this.t("chapter")} 1
              </button>
            ` : ""}
          </div>
        </div>
      </div>

      <div class="details-subsections">
        <!-- Interactive Reader Box (Embedded right in page when reading) -->
        <div id="inline-reader-box" class="details-section-box hidden">
          <div class="reader-container" id="inline-reader-container">
            <!-- Reader canvas/pages injected here -->
          </div>
        </div>

        <div class="details-section-box">
          <h3 class="details-section-title">${this.t("chapters")} (${chapters.length})</h3>
          <div class="episodes-grid" style="max-height: 420px;">
            ${chapters.length > 0 ? chapters.map(ch => {
              const chNum = ch.chapter_number || 1;
              return `
                <div class="chapter-row" data-ch="${chNum}" style="margin-bottom: 8px;">
                  <div>
                    <span style="font-weight: 600;">${this.t("chapter")} ${chNum}</span>
                    ${ch.title ? `<span style="color: var(--v-text-muted); margin-left: 8px;">- ${ch.title}</span>` : ""}
                  </div>
                  <button class="btn btn-secondary" style="padding: 5px 14px; font-size: 0.8rem;">${this.t("btn_read")}</button>
                </div>
              `;
            }).join("") : `<div style="text-align: center; color: var(--v-text-muted); padding: 24px;">No chapters found.</div>`}
          </div>
        </div>
      </div>
    `;

    document.getElementById("details-lib-btn").addEventListener("click", () => {
      this.toggleLibrary(item, details);
    });

    const readBtn = document.getElementById("details-read-now-btn");
    if (readBtn && chapters.length > 0) {
      readBtn.addEventListener("click", () => {
        this.openInlineChapter(item, details, chapters[0].chapter_number || 1, chapters);
      });
    }

    content.querySelectorAll(".chapter-row").forEach(row => {
      row.addEventListener("click", () => {
        const chNum = parseFloat(row.dataset.ch);
        this.openInlineChapter(item, details, chNum, chapters);
      });
    });
  }

  async openInlineChapter(item, details, chapterNum, chapters) {
    const readerBox = document.getElementById("inline-reader-box");
    const container = document.getElementById("inline-reader-container");
    readerBox.classList.remove("hidden");
    readerBox.scrollIntoView({ behavior: "smooth" });

    container.innerHTML = `<div style="text-align: center; padding: 40px; color: var(--v-text-muted);"><span style="font-size: 1.5rem;">⏳</span> Loading chapter ${chapterNum}...</div>`;

    try {
      const res = await fetch(`/api/chapter?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&chapter_num=${chapterNum}`);
      if (!res.ok) throw new Error("Could not fetch chapter content");
      const content = await res.json();

      let pagesHTML = "";
      if (content.pages && content.pages.length > 0) {
        pagesHTML = `
          <div class="reader-pages-flow">
            ${content.pages.map(p => `<img src="${p.url}" alt="Page ${p.page_number}" loading="lazy" style="margin-bottom: 12px;">`).join("")}
          </div>
        `;
      } else if (content.text_content) {
        pagesHTML = `<div class="reader-text-content">${content.text_content}</div>`;
      }

      container.innerHTML = `
        <div class="reader-header" style="margin-bottom: 20px;">
          <h4>${details.title || item.title} - ${this.t("chapter")} ${chapterNum}</h4>
          <button class="btn btn-secondary" id="close-reader-btn" style="padding: 4px 12px;">✕ Close Reader</button>
        </div>
        ${pagesHTML}
      `;

      document.getElementById("close-reader-btn").addEventListener("click", () => {
        readerBox.classList.add("hidden");
      });

      // Record reading progress
      await fetch("/api/progress/reading", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider_id: item.provider_id,
          media_id: item.id,
          domain: 2,
          chapter_number: chapterNum,
          current_page: 1,
          total_pages: content.pages?.length || 1,
          is_completed: false
        })
      });
    } catch (e) {
      container.innerHTML = `<div style="text-align: center; padding: 40px; color: var(--v-status-error);">Failed to load chapter: ${e.message}</div>`;
    }
  }

  async playMediaStream(item, season, episode) {
    const playerBox = document.getElementById("inline-player-box");
    const video = document.getElementById("vessel-video-player");
    playerBox.classList.remove("hidden");
    playerBox.scrollIntoView({ behavior: "smooth" });

    try {
      const res = await fetch(`/api/streams?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&season=${season}&episode=${episode}`);
      if (!res.ok) throw new Error("Could not fetch stream sources");
      const data = await res.json();
      const streams = data.streams || [];

      if (streams.length === 0) {
        this.showToast("No active stream found for this item.", "warning");
        return;
      }

      // Resolve best stream
      const best = streams[0];
      const resolveRes = await fetch("/api/stream/resolve", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: best.url, media_id: item.id, season, episode })
      });
      const resolved = resolveRes.ok ? (await resolveRes.json()) : { url: best.url };

      video.src = resolved.url;
      video.play();

      // Record playback progress
      video.ontimeupdate = () => {
        if (video.currentTime > 5 && Math.floor(video.currentTime) % 10 === 0) {
          fetch("/api/progress/playback", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              provider_id: item.provider_id,
              media_id: item.id,
              domain: 1,
              season_number: season,
              episode_number: episode,
              current_position: video.currentTime,
              total_duration: video.duration || 0,
              progress_percent: video.duration > 0 ? (video.currentTime / video.duration) * 100 : 0,
              is_completed: false
            })
          });
        }
      };
    } catch (e) {
      this.showToast(`Playback error: ${e.message}`, "error");
    }
  }

  async loadStreamsList(item, season, episode) {
    const grid = document.getElementById("details-streams-grid");
    if (!grid) return;

    try {
      const res = await fetch(`/api/streams?provider=${encodeURIComponent(item.provider_id || "")}&media=${encodeURIComponent(item.id)}&season=${season}&episode=${episode}`);
      if (!res.ok) throw new Error("Could not load streams");
      const data = await res.json();
      const streams = data.streams || [];

      if (streams.length === 0) {
        grid.innerHTML = `<div style="padding: 12px; color: var(--v-text-muted);">${this.t("no_streams")}</div>`;
        return;
      }

      grid.innerHTML = streams.map(st => `
        <div class="stream-card-row">
          <div>
            <span style="font-weight: 700; margin-right: 8px;">${st.quality || "1080p"}</span>
            <span>${st.title || "Stream Source"}</span>
          </div>
          <button class="btn btn-secondary stream-play-btn" style="padding: 4px 14px; font-size: 0.8rem;">▶ Stream</button>
        </div>
      `).join("");

      grid.querySelectorAll(".stream-play-btn").forEach((btn, idx) => {
        btn.addEventListener("click", () => {
          this.playMediaStream(item, season, episode);
        });
      });
    } catch (e) {
      grid.innerHTML = `<div style="padding: 12px; color: var(--v-text-muted);">${this.t("no_streams")}</div>`;
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

      installedList.innerHTML = "";
      if (installed.length === 0) {
        installedList.innerHTML = `<div style="grid-column: 1/-1; text-align: center; padding: 30px; color: var(--v-text-muted);">No plugins installed.</div>`;
      } else {
        installed.forEach(p => {
          const card = document.createElement("div");
          card.className = "plugin-card";
          card.innerHTML = `
            <div class="plugin-card-header">
              <div>
                <h4 class="plugin-card-title">${p.name || p.id}</h4>
                <div style="font-size: 0.8rem; color: var(--v-text-muted); margin-top: 4px;">v${p.version || "1.0.0"} • ${p.author || "Vessel"}</div>
              </div>
              <span class="badge-subtle" style="background: rgba(var(--v-status-success-rgb, 16, 185, 129), 0.15); color: var(--v-status-success);">
                ${this.t("btn_active")}
              </span>
            </div>
            <p class="plugin-card-desc">${p.description || "Media & catalog provider extension."}</p>
            <div class="plugin-card-footer">
              <span style="font-size: 0.8rem; color: var(--v-text-muted);">${p.is_builtin ? this.t("plugin_builtin") : this.t("plugin_external")}</span>
              <button class="btn btn-secondary" style="font-size: 0.8rem; padding: 4px 12px;">${this.t("btn_disable")}</button>
            </div>
          `;
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

        card.innerHTML = `
          <div class="plugin-card-header">
            <div>
              <h4 class="plugin-card-title">${p.name}</h4>
              <div style="font-size: 0.8rem; color: var(--v-text-muted); margin-top: 4px;">v${p.version} • ${p.author}</div>
            </div>
            <span class="badge-subtle">${p.domain.toUpperCase()}</span>
          </div>
          <p class="plugin-card-desc">${p.description}</p>
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
        const title = p.title || p.Title || p.media_id || p.MediaID || "Media";

        if (isReading) {
          const chNum = p.chapter_number || p.ChapterNumber || 1;
          const currPage = p.current_page || p.CurrentPage || 1;
          const totPages = p.total_pages || p.TotalPages || 1;
          const percent = totPages > 0 ? Math.min(100, Math.round((currPage / totPages) * 100)) : 0;

          card.innerHTML = `
            <div class="resume-info">
              <div class="resume-title">${title}</div>
              <div class="resume-sub">${this.t("chapter")} ${chNum} • ${currPage}/${totPages}</div>
              <div class="progress-bar-container">
                <div class="progress-bar-fill" style="width: ${percent}%;"></div>
              </div>
            </div>
          `;
          card.addEventListener("click", () => {
            this.openDetailsView({ id: p.media_id, provider_id: p.provider_id, title, type: 4 });
          });
        } else {
          const percent = Math.min(100, Math.round(p.progress_percent || p.ProgressPercent || 0));
          const sNum = p.season_number || p.SeasonNumber || 1;
          const epNum = p.episode_number || p.EpisodeNumber || 1;

          card.innerHTML = `
            <div class="resume-info">
              <div class="resume-title">${title}</div>
              <div class="resume-sub">${this.t("season")} ${sNum} • ${this.t("episode")} ${epNum}</div>
              <div class="progress-bar-container">
                <div class="progress-bar-fill" style="width: ${percent}%;"></div>
              </div>
            </div>
          `;
          card.addEventListener("click", () => {
            this.openDetailsView({ id: p.media_id, provider_id: p.provider_id, title, type: 1 });
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
    grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 40px;">⏳ Loading collection...</div>`;

    try {
      const res = await fetch(`/api/library?status=${status}`);
      if (!res.ok) throw new Error("Failed to load library");
      const data = await res.json();
      const items = data.items || [];

      grid.innerHTML = "";
      if (items.length === 0) {
        grid.innerHTML = `<div style="grid-column: 1/-1; text-align: center; color: var(--v-text-muted); padding: 50px;">No items in this collection tab yet.</div>`;
        return;
      }

      items.forEach(it => {
        grid.appendChild(this.createMediaCard(it));
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
