# yt-checker
Автоматический загрузчик видео с YouTube для использования в Xray [Balancer Observatory](https://xtls.github.io/ru/config/observatory.html)

## Under the hood
Из-за постоянных падений RU локаций хотелось бы иметь логику (Скачай несколько МБ видео с YouTube: если загрузка идёт HTTP 204, нет - 500 и fallback на Freedom direct). Этот checker реализует такую логику

## Запуск
```bash
git clone https://github.com/fumex-llc/yt-checker
cd yt-checker
docker compose up -d
```

## Параметры
* listen - Адрес прослушивания. По умолчанию 0.0.0.0:1111
* url_handler - URL обработчик. По умолчанию ytcheck
* urls - Список YouTube Video IDs (к примеру исходная ссылка https://www.youtube.com/watch?v=UELs4QY_oxs, следовательно ID - UELs4QY_oxs. Это может быть полезно для проверки загрузки с разных CDN edges)
* min_h - Желаемое качество для загрузки видео. По умолчанию 720p
* timeout - Значение таймаута в секундах. По умолчанию 10s
* strategy - Стратегия выбора из списка Video IDs (Round Rodin или Random). По умолчанию Random
* v6 - Флаг соединения с endpoint Video upstream через IPv6. По умолчанию false
