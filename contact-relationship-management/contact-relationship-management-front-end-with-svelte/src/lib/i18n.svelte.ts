// Lightweight, dependency-free i18n for the CRM SPA (family pattern,
// copy-adapted from the WPM front-end): a per-locale strings map plus
// a reactive `$state` current-locale (Svelte 5 runes), exposed via a
// `t(key)` accessor. `en-001` is the source of truth; every locale must
// cover the same key set (the parity test pins this). The chosen
// locale persists to localStorage and drives the UI strings and
// `<html dir>` (RTL for `ar-001`).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`), English (`en-001`, the source of truth),
// Spanish (`es-001`), French (`fr-001`), Hindi (`hi-001`) and Simplified
// Chinese for China (`zh-cn`). `-001` is the UN M.49 code for "world": a
// language with no regional variant.

import { browser } from "$app/environment";

/**
 * Locales the UI is translated into, sorted alphabetically by code (the
 * LocalePicker shows them in this order).
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

/** Whether `locale` is written right-to-left. */
export function isRtl(locale: string): boolean {
  const resolved = normaliseLocale(locale);
  return (
    resolved !== null && (RTL_LOCALES as readonly string[]).includes(resolved)
  );
}

/** localStorage key under which the chosen UI locale persists. */
export const LOCALE_KEY = "mxi.crm.locale";

// Every translatable UI string, keyed by a stable dotted key.
const STRINGS = {
  "ar-001": {
    "nav.engagement": "التفاعل",
    "nav.partners": "الشركاء",
    "nav.followups": "المتابعات",
    "nav.executive": "التنفيذية",
    "nav.dpo": "حماية البيانات",
    "common.board": "اللوحة",
    "contact.jobTitle": "المسمى الوظيفي",
    "account.tier": "الفئة",
    "account.industry": "القطاع",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "لوحة التحكم",
    "nav.contacts": "جهات الاتصال",
    "nav.accounts": "الحسابات",
    "nav.leads": "العملاء المحتملون",
    "nav.deals": "الصفقات",
    "nav.campaigns": "الحملات",
    "nav.tickets": "التذاكر",
    "nav.articles": "قاعدة المعرفة",
    "nav.signin": "تسجيل الدخول",
    "chrome.language": "اللغة",
    "nav.share": "مشاركة",
    "nav.text_size": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "common.loading": "جارٍ التحميل…",
    "common.error": "فشل التحميل",
    "common.status": "الحالة",
    "common.name": "الاسم",
    "common.actions": "إجراءات",
    "common.amount": "المبلغ",
    "common.masked": "مخفي",
    "dash.title": "لوحة إدارة العملاء",
    "dash.winRate": "معدل الفوز",
    "dash.openDeals": "الصفقات المفتوحة",
    "dash.openTickets": "التذاكر المفتوحة",
    "dash.forecast": "التوقعات",
    "dash.noData": "لا توجد بيانات بعد",
    "contact.consent": "موافقة تسويقية",
    "contact.timeline": "الجدول الزمني",
    "contact.grant": "منح الموافقة",
    "contact.withdraw": "سحب",
    "contact.subjectAccess": "تنزيل بياناتي",
    "contact.erase": "محو (إخفاء الهوية)",
    "contact.eraseConfirm":
      "هل تريد محو جهة الاتصال هذه؟ لا يمكن التراجع عن هذا.",
    "lead.score": "النقاط",
    "lead.breakdown": "تفصيل النقاط",
    "lead.source": "المصدر",
    "deal.board": "لوحة الصفقات",
    "deal.stage": "المرحلة",
    "deal.won": "فازت",
    "deal.lost": "خسرت",
    "campaign.funnel": "القمع",
    "campaign.roi": "العائد على الاستثمار",
    "campaign.recipients": "المستلمون",
    "campaign.run": "تشغيل (محاكاة)",
    "ticket.priority": "الأولوية",
    "ticket.due": "الرد المستحق",
    "ticket.breached": "تم الخرق",
    "article.version": "الإصدار",
    "article.publish": "نشر",
    "article.search": "بحث المقالات",
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
    "brand.tagline": "كل علاقة مع العملاء في مكان واحد",
    "nav.toggle": "تبديل التنقل",
    "nav.theme": "السمة",
    "splash.hero.title": "اعرف كل عميل وأغلق كل حلقة",
    "splash.hero.subtitle":
      "أدِر جهات الاتصال والصفقات والحملات وتذاكر الدعم في مساحة عمل واحدة، مع احترام الموافقة وتوثيق كل إجراء.",
    "splash.benefits.1.title": "نظرة واحدة لكل عميل",
    "splash.benefits.1.body":
      "يجمع الجدول الزمني لجهة الاتصال وصفقاتها وتذاكرها في مكان واحد، فلا يحتاج أحد إلى السؤال مرتين.",
    "splash.benefits.2.title": "الموافقة أولًا",
    "splash.benefits.2.body":
      "يصل التسويق فقط إلى جهات الاتصال التي وافقت، ويسري سحب الموافقة فورًا.",
    "splash.benefits.3.title": "ركّز على المهم",
    "splash.benefits.3.body":
      "تعرض درجات العملاء المحتملين أسبابها، فيعرف فريقك لماذا يستحق أحدهم اتصالًا.",
    "splash.benefits.4.title": "أرقام صادقة",
    "splash.benefits.4.body":
      "تعرض معدلات الفوز العدد وراء النسبة المئوية، وتظهر البيانات المفقودة على أنها مفقودة.",
    "splash.benefits.5.title": "دعم في الوقت المحدد",
    "splash.benefits.5.body":
      "تُظهر العدادات الحية أي التذاكر يقترب موعدها النهائي وأيها تأخر بالفعل.",
    "splash.benefits.6.title": "لا هويات مكررة",
    "splash.benefits.6.body":
      "تشير جهات الاتصال والحسابات إلى سجلات الأشخاص والمؤسسات المشتركة بدلًا من نسخها.",
    "splash.features.1.title": "جهات الاتصال والحسابات",
    "splash.features.1.body":
      "تصفّح الأشخاص والمؤسسات، مع موافقة كل جهة اتصال وجدولها الزمني في صفحة واحدة.",
    "splash.features.2.title": "تقييم العملاء المحتملين",
    "splash.features.2.body":
      "تتبّع العملاء المحتملين من مصدرهم، مع درجة وتفصيل لما أكسبهم إياها.",
    "splash.features.3.title": "لوحة الصفقات",
    "splash.features.3.body":
      "اسحب الصفقات عبر مراحل المسار وشاهد التوقعات تتحدث من الخادم.",
    "splash.features.4.title": "الحملات والعائد",
    "splash.features.4.body":
      "خطّط الحملات وتابع القمع من المستلمين إلى النتائج واطّلع على عائد كل حملة.",
    "splash.features.5.title": "تذاكر مع اتفاقيات مستوى الخدمة",
    "splash.features.5.body":
      "اعمل على قائمة الدعم حسب الأولوية، مع عدّاد تنازلي لمواعيد الرد في كل تذكرة.",
    "splash.features.6.title": "قاعدة المعرفة",
    "splash.features.6.body":
      "اكتب مقالات المساعدة وأصدِرها وانشرها، وابحث فيها للرد على التذاكر بسرعة أكبر.",
  },
  "cy-001": {
    "nav.engagement": "Ymgysylltu",
    "nav.partners": "Partneriaid",
    "nav.followups": "Dilyniannau",
    "nav.executive": "Gweithredol",
    "nav.dpo": "DPO",
    "common.board": "Bwrdd",
    "contact.jobTitle": "Teitl swydd",
    "account.tier": "Haen",
    "account.industry": "Diwydiant",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "Dangosfwrdd",
    "nav.contacts": "Cysylltiadau",
    "nav.accounts": "Cyfrifon",
    "nav.leads": "Arweinion",
    "nav.deals": "Bargeinion",
    "nav.campaigns": "Ymgyrchoedd",
    "nav.tickets": "Tocynnau",
    "nav.articles": "Sylfaen wybodaeth",
    "nav.signin": "Mewngofnodi",
    "chrome.language": "Iaith",
    "nav.share": "Rhannu",
    "nav.text_size": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "common.loading": "Wrthi'n llwytho…",
    "common.error": "Methwyd llwytho",
    "common.status": "Statws",
    "common.name": "Enw",
    "common.actions": "Camau",
    "common.amount": "Swm",
    "common.masked": "Cudd",
    "dash.title": "Dangosfwrdd CRM",
    "dash.winRate": "Cyfradd ennill",
    "dash.openDeals": "Bargeinion agored",
    "dash.openTickets": "Tocynnau agored",
    "dash.forecast": "Rhagolwg",
    "dash.noData": "Dim data eto",
    "contact.consent": "Caniatâd marchnata",
    "contact.timeline": "Llinell amser",
    "contact.grant": "Rhoi caniatâd",
    "contact.withdraw": "Tynnu'n ôl",
    "contact.subjectAccess": "Lawrlwytho fy nata",
    "contact.erase": "Dileu (dienw)",
    "contact.eraseConfirm": "Dileu'r cyswllt hwn? Ni ellir dadwneud hyn.",
    "lead.score": "Sgôr",
    "lead.breakdown": "Dadansoddiad sgôr",
    "lead.source": "Ffynhonnell",
    "deal.board": "Bwrdd bargeinion",
    "deal.stage": "Cam",
    "deal.won": "Enillwyd",
    "deal.lost": "Collwyd",
    "campaign.funnel": "Twndis",
    "campaign.roi": "ROI",
    "campaign.recipients": "Derbynwyr",
    "campaign.run": "Rhedeg (efelychiad)",
    "ticket.priority": "Blaenoriaeth",
    "ticket.due": "Ymateb erbyn",
    "ticket.breached": "Torrwyd",
    "article.version": "Fersiwn",
    "article.publish": "Cyhoeddi",
    "article.search": "Chwilio erthyglau",
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
    "brand.tagline": "Pob perthynas â chwsmeriaid mewn un lle",
    "nav.toggle": "Toglo'r llywio",
    "nav.theme": "Thema",
    "splash.hero.title": "Adnabod pob cwsmer, cau pob dolen",
    "splash.hero.subtitle":
      "Rheolwch cysylltiadau, bargeinion, ymgyrchoedd a thocynnau cymorth mewn un gweithle, gyda chaniatâd yn cael ei barchu a phob gweithred ar gofnod.",
    "splash.benefits.1.title": "Un golwg fesul cwsmer",
    "splash.benefits.1.body":
      "Mae llinell amser cyswllt, ei fargeinion a'i docynnau gyda'i gilydd, fel nad oes rhaid i neb ofyn ddwywaith.",
    "splash.benefits.2.title": "Caniatâd yn gyntaf",
    "splash.benefits.2.body":
      "Dim ond at gysylltiadau sydd wedi cytuno y mae marchnata'n mynd, ac mae tynnu caniatâd yn dod i rym ar unwaith.",
    "splash.benefits.3.title": "Canolbwyntio ar yr hyn sy'n bwysig",
    "splash.benefits.3.body":
      "Mae sgoriau arweinion yn dangos eu rhesymau, felly mae eich tîm yn gwybod pam mae rhywun werth ei ffonio.",
    "splash.benefits.4.title": "Rhifau gonest",
    "splash.benefits.4.body":
      "Mae cyfraddau ennill yn dangos y cyfrif y tu ôl i'r ganran, ac mae data coll yn cael ei ddangos fel data coll.",
    "splash.benefits.5.title": "Cymorth ar amser",
    "splash.benefits.5.body":
      "Mae cyfrif i lawr byw yn dangos pa docynnau sy'n agos at eu terfyn amser, a pha rai sydd eisoes wedi llithro.",
    "splash.benefits.6.title": "Dim hunaniaethau dyblyg",
    "splash.benefits.6.body":
      "Mae cysylltiadau a chyfrifon yn cyfeirio at y cofrestrau pobl a sefydliadau a rennir yn hytrach na'u copïo.",
    "splash.features.1.title": "Cysylltiadau a chyfrifon",
    "splash.features.1.body":
      "Pori drwy bobl a sefydliadau, gyda chaniatâd a llinell amser pob cyswllt ar un dudalen.",
    "splash.features.2.title": "Sgorio arweinion",
    "splash.features.2.body":
      "Dilynwch arweinion o'u ffynhonnell, gyda sgôr a dadansoddiad o'r hyn a'i enillodd.",
    "splash.features.3.title": "Bwrdd bargeinion",
    "splash.features.3.body":
      "Llusgwch fargeinion ar draws camau'r biblinell a gwyliwch y rhagolwg yn diweddaru o'r gweinydd.",
    "splash.features.4.title": "Ymgyrchoedd ac ROI",
    "splash.features.4.body":
      "Cynlluniwch ymgyrchoedd, dilynwch y twndis o dderbynwyr i ganlyniadau, a gweld yr elw ar bob un.",
    "splash.features.5.title": "Tocynnau â CLGau",
    "splash.features.5.body":
      "Gweithiwch ciw cymorth yn ôl blaenoriaeth, gyda therfynau amser ymateb yn cyfrif i lawr ar bob tocyn.",
    "splash.features.6.title": "Sylfaen wybodaeth",
    "splash.features.6.body":
      "Ysgrifennwch, fersiynwch a chyhoeddwch erthyglau cymorth, a chwiliwch drwyddynt i ateb tocynnau'n gynt.",
  },
  "en-001": {
    "nav.engagement": "Engagement",
    "nav.partners": "Partners",
    "nav.followups": "Follow-ups",
    "nav.executive": "Executive",
    "nav.dpo": "DPO",
    "common.board": "Board",
    "contact.jobTitle": "Job title",
    "account.tier": "Tier",
    "account.industry": "Industry",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "Dashboard",
    "nav.contacts": "Contacts",
    "nav.accounts": "Accounts",
    "nav.leads": "Leads",
    "nav.deals": "Deals",
    "nav.campaigns": "Campaigns",
    "nav.tickets": "Tickets",
    "nav.articles": "Knowledge base",
    "nav.signin": "Sign in",
    "chrome.language": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    "common.loading": "Loading…",
    "common.error": "Failed to load",
    "common.status": "Status",
    "common.name": "Name",
    "common.actions": "Actions",
    "common.amount": "Amount",
    "common.masked": "Hidden",
    "dash.title": "CRM dashboard",
    "dash.winRate": "Win rate",
    "dash.openDeals": "Open deals",
    "dash.openTickets": "Open tickets",
    "dash.forecast": "Forecast",
    "dash.noData": "No data yet",
    "contact.consent": "Marketing consent",
    "contact.timeline": "Timeline",
    "contact.grant": "Grant consent",
    "contact.withdraw": "Withdraw",
    "contact.subjectAccess": "Download my data",
    "contact.erase": "Erase (anonymise)",
    "contact.eraseConfirm": "Erase this contact? This cannot be undone.",
    "lead.score": "Score",
    "lead.breakdown": "Score breakdown",
    "lead.source": "Source",
    "deal.board": "Deal board",
    "deal.stage": "Stage",
    "deal.won": "Won",
    "deal.lost": "Lost",
    "campaign.funnel": "Funnel",
    "campaign.roi": "ROI",
    "campaign.recipients": "Recipients",
    "campaign.run": "Run (simulated)",
    "ticket.priority": "Priority",
    "ticket.due": "Response due",
    "ticket.breached": "Breached",
    "article.version": "Version",
    "article.publish": "Publish",
    "article.search": "Search articles",
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
    "brand.tagline": "Every customer relationship, in one place",
    "nav.toggle": "Toggle navigation",
    "nav.theme": "Theme",
    "splash.hero.title": "Know every customer, close every loop",
    "splash.hero.subtitle":
      "Manage contacts, deals, campaigns and support tickets in one workspace, with consent respected and every action on the record.",
    "splash.benefits.1.title": "One view per customer",
    "splash.benefits.1.body":
      "A contact's timeline, deals and tickets sit together, so nobody has to ask twice.",
    "splash.benefits.2.title": "Consent comes first",
    "splash.benefits.2.body":
      "Marketing goes only to contacts who have agreed, and withdrawing consent takes effect immediately.",
    "splash.benefits.3.title": "Focus on what matters",
    "splash.benefits.3.body":
      "Lead scores show their working, so your team knows why someone is worth a call.",
    "splash.benefits.4.title": "Honest numbers",
    "splash.benefits.4.body":
      "Win rates show the count behind the percentage, and missing data is shown as missing.",
    "splash.benefits.5.title": "Support on time",
    "splash.benefits.5.body":
      "Live countdowns show which tickets are close to their deadline, and which have already slipped.",
    "splash.benefits.6.title": "No duplicate identities",
    "splash.benefits.6.body":
      "Contacts and accounts point to the shared people and organization registers instead of copying them.",
    "splash.features.1.title": "Contacts and accounts",
    "splash.features.1.body":
      "Browse people and organizations, with each contact's consent and timeline on one page.",
    "splash.features.2.title": "Lead scoring",
    "splash.features.2.body":
      "Track leads from their source, with a score and a breakdown of what earned it.",
    "splash.features.3.title": "Deal board",
    "splash.features.3.body":
      "Drag deals across pipeline stages and watch the forecast update from the server.",
    "splash.features.4.title": "Campaigns and ROI",
    "splash.features.4.body":
      "Plan campaigns, follow the funnel from recipients to results, and see the return on each one.",
    "splash.features.5.title": "Tickets with SLAs",
    "splash.features.5.body":
      "Work a support queue by priority, with response deadlines counting down on every ticket.",
    "splash.features.6.title": "Knowledge base",
    "splash.features.6.body":
      "Write, version and publish help articles, and search them to answer tickets faster.",
  },
  "es-001": {
    "nav.engagement": "Compromiso",
    "nav.partners": "Socios",
    "nav.followups": "Seguimientos",
    "nav.executive": "Ejecutivo",
    "nav.dpo": "DPO",
    "common.board": "Tablero",
    "contact.jobTitle": "Puesto",
    "account.tier": "Nivel",
    "account.industry": "Sector",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "Panel",
    "nav.contacts": "Contactos",
    "nav.accounts": "Cuentas",
    "nav.leads": "Prospectos",
    "nav.deals": "Oportunidades",
    "nav.campaigns": "Campañas",
    "nav.tickets": "Tickets",
    "nav.articles": "Base de conocimiento",
    "nav.signin": "Iniciar sesión",
    "chrome.language": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "common.loading": "Cargando…",
    "common.error": "Error al cargar",
    "common.status": "Estado",
    "common.name": "Nombre",
    "common.actions": "Acciones",
    "common.amount": "Importe",
    "common.masked": "Oculto",
    "dash.title": "Panel CRM",
    "dash.winRate": "Tasa de éxito",
    "dash.openDeals": "Oportunidades abiertas",
    "dash.openTickets": "Tickets abiertos",
    "dash.forecast": "Pronóstico",
    "dash.noData": "Sin datos aún",
    "contact.consent": "Consentimiento de marketing",
    "contact.timeline": "Cronología",
    "contact.grant": "Otorgar consentimiento",
    "contact.withdraw": "Retirar",
    "contact.subjectAccess": "Descargar mis datos",
    "contact.erase": "Borrar (anonimizar)",
    "contact.eraseConfirm": "¿Borrar este contacto? Esto no se puede deshacer.",
    "lead.score": "Puntuación",
    "lead.breakdown": "Desglose de puntuación",
    "lead.source": "Origen",
    "deal.board": "Tablero de oportunidades",
    "deal.stage": "Etapa",
    "deal.won": "Ganada",
    "deal.lost": "Perdida",
    "campaign.funnel": "Embudo",
    "campaign.roi": "ROI",
    "campaign.recipients": "Destinatarios",
    "campaign.run": "Ejecutar (simulado)",
    "ticket.priority": "Prioridad",
    "ticket.due": "Respuesta antes de",
    "ticket.breached": "Incumplido",
    "article.version": "Versión",
    "article.publish": "Publicar",
    "article.search": "Buscar artículos",
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
    "brand.tagline": "Cada relación con el cliente, en un solo lugar",
    "nav.toggle": "Alternar navegación",
    "nav.theme": "Tema",
    "splash.hero.title": "Conoce a cada cliente, cierra cada ciclo",
    "splash.hero.subtitle":
      "Gestiona contactos, oportunidades, campañas y tickets de soporte en un solo espacio, con el consentimiento respetado y cada acción registrada.",
    "splash.benefits.1.title": "Una vista por cliente",
    "splash.benefits.1.body":
      "La cronología, las oportunidades y los tickets de un contacto están juntos, así nadie tiene que preguntar dos veces.",
    "splash.benefits.2.title": "El consentimiento primero",
    "splash.benefits.2.body":
      "El marketing solo llega a quienes han dado su consentimiento, y retirarlo surte efecto de inmediato.",
    "splash.benefits.3.title": "Céntrate en lo importante",
    "splash.benefits.3.body":
      "Las puntuaciones de los prospectos muestran su cálculo, así tu equipo sabe por qué merece una llamada.",
    "splash.benefits.4.title": "Cifras honestas",
    "splash.benefits.4.body":
      "Las tasas de éxito muestran el recuento tras el porcentaje, y los datos ausentes se muestran como ausentes.",
    "splash.benefits.5.title": "Soporte a tiempo",
    "splash.benefits.5.body":
      "Las cuentas atrás en vivo muestran qué tickets se acercan a su plazo y cuáles ya lo han superado.",
    "splash.benefits.6.title": "Sin identidades duplicadas",
    "splash.benefits.6.body":
      "Los contactos y las cuentas apuntan a los registros compartidos de personas y organizaciones en lugar de copiarlos.",
    "splash.features.1.title": "Contactos y cuentas",
    "splash.features.1.body":
      "Explora personas y organizaciones, con el consentimiento y la cronología de cada contacto en una página.",
    "splash.features.2.title": "Puntuación de prospectos",
    "splash.features.2.body":
      "Sigue los prospectos desde su origen, con una puntuación y el desglose de lo que la generó.",
    "splash.features.3.title": "Tablero de oportunidades",
    "splash.features.3.body":
      "Arrastra oportunidades entre las etapas del embudo y observa cómo se actualiza la previsión desde el servidor.",
    "splash.features.4.title": "Campañas y ROI",
    "splash.features.4.body":
      "Planifica campañas, sigue el embudo desde los destinatarios hasta los resultados y consulta el retorno de cada una.",
    "splash.features.5.title": "Tickets con SLA",
    "splash.features.5.body":
      "Trabaja una cola de soporte por prioridad, con plazos de respuesta que cuentan atrás en cada ticket.",
    "splash.features.6.title": "Base de conocimiento",
    "splash.features.6.body":
      "Redacta, versiona y publica artículos de ayuda, y búscalos para responder a los tickets más rápido.",
  },
  "fr-001": {
    "nav.engagement": "Engagement",
    "nav.partners": "Partenaires",
    "nav.followups": "Relances",
    "nav.executive": "Direction",
    "nav.dpo": "DPO",
    "common.board": "Tableau",
    "contact.jobTitle": "Intitulé du poste",
    "account.tier": "Niveau",
    "account.industry": "Secteur",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "Tableau de bord",
    "nav.contacts": "Contacts",
    "nav.accounts": "Comptes",
    "nav.leads": "Prospects",
    "nav.deals": "Affaires",
    "nav.campaigns": "Campagnes",
    "nav.tickets": "Tickets",
    "nav.articles": "Base de connaissances",
    "nav.signin": "Se connecter",
    "chrome.language": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "common.loading": "Chargement…",
    "common.error": "Échec du chargement",
    "common.status": "Statut",
    "common.name": "Nom",
    "common.actions": "Actions",
    "common.amount": "Montant",
    "common.masked": "Masqué",
    "dash.title": "Tableau de bord CRM",
    "dash.winRate": "Taux de réussite",
    "dash.openDeals": "Affaires ouvertes",
    "dash.openTickets": "Tickets ouverts",
    "dash.forecast": "Prévision",
    "dash.noData": "Pas encore de données",
    "contact.consent": "Consentement marketing",
    "contact.timeline": "Chronologie",
    "contact.grant": "Accorder le consentement",
    "contact.withdraw": "Retirer",
    "contact.subjectAccess": "Télécharger mes données",
    "contact.erase": "Effacer (anonymiser)",
    "contact.eraseConfirm":
      "Effacer ce contact ? Cette action est irréversible.",
    "lead.score": "Score",
    "lead.breakdown": "Détail du score",
    "lead.source": "Source",
    "deal.board": "Tableau des affaires",
    "deal.stage": "Étape",
    "deal.won": "Gagnée",
    "deal.lost": "Perdue",
    "campaign.funnel": "Entonnoir",
    "campaign.roi": "ROI",
    "campaign.recipients": "Destinataires",
    "campaign.run": "Lancer (simulé)",
    "ticket.priority": "Priorité",
    "ticket.due": "Réponse due",
    "ticket.breached": "Dépassé",
    "article.version": "Version",
    "article.publish": "Publier",
    "article.search": "Rechercher des articles",
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
    "brand.tagline": "Chaque relation client, au même endroit",
    "nav.toggle": "Afficher ou masquer la navigation",
    "nav.theme": "Thème",
    "splash.hero.title": "Connaître chaque client, boucler chaque suivi",
    "splash.hero.subtitle":
      "Gérez contacts, affaires, campagnes et tickets d'assistance dans un seul espace, avec le consentement respecté et chaque action consignée.",
    "splash.benefits.1.title": "Une vue par client",
    "splash.benefits.1.body":
      "La chronologie, les affaires et les tickets d'un contact sont réunis, personne n'a à poser deux fois la question.",
    "splash.benefits.2.title": "Le consentement d'abord",
    "splash.benefits.2.body":
      "Le marketing ne vise que les contacts consentants, et le retrait du consentement est immédiat.",
    "splash.benefits.3.title": "Concentrez-vous sur l'essentiel",
    "splash.benefits.3.body":
      "Les scores des prospects détaillent leur calcul, votre équipe sait pourquoi un appel s'impose.",
    "splash.benefits.4.title": "Des chiffres honnêtes",
    "splash.benefits.4.body":
      "Les taux de réussite affichent l'effectif derrière le pourcentage, et les données manquantes apparaissent comme telles.",
    "splash.benefits.5.title": "Un support dans les délais",
    "splash.benefits.5.body":
      "Des comptes à rebours en direct montrent quels tickets approchent de l'échéance et lesquels l'ont déjà dépassée.",
    "splash.benefits.6.title": "Aucune identité en double",
    "splash.benefits.6.body":
      "Les contacts et les comptes renvoient aux registres partagés des personnes et des organisations au lieu de les copier.",
    "splash.features.1.title": "Contacts et comptes",
    "splash.features.1.body":
      "Parcourez personnes et organisations, avec le consentement et la chronologie de chaque contact sur une page.",
    "splash.features.2.title": "Notation des prospects",
    "splash.features.2.body":
      "Suivez les prospects depuis leur source, avec un score et le détail de ce qui l'a produit.",
    "splash.features.3.title": "Tableau des affaires",
    "splash.features.3.body":
      "Faites glisser les affaires d'une étape du pipeline à l'autre et voyez les prévisions se mettre à jour depuis le serveur.",
    "splash.features.4.title": "Campagnes et ROI",
    "splash.features.4.body":
      "Planifiez des campagnes, suivez l'entonnoir des destinataires aux résultats et voyez le retour de chacune.",
    "splash.features.5.title": "Tickets avec SLA",
    "splash.features.5.body":
      "Traitez une file de support par priorité, avec les échéances de réponse décomptées sur chaque ticket.",
    "splash.features.6.title": "Base de connaissances",
    "splash.features.6.body":
      "Rédigez, versionnez et publiez des articles d'aide, et recherchez-les pour répondre plus vite aux tickets.",
  },
  "hi-001": {
    "nav.engagement": "सहभागिता",
    "nav.partners": "साझेदार",
    "nav.followups": "फ़ॉलो-अप",
    "nav.executive": "कार्यकारी",
    "nav.dpo": "DPO",
    "common.board": "बोर्ड",
    "contact.jobTitle": "पद",
    "account.tier": "स्तर",
    "account.industry": "उद्योग",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "डैशबोर्ड",
    "nav.contacts": "संपर्क",
    "nav.accounts": "खाते",
    "nav.leads": "लीड",
    "nav.deals": "डील",
    "nav.campaigns": "अभियान",
    "nav.tickets": "टिकट",
    "nav.articles": "ज्ञान आधार",
    "nav.signin": "साइन इन",
    "chrome.language": "भाषा",
    "nav.share": "साझा करें",
    "nav.text_size": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "common.loading": "लोड हो रहा है…",
    "common.error": "लोड विफल",
    "common.status": "स्थिति",
    "common.name": "नाम",
    "common.actions": "कार्रवाइयां",
    "common.amount": "राशि",
    "common.masked": "छिपा हुआ",
    "dash.title": "सीआरएम डैशबोर्ड",
    "dash.winRate": "जीत दर",
    "dash.openDeals": "खुली डील",
    "dash.openTickets": "खुले टिकट",
    "dash.forecast": "पूर्वानुमान",
    "dash.noData": "अभी कोई डेटा नहीं",
    "contact.consent": "विपणन सहमति",
    "contact.timeline": "समयरेखा",
    "contact.grant": "सहमति दें",
    "contact.withdraw": "वापस लें",
    "contact.subjectAccess": "मेरा डेटा डाउनलोड करें",
    "contact.erase": "मिटाएँ (गुमनाम करें)",
    "contact.eraseConfirm":
      "इस संपर्क को मिटाएँ? इसे पूर्ववत नहीं किया जा सकता।",
    "lead.score": "स्कोर",
    "lead.breakdown": "स्कोर विवरण",
    "lead.source": "स्रोत",
    "deal.board": "डील बोर्ड",
    "deal.stage": "चरण",
    "deal.won": "जीती",
    "deal.lost": "हारी",
    "campaign.funnel": "फ़नल",
    "campaign.roi": "आरओआई",
    "campaign.recipients": "प्राप्तकर्ता",
    "campaign.run": "चलाएं (नकली)",
    "ticket.priority": "प्राथमिकता",
    "ticket.due": "प्रतिक्रिया देय",
    "ticket.breached": "उल्लंघन",
    "article.version": "संस्करण",
    "article.publish": "प्रकाशित करें",
    "article.search": "लेख खोजें",
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
    "brand.tagline": "हर ग्राहक संबंध, एक ही जगह",
    "nav.toggle": "नेविगेशन खोलें या बंद करें",
    "nav.theme": "थीम",
    "splash.hero.title": "हर ग्राहक को जानें, हर बात पूरी करें",
    "splash.hero.subtitle":
      "संपर्क, डील, अभियान और सहायता टिकट एक ही कार्यक्षेत्र में संभालें, सहमति का सम्मान करते हुए और हर कार्रवाई दर्ज रखते हुए।",
    "splash.benefits.1.title": "हर ग्राहक की एक तस्वीर",
    "splash.benefits.1.body":
      "संपर्क की समयरेखा, डील और टिकट एक साथ रहते हैं, इसलिए किसी को दोबारा पूछना नहीं पड़ता।",
    "splash.benefits.2.title": "सहमति पहले",
    "splash.benefits.2.body":
      "विपणन केवल उन्हीं संपर्कों तक जाता है जिन्होंने सहमति दी है, और सहमति वापस लेना तुरंत लागू होता है।",
    "splash.benefits.3.title": "जो ज़रूरी है उस पर ध्यान दें",
    "splash.benefits.3.body":
      "लीड स्कोर अपना हिसाब दिखाते हैं, इसलिए आपकी टीम जानती है कि किसे कॉल करना सार्थक है।",
    "splash.benefits.4.title": "ईमानदार आँकड़े",
    "splash.benefits.4.body":
      "जीत की दर प्रतिशत के पीछे की गिनती दिखाती है, और अनुपलब्ध डेटा अनुपलब्ध ही दिखता है।",
    "splash.benefits.5.title": "समय पर सहायता",
    "splash.benefits.5.body":
      "लाइव काउंटडाउन दिखाते हैं कि कौन-से टिकट समय-सीमा के करीब हैं और कौन-से पहले ही चूक चुके हैं।",
    "splash.benefits.6.title": "कोई दोहरी पहचान नहीं",
    "splash.benefits.6.body":
      "संपर्क और खाते साझा व्यक्ति और संगठन रजिस्टरों की ओर संकेत करते हैं, उनकी नकल नहीं बनाते।",
    "splash.features.1.title": "संपर्क और खाते",
    "splash.features.1.body":
      "व्यक्तियों और संगठनों को देखें, हर संपर्क की सहमति और समयरेखा एक ही पृष्ठ पर।",
    "splash.features.2.title": "लीड स्कोरिंग",
    "splash.features.2.body":
      "लीड को उनके स्रोत से ट्रैक करें, स्कोर और उसके कारणों के विवरण के साथ।",
    "splash.features.3.title": "डील बोर्ड",
    "splash.features.3.body":
      "डील को पाइपलाइन के चरणों में खिसकाएँ और सर्वर से पूर्वानुमान अपडेट होते देखें।",
    "splash.features.4.title": "अभियान और ROI",
    "splash.features.4.body":
      "अभियान बनाएँ, प्राप्तकर्ताओं से नतीजों तक फ़नल देखें और हर अभियान का रिटर्न जानें।",
    "splash.features.5.title": "SLA वाले टिकट",
    "splash.features.5.body":
      "सहायता कतार को प्राथमिकता के अनुसार संभालें, हर टिकट पर उत्तर की समय-सीमा का काउंटडाउन चलता है।",
    "splash.features.6.title": "ज्ञान आधार",
    "splash.features.6.body":
      "सहायता लेख लिखें, उनके संस्करण रखें और प्रकाशित करें, और टिकटों का जल्दी जवाब देने के लिए उन्हें खोजें।",
  },
  "zh-cn": {
    "nav.engagement": "互动",
    "nav.partners": "伙伴",
    "nav.followups": "跟进",
    "nav.executive": "高管",
    "nav.dpo": "数据保护",
    "common.board": "看板",
    "contact.jobTitle": "职位",
    "account.tier": "层级",
    "account.industry": "行业",
    "brand.name": "Main X · CRM",
    "nav.dashboard": "仪表盘",
    "nav.contacts": "联系人",
    "nav.accounts": "客户",
    "nav.leads": "线索",
    "nav.deals": "商机",
    "nav.campaigns": "营销活动",
    "nav.tickets": "工单",
    "nav.articles": "知识库",
    "nav.signin": "登录",
    "chrome.language": "语言",
    "nav.share": "分享",
    "nav.text_size": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "common.loading": "加载中…",
    "common.error": "加载失败",
    "common.status": "状态",
    "common.name": "名称",
    "common.actions": "操作",
    "common.amount": "金额",
    "common.masked": "已隐藏",
    "dash.title": "客户关系仪表盘",
    "dash.winRate": "赢单率",
    "dash.openDeals": "进行中商机",
    "dash.openTickets": "未结工单",
    "dash.forecast": "预测",
    "dash.noData": "暂无数据",
    "contact.consent": "营销同意",
    "contact.timeline": "时间线",
    "contact.grant": "授予同意",
    "contact.withdraw": "撤回",
    "contact.subjectAccess": "下载我的数据",
    "contact.erase": "删除(匿名化)",
    "contact.eraseConfirm": "删除此联系人？此操作无法撤销。",
    "lead.score": "评分",
    "lead.breakdown": "评分明细",
    "lead.source": "来源",
    "deal.board": "商机看板",
    "deal.stage": "阶段",
    "deal.won": "已赢",
    "deal.lost": "已失",
    "campaign.funnel": "漏斗",
    "campaign.roi": "投资回报率",
    "campaign.recipients": "收件人",
    "campaign.run": "运行（模拟）",
    "ticket.priority": "优先级",
    "ticket.due": "应答期限",
    "ticket.breached": "已超时",
    "article.version": "版本",
    "article.publish": "发布",
    "article.search": "搜索文章",
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
    "brand.tagline": "每一段客户关系，尽在一处",
    "nav.toggle": "切换导航",
    "nav.theme": "主题",
    "splash.hero.title": "了解每位客户，跟进每一件事",
    "splash.hero.subtitle":
      "在同一个工作区管理联系人、商机、营销活动和支持工单，尊重客户同意，并记录每一项操作。",
    "splash.benefits.1.title": "每位客户一个视图",
    "splash.benefits.1.body":
      "联系人的时间线、商机和工单汇集在一起，无需重复询问。",
    "splash.benefits.2.title": "同意优先",
    "splash.benefits.2.body": "营销仅发送给已同意的联系人，撤回同意立即生效。",
    "splash.benefits.3.title": "专注重要之事",
    "splash.benefits.3.body": "线索评分会展示计算依据，团队清楚为何值得联系。",
    "splash.benefits.4.title": "真实的数字",
    "splash.benefits.4.body":
      "成交率同时显示百分比背后的数量，缺失的数据就显示为缺失。",
    "splash.benefits.5.title": "及时的支持",
    "splash.benefits.5.body": "实时倒计时显示哪些工单临近截止，哪些已经超时。",
    "splash.benefits.6.title": "没有重复身份",
    "splash.benefits.6.body":
      "联系人和客户指向共享的人员与组织登记库，而不是复制其数据。",
    "splash.features.1.title": "联系人与客户",
    "splash.features.1.body":
      "浏览人员与组织，每位联系人的同意状态和时间线集中在一页。",
    "splash.features.2.title": "线索评分",
    "splash.features.2.body": "从来源跟踪线索，附带评分及得分明细。",
    "splash.features.3.title": "商机看板",
    "splash.features.3.body":
      "在销售管道各阶段间拖动商机，预测由服务器实时更新。",
    "splash.features.4.title": "营销活动与投资回报",
    "splash.features.4.body":
      "规划营销活动，跟踪从收件人到成果的漏斗，并查看每次活动的回报。",
    "splash.features.5.title": "带 SLA 的工单",
    "splash.features.5.body":
      "按优先级处理支持队列，每张工单都有响应期限倒计时。",
    "splash.features.6.title": "知识库",
    "splash.features.6.body":
      "撰写、版本化并发布帮助文章，检索它们以更快答复工单。",
  },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)["en-001"];

/** Every translatable key, for the per-locale coverage test. */
export const STRING_KEYS = Object.keys(STRINGS["en-001"]) as StringKey[];

/** Raw per-locale strings table, exposed for coverage testing. */
export const STRINGS_BY_LOCALE: Record<
  Locale,
  Record<string, string>
> = STRINGS;

// Normalise raw input to a supported locale, or null if unsupported.
// Exact match first (hyphen/underscore-insensitive), so "zh-CN" resolves to
// `zh-cn`; otherwise the primary language subtag picks the locale whose
// code starts with it (`en`, `en-US` -> `en-001`; `zh-TW` -> `zh-cn`).
function normaliseLocale(raw: string | null | undefined): Locale | null {
  if (!raw) return null;
  const trimmed = raw.trim().replace(/_/g, "-").toLowerCase();
  const exact = (LOCALES as readonly string[]).find((l) => l === trimmed);
  if (exact) return exact as Locale;
  const primary = trimmed.split("-")[0] ?? "";
  const byPrimary = (LOCALES as readonly string[]).find(
    (l) => l.split("-")[0] === primary,
  );
  return (byPrimary as Locale | undefined) ?? null;
}

// Seed the reactive locale from localStorage (default off the browser).
function readStoredLocale(): Locale {
  if (!browser || typeof localStorage === "undefined") return DEFAULT_LOCALE;
  return normaliseLocale(localStorage.getItem(LOCALE_KEY)) ?? DEFAULT_LOCALE;
}

// Reactive current-locale state; mutating it re-renders every `t(...)`.
let current = $state<Locale>(readStoredLocale());

/** Reactive current-locale store with persistence. */
export const i18n = {
  /** The currently selected locale. */
  get locale(): Locale {
    return current;
  },
  /** Switch the active locale and persist the choice. */
  set(next: string): void {
    const locale = normaliseLocale(next) ?? DEFAULT_LOCALE;
    current = locale;
    if (browser && typeof localStorage !== "undefined")
      localStorage.setItem(LOCALE_KEY, locale);
  },
  /** The list of supported locales (for the switcher). */
  get locales(): readonly Locale[] {
    return LOCALES;
  },
};

/**
 * Translate `key` in `locale` with graceful fallbacks (locale → en →
 * the key itself). Pure — unit-testable without a component.
 */
export function translate(key: StringKey, locale: Locale = current): string {
  const table = STRINGS[locale] ?? STRINGS[DEFAULT_LOCALE];
  return table[key] ?? STRINGS[DEFAULT_LOCALE][key] ?? key;
}

/** Reactive translation accessor for components: `t("dash.title")`. */
export function t(key: StringKey): string {
  return translate(key, current);
}
