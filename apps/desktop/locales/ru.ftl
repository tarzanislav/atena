# ATENA – тексты интерфейса на русском языке.
# Эталонный язык – словацкий (sk.ftl); у каждого сообщения здесь тот же идентификатор.
# Машинный перевод: черновик, пока его не проверит носитель языка.

language-name = Русский

## Приложение

app-claim = Проверенная информация об образовании
app-version = Версия { $version } · открытое программное обеспечение
translation-draft = перевод – черновик
yes = да
no = нет
action-close = Закрыть
action-cancel = Отмена
action-open-original = Открыть оригинал

## Запуск

startup-starting = ATENA запускается…
startup-first-time = При первом запуске ATENA создаёт собственную базу данных на этом компьютере и загружает реестр источников. Это занимает около полуминуты; в следующий раз будет быстрее.
startup-failed-hint = Закройте ATENA и откройте снова. Если ошибка повторится, подробности есть в журнале в папке приложения (logs/atena.log).

## Навигация

nav-label = Главное меню
nav-overview = Обзор
nav-search = Поиск
nav-tools = Инструменты
nav-sources = Источники
nav-proposals = Предложения
nav-crawl = Сбор
nav-settings = Настройки

## Обзор

overview-title = Обзор
overview-empty-title = Пока ничего не собрано
overview-empty-text = Запустите сбор. ATENA посещает источники реестра как вежливый посетитель: соблюдает их правила, никуда не спешит и берёт только то, что разрешают закон и лицензия.
overview-empty-action = Перейти к сбору
overview-documents =
    { $count ->
        [one] документ
        [few] документа
       *[many] документов
    }
overview-versions =
    { $count ->
        [one] { $count } версия
        [few] { $count } версии
       *[many] { $count } версий
    }
overview-searchable = документов доступно для поиска
overview-sources = источников собирается (из всех в реестре)
overview-proposals =
    { $count ->
        [one] предложение ждёт решения
        [few] предложения ждут решения
       *[many] предложений ждут решения
    }
overview-last-check = Последняя проверка: { $time } · за последние 24 часа { $checks ->
        [one] { $checks } проверка
        [few] { $checks } проверки
       *[many] { $checks } проверок
    }
overview-never-checked = Источники ещё не проверялись.
overview-newest = Новое
overview-search-all = Искать везде
overview-sections = Категории
overview-in-scope =
    { $total ->
        [one] { $total } документ
        [few] { $total } документа
       *[other] { $total } документов
    } в этой выборке
overview-new-in-scope = новых за { $days } дн.: { $new }
overview-counts-legend = У категорий: новые за последние { $days } дн. / все
overview-counts-title = новых за последние { $days } дн.: { $new }, всего { $total }
overview-sources-line = Собирается { $crawled } из { $sources } источников

## Словацкие или зарубежные данные, надёжность источника

scope-label = Какие данные показывать
scope-sk = Словакия
scope-abroad = Зарубежье
scope-languages = Язык зарубежных данных
scope-all-languages = Все языки
trust-official = Официальный источник
trust-expert = Экспертный источник
trust-other = Другой источник – проверьте
tools-title = Инструменты и словари
tools-intro = Словари, корпусы и приложения, которые ATENA не собирает, – поиск в них ведётся напрямую. Они открываются в вашем браузере.
tools-open = Открыть инструмент
tools-none = Для этих данных инструментов пока нет.

## Поиск

search-title = Поиск
search-placeholder = Методы, нормативные акты, материалы… на любом языке
search-action = Искать
search-hint = Напишите, что вы ищете, на любом языке. ATENA ищет и на других языках с помощью понятий, которые распознаёт в словах.
search-hint-syntax = Фразу заключите в кавычки («читательская грамотность»), слово исключите минусом (-каникулы).
search-filters = Сужено до:
search-filter-remove = Отменить это сужение
search-narrow = Сузить результаты
search-found =
    { $count ->
        [0] Не найдено ни одного документа.
        [one] Найден { $count } документ.
        [few] Найдено { $count } документа.
       *[many] Найдено { $count } документов.
    }
search-some-words = Ни в одном документе нет всех слов – это документы с некоторыми из них.
search-concepts = Также искал понятия:
search-nothing = Попробуйте другие слова или отмените какое-либо сужение.
search-previous = Назад
search-next = Далее
search-page = Страница { $page } из { $pages }
search-only-official = Только официальные источники
search-tools = Инструменты по искомым словам
hit-no-text = Текст этого источника в ATENA не показывается – лицензия этого не разрешает. Откройте оригинал.

facet-section = Раздел
facet-school-type = Тип школы
facet-resource-type = Тип материала
facet-topic = Тема
facet-country = Страна
facet-authority = Авторитет источника
facet-language = Язык
facet-source = Источник
facet-trust = Надёжность источника

## Карточка документа

document-title = Документ
document-untitled = (без названия)
document-not-shown = ATENA не показывает этот документ: он удалён, его издатель не хочет, чтобы его находили, или его источник нельзя собирать.
document-published = опубликовано { $date }
document-labels-are-suggestions = Отнесение к разделам и темам – предложение ATENA, пока его не проверит человек.
document-text = Текст
document-text-not-shown = Полный текст в ATENA не показывается – лицензия источника ({ $licence }) этого не разрешает. Прочитайте его в оригинале.
document-versions =
    { $count ->
        [one] { $count } версия
        [few] { $count } версии
       *[many] { $count } версий
    }
version-number = Версия
version-captured = Сохранена
version-change = Изменение
version-paragraphs = (+{ $added } / −{ $removed } абзацев)
version-numbers-changed = изменилось число или дата

change-initial = первое сохранение
change-minor = незначительное изменение
change-substantive = существенное изменение
change-changed = текст изменился (предыдущий не хранится)

## Источники

sources-title = Источники
sources-intro = Источники реестра и подтверждённые предложения. Для каждого видно, что разрешает принцип законности: можно ли его собирать, можно ли хранить текст и можно ли показывать его полностью.
sources-sync = Синхронизировать общий реестр
sources-sync-explain = ATENA скачает актуальный общий реестр источников из репозитория ATENA на GitHub (файл data/sources.yaml, только по шифрованному соединению https) и приведёт базу данных в соответствие с ним. То, что принцип законности больше не разрешает, будет удалено.
sources-sync-privacy = Отправляется только обычный запрос файла; о вас и ваших предложениях ничего не отправляется.
sources-sync-confirm = Скачать и синхронизировать
sources-synced = Реестр синхронизирован: источников – { $sources }, новых адресов – { $added }. Удалено документов: { $purged }, версий сокращено до метаданных: { $restricted }.
sources-show-all = Все
sources-show-crawled = Собираются
sources-show-not-crawled = Не собираются
sources-show-imported = Из предложений
sources-filter = Фильтр по названию, адресу или стране
sources-column-source = Источник
sources-column-country = Страна
sources-column-rights = Лицензия и условия
sources-column-crawl = Сбор
sources-column-text = Текст
sources-column-show = Показ
sources-column-documents = Документы
sources-checked = проверка { $date }
sources-tool = Инструмент (не собирается)
sources-site-wide = Весь сайт
sources-none = Ни один источник не соответствует выбору.

allowed-crawl = можно собирать
allowed-text = можно хранить текст
allowed-show = можно показывать полный текст

licence-copyrighted = защищено авторским правом
licence-official = официальный текст (без авторско-правовой охраны)

terms-allowed = условия разрешают
terms-forbidden = условия запрещают
terms-reserved = оговорено (анализ текстов)
terms-unknown = условия не проверены

state-proposed = предложение
state-confirmed = подтверждён
state-rejected = отклонён
state-merged = принят в реестр

restriction-no-profile = для страны источника у ATENA нет правового профиля
restriction-terms-forbid = условия использования запрещают автоматический доступ
restriction-terms-not-verified = охраняемый контент: условия использования ещё никто не проверил
restriction-research-only = охраняемый контент: страна источника разрешает анализ данных только для некоммерческих исследований
restriction-no-mining-exception = охраняемый контент: в стране источника нет исключения для анализа текстов и данных
restriction-mining-reserved = издатель оговорил право на анализ текстов и данных
restriction-privacy-law = закон о защите данных страны источника не разрешает хранить тексты за рубежом

## Предложения

proposals-title = Предложенные источники
proposals-intro = Предложите страницы, за которыми ATENA должна следить. Сначала она оценит каждый адрес; сбор начнётся, только когда предложение подтвердит человек.
proposals-tab-add = Добавить адреса
proposals-tab-waiting = Ждут решения
proposals-tab-decided = Решённые
proposals-tab-share = Передать редактору
proposals-tab-receive = Принять список

import-addresses = Адреса или DOI, каждый на отдельной строке
import-placeholder = https://www.school.ru/news/
import-format = После адреса можно добавить код страны, право которой применяется (например «https://www.skola.cz/ CZ»). Не более 500 адресов за раз.
import-country = Страна для адресов без кода
import-country-auto = по адресу
import-how = ATENA открывает каждый адрес как вежливый посетитель: читает robots.txt, указание лицензии и условия использования и предлагает то, что разрешает закон. Ничего не собирается, пока вы не подтвердите предложение. Страницы о людях не сохраняются даже как предложения.
import-action =
    { $count ->
        [0] Оценить
        [one] Оценить { $count } адрес
        [few] Оценить { $count } адреса
       *[many] Оценить { $count } адресов
    }
import-busy = Оцениваю… при большом количестве адресов это может занять несколько минут.
import-done =
    { $count ->
        [one] Оценён { $count } адрес.
        [few] Оценено { $count } адреса.
       *[many] Оценено { $count } адресов.
    }
import-kept-for-sharing =
    { $count ->
        [one] { $count } адрес ждёт передачи редактору.
        [few] { $count } адреса ждут передачи редактору.
       *[many] { $count } адресов ждут передачи редактору.
    }
import-column-line = Строка
import-column-address = Адрес
import-column-result = Результат
import-column-proposal = Предложение

outcome-new = новое предложение
outcome-reassessed = предложение оценено заново
outcome-already-proposed = уже предложен
outcome-already-confirmed = уже подтверждён
outcome-already-rejected = уже отклонён
outcome-already-merged = уже в реестре
outcome-in-register = уже в реестре ({ $register })
outcome-blocked = заблокировано по запросу об удалении
outcome-personal-page = страница о людях – не сохраняется

problem-robots = robots.txt не разрешает ATENA читать страницу
problem-robots-unreachable = robots.txt не удалось загрузить, поэтому пока ничего не читается
problem-blocked = этот адрес не разрешён (частная сеть, необычный порт…)
problem-status = сервер ответил ошибкой
problem-unreachable = сервер недоступен
problem-retry-later = сервер попросил прийти позже
problem-unsupported-type = такой вид содержимого ATENA не читает
problem-too-large = содержимое больше, чем читает ATENA
problem-dangerous = предлагает программу или документ с активным содержимым – из таких источников ATENA не черпает
problem-too-demanding = содержимому потребовалось бы больше памяти или времени, чем любому настоящему документу – ATENA его не использует

reason-cannot-read = адрес не удалось прочитать
reason-personal-page = страница о людях
reason-automation-ban = условия запрещают автоматический доступ
reason-personal-use-only = условия разрешают только личное использование
reason-mining-reserved = условия оговаривают анализ текстов или ИИ
reason-tdm-reserved = TDMRep оговаривает анализ текстов и данных
reason-ai-crawlers-refused = robots.txt отказывает сборщикам для ИИ
reason-open-licence = открытая лицензия
reason-licence-not-open = лицензия не открытая или противоречит себе
reason-non-commercial = только некоммерческое использование
reason-rights-reserved = все права защищены
reason-terms-read = условия прочитаны
reason-no-terms-found = условия не найдены
reason-terms-unreadable = условия не удалось прочитать
reason-unknown-country = нужно указать страну
reason-dangerous-content = опасное содержимое

rejection-not-an-address = это не адрес и не DOI
rejection-unsupported-scheme = только адреса http и https
rejection-credentials = адрес содержит имя или пароль
rejection-duplicate = повторяется
rejection-too-many = слишком много адресов за раз
rejection-too-long = строка слишком длинная

proposals-none-waiting = Нет предложений, ждущих решения.
proposals-none-decided = Вы ещё не приняли решения ни по одному предложению.
proposal-shared = из другой установки
proposal-confirm = Подтвердить
proposal-reject = Отклонить
proposal-automatic = Предложение ATENA ({ $date }): { $licence } · { $terms } · страна { $country }
proposal-decided = Решение: { $licence } · { $terms } · страна { $country }
proposal-would-allow = Подтверждение разрешило бы:
proposal-finding = Запись о проверке (на английском, как она хранится)
proposal-rights-page = Открыть страницу с лицензией или условиями
proposal-confirm-title = Подтвердить «{ $name }»
proposal-reject-title = Отклонить «{ $name }»
proposal-confirmed-crawled = Подтверждено. Источник будет собираться при следующем сборе.
proposal-confirmed-not-crawled = Подтверждено. Однако принцип законности не разрешает собирать этот источник.
proposal-rejected = Отклонено. Всё, что было собрано из этого источника, удалено.

decision-licence = Лицензия
decision-terms = Условия использования
decision-country = Страна, право которой применяется
decision-reason-confirm = Почему – что вы прочитали и когда (обязательно)
decision-reason-reject = Почему вы отклоняете (обязательно)
decision-by = Решает: { $name } (имя хранится только на этом компьютере)

share-explain = Список содержит только адреса и коды стран – ничего о вас. ATENA никуда его не отправляет: скопируйте его и передайте редактору ATENA удобным вам способом (например, по электронной почте). Затем отметьте его как переданный.
share-nothing = Нечего передавать.
share-list =
    { $count ->
        [one] Список с { $count } адресом
       *[many] Список с { $count } адресами
    }
share-copy = Копировать
share-copied = Скопировано
share-mark-handed-over = Отметить как переданное
share-handed-over = Список отмечен как переданный.

receive-explain = Для редактора: вставьте список, который вам прислал пользователь другой установки. ATENA сама оценит каждый адрес – полученной оценке нечего доверять – и сохранит их как предложения.
receive-list = Список (начинается строкой ATENA-NAVRHY 1)
receive-action = Прочитать список
receive-not-a-list = Это не список ATENA: первая строка должна быть ATENA-NAVRHY 1.
receive-read-before = Этот список уже был прочитан ({ $list }).

## Сбор

crawl-title = Сбор
crawl-intro = ATENA посещает источники реестра и подтверждённые предложения, сохраняет новые и изменённые документы и делает их доступными для поиска. Сбор идёт, только когда вы его запускаете.
crawl-state-running = Сбор идёт
crawl-state-stopping = Сбор останавливается – завершаются начатые проверки
crawl-state-stopped = Сбор остановлен
crawl-running = сбор идёт
crawl-start = Запустить сбор
crawl-stop = Остановить
crawl-started = запущен { $time }
crawl-checks = проверок
crawl-hosts = посещённых серверов
crawl-due = адресов ждут проверки
crawl-store-errors = ошибок базы данных
crawl-outcomes = Результаты проверок
crawl-rules-title = Как ATENA собирает
crawl-rule-robots = Соблюдает robots.txt и правила издателя (TDMRep, noindex, nosnippet).
crawl-rule-pace = На каждый сервер ходит медленно, запрос за запросом, а когда сервер просит покоя, ждёт.
crawl-rule-open = Черпает только из открытых источников и хранит только то, что разрешают закон и лицензия.
crawl-rule-privacy = Ходит как анонимный посетитель: без cookie, без скриптов и без изображений. О вас ничего не отправляет.
crawl-rule-safety = Отклоняет программы и документы с активным содержимым, обрабатывает документы в ограниченном процессе и не хранит исходные файлы.
crawl-concurrency =
    { $count ->
        [one] Одновременно идёт работа с { $count } сервером (можно изменить в настройках).
       *[many] Одновременно идёт работа с { $count } серверами (можно изменить в настройках).
    }

check-new = новый документ
check-changed = изменённый документ
check-unchanged = без изменений
check-not-modified = без изменений (по данным сервера)
check-feed = канал RSS/Atom прочитан
check-sitemap = карта сайта прочитана
check-robots = robots.txt не разрешает
check-robots-unreachable = robots.txt недоступен
check-blocked = адрес не разрешён
check-http-error = ошибка сервера
check-gone = документ больше не существует
check-network = сеть недоступна
check-timeout = истекло время
check-redirects = слишком много перенаправлений
check-retry-later = сервер просит подождать
check-too-large = слишком большое
check-unsupported = неподдерживаемый вид содержимого
check-dangerous = опасное содержимое отклонено
check-too-demanding = слишком требовательное содержимое отклонено
check-extract-error = содержимое не удалось обработать
check-feed-error = ошибочный канал RSS/Atom
check-sitemap-error = ошибочная карта сайта
check-moved-elsewhere = перенесено на другой сайт
check-erased = удалено по запросу
check-not-permitted = принцип законности не разрешает
check-invalid-url = недействительный адрес
check-store-error = ошибка базы данных

## Настройки

settings-title = Настройки
settings-saved = Сохранено.
settings-appearance = Язык и оформление
settings-language = Язык интерфейса
settings-theme = Оформление
settings-theme-system = как в системе
settings-theme-light = светлое
settings-theme-dark = тёмное
settings-decisions = Решения по предложениям
settings-editor = Ваше имя или псевдоним
settings-editor-privacy = Записывается к вашим решениям и остаётся только на этом компьютере. Общий реестр не называет ни одного человека.
settings-share = Откладывать предложенные адреса для редактора
settings-share-explain = ATENA сама ничего не отправляет; отложенные адреса находятся в разделе Предложения → Передать редактору.
settings-crawler = Сбор
settings-concurrency = Серверов одновременно (1–16)
settings-concurrency-explain = На каждый сервер ATENA всегда отправляет только один запрос за раз; это число показывает, сколько разных серверов она посещает одновременно.

erasure-title = Запрос на удаление
erasure-explain = Если кто-то просит об удалении (ст. 17 GDPR), ATENA удаляет всё об этом адресе и больше никогда его не посещает.
erasure-address = Адрес
erasure-prefix = и все адреса, которые с него начинаются
erasure-reason = Причина (например: запрос от…)
erasure-action = Удалить и заблокировать
erasure-done = Удалено: документов – { $documents }, версий – { $versions }. Адрес заблокирован.

about-title = Об ATENA
about-version = ATENA { $version } – открытое программное обеспечение под лицензией Apache 2.0, бесплатное для частного и коммерческого использования.
about-connections = ATENA никому ничего не отправляет о своих пользователях. Сама она никуда не подключается; в сеть выходит, только когда вы запускаете сбор, оценку адресов или синхронизацию реестра, и тогда как анонимный посетитель. Сторонние сервисы не используются.
about-pillars = Четыре опоры ATENA
pillar-1 = Серьёзность, открытость и стремление продвигать знания каждого человека.
pillar-2 = Анонимность и уважение к частной жизни – нашей и чужой.
pillar-3 = Безопасность и защита от опасных людей, роботов, кода и процессов.
pillar-4 = Соблюдение закона: ни один закон не может быть нарушен, нигде в мире.
about-licence = Лицензии всех компонентов находятся в папке приложения «notices».
about-repository = Исходный код на GitHub

## Ошибки

error-detail = Подробности
error-already-running = ATENA уже запущена. Работайте в открытом окне.
error-crawl = Не удалось подготовить сбор.
error-database = Ошибка базы данных.
error-database-start = Не удалось запустить базу данных.
error-database-version = База данных создана другой версией PostgreSQL, чем та, которую приносит эта ATENA.
error-decision-country = Для этой страны у ATENA нет правового профиля.
error-decision-incomplete = Для решения нужны имя и причина.
error-decision-licence = Реестр не знает такой лицензии.
error-decision-official = Официальные тексты этой страны не свободны; выберите её открытую лицензию.
error-decision-terms = Реестр не знает таких условий.
error-editor-missing = Для решения нужно ваше имя – укажите его в настройках.
error-index = Ошибка поискового индекса.
error-index-foreign = Папка индекса содержит что-то другое, а не индекс ATENA.
error-input-too-long = Текст слишком длинный.
error-internal = Произошла непредвиденная ошибка.
error-io = Не удалось прочитать или записать файл или папку.
error-not-a-list = Это не список ATENA.
error-not-found = Не найдено.
error-not-ready = ATENA ещё запускается.
error-reason-missing = Укажите причину.
error-search = Поиск не удался.
error-search-empty = Напишите, что вы ищете, или выберите сужение.
error-search-filter = ATENA не знает такого сужения.
error-settings = Не удалось сохранить настройки.
error-shared-register-invalid = Общий реестр недействителен; ничего не изменилось.
error-shared-register-unreachable = Не удалось скачать общий реестр. Проверьте подключение к интернету.
error-url = Это недействительный веб-адрес.
error-private-damaged = Файл личной учётной записи повреждён.
error-private-exists = Личная учётная запись на этом компьютере уже существует.
error-private-locked = Личный раздел закрыт – сначала войдите.
error-private-login = Неверное имя или пароль.
error-private-name = Имя должно содержать от 1 до 40 символов.
error-private-none = Личная учётная запись ещё не создана.
error-private-recovery = Это не ключ восстановления этой учётной записи.
error-private-weak = Пароль должен содержать не менее 10 символов и отличаться от имени.
error-private-keychain = Диспетчер учётных данных Windows сейчас недоступен. Попробуйте ещё раз.
error-private-remember-none = Постоянный вход доступен только в Windows (Диспетчер учётных данных).
error-private-library-full = Личная библиотека заполнена – удалите часть сохранённых документов.

## Súkromné / Private

private-group = Личное
private-overview-title = Моя ATENA
private-overview-of = Личный раздел пользователя { $name } – только на этом компьютере.
private-note-title = Личное – только для вас
private-note-text = Этот раздел зашифрован вашим паролем и хранится только на этом компьютере. Официальные и свободно лицензированные документы не сохраняются повторно – сохраняется ссылка на общий раздел ATENA. Остальные документы сохраняются как копия для личного использования.
private-all = Все
private-empty = В этой категории пока нет словацких документов. Запустите сбор.
private-search-all = Искать в категории
private-create-first = Сначала создайте личную учётную запись в настройках.
private-lock = Закрыть личный раздел
private-unlock-title = Вход в личный раздел
private-unlock = Войти
private-name = Имя
private-password = Пароль
private-remember-title = Запомнить вход?
private-remember-text = Если выбрать «да», личный раздел останется открытым до закрытия ATENA. Если нет, он закроется, как только вы его покинете.
private-remember-yes = Да, до закрытия ATENA
private-remember-no = Нет, закрыть при выходе
private-remember-forever-title = Постоянный вход на этом компьютере
private-remember-forever-text = Тогда личный раздел будет открываться без пароля при каждом запуске ATENA. Ключ хранит Диспетчер учётных данных Windows, защищённый вашим входом в Windows; ничего не покидает этот компьютер. Отключить его можно в любой момент в настройках.
private-remember-risk = Я понимаю: любой, кто войдёт в мою учётную запись Windows, откроет мой личный раздел без пароля. Я включаю постоянный вход на свой страх и риск.
private-remember-forever = Входить постоянно
private-remember-on = Постоянный вход включён – личный раздел открывается на этом компьютере без пароля.
private-remember-off = Отключить постоянный вход
private-remember-enable = Включить постоянный вход
private-account-title = Личная учётная запись
private-account-intro = Учётная запись существует только на этом компьютере – ATENA ничего не отправляет на серверы. Пароль нигде не хранится; из него выводится ключ, шифрующий ваш личный раздел. Одна учётная запись на компьютер.
private-password-again = Пароль ещё раз
private-password-rules = Пароль должен содержать не менее 10 символов и отличаться от имени. При создании вы получите ключ восстановления – с ним можно задать новый пароль, если вы его забудете.
private-password-mismatch = Пароли не совпадают.
private-create = Создать личную учётную запись
private-account-of = Учётная запись { $name }:
private-state-open = открыта
private-state-locked = закрыта
private-change-password = Сменить пароль
private-forgot = Забыли пароль?
private-forgot-text = Введите ключ восстановления, полученный при создании учётной записи, и новый пароль. Ваши данные сохранятся.
private-password-old = Текущий пароль
private-password-new = Новый пароль
private-set-password = Задать новый пароль
private-password-changed = Пароль изменён.
private-recovery-title = Ключ восстановления
private-recovery-text = Это ваш ключ восстановления. Распечатайте его или сохраните в надёжном месте вне этого компьютера. Он показывается только сейчас – без него и без пароля личный раздел открыть нельзя.
private-recovery-copy = Копировать
private-recovery-copied = Скопировано
private-recovery-kept = Я сохранил ключ восстановления.
private-recovery-done = Продолжить
private-select = Выбрать: { $title }
private-save-all = Сохранить весь список ({ $count })
private-save-selected = Сохранить выбранные ({ $count })
private-select-all = Выбрать все несохранённые
private-view-saved = Сохранённые
private-view-documents = Документы
private-saved-counts = Всего { $all } · копии для себя { $copies } · ссылки { $references }
private-saved-title = Сохранённые документы
private-text-chars = символов текста: { $chars }
private-saved-empty = Сохранённых документов пока нет. В категориях нажмите у документа «Сохранить для себя».
private-text-gone = Текста этого документа в общем разделе больше нет. Откройте оригинал.
private-remove-confirm = Точно удалить
private-remove = Удалить
private-open-file-note = На время просмотра файл расшифровывается в папку ATENA и удаляется при закрытии личного раздела или ATENA.
private-open-file = Открыть файл ({ $size })
private-licence = Лицензия источника: { $licence }
private-saved-on = сохранено { $date }
private-save-all-confirm = Сохранить ({ $count })
private-save-all-pace = Копии загружаются по одной и в темпе, который разрешают серверы, – это может занять несколько минут. Тем временем можно продолжать работу.
private-save-all-saved = (Уже сохранено: { $saved }.)
private-save-all-text = Документов для сохранения: { $count }.
private-save-all-title = Сохранить весь список?
private-saving-counts = Сохранено: { $saved } · уже было сохранено: { $already } · не сохранено: { $refused } (причина указана у документа)
private-saving-stop = Остановить
private-saving-done = Готово: обработано { $done } из { $total }
private-saving = Сохраняется { $done } из { $total }…
private-reason-unreadable = Не сохранено: документ невозможно прочитать.
private-reason-dangerous = Не сохранено: файл содержит активное содержимое (например, скрипт) и не сохраняется из соображений безопасности.
private-reason-unavailable = Не сохранено: документ не удалось загрузить.
private-reason-busy = Не сохранено: сервер сейчас занят – попробуйте позже.
private-reason-robots = Не сохранено: файл robots.txt сайта этого не разрешает.
private-reason-noarchive = Не сохранено: издатель не разрешает хранить копии.
private-reason-not-allowed = Не сохранено: условия этого сайта не разрешают автоматический доступ.
private-reason-gone = Не сохранено: документа уже нет ни в общем разделе, ни в интернете.
private-save = Сохранить для себя
private-read = Читать
private-kind-copy-title = Копия для личного использования – зашифрована только для вас на этом компьютере.
private-kind-reference-title = Официальный или свободно лицензированный документ – его хранит общий раздел ATENA, личный раздел только ссылается на него.
private-kind-copy = Копия для себя
private-kind-reference = Ссылка
private-copy-right = Копия для личного использования: по § 42 Закона Словакии об авторском праве (№ 185/2015) физическое лицо может без согласия автора изготовить копию произведения для личных нужд, если цель ни прямо, ни косвенно не является коммерческой. ATENA загружает копию с сайта издателя только по вашей команде, соблюдает его правила (robots.txt, запрет копий) и хранит её зашифрованной только для вас. Не передавайте, не распространяйте и не используйте копии в других целях – авторские права остаются за авторами.

# ATENA's archive of original files
facet-archive = Архив ATENA
archive-facet-removed = Удалено из интернета
archive-gone = Издатель удалил документ из интернета ({ $date }).
archive-gone-text = ATENA хранит его в своём архиве, потому что этот документ может копировать и показывать любой ({ $licence }). Источник и лицензия по-прежнему указаны.
archive-text = Исходный файл хранится в архиве ATENA – он останется доступен, даже если издатель удалит его из интернета ({ $licence }).
archive-file = Исходный файл в архиве с { $date } · { $size }
archive-open = Открыть исходный файл из архива
archive-open-size = Открыть исходный файл из архива ({ $size })
archive-gone-hit = Удалено из интернета
archive-gone-hit-title = Издатель удалил документ из интернета; ATENA хранит его в своём архиве.
overview-archive = Архив: исходных файлов { $files } ({ $size }) · удалено из интернета: { $gone }
settings-keep-originals = Хранить исходные файлы официальных и свободно лицензированных документов
settings-keep-originals-explain = Документы PDF и Word/OpenDocument, которые может копировать любой, останутся доступны, даже если издатель удалит их из интернета. Они сохраняются только после проверки на опасное содержимое и только если их текст не содержит персональных данных. Они занимают место на диске.
