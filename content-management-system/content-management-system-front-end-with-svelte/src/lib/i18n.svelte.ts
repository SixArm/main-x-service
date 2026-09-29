// Lightweight, dependency-free i18n for the CMS authoring SPA (family
// pattern, copy-adapted from the CRM front-end): a per-locale strings
// map plus a reactive `$state` current locale (Svelte 5 runes), exposed
// through a `t(key)` accessor.
//
// `en-001` is the source of truth and **every locale must cover the same
// key set** — the parity test pins this. A missing key is not a
// cosmetic problem here: it silently falls back to English inside an
// otherwise-translated page, which reads as a bug in the content rather
// than in the app.
//
// Seven locales from the start rather than "English now, i18n
// later": the family learned that retrofitting means auditing every
// string in every view, and RTL in particular changes layout decisions
// that are cheap now and expensive after the fact.
//
// The chosen locale persists to localStorage and drives both the UI
// strings and `<html lang>` / `<html dir>`.
//
// This is the **interface** locale, which is not the same thing as the
// **content** locale a site declares. A Welsh-speaking editor may work
// on a French page; conflating the two would make the locale switcher
// silently change which content is being edited.

import { browser } from "$app/environment";

/**
 * Locales the UI is translated into, sorted alphabetically by code (the
 * LocalePicker shows them in this order). `-001` is the UN M.49 code for
 * "world": a language with no regional variant.
 */
export const LOCALES = [
  "ar-001",
  "cy-001",
  "en-001",
  "es-001",
  "fr-001",
  "hi-001",
  "zh-cn",
] as const;

/** A supported locale code (one of {@link LOCALES}). */
export type Locale = (typeof LOCALES)[number];

/** Fallback locale for an unknown key, locale, or missing translation. */
export const DEFAULT_LOCALE: Locale = "en-001";

/**
 * Human-readable name for the locale switcher, written in that locale.
 * A `-001` (world) locale is labelled by its language alone; a regional
 * locale reads "<language> - <region>" (never "<language> (<region>)").
 */
export const LOCALE_LABELS: Record<Locale, string> = {
  "ar-001": "العربية",
  "cy-001": "Cymraeg",
  "en-001": "English",
  "es-001": "Español",
  "fr-001": "Français",
  "hi-001": "हिन्दी",
  "zh-cn": "中文 - 中国",
};

/** Right-to-left locales; the layout mirrors `<html dir>`. */
export const RTL_LOCALES = ["ar-001"] as const satisfies readonly Locale[];

/** localStorage key under which the chosen UI locale persists. */
export const LOCALE_KEY = "mxi.cms.locale";

// Every translatable UI string, keyed by a stable dotted key.
const STRINGS = {
  "ar-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "لوحة المعلومات",
    "nav.entries": "المدخلات",
    "nav.assets": "الوسائط",
    "nav.workflow": "سير العمل",
    "nav.translations": "الترجمات",
    "nav.insights": "التحليلات",
    "nav.settings": "الإعدادات",
    "nav.signin": "تسجيل الدخول",
    "nav.signout": "تسجيل الخروج",
    "chrome.language": "اللغة",
    "nav.share": "مشاركة",
    "nav.text_size": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "chrome.theme": "السمة",
    "common.loading": "جارٍ التحميل…",
    "common.error": "فشل التحميل",
    "common.retry": "إعادة المحاولة",
    "common.status": "الحالة",
    "common.locale": "الإعداد المحلي",
    "common.title": "العنوان",
    "common.actions": "الإجراءات",
    "common.updated": "محدَّث",
    "common.noData": "لا توجد بيانات بعد",
    "site.choose": "اختر موقعًا",
    "entry.published": "منشور",
    "entry.draft": "مسودة",
    "entry.inReview": "قيد المراجعة",
    "entry.approved": "معتمد",
    "entry.archived": "مؤرشف",
    "entry.liveRevision": "النسخة المنشورة",
    "entry.draftAhead": "المسودة متقدمة على النسخة المنشورة",
    "insights.health": "سلامة المحتوى",
    "insights.throughput": "معدل الإنجاز",
    "insights.backlog": "المتراكم",
    "insights.asOf": "حتى تاريخ",
    "insights.findings": "النتائج",
    "insights.rule": "القاعدة",
    "insights.noFindings": "لا شيء للإبلاغ عنه",
    "preview.heading": "معاينة",
    "preview.notLive": "هذا ليس ما يراه القراء",
    "preview.localeServed": "الإعداد المحلي المقدَّم",
    "entry.key": "المفتاح",
    "entry.type": "النوع",
    "entry.locales": "اللغات",
    "entry.history": "سجل النسخ",
    "entry.blocks": "كتل المحتوى",
    "entry.addBlock": "إضافة كتلة",
    "entry.save": "حفظ النسخة",
    "entry.conflict": "حفظ شخص آخر قبلك",
    "entry.conflictHelp": "استند مسودتك إلى نسخة أقدم. قارن قبل الكتابة فوقها.",
    "entry.restore": "استعادة",
    "entry.restoreHelp":
      "الاستعادة تكتب نسخة جديدة؛ لا يُعاد كتابة السجل أبدًا.",
    "entry.diff": "مقارنة",
    "entry.identical": "لا توجد فروق",
    "workflow.action": "إجراء",
    "workflow.reason": "السبب",
    "workflow.reasonRequired": "يتطلب هذا الإجراء سببًا",
    "workflow.blockers": "لا يمكن النشر بعد",
    "workflow.remedy": "ما ينبغي فعله",
    "workflow.ready": "جاهز للنشر",
    "workflow.scheduled": "مجدول",
    "assets.altMissing": "لا يوجد نص بديل",
    "assets.altGate": "الصورة بدون نص بديل تمنع نشر الصفحة",
    "assets.orphans": "لا يشير إليه شيء",
    "assets.orphansNote": "يُبلَّغ عنه ولا يُحذف أبدًا",
    "assets.storage": "المساحة المستخدمة",
    "translations.queue": "الطلبات المفتوحة",
    "translations.stale": "متأخر عن المصدر",
    "translations.source": "لغة المصدر",
    "settings.templates": "القوالب",
    "settings.menus": "القوائم",
    "settings.redirects": "إعادة التوجيه",
    "settings.webhooks": "خطافات الويب",
    "settings.contentTypes": "أنواع المحتوى",
    "preview.open": "معاينة",
    "preview.serverSide": "يبقى رابط المعاينة على الخادم",
    "common.cancel": "إلغاء",
    "common.remove": "إزالة",
    "common.moveUp": "تحريك لأعلى",
    "common.moveDown": "تحريك لأسفل",
    "common.saved": "تم الحفظ",
    "common.author": "المؤلف",
    "common.size": "الحجم",
    "common.path": "المسار",
    "auth.signin": "تسجيل الدخول",
    "auth.signout": "تسجيل الخروج",
    "share.email": "إرسال الرابط بالبريد",
    "share.linkedin": "المشاركة على LinkedIn",
    "share.reddit": "المشاركة على Reddit",
    "share.bluesky": "المشاركة على Bluesky",
    "share.mastodon": "المشاركة على Mastodon",
    "splash.hero.secondary": "اكتشف ما بالداخل",
    "splash.benefits.title": "لماذا تختارها الفرق",
    "splash.features.title": "ما يمكنك فعله",
    "splash.trust.title": "مصمَّم للثقة",
    "splash.trust.1.title": "تسجيل دخول بلا كلمة مرور",
    "splash.trust.1.body":
      "رابط سحري يصل إلى بريدك الإلكتروني: لا كلمة مرور لتُسرَّب أو يُعاد استخدامها.",
    "splash.trust.2.title": "أذونات قائمة على السمات",
    "splash.trust.2.body":
      "قواعد دقيقة تحدد من يجوز له القراءة أو الكتابة أو الدمج أو الحذف.",
    "splash.trust.3.title": "سجل تدقيق يكشف العبث",
    "splash.trust.3.body": "سجل للإضافة فقط يوثّق كل تغيير ومن أجراه.",
    "splash.trust.4.title": "ضوابط الخصوصية",
    "splash.trust.4.body":
      "تُخفى التفاصيل الحساسة ما لم يكن من حقك الاطلاع عليها.",
    "splash.trust.5.title": "معايير مفتوحة",
    "splash.trust.5.body":
      "REST مع OpenAPI، وHL7 FHIR حيثما تحتاجه الأنظمة الصحية.",
    "splash.trust.6.title": "يتحدث لغتك",
    "splash.trust.6.body":
      "العربية والصينية والإنجليزية والفرنسية والهندية والإسبانية والويلزية.",
    "splash.cta.title": "هل أنت مستعد للبدء؟",
    "splash.cta.body":
      "سجّل الدخول برابط سحري يصلك على بريدك الإلكتروني. لا حاجة لكلمة مرور.",
    "splash.hero.title": "انشر بثقة، بكل اللغات",
    "splash.hero.subtitle":
      "أنشئ محتوى منظّمًا وراجعه وانشره، مع سجل النسخ والترجمة وفحوص سلامة المحتوى مدمجة.",
    "splash.benefits.1.title": "اعرف ما هو منشور",
    "splash.benefits.1.body":
      "يُظهر مؤشر واضح متى تقدّمت المسودة على ما يراه القرّاء حاليًا.",
    "splash.benefits.2.title": "لا تعديلات ضائعة",
    "splash.benefits.2.body":
      "إذا حفظ شخص آخر قبلك، يتم إبلاغك ويمكنك المقارنة قبل الكتابة فوق أي شيء.",
    "splash.benefits.3.title": "تراجع بلا خوف",
    "splash.benefits.3.body":
      "استعادة نسخة قديمة تكتب نسخة جديدة، فلا يُعاد كتابة السجل أبدًا.",
    "splash.benefits.4.title": "قواعد نشر واضحة",
    "splash.benefits.4.body":
      "عندما لا يمكن نشر مدخل بعد، يُخبرك النظام بالضبط بما يمنعه.",
    "splash.benefits.5.title": "ترجمات صادقة",
    "splash.benefits.5.body":
      "اعرف أي الترجمات تأخرت عن الأصل قبل أن يلاحظ القرّاء.",
    "splash.benefits.6.title": "اكتشاف المشكلات مبكرًا",
    "splash.benefits.6.body":
      "تحدد نتائج سلامة المحتوى القاعدة التي خُرقت، فيسهل التصرف حيال كل منها.",
    "splash.features.1.title": "محتوى قائم على الكتل",
    "splash.features.1.body":
      "ألّف المدخلات من كتل محتوى منظّمة بدلًا من HTML مخزَّن.",
    "splash.features.2.title": "سجل النسخ",
    "splash.features.2.body":
      "كل حفظ هو نسخة يمكنك مقارنتها بغيرها واستعادتها.",
    "splash.features.3.title": "سير العمل التحريري",
    "splash.features.3.body":
      "انقل المدخلات من المسودة إلى المراجعة والاعتماد والنشر، مع تسجيل سبب لكل خطوة.",
    "splash.features.4.title": "التوطين",
    "splash.features.4.body":
      "احتفظ بنسخة لكل لغة، مع سلاسل بديلة لما لم يُترجم بعد.",
    "splash.features.5.title": "مكتبة الوسائط",
    "splash.features.5.body":
      "أدِر الصور والملفات إلى جانب المدخلات التي تستخدمها.",
    "splash.features.6.title": "المعاينة والتحليلات",
    "splash.features.6.body":
      "عاين لغة قبل نشرها، وتابع سلامة المحتوى والأعمال المتراكمة على لوحة المعلومات.",
    "nav.toggle": "تبديل التنقل",
  },
  "cy-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "Dangosfwrdd",
    "nav.entries": "Cofnodion",
    "nav.assets": "Asedau",
    "nav.workflow": "Llif gwaith",
    "nav.translations": "Cyfieithiadau",
    "nav.insights": "Mewnwelediadau",
    "nav.settings": "Gosodiadau",
    "nav.signin": "Mewngofnodi",
    "nav.signout": "Allgofnodi",
    "chrome.language": "Iaith",
    "nav.share": "Rhannu",
    "nav.text_size": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "chrome.theme": "Thema",
    "common.loading": "Yn llwytho…",
    "common.error": "Methwyd â llwytho",
    "common.retry": "Ceisio eto",
    "common.status": "Statws",
    "common.locale": "Locale",
    "common.title": "Teitl",
    "common.actions": "Gweithredoedd",
    "common.updated": "Diweddarwyd",
    "common.noData": "Dim data eto",
    "site.choose": "Dewiswch safle",
    "entry.published": "Cyhoeddwyd",
    "entry.draft": "Drafft",
    "entry.inReview": "Dan adolygiad",
    "entry.approved": "Cymeradwywyd",
    "entry.archived": "Archifwyd",
    "entry.liveRevision": "Fersiwn fyw",
    "entry.draftAhead": "Mae'r drafft ar y blaen i'r hyn sy'n fyw",
    "insights.health": "Iechyd cynnwys",
    "insights.throughput": "Trwybwn",
    "insights.backlog": "Ôl-groniad",
    "insights.asOf": "Fel ar",
    "insights.findings": "Canfyddiadau",
    "insights.rule": "Rheol",
    "insights.noFindings": "Dim i'w adrodd",
    "preview.heading": "Rhagolwg",
    "preview.notLive": "Nid dyma mae darllenwyr yn ei weld",
    "preview.localeServed": "Locale a weinyddwyd",
    "entry.key": "Allwedd",
    "entry.type": "Math",
    "entry.locales": "Localau",
    "entry.history": "Hanes fersiynau",
    "entry.blocks": "Blociau cynnwys",
    "entry.addBlock": "Ychwanegu bloc",
    "entry.save": "Cadw fersiwn",
    "entry.conflict": "Cadwodd rhywun arall yn gyntaf",
    "entry.conflictHelp":
      "Roedd eich drafft yn seiliedig ar fersiwn hŷn. Cymharwch cyn trosysgrifo.",
    "entry.restore": "Adfer",
    "entry.restoreHelp":
      "Mae adfer yn ysgrifennu fersiwn newydd; nid yw hanes byth yn cael ei ailysgrifennu.",
    "entry.diff": "Cymharu",
    "entry.identical": "Dim gwahaniaethau",
    "workflow.action": "Gweithred",
    "workflow.reason": "Rheswm",
    "workflow.reasonRequired": "Mae angen rheswm ar y weithred hon",
    "workflow.blockers": "Methu cyhoeddi eto",
    "workflow.remedy": "Beth i'w wneud",
    "workflow.ready": "Yn barod i gyhoeddi",
    "workflow.scheduled": "Wedi'i drefnu",
    "assets.altMissing": "Dim testun amgen",
    "assets.altGate":
      "Mae delwedd heb destun amgen yn atal y dudalen rhag cyhoeddi",
    "assets.orphans": "Heb ei gyfeirio gan ddim",
    "assets.orphansNote": "Adroddir, ni ddilëir byth",
    "assets.storage": "Storfa a ddefnyddiwyd",
    "translations.queue": "Ceisiadau agored",
    "translations.stale": "Ar ôl y ffynhonnell",
    "translations.source": "Iaith ffynhonnell",
    "settings.templates": "Templedi",
    "settings.menus": "Dewislenni",
    "settings.redirects": "Ailgyfeiriadau",
    "settings.webhooks": "Bachau gwe",
    "settings.contentTypes": "Mathau o gynnwys",
    "preview.open": "Rhagolwg",
    "preview.serverSide": "Mae'r ddolen ragolwg yn aros ar y gweinydd",
    "common.cancel": "Canslo",
    "common.remove": "Tynnu",
    "common.moveUp": "Symud i fyny",
    "common.moveDown": "Symud i lawr",
    "common.saved": "Wedi'i gadw",
    "common.author": "Awdur",
    "common.size": "Maint",
    "common.path": "Llwybr",
    "auth.signin": "Mewngofnodi",
    "auth.signout": "Allgofnodi",
    "share.email": "E-bostio'r ddolen",
    "share.linkedin": "Rhannu ar LinkedIn",
    "share.reddit": "Rhannu ar Reddit",
    "share.bluesky": "Rhannu ar Bluesky",
    "share.mastodon": "Rhannu ar Mastodon",
    "splash.hero.secondary": "Gweld beth sydd y tu mewn",
    "splash.benefits.title": "Pam mae timau'n ei ddewis",
    "splash.features.title": "Beth allwch chi ei wneud",
    "splash.trust.title": "Wedi'i adeiladu ar gyfer ymddiriedaeth",
    "splash.trust.1.title": "Mewngofnodi heb gyfrinair",
    "splash.trust.1.body":
      "Dolen hud a anfonir i'ch e-bost: dim cyfrinair i'w ollwng na'i ailddefnyddio.",
    "splash.trust.2.title": "Caniatâd yn seiliedig ar briodoleddau",
    "splash.trust.2.body":
      "Mae rheolau manwl yn penderfynu pwy sy'n cael darllen, ysgrifennu, uno neu ddileu.",
    "splash.trust.3.title": "Llwybr archwilio sy'n dangos ymyrryd",
    "splash.trust.3.body":
      "Mae hanes atodi-yn-unig yn cofnodi pob newid a phwy a'i gwnaeth.",
    "splash.trust.4.title": "Rheolaethau preifatrwydd",
    "splash.trust.4.body":
      "Mae manylion sensitif wedi'u cuddio oni bai bod hawl gennych i'w gweld.",
    "splash.trust.5.title": "Safonau agored",
    "splash.trust.5.body":
      "REST gydag OpenAPI, a HL7 FHIR lle mae systemau iechyd ei angen.",
    "splash.trust.6.title": "Yn siarad eich iaith",
    "splash.trust.6.body":
      "Arabeg, Tsieinëeg, Saesneg, Ffrangeg, Hindi, Sbaeneg a Chymraeg.",
    "splash.cta.title": "Barod i ddechrau?",
    "splash.cta.body":
      "Mewngofnodwch gyda dolen hud a anfonir i'ch e-bost. Dim angen cyfrinair.",
    "splash.hero.title": "Cyhoeddwch yn hyderus, ym mhob iaith",
    "splash.hero.subtitle":
      "Crëwch gynnwys strwythuredig, adolygwch ef a'i gyhoeddi, gyda hanes fersiynau, lleoleiddio a gwiriadau iechyd cynnwys wedi'u cynnwys.",
    "splash.benefits.1.title": "Gwybod beth sy'n fyw",
    "splash.benefits.1.body":
      "Mae marc clir yn dangos pan fo drafft wedi mynd ymhellach na'r hyn y mae darllenwyr yn ei weld ar hyn o bryd.",
    "splash.benefits.2.title": "Dim golli golygiadau",
    "splash.benefits.2.body":
      "Os cadwodd rhywun arall yn gyntaf, cewch wybod a gallwch gymharu cyn trosysgrifo unrhyw beth.",
    "splash.benefits.3.title": "Dadwneud heb ofn",
    "splash.benefits.3.body":
      "Mae adfer fersiwn hŷn yn ysgrifennu un newydd, felly ni chaiff hanes ei ailysgrifennu byth.",
    "splash.benefits.4.title": "Rheolau cyhoeddi clir",
    "splash.benefits.4.body":
      "Pan na ellir cyhoeddi cofnod eto, dywedir wrthych yn union beth sy'n ei rwystro.",
    "splash.benefits.5.title": "Cyfieithiadau gonest",
    "splash.benefits.5.body":
      "Gwelwch pa gyfieithiadau sydd wedi syrthio y tu ôl i'r ffynhonnell, cyn i ddarllenwyr sylwi.",
    "splash.benefits.6.title": "Canfod problemau'n gynnar",
    "splash.benefits.6.body":
      "Mae canfyddiadau iechyd cynnwys yn enwi'r rheol a dorrwyd, felly mae'n hawdd gweithredu ar bob un.",
    "splash.features.1.title": "Cynnwys ar sail blociau",
    "splash.features.1.body":
      "Lluniwch gofnodion o flociau cynnwys strwythuredig yn hytrach na HTML wedi'i storio.",
    "splash.features.2.title": "Hanes fersiynau",
    "splash.features.2.body":
      "Mae pob cadw yn fersiwn y gallwch ei gymharu ag un arall a'i adfer.",
    "splash.features.3.title": "Llif gwaith golygyddol",
    "splash.features.3.body":
      "Symudwch gofnodion o ddrafft i adolygiad, cymeradwyaeth a chyhoeddi, gyda rheswm wedi'i gofnodi ar gyfer pob cam.",
    "splash.features.4.title": "Lleoleiddio",
    "splash.features.4.body":
      "Cadwch amrywiad ar gyfer pob iaith, gyda chadwyni wrth gefn ar gyfer unrhyw beth heb ei gyfieithu eto.",
    "splash.features.5.title": "Llyfrgell asedau",
    "splash.features.5.body":
      "Rheolwch ddelweddau a ffeiliau ochr yn ochr â'r cofnodion sy'n eu defnyddio.",
    "splash.features.6.title": "Rhagolwg a mewnwelediadau",
    "splash.features.6.body":
      "Rhagolwg iaith cyn iddi fynd yn fyw, a dilyn iechyd cynnwys a'r ôl-groniad ar y dangosfwrdd.",
    "nav.toggle": "Toglo llywio",
  },
  "en-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "Dashboard",
    "nav.entries": "Entries",
    "nav.assets": "Assets",
    "nav.workflow": "Workflow",
    "nav.translations": "Translations",
    "nav.insights": "Insights",
    "nav.settings": "Settings",
    "nav.signin": "Sign in",
    "nav.signout": "Sign out",
    "chrome.language": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    "chrome.theme": "Theme",
    "common.loading": "Loading…",
    "common.error": "Failed to load",
    "common.retry": "Retry",
    "common.status": "Status",
    "common.locale": "Locale",
    "common.title": "Title",
    "common.actions": "Actions",
    "common.updated": "Updated",
    "common.noData": "No data yet",
    "site.choose": "Choose a site",
    "entry.published": "Published",
    "entry.draft": "Draft",
    "entry.inReview": "In review",
    "entry.approved": "Approved",
    "entry.archived": "Archived",
    "entry.liveRevision": "Live revision",
    "entry.draftAhead": "Draft is ahead of what is live",
    "insights.health": "Content health",
    "insights.throughput": "Throughput",
    "insights.backlog": "Backlog",
    "insights.asOf": "As of",
    "insights.findings": "Findings",
    "insights.rule": "Rule",
    "insights.noFindings": "Nothing to report",
    "preview.heading": "Preview",
    "preview.notLive": "This is not what readers see",
    "preview.localeServed": "Locale served",
    "entry.key": "Key",
    "entry.type": "Type",
    "entry.locales": "Locales",
    "entry.history": "Revision history",
    "entry.blocks": "Content blocks",
    "entry.addBlock": "Add block",
    "entry.save": "Save revision",
    "entry.conflict": "Someone else saved first",
    "entry.conflictHelp":
      "Your draft was based on an older revision. Compare before overwriting.",
    "entry.restore": "Restore",
    "entry.restoreHelp":
      "Restoring writes a new revision; history is never rewritten.",
    "entry.diff": "Compare",
    "entry.identical": "No differences",
    "workflow.action": "Action",
    "workflow.reason": "Reason",
    "workflow.reasonRequired": "This action needs a reason",
    "workflow.blockers": "Cannot publish yet",
    "workflow.remedy": "What to do",
    "workflow.ready": "Ready to publish",
    "workflow.scheduled": "Scheduled",
    "assets.altMissing": "No alt text",
    "assets.altGate": "An image without alt text stops the page publishing",
    "assets.orphans": "Referenced by nothing",
    "assets.orphansNote": "Reported, never deleted",
    "assets.storage": "Storage used",
    "translations.queue": "Open requests",
    "translations.stale": "Behind the source",
    "translations.source": "Source locale",
    "settings.templates": "Templates",
    "settings.menus": "Menus",
    "settings.redirects": "Redirects",
    "settings.webhooks": "Webhooks",
    "settings.contentTypes": "Content types",
    "preview.open": "Preview",
    "preview.serverSide": "The preview link stays on the server",
    "common.cancel": "Cancel",
    "common.remove": "Remove",
    "common.moveUp": "Move up",
    "common.moveDown": "Move down",
    "common.saved": "Saved",
    "common.author": "Author",
    "common.size": "Size",
    "common.path": "Path",
    "auth.signin": "Sign in",
    "auth.signout": "Sign out",
    "share.email": "Email Link",
    "share.linkedin": "Share on LinkedIn",
    "share.reddit": "Share on Reddit",
    "share.bluesky": "Share on Bluesky",
    "share.mastodon": "Share on Mastodon",
    "splash.hero.secondary": "See what's inside",
    "splash.benefits.title": "Why teams choose it",
    "splash.features.title": "What you can do",
    "splash.trust.title": "Built for trust",
    "splash.trust.1.title": "Passwordless sign-in",
    "splash.trust.1.body":
      "A magic link sent to your email: no password to leak or reuse.",
    "splash.trust.2.title": "Attribute-based permissions",
    "splash.trust.2.body":
      "Fine-grained rules decide who may read, write, merge or delete.",
    "splash.trust.3.title": "Tamper-evident audit trail",
    "splash.trust.3.body":
      "An append-only history records every change and who made it.",
    "splash.trust.4.title": "Privacy controls",
    "splash.trust.4.body":
      "Sensitive details are masked unless you are entitled to see them.",
    "splash.trust.5.title": "Open standards",
    "splash.trust.5.body":
      "REST with OpenAPI, and HL7 FHIR where health systems need it.",
    "splash.trust.6.title": "Speaks your language",
    "splash.trust.6.body":
      "Arabic, Chinese, English, French, Hindi, Spanish and Welsh.",
    "splash.cta.title": "Ready to get started?",
    "splash.cta.body":
      "Sign in with a magic link sent to your email. No password needed.",
    "splash.hero.title": "Publish with confidence, in every language",
    "splash.hero.subtitle":
      "Author structured content, review it, and publish it, with revision history, localization, and content health checks built in.",
    "splash.benefits.1.title": "Know what is live",
    "splash.benefits.1.body":
      "A clear marker shows when a draft has moved ahead of what readers currently see.",
    "splash.benefits.2.title": "No lost edits",
    "splash.benefits.2.body":
      "If someone else saved first, you are told and can compare before overwriting anything.",
    "splash.benefits.3.title": "Undo without fear",
    "splash.benefits.3.body":
      "Restoring an old revision writes a new one, so history is never rewritten.",
    "splash.benefits.4.title": "Clear publishing rules",
    "splash.benefits.4.body":
      "When an entry cannot be published yet, you are told exactly what is blocking it.",
    "splash.benefits.5.title": "Honest translations",
    "splash.benefits.5.body":
      "See which translations have fallen behind the source, before readers notice.",
    "splash.benefits.6.title": "Problems found early",
    "splash.benefits.6.body":
      "Content health findings name the rule they broke, so each one is easy to act on.",
    "splash.features.1.title": "Block-based content",
    "splash.features.1.body":
      "Compose entries from structured content blocks rather than stored HTML.",
    "splash.features.2.title": "Revision history",
    "splash.features.2.body":
      "Every save is a revision you can compare with another and restore.",
    "splash.features.3.title": "Editorial workflow",
    "splash.features.3.body":
      "Move entries from draft to review, approval, and publication, with a reason recorded for each step.",
    "splash.features.4.title": "Localization",
    "splash.features.4.body":
      "Keep a variant per locale, with fallback chains for anything not yet translated.",
    "splash.features.5.title": "Asset library",
    "splash.features.5.body":
      "Manage images and files alongside the entries that use them.",
    "splash.features.6.title": "Preview and insights",
    "splash.features.6.body":
      "Preview a locale before it goes live, and track content health and backlog on the dashboard.",
    "nav.toggle": "Toggle navigation",
  },
  "es-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "Panel",
    "nav.entries": "Entradas",
    "nav.assets": "Recursos",
    "nav.workflow": "Flujo de trabajo",
    "nav.translations": "Traducciones",
    "nav.insights": "Análisis",
    "nav.settings": "Ajustes",
    "nav.signin": "Iniciar sesión",
    "nav.signout": "Cerrar sesión",
    "chrome.language": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "chrome.theme": "Tema",
    "common.loading": "Cargando…",
    "common.error": "Error al cargar",
    "common.retry": "Reintentar",
    "common.status": "Estado",
    "common.locale": "Configuración regional",
    "common.title": "Título",
    "common.actions": "Acciones",
    "common.updated": "Actualizado",
    "common.noData": "Aún no hay datos",
    "site.choose": "Elige un sitio",
    "entry.published": "Publicado",
    "entry.draft": "Borrador",
    "entry.inReview": "En revisión",
    "entry.approved": "Aprobado",
    "entry.archived": "Archivado",
    "entry.liveRevision": "Revisión publicada",
    "entry.draftAhead": "El borrador va por delante de lo publicado",
    "insights.health": "Salud del contenido",
    "insights.throughput": "Rendimiento",
    "insights.backlog": "Pendientes",
    "insights.asOf": "A fecha de",
    "insights.findings": "Hallazgos",
    "insights.rule": "Regla",
    "insights.noFindings": "Nada que informar",
    "preview.heading": "Vista previa",
    "preview.notLive": "Esto no es lo que ven los lectores",
    "preview.localeServed": "Idioma servido",
    "entry.key": "Clave",
    "entry.type": "Tipo",
    "entry.locales": "Idiomas",
    "entry.history": "Historial de revisiones",
    "entry.blocks": "Bloques de contenido",
    "entry.addBlock": "Añadir bloque",
    "entry.save": "Guardar revisión",
    "entry.conflict": "Otra persona guardó antes",
    "entry.conflictHelp":
      "Tu borrador se basaba en una revisión anterior. Compara antes de sobrescribir.",
    "entry.restore": "Restaurar",
    "entry.restoreHelp":
      "Restaurar crea una nueva revisión; el historial nunca se reescribe.",
    "entry.diff": "Comparar",
    "entry.identical": "Sin diferencias",
    "workflow.action": "Acción",
    "workflow.reason": "Motivo",
    "workflow.reasonRequired": "Esta acción necesita un motivo",
    "workflow.blockers": "Aún no se puede publicar",
    "workflow.remedy": "Qué hacer",
    "workflow.ready": "Listo para publicar",
    "workflow.scheduled": "Programado",
    "assets.altMissing": "Sin texto alternativo",
    "assets.altGate":
      "Una imagen sin texto alternativo impide publicar la página",
    "assets.orphans": "Sin referencias",
    "assets.orphansNote": "Se informa, nunca se elimina",
    "assets.storage": "Almacenamiento usado",
    "translations.queue": "Solicitudes abiertas",
    "translations.stale": "Por detrás del original",
    "translations.source": "Idioma de origen",
    "settings.templates": "Plantillas",
    "settings.menus": "Menús",
    "settings.redirects": "Redirecciones",
    "settings.webhooks": "Webhooks",
    "settings.contentTypes": "Tipos de contenido",
    "preview.open": "Vista previa",
    "preview.serverSide": "El enlace de vista previa se queda en el servidor",
    "common.cancel": "Cancelar",
    "common.remove": "Quitar",
    "common.moveUp": "Subir",
    "common.moveDown": "Bajar",
    "common.saved": "Guardado",
    "common.author": "Autor",
    "common.size": "Tamaño",
    "common.path": "Ruta",
    "auth.signin": "Iniciar sesión",
    "auth.signout": "Cerrar sesión",
    "share.email": "Enviar enlace por correo",
    "share.linkedin": "Compartir en LinkedIn",
    "share.reddit": "Compartir en Reddit",
    "share.bluesky": "Compartir en Bluesky",
    "share.mastodon": "Compartir en Mastodon",
    "splash.hero.secondary": "Descubre qué incluye",
    "splash.benefits.title": "Por qué la eligen los equipos",
    "splash.features.title": "Qué puedes hacer",
    "splash.trust.title": "Diseñado para la confianza",
    "splash.trust.1.title": "Acceso sin contraseña",
    "splash.trust.1.body":
      "Un enlace mágico enviado a tu correo: sin contraseñas que filtrar ni reutilizar.",
    "splash.trust.2.title": "Permisos basados en atributos",
    "splash.trust.2.body":
      "Reglas detalladas deciden quién puede leer, escribir, fusionar o eliminar.",
    "splash.trust.3.title": "Auditoría a prueba de manipulación",
    "splash.trust.3.body":
      "Un historial de solo anexado registra cada cambio y quién lo hizo.",
    "splash.trust.4.title": "Controles de privacidad",
    "splash.trust.4.body":
      "Los datos sensibles se ocultan salvo que tengas derecho a verlos.",
    "splash.trust.5.title": "Estándares abiertos",
    "splash.trust.5.body":
      "REST con OpenAPI y HL7 FHIR donde lo necesitan los sistemas de salud.",
    "splash.trust.6.title": "Habla tu idioma",
    "splash.trust.6.body":
      "Árabe, chino, español, francés, galés, hindi e inglés.",
    "splash.cta.title": "¿Listo para empezar?",
    "splash.cta.body":
      "Inicia sesión con un enlace mágico enviado a tu correo. No necesitas contraseña.",
    "splash.hero.title": "Publica con confianza, en todos los idiomas",
    "splash.hero.subtitle":
      "Crea contenido estructurado, revísalo y publícalo, con historial de revisiones, localización y comprobaciones de salud del contenido integrados.",
    "splash.benefits.1.title": "Sabe qué está publicado",
    "splash.benefits.1.body":
      "Un indicador claro muestra cuándo un borrador va por delante de lo que ven hoy los lectores.",
    "splash.benefits.2.title": "Sin ediciones perdidas",
    "splash.benefits.2.body":
      "Si otra persona guardó antes, se te avisa y puedes comparar antes de sobrescribir nada.",
    "splash.benefits.3.title": "Deshaz sin miedo",
    "splash.benefits.3.body":
      "Restaurar una revisión antigua crea una nueva, así que el historial nunca se reescribe.",
    "splash.benefits.4.title": "Reglas de publicación claras",
    "splash.benefits.4.body":
      "Cuando una entrada aún no se puede publicar, se te dice exactamente qué lo impide.",
    "splash.benefits.5.title": "Traducciones honestas",
    "splash.benefits.5.body":
      "Mira qué traducciones se han quedado atrás respecto al original, antes de que los lectores lo noten.",
    "splash.benefits.6.title": "Problemas detectados a tiempo",
    "splash.benefits.6.body":
      "Los hallazgos de salud del contenido indican la regla incumplida, así que es fácil actuar sobre cada uno.",
    "splash.features.1.title": "Contenido por bloques",
    "splash.features.1.body":
      "Compón las entradas con bloques de contenido estructurados en lugar de HTML almacenado.",
    "splash.features.2.title": "Historial de revisiones",
    "splash.features.2.body":
      "Cada guardado es una revisión que puedes comparar con otra y restaurar.",
    "splash.features.3.title": "Flujo editorial",
    "splash.features.3.body":
      "Lleva las entradas de borrador a revisión, aprobación y publicación, con el motivo de cada paso registrado.",
    "splash.features.4.title": "Localización",
    "splash.features.4.body":
      "Mantén una variante por idioma, con cadenas de respaldo para lo que aún no está traducido.",
    "splash.features.5.title": "Biblioteca de recursos",
    "splash.features.5.body":
      "Gestiona imágenes y archivos junto a las entradas que los usan.",
    "splash.features.6.title": "Vista previa y análisis",
    "splash.features.6.body":
      "Previsualiza un idioma antes de publicarlo y sigue la salud del contenido y las tareas pendientes en el panel.",
    "nav.toggle": "Alternar navegación",
  },
  "fr-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "Tableau de bord",
    "nav.entries": "Entrées",
    "nav.assets": "Médias",
    "nav.workflow": "Flux de travail",
    "nav.translations": "Traductions",
    "nav.insights": "Analyses",
    "nav.settings": "Paramètres",
    "nav.signin": "Se connecter",
    "nav.signout": "Se déconnecter",
    "chrome.language": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "chrome.theme": "Thème",
    "common.loading": "Chargement…",
    "common.error": "Échec du chargement",
    "common.retry": "Réessayer",
    "common.status": "Statut",
    "common.locale": "Locale",
    "common.title": "Titre",
    "common.actions": "Actions",
    "common.updated": "Mis à jour",
    "common.noData": "Pas encore de données",
    "site.choose": "Choisir un site",
    "entry.published": "Publié",
    "entry.draft": "Brouillon",
    "entry.inReview": "En relecture",
    "entry.approved": "Approuvé",
    "entry.archived": "Archivé",
    "entry.liveRevision": "Révision en ligne",
    "entry.draftAhead": "Le brouillon est en avance sur la version en ligne",
    "insights.health": "Santé du contenu",
    "insights.throughput": "Débit éditorial",
    "insights.backlog": "En attente",
    "insights.asOf": "À la date du",
    "insights.findings": "Constats",
    "insights.rule": "Règle",
    "insights.noFindings": "Rien à signaler",
    "preview.heading": "Aperçu",
    "preview.notLive": "Ce n'est pas ce que voient les lecteurs",
    "preview.localeServed": "Locale servie",
    "entry.key": "Clé",
    "entry.type": "Type",
    "entry.locales": "Locales",
    "entry.history": "Historique des révisions",
    "entry.blocks": "Blocs de contenu",
    "entry.addBlock": "Ajouter un bloc",
    "entry.save": "Enregistrer la révision",
    "entry.conflict": "Quelqu'un a enregistré avant vous",
    "entry.conflictHelp":
      "Votre brouillon reposait sur une révision antérieure. Comparez avant d'écraser.",
    "entry.restore": "Restaurer",
    "entry.restoreHelp":
      "Restaurer écrit une nouvelle révision ; l'historique n'est jamais réécrit.",
    "entry.diff": "Comparer",
    "entry.identical": "Aucune différence",
    "workflow.action": "Action",
    "workflow.reason": "Motif",
    "workflow.reasonRequired": "Cette action exige un motif",
    "workflow.blockers": "Publication impossible pour l'instant",
    "workflow.remedy": "Que faire",
    "workflow.ready": "Prêt à publier",
    "workflow.scheduled": "Planifié",
    "assets.altMissing": "Pas de texte alternatif",
    "assets.altGate":
      "Une image sans texte alternatif empêche la publication de la page",
    "assets.orphans": "Référencé par rien",
    "assets.orphansNote": "Signalé, jamais supprimé",
    "assets.storage": "Stockage utilisé",
    "translations.queue": "Demandes en cours",
    "translations.stale": "En retard sur la source",
    "translations.source": "Locale source",
    "settings.templates": "Modèles",
    "settings.menus": "Menus",
    "settings.redirects": "Redirections",
    "settings.webhooks": "Webhooks",
    "settings.contentTypes": "Types de contenu",
    "preview.open": "Aperçu",
    "preview.serverSide": "Le lien d'aperçu reste sur le serveur",
    "common.cancel": "Annuler",
    "common.remove": "Retirer",
    "common.moveUp": "Monter",
    "common.moveDown": "Descendre",
    "common.saved": "Enregistré",
    "common.author": "Auteur",
    "common.size": "Taille",
    "common.path": "Chemin",
    "auth.signin": "Se connecter",
    "auth.signout": "Se déconnecter",
    "share.email": "Envoyer le lien par e-mail",
    "share.linkedin": "Partager sur LinkedIn",
    "share.reddit": "Partager sur Reddit",
    "share.bluesky": "Partager sur Bluesky",
    "share.mastodon": "Partager sur Mastodon",
    "splash.hero.secondary": "Découvrir le contenu",
    "splash.benefits.title": "Pourquoi les équipes la choisissent",
    "splash.features.title": "Ce que vous pouvez faire",
    "splash.trust.title": "Conçu pour la confiance",
    "splash.trust.1.title": "Connexion sans mot de passe",
    "splash.trust.1.body":
      "Un lien magique envoyé par e-mail : aucun mot de passe à divulguer ni à réutiliser.",
    "splash.trust.2.title": "Autorisations par attributs",
    "splash.trust.2.body":
      "Des règles fines déterminent qui peut lire, écrire, fusionner ou supprimer.",
    "splash.trust.3.title": "Piste d'audit inviolable",
    "splash.trust.3.body":
      "Un historique en ajout seul consigne chaque modification et son auteur.",
    "splash.trust.4.title": "Contrôles de confidentialité",
    "splash.trust.4.body":
      "Les données sensibles sont masquées, sauf droit d'accès.",
    "splash.trust.5.title": "Standards ouverts",
    "splash.trust.5.body":
      "REST avec OpenAPI, et HL7 FHIR là où les systèmes de santé en ont besoin.",
    "splash.trust.6.title": "Parle votre langue",
    "splash.trust.6.body":
      "Anglais, arabe, chinois, espagnol, français, gallois et hindi.",
    "splash.cta.title": "Prêt à commencer ?",
    "splash.cta.body":
      "Connectez-vous avec un lien magique envoyé par e-mail. Aucun mot de passe requis.",
    "splash.hero.title": "Publiez en toute confiance, dans toutes les langues",
    "splash.hero.subtitle":
      "Rédigez du contenu structuré, relisez-le et publiez-le, avec historique des révisions, localisation et contrôles de santé du contenu intégrés.",
    "splash.benefits.1.title": "Sachez ce qui est en ligne",
    "splash.benefits.1.body":
      "Un repère clair indique quand un brouillon a dépassé ce que les lecteurs voient actuellement.",
    "splash.benefits.2.title": "Aucune modification perdue",
    "splash.benefits.2.body":
      "Si quelqu'un a enregistré avant vous, vous en êtes informé et pouvez comparer avant d'écraser quoi que ce soit.",
    "splash.benefits.3.title": "Annulez sans crainte",
    "splash.benefits.3.body":
      "Restaurer une ancienne révision en crée une nouvelle : l'historique n'est jamais réécrit.",
    "splash.benefits.4.title": "Règles de publication claires",
    "splash.benefits.4.body":
      "Lorsqu'une entrée ne peut pas encore être publiée, on vous dit exactement ce qui la bloque.",
    "splash.benefits.5.title": "Des traductions honnêtes",
    "splash.benefits.5.body":
      "Voyez quelles traductions ont pris du retard sur la source, avant que les lecteurs ne le remarquent.",
    "splash.benefits.6.title": "Problèmes détectés tôt",
    "splash.benefits.6.body":
      "Les constats de santé du contenu nomment la règle enfreinte, ce qui facilite chaque correction.",
    "splash.features.1.title": "Contenu par blocs",
    "splash.features.1.body":
      "Composez les entrées avec des blocs de contenu structurés plutôt que du HTML stocké.",
    "splash.features.2.title": "Historique des révisions",
    "splash.features.2.body":
      "Chaque enregistrement est une révision que vous pouvez comparer à une autre et restaurer.",
    "splash.features.3.title": "Flux éditorial",
    "splash.features.3.body":
      "Faites passer les entrées du brouillon à la relecture, l'approbation et la publication, avec le motif de chaque étape consigné.",
    "splash.features.4.title": "Localisation",
    "splash.features.4.body":
      "Gardez une variante par langue, avec des chaînes de repli pour tout ce qui n'est pas encore traduit.",
    "splash.features.5.title": "Médiathèque",
    "splash.features.5.body":
      "Gérez les images et les fichiers avec les entrées qui les utilisent.",
    "splash.features.6.title": "Aperçu et analyses",
    "splash.features.6.body":
      "Prévisualisez une langue avant sa mise en ligne et suivez la santé du contenu et le travail en attente sur le tableau de bord.",
    "nav.toggle": "Basculer la navigation",
  },
  "hi-001": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "डैशबोर्ड",
    "nav.entries": "प्रविष्टियाँ",
    "nav.assets": "संपत्तियाँ",
    "nav.workflow": "कार्यप्रवाह",
    "nav.translations": "अनुवाद",
    "nav.insights": "अंतर्दृष्टि",
    "nav.settings": "सेटिंग्स",
    "nav.signin": "साइन इन",
    "nav.signout": "साइन आउट",
    "chrome.language": "भाषा",
    "nav.share": "साझा करें",
    "nav.text_size": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "chrome.theme": "थीम",
    "common.loading": "लोड हो रहा है…",
    "common.error": "लोड करने में विफल",
    "common.retry": "पुनः प्रयास करें",
    "common.status": "स्थिति",
    "common.locale": "लोकेल",
    "common.title": "शीर्षक",
    "common.actions": "क्रियाएँ",
    "common.updated": "अद्यतन",
    "common.noData": "अभी कोई डेटा नहीं",
    "site.choose": "साइट चुनें",
    "entry.published": "प्रकाशित",
    "entry.draft": "मसौदा",
    "entry.inReview": "समीक्षाधीन",
    "entry.approved": "स्वीकृत",
    "entry.archived": "संग्रहित",
    "entry.liveRevision": "प्रकाशित संस्करण",
    "entry.draftAhead": "मसौदा प्रकाशित संस्करण से आगे है",
    "insights.health": "सामग्री स्वास्थ्य",
    "insights.throughput": "निष्पादन",
    "insights.backlog": "लंबित कार्य",
    "insights.asOf": "इस तिथि तक",
    "insights.findings": "निष्कर्ष",
    "insights.rule": "नियम",
    "insights.noFindings": "बताने योग्य कुछ नहीं",
    "preview.heading": "पूर्वावलोकन",
    "preview.notLive": "पाठक यह नहीं देखते",
    "preview.localeServed": "परोसा गया लोकेल",
    "entry.key": "कुंजी",
    "entry.type": "प्रकार",
    "entry.locales": "लोकेल",
    "entry.history": "संस्करण इतिहास",
    "entry.blocks": "सामग्री ब्लॉक",
    "entry.addBlock": "ब्लॉक जोड़ें",
    "entry.save": "संस्करण सहेजें",
    "entry.conflict": "किसी और ने पहले सहेजा",
    "entry.conflictHelp":
      "आपका मसौदा पुराने संस्करण पर आधारित था। अधिलेखित करने से पहले तुलना करें।",
    "entry.restore": "पुनर्स्थापित करें",
    "entry.restoreHelp":
      "पुनर्स्थापना नया संस्करण लिखती है; इतिहास कभी नहीं बदला जाता।",
    "entry.diff": "तुलना करें",
    "entry.identical": "कोई अंतर नहीं",
    "workflow.action": "कार्रवाई",
    "workflow.reason": "कारण",
    "workflow.reasonRequired": "इस कार्रवाई के लिए कारण चाहिए",
    "workflow.blockers": "अभी प्रकाशित नहीं कर सकते",
    "workflow.remedy": "क्या करें",
    "workflow.ready": "प्रकाशन के लिए तैयार",
    "workflow.scheduled": "निर्धारित",
    "assets.altMissing": "कोई वैकल्पिक पाठ नहीं",
    "assets.altGate": "वैकल्पिक पाठ के बिना छवि पृष्ठ का प्रकाशन रोक देती है",
    "assets.orphans": "किसी ने संदर्भित नहीं किया",
    "assets.orphansNote": "बताया जाता है, कभी हटाया नहीं जाता",
    "assets.storage": "उपयोग किया गया संग्रहण",
    "translations.queue": "खुले अनुरोध",
    "translations.stale": "स्रोत से पीछे",
    "translations.source": "स्रोत लोकेल",
    "settings.templates": "टेम्पलेट",
    "settings.menus": "मेन्यू",
    "settings.redirects": "पुनर्निर्देश",
    "settings.webhooks": "वेबहुक",
    "settings.contentTypes": "सामग्री प्रकार",
    "preview.open": "पूर्वावलोकन",
    "preview.serverSide": "पूर्वावलोकन लिंक सर्वर पर ही रहता है",
    "common.cancel": "रद्द करें",
    "common.remove": "हटाएँ",
    "common.moveUp": "ऊपर ले जाएँ",
    "common.moveDown": "नीचे ले जाएँ",
    "common.saved": "सहेजा गया",
    "common.author": "लेखक",
    "common.size": "आकार",
    "common.path": "पथ",
    "auth.signin": "साइन इन करें",
    "auth.signout": "साइन आउट करें",
    "share.email": "लिंक ईमेल करें",
    "share.linkedin": "LinkedIn पर साझा करें",
    "share.reddit": "Reddit पर साझा करें",
    "share.bluesky": "Bluesky पर साझा करें",
    "share.mastodon": "Mastodon पर साझा करें",
    "splash.hero.secondary": "अंदर क्या है, देखें",
    "splash.benefits.title": "टीमें इसे क्यों चुनती हैं",
    "splash.features.title": "आप क्या कर सकते हैं",
    "splash.trust.title": "भरोसे के लिए बनाया गया",
    "splash.trust.1.title": "बिना पासवर्ड साइन-इन",
    "splash.trust.1.body":
      "आपके ईमेल पर भेजा गया मैजिक लिंक: न लीक होने वाला, न दोबारा इस्तेमाल होने वाला पासवर्ड।",
    "splash.trust.2.title": "विशेषता-आधारित अनुमतियाँ",
    "splash.trust.2.body":
      "सूक्ष्म नियम तय करते हैं कि कौन पढ़, लिख, मर्ज या हटा सकता है।",
    "splash.trust.3.title": "छेड़छाड़-प्रमाण ऑडिट ट्रेल",
    "splash.trust.3.body":
      "केवल-जोड़ने वाला इतिहास हर बदलाव और उसे करने वाले को दर्ज करता है।",
    "splash.trust.4.title": "गोपनीयता नियंत्रण",
    "splash.trust.4.body":
      "संवेदनशील विवरण तब तक छिपे रहते हैं जब तक आपको उन्हें देखने का अधिकार न हो।",
    "splash.trust.5.title": "खुले मानक",
    "splash.trust.5.body":
      "OpenAPI के साथ REST, और जहाँ स्वास्थ्य प्रणालियों को ज़रूरत हो वहाँ HL7 FHIR।",
    "splash.trust.6.title": "आपकी भाषा में",
    "splash.trust.6.body":
      "अरबी, चीनी, अंग्रेज़ी, फ़्रेंच, हिन्दी, स्पेनिश और वेल्श।",
    "splash.cta.title": "शुरू करने के लिए तैयार हैं?",
    "splash.cta.body":
      "अपने ईमेल पर भेजे गए मैजिक लिंक से साइन इन करें। पासवर्ड की ज़रूरत नहीं।",
    "splash.hero.title": "हर भाषा में पूरे भरोसे के साथ प्रकाशित करें",
    "splash.hero.subtitle":
      "संरचित सामग्री तैयार करें, उसकी समीक्षा करें और उसे प्रकाशित करें, संस्करण इतिहास, स्थानीयकरण और सामग्री स्वास्थ्य जाँच के साथ।",
    "splash.benefits.1.title": "जानें क्या लाइव है",
    "splash.benefits.1.body":
      "एक स्पष्ट संकेत दिखाता है कि मसौदा पाठकों को अभी दिख रही सामग्री से आगे बढ़ चुका है।",
    "splash.benefits.2.title": "कोई संपादन नहीं खोता",
    "splash.benefits.2.body":
      "यदि किसी और ने पहले सहेजा है, तो आपको बताया जाता है और कुछ भी अधिलेखित करने से पहले आप तुलना कर सकते हैं।",
    "splash.benefits.3.title": "बेफिक्र होकर पूर्ववत करें",
    "splash.benefits.3.body":
      "पुरानी संशोधित प्रति को पुनर्स्थापित करने पर नई प्रति बनती है, इसलिए इतिहास कभी दोबारा नहीं लिखा जाता।",
    "splash.benefits.4.title": "प्रकाशन के स्पष्ट नियम",
    "splash.benefits.4.body":
      "जब कोई प्रविष्टि अभी प्रकाशित नहीं हो सकती, तो आपको ठीक-ठीक बताया जाता है कि क्या रोक रहा है।",
    "splash.benefits.5.title": "ईमानदार अनुवाद",
    "splash.benefits.5.body":
      "देखें कि कौन-से अनुवाद मूल से पीछे रह गए हैं, पाठकों के ध्यान देने से पहले।",
    "splash.benefits.6.title": "समस्याएँ जल्दी पकड़ में आती हैं",
    "splash.benefits.6.body":
      "सामग्री स्वास्थ्य के निष्कर्ष बताते हैं कि कौन-सा नियम टूटा, इसलिए हर एक पर कार्रवाई आसान होती है।",
    "splash.features.1.title": "ब्लॉक-आधारित सामग्री",
    "splash.features.1.body":
      "संग्रहीत HTML के बजाय संरचित सामग्री ब्लॉकों से प्रविष्टियाँ बनाएँ।",
    "splash.features.2.title": "संस्करण इतिहास",
    "splash.features.2.body":
      "हर बार सहेजना एक संस्करण है जिसकी आप दूसरे से तुलना कर सकते हैं और उसे पुनर्स्थापित कर सकते हैं।",
    "splash.features.3.title": "संपादकीय कार्यप्रवाह",
    "splash.features.3.body":
      "प्रविष्टियों को मसौदे से समीक्षा, स्वीकृति और प्रकाशन तक ले जाएँ, हर चरण का कारण दर्ज करते हुए।",
    "splash.features.4.title": "स्थानीयकरण",
    "splash.features.4.body":
      "हर भाषा के लिए एक रूपांतर रखें, और जो अभी अनूदित नहीं है उसके लिए वैकल्पिक क्रम रखें।",
    "splash.features.5.title": "संपत्ति पुस्तकालय",
    "splash.features.5.body":
      "छवियों और फ़ाइलों को उन्हें उपयोग करने वाली प्रविष्टियों के साथ प्रबंधित करें।",
    "splash.features.6.title": "पूर्वावलोकन और अंतर्दृष्टि",
    "splash.features.6.body":
      "किसी भाषा को लाइव होने से पहले देखें, और डैशबोर्ड पर सामग्री स्वास्थ्य और लंबित कार्य ट्रैक करें।",
    "nav.toggle": "नेविगेशन टॉगल करें",
  },
  "zh-cn": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "仪表板",
    "nav.entries": "条目",
    "nav.assets": "素材",
    "nav.workflow": "工作流",
    "nav.translations": "翻译",
    "nav.insights": "洞察",
    "nav.settings": "设置",
    "nav.signin": "登录",
    "nav.signout": "退出",
    "chrome.language": "语言",
    "nav.share": "分享",
    "nav.text_size": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "chrome.theme": "主题",
    "common.loading": "加载中…",
    "common.error": "加载失败",
    "common.retry": "重试",
    "common.status": "状态",
    "common.locale": "区域设置",
    "common.title": "标题",
    "common.actions": "操作",
    "common.updated": "已更新",
    "common.noData": "暂无数据",
    "site.choose": "选择站点",
    "entry.published": "已发布",
    "entry.draft": "草稿",
    "entry.inReview": "审核中",
    "entry.approved": "已批准",
    "entry.archived": "已归档",
    "entry.liveRevision": "线上版本",
    "entry.draftAhead": "草稿已领先于线上版本",
    "insights.health": "内容健康度",
    "insights.throughput": "编辑吞吐量",
    "insights.backlog": "待办",
    "insights.asOf": "截至",
    "insights.findings": "发现项",
    "insights.rule": "规则",
    "insights.noFindings": "没有需要报告的内容",
    "preview.heading": "预览",
    "preview.notLive": "读者看到的不是这个",
    "preview.localeServed": "实际提供的区域设置",
    "entry.key": "键",
    "entry.type": "类型",
    "entry.locales": "语言版本",
    "entry.history": "版本历史",
    "entry.blocks": "内容块",
    "entry.addBlock": "添加块",
    "entry.save": "保存版本",
    "entry.conflict": "有人先保存了",
    "entry.conflictHelp": "您的草稿基于较早的版本。覆盖前请先比较。",
    "entry.restore": "恢复",
    "entry.restoreHelp": "恢复会写入一个新版本；历史绝不会被改写。",
    "entry.diff": "比较",
    "entry.identical": "没有差异",
    "workflow.action": "操作",
    "workflow.reason": "原因",
    "workflow.reasonRequired": "此操作需要填写原因",
    "workflow.blockers": "尚不能发布",
    "workflow.remedy": "该怎么做",
    "workflow.ready": "可以发布",
    "workflow.scheduled": "已排程",
    "assets.altMissing": "缺少替代文本",
    "assets.altGate": "缺少替代文本的图片会阻止页面发布",
    "assets.orphans": "无任何引用",
    "assets.orphansNote": "仅报告，绝不删除",
    "assets.storage": "已用存储",
    "translations.queue": "待处理请求",
    "translations.stale": "落后于源内容",
    "translations.source": "源语言",
    "settings.templates": "模板",
    "settings.menus": "菜单",
    "settings.redirects": "重定向",
    "settings.webhooks": "Webhook",
    "settings.contentTypes": "内容类型",
    "preview.open": "预览",
    "preview.serverSide": "预览链接仅保留在服务器上",
    "common.cancel": "取消",
    "common.remove": "移除",
    "common.moveUp": "上移",
    "common.moveDown": "下移",
    "common.saved": "已保存",
    "common.author": "作者",
    "common.size": "大小",
    "common.path": "路径",
    "auth.signin": "登录",
    "auth.signout": "退出登录",
    "share.email": "通过邮件发送链接",
    "share.linkedin": "分享到 LinkedIn",
    "share.reddit": "分享到 Reddit",
    "share.bluesky": "分享到 Bluesky",
    "share.mastodon": "分享到 Mastodon",
    "splash.hero.secondary": "了解详情",
    "splash.benefits.title": "团队为何选择它",
    "splash.features.title": "你可以做什么",
    "splash.trust.title": "以信任为本",
    "splash.trust.1.title": "免密码登录",
    "splash.trust.1.body": "魔法链接发送到你的邮箱：没有密码可泄露或重复使用。",
    "splash.trust.2.title": "基于属性的权限",
    "splash.trust.2.body": "细粒度规则决定谁可以读取、写入、合并或删除。",
    "splash.trust.3.title": "防篡改审计记录",
    "splash.trust.3.body": "仅追加的历史记录会记下每一次更改及操作者。",
    "splash.trust.4.title": "隐私控制",
    "splash.trust.4.body": "除非你有权查看，否则敏感信息会被屏蔽。",
    "splash.trust.5.title": "开放标准",
    "splash.trust.5.body":
      "REST 搭配 OpenAPI，并在医疗系统需要时支持 HL7 FHIR。",
    "splash.trust.6.title": "支持你的语言",
    "splash.trust.6.body":
      "阿拉伯语、中文、英语、法语、印地语、西班牙语和威尔士语。",
    "splash.cta.title": "准备好开始了吗？",
    "splash.cta.body": "通过发送到邮箱的魔法链接登录，无需密码。",
    "splash.hero.title": "放心发布，覆盖每一种语言",
    "splash.hero.subtitle":
      "撰写结构化内容，审核并发布，内置版本历史、本地化和内容健康检查。",
    "splash.benefits.1.title": "清楚哪些已上线",
    "splash.benefits.1.body":
      "当草稿领先于读者当前看到的内容时，会有清晰的标记提示。",
    "splash.benefits.2.title": "编辑不再丢失",
    "splash.benefits.2.body":
      "如果有人先保存了，系统会提示您，并可在覆盖之前先做比较。",
    "splash.benefits.3.title": "放心撤销",
    "splash.benefits.3.body":
      "恢复旧版本会生成一个新版本，因此历史记录永远不会被改写。",
    "splash.benefits.4.title": "清晰的发布规则",
    "splash.benefits.4.body":
      "当条目暂时无法发布时，会明确告诉您是什么阻止了发布。",
    "splash.benefits.5.title": "如实的翻译状态",
    "splash.benefits.5.body": "在读者察觉之前，就能看到哪些译文落后于原文。",
    "splash.benefits.6.title": "及早发现问题",
    "splash.benefits.6.body":
      "内容健康检查会指明违反了哪条规则，便于逐项处理。",
    "splash.features.1.title": "基于内容块",
    "splash.features.1.body": "用结构化内容块组成条目，而不是存储 HTML。",
    "splash.features.2.title": "版本历史",
    "splash.features.2.body": "每次保存都是一个版本，可与其他版本比较并恢复。",
    "splash.features.3.title": "编辑工作流",
    "splash.features.3.body":
      "将条目从草稿推进到审核、批准和发布，并记录每一步的原因。",
    "splash.features.4.title": "本地化",
    "splash.features.4.body":
      "每种语言保留一个版本，未翻译的内容可设置回退链。",
    "splash.features.5.title": "素材库",
    "splash.features.5.body": "在使用素材的条目旁统一管理图片和文件。",
    "splash.features.6.title": "预览与洞察",
    "splash.features.6.body":
      "在语言版本上线前预览，并在仪表板上跟踪内容健康度和待办事项。",
    "nav.toggle": "切换导航",
  },
} as const satisfies Record<Locale, Record<string, string>>;

/** A translatable key (the `en` key set is the contract). */
export type MessageKey = keyof (typeof STRINGS)["en-001"];

/** Every key, for the parity test and for tooling. */
export const MESSAGE_KEYS = Object.keys(STRINGS["en-001"]) as MessageKey[];

/** Narrow a string to a supported locale, honouring a region subtag
 *  (`fr-CA` → `fr`); `null` when nothing matches.
 *
 *  Exact match is tried first (hyphen/underscore-insensitive, so
 *  `en_US`/`en-US` both resolve to that entry) before falling back to
 *  the primary subtag — otherwise a region variant that is itself a
 *  supported locale, like `en_US`, would silently collapse to `en`. */
export function normaliseLocale(
  value: string | null | undefined,
): Locale | null {
  if (!value) return null;
  const normalized = value.trim().replace(/_/g, "-").toLowerCase();
  const exact = LOCALES.find((code) => code === normalized);
  if (exact) return exact;
  const primary = normalized.split("-")[0] ?? "";
  return LOCALES.find((code) => code.split("-")[0] === primary) ?? null;
}

/** Whether `locale` is written right-to-left. */
export function isRtl(locale: string): boolean {
  const primary = normaliseLocale(locale);
  return (
    primary !== null && (RTL_LOCALES as readonly string[]).includes(primary)
  );
}

/** Look a key up in `locale`, falling back to English. */
export function translate(locale: Locale, key: MessageKey): string {
  const table = STRINGS[locale] as Record<string, string>;
  return table[key] ?? STRINGS[DEFAULT_LOCALE][key];
}

/**
 * Read the stored locale, tolerating storage that is not there.
 *
 * `browser` being true does not guarantee `localStorage` works: Safari
 * in private mode throws on access, some embedded webviews disable it,
 * and a strict-privacy setting can remove it entirely. This runs in a
 * module-level constructor, so an unguarded access does not degrade the
 * locale switcher — it throws before the app renders anything at all.
 */
function storedLocale(): Locale | null {
  if (!browser) return null;
  try {
    return normaliseLocale(globalThis.localStorage?.getItem(LOCALE_KEY));
  } catch {
    return null;
  }
}

/** Persist the chosen locale, tolerating storage that refuses. */
function persistLocale(locale: Locale): void {
  if (!browser) return;
  try {
    globalThis.localStorage?.setItem(LOCALE_KEY, locale);
  } catch {
    // The choice still applies for this session; not being able to
    // remember it is not a reason to refuse to honour it.
  }
}

/** The reactive current-locale holder (Svelte 5 runes). */
class I18n {
  #locale = $state<Locale>(DEFAULT_LOCALE);

  constructor() {
    if (browser) {
      const preferred = normaliseLocale(globalThis.navigator?.language);
      this.#locale = storedLocale() ?? preferred ?? DEFAULT_LOCALE;
    }
  }

  /** The current UI locale. */
  get locale(): Locale {
    return this.#locale;
  }

  /** Every offered locale, for the switcher. */
  get locales(): readonly Locale[] {
    return LOCALES;
  }

  /** Switch locale, persisting the choice. An unsupported code is
   *  ignored rather than resetting to English: a bad value in storage
   *  should not quietly undo a user's setting. */
  set(value: string): void {
    const next = normaliseLocale(value);
    if (!next) return;
    this.#locale = next;
    persistLocale(next);
  }
}

/** The app-wide locale holder. */
export const i18n = new I18n();

/** Translate `key` in the current locale. */
export function t(key: MessageKey): string {
  return translate(i18n.locale, key);
}
