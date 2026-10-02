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

import { browser } from "$app/env";

/**
 * Locales the UI is translated into, sorted alphabetically by code (the
 * LocalePicker shows them in this order). `-001` is the UN M.49 code for
 * "world": a language with no regional variant.
 */
export const LOCALES = [
  "ar-001",
  "cy-001",
  "de-de",
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
  "de-de": "Deutsch - Deutschland",
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
      "العربية والصينية والألمانية والإنجليزية والفرنسية والهندية والإسبانية والويلزية.",
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
    "nav.tour": "جولة",
    "splash.hero.tour": "ابدأ الجولة",
    "tour.head": "ابدأ الجولة",
    "tour.toc": "في هذه الصفحة",
    "tour.open": "افتح هذه الشاشة",
    "tour.top": "العودة إلى الأعلى",
    "tour.start.title": "قبل أن تبدأ",
    "tour.start.summary":
      "تحتاج إلى حساب للعمل مع البيانات الفعلية. يستغرق تسجيل الدخول أقل من دقيقة ولا يتطلب كلمة مرور.",
    "tour.start.step.1":
      "اختر «تسجيل الدخول» في أعلى اليمين وأدخل بريدك الإلكتروني.",
    "tour.start.step.2":
      "افتح الرابط السحري الذي نرسله إلى بريدك. يعمل مرة واحدة وتنتهي صلاحيته سريعًا.",
    "tour.start.step.3":
      "تعود إلى التطبيق وقد سجّلت الدخول، دون شيء لتتذكره أو تعيده.",
    "tour.start.step.4":
      "استخدم الأزرار بجوار «تسجيل الدخول» لتغيير السمة واللغة وحجم النص أو لمشاركة الصفحة.",
    "tour.s1.title": "العثور على مدخل وفتحه",
    "tour.s1.summary": "حدّد أي مدخل في موقع وافتحه للتحرير.",
    "tour.s1.step.1":
      "افتح المدخلات، ثم اختر الموقع الذي تعمل عليه ضمن «اختر موقعًا».",
    "tour.s1.step.2":
      "ضيّق القائمة بقائمة «النوع» المنسدلة، أو اكتب جزءًا من الاسم في خانة «المفتاح».",
    "tour.s1.step.3":
      "اقرأ أعمدة المفتاح والنوع ولغة المصدر والحالة؛ وتحمل المدخلات المؤرشفة علامة «مؤرشف».",
    "tour.s1.step.4": "اختر مفتاح مدخل لفتح صفحته، حيث تحرره وتراجعه وتنشره.",
    "tour.s2.title": "تحرير كتل المحتوى وحفظ نسخة",
    "tour.s2.summary":
      "تُبنى المدخلات من كتل منظّمة بدلًا من HTML مخزَّن، وكل حفظ يصبح نسخة.",
    "tour.s2.step.1":
      "في صفحة المدخل، اختر لغة من جدول اللغات الذي يعرض حالة كل لغة ونسختها المنشورة وهل تأخرت عن المصدر.",
    "tour.s2.step.2":
      "ضمن «كتل المحتوى» حرّر الكتل واستخدم «إضافة كتلة» لإدراج عنوان أو فقرة أو قائمة أو اقتباس أو صورة أو كود؛ وتعيد «تحريك لأعلى» و«تحريك لأسفل» و«إزالة» الترتيب أو الحذف.",
    "tour.s2.step.3": "غيّر العنوان عند الحاجة، ثم اختر «حفظ النسخة».",
    "tour.s2.step.4":
      "عندما تتقدم المسودة على المنشور، تذكر الصفحة ذلك، فتعرف دائمًا أن القرّاء ما زالوا يرون النسخة الأقدم.",
    "tour.s3.title": "مقارنة النسخ واستعادتها",
    "tour.s3.summary":
      "لا يُعاد كتابة السجل أبدًا، فيمكنك النظر إلى الوراء والتراجع بأمان.",
    "tour.s3.step.1":
      "مرّر إلى «سجل النسخ» الذي يسرد كل نسخة بعنوانها ومؤلفها وتاريخها؛ وتُعلَّم المنشورة بـ«منشور».",
    "tour.s3.step.2":
      "اختر «مقارنة» بجانب نسخة لترى الفرق بينها وبين ما تحرره؛ وتذكر الصفحة إن كانتا متطابقتين.",
    "tour.s3.step.3":
      "اختر «استعادة» على نسخة أقدم؛ فتُكتب نسخة جديدة وتبقى كل النسخ السابقة كما هي.",
    "tour.s3.step.4":
      "إذا حفظ شخص آخر قبلك، تُبلغك الصفحة وتعرض النسخة الفائزة لتقارن قبل الكتابة فوق أي شيء.",
    "tour.s4.title": "المراجعة والنشر",
    "tour.s4.summary":
      "انقل المدخل عبر سير العمل التحريري مع إظهار العوائق والأسباب بوضوح.",
    "tour.s4.step.1":
      "في صفحة المدخل، راجع لوحة النشر: «جاهز للنشر»، أو «لا يمكن النشر بعد» مع جدول القاعدة والعنوان وما ينبغي فعله.",
    "tour.s4.step.2":
      "في لوحة سير العمل، اختر إجراءً (submit أو approve أو reject أو publish أو unpublish أو archive) واكتب السبب؛ ويتطلب reject وunpublish وarchive سببًا.",
    "tour.s4.step.3":
      "اختر «معاينة» لرؤية اللغة المحددة كمعاينة من الخادم؛ وينبّهك إشعار عندما لا تكون ما يراه القرّاء.",
    "tour.s4.step.4":
      "افتح «سير العمل» من القائمة لترى ما ينتظر: المدخلات «قيد المراجعة» وطلبات الترجمة المفتوحة والعناصر «المجدولة».",
    "tour.s5.title": "حافظ على حداثة الترجمات",
    "tour.s5.summary": "اعرف أي اللغات تأخرت عن المصدر قبل أن يلاحظ القرّاء.",
    "tour.s5.step.1":
      "افتح «الترجمات» واختر موقعًا واقرأ جدول اللغات: لكل لغة مدخلاتها وعدد المسودة والمنشور منها.",
    "tour.s5.step.2":
      "اقرأ القاعدة المطبوعة في الصفحة لتعرف بدقة ما يُعدّ متأخرًا عن المصدر، ووقت «حتى تاريخ» الذي حُسبت فيه.",
    "tour.s5.step.3":
      "ضمن «الطلبات المفتوحة» ابحث عن كل مفتاح مدخل مع اللغة والحالة والمؤلف ووقت آخر تحديث.",
    "tour.s5.step.4":
      "لإصلاح واحد، افتح المدخل من «المدخلات»، واختر اللغة في جدول اللغات فيه، واحفظ نسخة جديدة بتلك اللغة.",
    "tour.s6.title": "فحص الأصول والنص البديل",
    "tour.s6.summary":
      "اطّلع على كل صورة وملف في الموقع والتقط النص البديل المفقود قبل أن يمنع النشر.",
    "tour.s6.step.1":
      "افتح «الأصول» واقرأ الملخص: المساحة المستخدمة والصور بدون نص بديل والملفات التي لا يشير إليها شيء.",
    "tour.s6.step.2":
      "في الجدول، افحص عنوان كل أصل ونوعه وحجمه ونصه البديل؛ وتحمل الصورة بلا نص بديل علامة «لا يوجد نص بديل».",
    "tour.s6.step.3":
      "تذكّر القاعدة: الصورة بدون نص بديل تمنع نشر الصفحة، فعالجها قبل نقل المدخل إلى النشر.",
    "tour.s6.step.4":
      "تُبلَّغ بالملفات «التي لا يشير إليها شيء» لانتباهك، لكنها لا تُحذف تلقائيًا أبدًا.",
    "tour.intro":
      "جولة إرشادية في نظام إدارة المحتوى: كيف تجد مدخلًا وتحرر كتل المحتوى فيه وتستعيد نسخًا قديمة وتنشر عبر سير العمل وتحافظ على حداثة الترجمات وتدير الأصول.",
    "signin.sso": "تسجيل الدخول عبر SSO",
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
      "Arabeg, Tsieinëeg, Almaeneg, Saesneg, Ffrangeg, Hindi, Sbaeneg a Chymraeg.",
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
    "nav.tour": "Taith",
    "splash.hero.tour": "Cymerwch y daith",
    "tour.head": "Cymerwch y daith",
    "tour.toc": "Ar y dudalen hon",
    "tour.open": "Agor y sgrin hon",
    "tour.top": "Yn ôl i'r brig",
    "tour.start.title": "Cyn i chi ddechrau",
    "tour.start.summary":
      "Mae angen cyfrif arnoch i weithio gyda data go iawn. Mae mewngofnodi'n cymryd llai na munud ac nid oes angen cyfrinair.",
    "tour.start.step.1":
      "Dewiswch Mewngofnodi ar y brig ar y dde a rhowch eich cyfeiriad e-bost.",
    "tour.start.step.2":
      "Agorwch y ddolen hud a anfonwn atoch. Mae'n gweithio unwaith ac yn dod i ben yn gyflym.",
    "tour.start.step.3":
      "Byddwch yn dychwelyd i'r ap wedi mewngofnodi, heb ddim i'w gofio na'i ailosod.",
    "tour.start.step.4":
      "Defnyddiwch y botymau wrth ymyl Mewngofnodi i newid y thema, yr iaith a maint y testun, neu i rannu'r dudalen.",
    "tour.s1.title": "Dod o hyd i gofnod a'i agor",
    "tour.s1.summary":
      "Dewch o hyd i unrhyw gofnod mewn safle a'i agor i'w olygu.",
    "tour.s1.step.1":
      "Agorwch Cofnodion ac, o dan Dewis safle, dewiswch y safle rydych chi'n gweithio arno.",
    "tour.s1.step.2":
      "Culhewch y rhestr gyda'r gwymplen Math, neu teipiwch ran o enw yn y blwch Allwedd.",
    "tour.s1.step.3":
      "Darllenwch y colofnau Allwedd, Math, Iaith ffynhonnell a Statws; mae cofnodion wedi'u harchifo'n dangos marc Archifwyd.",
    "tour.s1.step.4":
      "Dewiswch allwedd cofnod i agor ei dudalen, lle byddwch yn ei olygu, ei adolygu a'i gyhoeddi.",
    "tour.s2.title": "Golygu blociau cynnwys a chadw diwygiad",
    "tour.s2.summary":
      "Mae cofnodion wedi'u hadeiladu o flociau strwythuredig yn hytrach na HTML wedi'i storio, ac mae pob cadw yn dod yn ddiwygiad.",
    "tour.s2.step.1":
      "Ar dudalen y cofnod, dewiswch leoliad yn y tabl Ieithoedd, sy'n dangos statws pob un, ei ddiwygiad byw ac a yw y tu ôl i'r ffynhonnell.",
    "tour.s2.step.2":
      "O dan Blociau cynnwys, golygwch y blociau a defnyddiwch Ychwanegu bloc i osod pennawd, paragraff, rhestr, dyfyniad, delwedd neu god; mae Symud i fyny, Symud i lawr a Dileu yn ad-drefnu neu'n dileu.",
    "tour.s2.step.3":
      "Newidiwch y Teitl os oes angen, yna dewiswch Cadw diwygiad.",
    "tour.s2.step.4":
      "Pan fo'r drafft ar y blaen i'r hyn sy'n fyw, mae'r dudalen yn dweud hynny, felly rydych bob amser yn gwybod bod darllenwyr yn dal i weld yr hen fersiwn.",
    "tour.s3.title": "Cymharu ac adfer diwygiadau",
    "tour.s3.summary":
      "Nid yw hanes byth yn cael ei ailysgrifennu, felly gallwch edrych yn ôl a dadwneud yn ddiogel.",
    "tour.s3.step.1":
      "Sgroliwch i Hanes diwygiadau, sy'n rhestru pob diwygiad gyda'i deitl, ei awdur a'i ddyddiad; mae'r un byw wedi'i farcio Cyhoeddwyd.",
    "tour.s3.step.2":
      "Dewiswch Cymharu wrth ymyl diwygiad i weld sut mae'n wahanol i'r un rydych yn ei olygu; mae diwygiadau unfath yn dweud hynny.",
    "tour.s3.step.3":
      "Dewiswch Adfer ar ddiwygiad hŷn; mae hyn yn ysgrifennu diwygiad newydd ac yn gadael pob un blaenorol yn gyfan.",
    "tour.s3.step.4":
      "Os yw rhywun arall wedi cadw'n gyntaf, mae'r dudalen yn dweud hynny ac yn dangos y diwygiad buddugol, fel y gallwch gymharu cyn trosysgrifo unrhyw beth.",
    "tour.s4.title": "Adolygu a chyhoeddi",
    "tour.s4.summary":
      "Symudwch gofnod drwy'r llif gwaith golygyddol, gyda rhwystrau a rhesymau'n cael eu gwneud yn eglur.",
    "tour.s4.step.1":
      "Ar dudalen y cofnod, gwiriwch y panel cyhoeddi: Yn barod i'w gyhoeddi, neu Methu cyhoeddi eto gyda thabl o'r Rheol, y Teitl a Beth i'w wneud.",
    "tour.s4.step.2":
      "Ym mhanel Llif gwaith, dewiswch Gweithred (submit, approve, reject, publish, unpublish neu archive) a theipiwch Reswm; mae reject, unpublish ac archive angen un.",
    "tour.s4.step.3":
      "Dewiswch Rhagolwg i weld yr iaith ddewisedig fel rhagolwg ochr-gweinydd; mae hysbysiad yn eich rhybuddio pan nad dyna mae darllenwyr yn ei weld.",
    "tour.s4.step.4":
      "Agorwch Llif gwaith yn y ddewislen i weld beth sy'n aros: cofnodion Dan adolygiad, ceisiadau cyfieithu agored ac eitemau Wedi'u trefnu.",
    "tour.s5.title": "Cadw cyfieithiadau'n gyfredol",
    "tour.s5.summary":
      "Gwelwch pa ieithoedd sydd wedi syrthio y tu ôl i'r ffynhonnell cyn i ddarllenwyr sylwi.",
    "tour.s5.step.1":
      "Agorwch Cyfieithiadau, dewiswch safle, a darllenwch y tabl Ieithoedd: ar gyfer pob iaith, ei chofnodion a faint sy'n Drafft ac wedi'u Cyhoeddi.",
    "tour.s5.step.2":
      "Darllenwch y rheol sydd wedi'i hargraffu ar y dudalen, i wybod yn union beth sy'n cyfrif fel y tu ôl i'r ffynhonnell, a'r amser Fel ar y pryd y cafodd ei gyfrifo.",
    "tour.s5.step.3":
      "O dan Ceisiadau agored, dewch o hyd i bob Allwedd cofnod gyda'i Iaith, ei Statws, ei Awdur a'i amser Diweddarwyd diwethaf.",
    "tour.s5.step.4":
      "I drwsio un, agorwch y cofnod o Gofnodion, dewiswch yr iaith yn ei dabl Ieithoedd, a chadwch ddiwygiad newydd yn yr iaith honno.",
    "tour.s6.title": "Gwirio asedau a thestun amgen",
    "tour.s6.summary":
      "Gwelwch bob delwedd a ffeil mewn safle a dalwch destun amgen coll cyn iddo rwystro cyhoeddi.",
    "tour.s6.step.1":
      "Agorwch Asedau a darllenwch y crynodeb: Storfa a ddefnyddiwyd, delweddau Heb destun amgen, a ffeiliau Heb eu cyfeirio gan ddim.",
    "tour.s6.step.2":
      "Yn y tabl, gwiriwch Teitl, Math, Maint a thestun amgen pob ased; mae delwedd heb destun amgen yn dangos marc Heb destun amgen.",
    "tour.s6.step.3":
      "Cofiwch y rheol: mae delwedd heb destun amgen yn atal cyhoeddi'r dudalen, felly trwsiwch hi cyn symud cofnod i'w gyhoeddi.",
    "tour.s6.step.4":
      "Mae ffeiliau a restrir fel Heb eu cyfeirio gan ddim yn cael eu hadrodd i'ch sylw, ond ni chânt byth eu dileu'n awtomatig.",
    "tour.intro":
      "Taith dywys drwy'r system rheoli cynnwys: sut i ddod o hyd i gofnod, golygu ei flociau cynnwys, adfer fersiynau hŷn, cyhoeddi drwy'r llif gwaith, cadw cyfieithiadau'n gyfredol a rheoli asedau.",
    "signin.sso": "Mewngofnodi gydag SSO",
  },
  "de-de": {
    "brand.name": "Main X · CMS",
    "nav.dashboard": "Übersicht",
    "nav.entries": "Einträge",
    "nav.assets": "Medien",
    "nav.workflow": "Arbeitsablauf",
    "nav.translations": "Übersetzungen",
    "nav.insights": "Auswertungen",
    "nav.settings": "Einstellungen",
    "nav.signin": "Anmelden",
    "nav.signout": "Abmelden",
    "chrome.language": "Sprache",
    "nav.share": "Teilen",
    "nav.text_size": "Textgröße",
    "share.copy_link": "Link kopieren",
    "share.copied": "Link kopiert",
    "share.copy_failed":
      "Kopieren fehlgeschlagen — bitte aus der Adressleiste kopieren",
    "chrome.theme": "Design",
    "common.loading": "Wird geladen…",
    "common.error": "Laden fehlgeschlagen",
    "common.retry": "Erneut versuchen",
    "common.status": "Status",
    "common.locale": "Gebietsschema",
    "common.title": "Titel",
    "common.actions": "Aktionen",
    "common.updated": "Aktualisiert",
    "common.noData": "Noch keine Daten",
    "site.choose": "Website wählen",
    "entry.published": "Veröffentlicht",
    "entry.draft": "Entwurf",
    "entry.inReview": "In Prüfung",
    "entry.approved": "Freigegeben",
    "entry.archived": "Archiviert",
    "entry.liveRevision": "Veröffentlichte Fassung",
    "entry.draftAhead": "Der Entwurf ist weiter als die Live-Fassung",
    "insights.health": "Inhaltsqualität",
    "insights.throughput": "Durchsatz",
    "insights.backlog": "Rückstand",
    "insights.asOf": "Stand",
    "insights.findings": "Befunde",
    "insights.rule": "Regel",
    "insights.noFindings": "Nichts zu melden",
    "preview.heading": "Vorschau",
    "preview.notLive": "Das sehen Leser nicht",
    "preview.localeServed": "Ausgeliefertes Gebietsschema",
    "entry.key": "Schlüssel",
    "entry.type": "Typ",
    "entry.locales": "Sprachen",
    "entry.history": "Versionsverlauf",
    "entry.blocks": "Inhaltsblöcke",
    "entry.addBlock": "Block hinzufügen",
    "entry.save": "Fassung speichern",
    "entry.conflict": "Jemand anderes hat zuerst gespeichert",
    "entry.conflictHelp":
      "Ihr Entwurf beruhte auf einer älteren Fassung. Vergleichen Sie, bevor Sie überschreiben.",
    "entry.restore": "Wiederherstellen",
    "entry.restoreHelp":
      "Wiederherstellen schreibt eine neue Fassung; der Verlauf wird nie umgeschrieben.",
    "entry.diff": "Vergleichen",
    "entry.identical": "Keine Unterschiede",
    "workflow.action": "Aktion",
    "workflow.reason": "Grund",
    "workflow.reasonRequired": "Diese Aktion braucht einen Grund",
    "workflow.blockers": "Noch nicht veröffentlichbar",
    "workflow.remedy": "Was zu tun ist",
    "workflow.ready": "Bereit zur Veröffentlichung",
    "workflow.scheduled": "Geplant",
    "assets.altMissing": "Kein Alternativtext",
    "assets.altGate":
      "Ein Bild ohne Alternativtext verhindert die Veröffentlichung",
    "assets.orphans": "Von nichts referenziert",
    "assets.orphansNote": "Gemeldet, nie gelöscht",
    "assets.storage": "Belegter Speicher",
    "translations.queue": "Offene Anfragen",
    "translations.stale": "Hinter der Quelle",
    "translations.source": "Quellsprache",
    "settings.templates": "Vorlagen",
    "settings.menus": "Menüs",
    "settings.redirects": "Weiterleitungen",
    "settings.webhooks": "Webhooks",
    "settings.contentTypes": "Inhaltstypen",
    "preview.open": "Vorschau",
    "preview.serverSide": "Der Vorschaulink bleibt auf dem Server",
    "common.cancel": "Abbrechen",
    "common.remove": "Entfernen",
    "common.moveUp": "Nach oben",
    "common.moveDown": "Nach unten",
    "common.saved": "Gespeichert",
    "common.author": "Autor",
    "common.size": "Größe",
    "common.path": "Pfad",
    "auth.signin": "Anmelden",
    "auth.signout": "Abmelden",
    "share.email": "Link per E-Mail senden",
    "share.linkedin": "Auf LinkedIn teilen",
    "share.reddit": "Auf Reddit teilen",
    "share.bluesky": "Auf Bluesky teilen",
    "share.mastodon": "Auf Mastodon teilen",
    "splash.hero.secondary": "Was Sie erwartet",
    "splash.benefits.title": "Warum Teams sich dafür entscheiden",
    "splash.features.title": "Was Sie tun können",
    "splash.trust.title": "Für Vertrauen gebaut",
    "splash.trust.1.title": "Anmeldung ohne Passwort",
    "splash.trust.1.body":
      "Ein Magic-Link per E-Mail: kein Passwort, das verloren gehen oder mehrfach verwendet werden könnte.",
    "splash.trust.2.title": "Attributbasierte Berechtigungen",
    "splash.trust.2.body":
      "Fein abgestufte Regeln legen fest, wer lesen, schreiben, zusammenführen oder löschen darf.",
    "splash.trust.3.title": "Manipulationssicheres Audit-Protokoll",
    "splash.trust.3.body":
      "Ein nur ergänzbarer Verlauf hält jede Änderung fest und wer sie vorgenommen hat.",
    "splash.trust.4.title": "Datenschutzkontrollen",
    "splash.trust.4.body":
      "Sensible Angaben sind maskiert, sofern Sie sie nicht einsehen dürfen.",
    "splash.trust.5.title": "Offene Standards",
    "splash.trust.5.body":
      "REST mit OpenAPI und HL7 FHIR, wo Gesundheitssysteme es benötigen.",
    "splash.trust.6.title": "Spricht Ihre Sprache",
    "splash.trust.6.body":
      "Arabisch, Chinesisch, Deutsch, Englisch, Französisch, Hindi, Spanisch und Walisisch.",
    "splash.cta.title": "Bereit für den Einstieg?",
    "splash.cta.body":
      "Melden Sie sich mit einem Magic-Link an, der an Ihre E-Mail-Adresse gesendet wird. Kein Passwort nötig.",
    "splash.benefits.1.title": "Wissen, was live ist",
    "nav.tour": "Rundgang",
    "splash.hero.tour": "Rundgang starten",
    "tour.head": "Machen Sie den Rundgang",
    "tour.toc": "Auf dieser Seite",
    "tour.open": "Diese Ansicht öffnen",
    "tour.top": "Nach oben",
    "tour.start.title": "Bevor Sie beginnen",
    "tour.start.summary":
      "Für die Arbeit mit echten Daten benötigen Sie ein Konto. Die Anmeldung dauert weniger als eine Minute und kommt ohne Passwort aus.",
    "tour.start.step.1":
      "Wählen Sie oben rechts Anmelden und geben Sie Ihre E-Mail-Adresse ein.",
    "tour.start.step.2":
      "Öffnen Sie den Magic-Link, den wir Ihnen per E-Mail senden. Er funktioniert nur einmal und läuft schnell ab.",
    "tour.start.step.3":
      "Sie kehren angemeldet in die Anwendung zurück, ohne etwas merken oder zurücksetzen zu müssen.",
    "tour.start.step.4":
      "Mit den Schaltflächen neben Anmelden ändern Sie Thema, Sprache und Textgröße oder teilen die Seite.",
    "signin.sso": "Mit SSO anmelden",
    "splash.hero.title": "Mit Sicherheit veröffentlichen, in jeder Sprache",
    "splash.hero.subtitle":
      "Verfassen Sie strukturierte Inhalte, prüfen Sie sie und veröffentlichen Sie sie, mit integriertem Versionsverlauf, Lokalisierung und Inhaltsqualitätsprüfungen.",
    "splash.benefits.1.body":
      "Eine deutliche Markierung zeigt, wenn ein Entwurf dem voraus ist, was Leser derzeit sehen.",
    "splash.benefits.2.title": "Keine verlorenen Änderungen",
    "splash.benefits.2.body":
      "Hat jemand anderes zuerst gespeichert, werden Sie informiert und können vergleichen, bevor Sie etwas überschreiben.",
    "splash.benefits.3.title": "Rückgängig machen ohne Angst",
    "splash.benefits.3.body":
      "Das Wiederherstellen einer alten Fassung schreibt eine neue, der Verlauf wird also nie umgeschrieben.",
    "splash.benefits.4.title": "Klare Veröffentlichungsregeln",
    "splash.benefits.4.body":
      "Kann ein Eintrag noch nicht veröffentlicht werden, erfahren Sie genau, was ihn aufhält.",
    "splash.benefits.5.title": "Ehrliche Übersetzungen",
    "splash.benefits.5.body":
      "Sehen Sie, welche Übersetzungen hinter der Quelle zurückliegen, bevor es Leser bemerken.",
    "splash.benefits.6.title": "Probleme früh erkannt",
    "splash.benefits.6.body":
      "Befunde zur Inhaltsqualität nennen die verletzte Regel, sodass jeder leicht zu beheben ist.",
    "splash.features.1.title": "Blockbasierte Inhalte",
    "splash.features.1.body":
      "Setzen Sie Einträge aus strukturierten Inhaltsblöcken zusammen statt aus gespeichertem HTML.",
    "splash.features.2.title": "Versionsverlauf",
    "splash.features.2.body":
      "Jedes Speichern ist eine Fassung, die Sie mit einer anderen vergleichen und wiederherstellen können.",
    "splash.features.3.title": "Redaktioneller Arbeitsablauf",
    "splash.features.3.body":
      "Führen Sie Einträge vom Entwurf über Prüfung und Freigabe bis zur Veröffentlichung, mit festgehaltenem Grund für jeden Schritt.",
    "splash.features.4.title": "Lokalisierung",
    "splash.features.4.body":
      "Halten Sie eine Variante pro Sprache vor, mit Rückfallketten für alles, was noch nicht übersetzt ist.",
    "splash.features.5.title": "Medienbibliothek",
    "splash.features.5.body":
      "Verwalten Sie Bilder und Dateien neben den Einträgen, die sie verwenden.",
    "splash.features.6.title": "Vorschau und Auswertungen",
    "splash.features.6.body":
      "Sehen Sie eine Sprache vor der Veröffentlichung in der Vorschau und verfolgen Sie Inhaltsqualität und Rückstand auf dem Dashboard.",
    "nav.toggle": "Navigation umschalten",
    "tour.intro":
      "Ein geführter Rundgang durch das CMS: wie Sie einen Eintrag finden, seine Inhaltsblöcke bearbeiten, alte Fassungen wiederherstellen, über den Arbeitsablauf veröffentlichen, Übersetzungen aktuell halten und Medien verwalten.",
    "tour.s1.title": "Einen Eintrag finden und öffnen",
    "tour.s1.summary":
      "Finden Sie einen beliebigen Eintrag einer Website und öffnen Sie ihn zum Bearbeiten.",
    "tour.s1.step.1":
      "Öffnen Sie „Einträge“ und wählen Sie unter „Website wählen“ die Website, an der Sie arbeiten.",
    "tour.s1.step.2":
      "Grenzen Sie die Liste mit der Auswahlliste „Typ“ ein oder tippen Sie einen Namensteil in das Feld „Schlüssel“.",
    "tour.s1.step.3":
      "Lesen Sie die Spalten „Schlüssel“, „Typ“, „Quellsprache“ und „Status“; archivierte Einträge tragen die Markierung „Archiviert“.",
    "tour.s1.step.4":
      "Wählen Sie den Schlüssel eines Eintrags, um seine Seite zu öffnen, auf der Sie ihn bearbeiten, prüfen und veröffentlichen.",
    "tour.s2.title": "Inhaltsblöcke bearbeiten und eine Fassung speichern",
    "tour.s2.summary":
      "Einträge bestehen aus strukturierten Blöcken statt aus gespeichertem HTML, und jedes Speichern wird zu einer Fassung.",
    "tour.s2.step.1":
      "Wählen Sie auf der Eintragsseite eine Sprache in der Tabelle „Sprachen“, die Status, Live-Fassung und Rückstand gegenüber der Quelle jeder Sprache zeigt.",
    "tour.s2.step.2":
      "Bearbeiten Sie unter „Inhaltsblöcke“ die Blöcke und fügen Sie mit „Block hinzufügen“ eine Überschrift, einen Absatz, eine Liste, ein Zitat, ein Bild oder einen Codeblock ein; „Nach oben“, „Nach unten“ und „Entfernen“ ordnen um oder löschen.",
    "tour.s2.step.3":
      "Ändern Sie bei Bedarf den Titel und wählen Sie dann „Fassung speichern“.",
    "tour.s2.step.4":
      "Ist der Entwurf der Live-Fassung voraus, sagt die Seite es, sodass Sie stets wissen, dass Leser noch die ältere Fassung sehen.",
    "tour.s3.title": "Fassungen vergleichen und wiederherstellen",
    "tour.s3.summary":
      "Der Verlauf wird nie umgeschrieben, sodass Sie zurückblicken und sicher rückgängig machen können.",
    "tour.s3.step.1":
      "Scrollen Sie zum „Versionsverlauf“, der jede Fassung mit Titel, Autor und Datum auflistet; die Live-Fassung ist als „Veröffentlicht“ markiert.",
    "tour.s3.step.2":
      "Wählen Sie „Vergleichen“ bei einer Fassung, um zu sehen, wie sie sich von der bearbeiteten unterscheidet; identische Fassungen sagen das.",
    "tour.s3.step.3":
      "Wählen Sie „Wiederherstellen“ bei einer älteren Fassung; das schreibt eine neue Fassung und lässt jede frühere unangetastet.",
    "tour.s3.step.4":
      "Hat jemand anderes zuerst gespeichert, sagt die Seite es und zeigt die maßgebliche Fassung, sodass Sie vergleichen können, bevor Sie etwas überschreiben.",
    "tour.s4.title": "Prüfen und veröffentlichen",
    "tour.s4.summary":
      "Führen Sie einen Eintrag durch den redaktionellen Arbeitsablauf, mit ausdrücklich genannten Hindernissen und Gründen.",
    "tour.s4.step.1":
      "Prüfen Sie auf der Eintragsseite den Veröffentlichungsbereich: „Bereit zur Veröffentlichung“ oder „Noch nicht veröffentlichbar“ mit einer Tabelle aus Regel, Titel und „Was zu tun ist“.",
    "tour.s4.step.2":
      "Wählen Sie im Bereich „Arbeitsablauf“ eine „Aktion“ (einreichen, freigeben, ablehnen, veröffentlichen, zurückziehen oder archivieren) und geben Sie einen „Grund“ ein; Ablehnen, Zurückziehen und Archivieren verlangen einen.",
    "tour.s4.step.3":
      "Wählen Sie „Vorschau“, um die gewählte Sprache als serverseitige Vorschau zu sehen; ein Hinweis warnt, wenn sie nicht dem entspricht, was Leser sehen.",
    "tour.s4.step.4":
      "Öffnen Sie im Menü „Arbeitsablauf“, um zu sehen, was wartet: Einträge „In Prüfung“, offene Übersetzungsanfragen und Elemente unter „Geplant“.",
    "tour.s5.title": "Übersetzungen aktuell halten",
    "tour.s5.summary":
      "Sehen Sie, welche Sprachen hinter der Quelle zurückliegen, bevor es Leser bemerken.",
    "tour.s5.step.1":
      "Öffnen Sie „Übersetzungen“, wählen Sie eine Website und lesen Sie die Tabelle „Sprachen“: je Sprache ihre Einträge und wie viele davon „Entwurf“ und „Veröffentlicht“ sind.",
    "tour.s5.step.2":
      "Lesen Sie die auf der Seite genannte Regel, damit Sie genau wissen, was als hinter der Quelle zurückliegend gilt, und den „Stand“, zu dem sie ermittelt wurde.",
    "tour.s5.step.3":
      "Suchen Sie unter „Offene Anfragen“ den Schlüssel jedes Eintrags mit Sprache, Status, Autor und Zeitpunkt der letzten Aktualisierung.",
    "tour.s5.step.4":
      "Zum Beheben öffnen Sie den Eintrag unter „Einträge“, wählen die Sprache in seiner Tabelle „Sprachen“ und speichern eine neue Fassung in dieser Sprache.",
    "tour.s6.title": "Medien und Alternativtexte prüfen",
    "tour.s6.summary":
      "Sehen Sie jedes Bild und jede Datei einer Website und erkennen Sie fehlende Alternativtexte, bevor sie die Veröffentlichung blockieren.",
    "tour.s6.step.1":
      "Öffnen Sie „Medien“ und lesen Sie die Zusammenfassung: „Belegter Speicher“, Bilder mit „Kein Alternativtext“ und Dateien „Von nichts referenziert“.",
    "tour.s6.step.2":
      "Prüfen Sie in der Tabelle Titel, Typ, Größe und Alternativtext jedes Mediums; ein Bild ohne Alternativtext trägt die Markierung „Kein Alternativtext“.",
    "tour.s6.step.3":
      "Merken Sie sich die Regel: Ein Bild ohne Alternativtext verhindert die Veröffentlichung der Seite, beheben Sie es also, bevor Sie einen Eintrag zur Veröffentlichung führen.",
    "tour.s6.step.4":
      "Als „Von nichts referenziert“ aufgeführte Dateien werden zu Ihrer Beachtung gemeldet, aber nie automatisch gelöscht.",
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
      "Arabic, Chinese, English, French, German, Hindi, Spanish and Welsh.",
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
    "nav.tour": "Tour",
    "splash.hero.tour": "Take the tour",
    "tour.head": "Take the tour",
    "tour.toc": "On this page",
    "tour.open": "Open this screen",
    "tour.top": "Back to top",
    "tour.start.title": "Before you begin",
    "tour.start.summary":
      "You need an account to work with real data. Signing in takes under a minute and needs no password.",
    "tour.start.step.1":
      "Choose Sign in at the top right and enter your email address.",
    "tour.start.step.2":
      "Open the magic link we email you. It works once and expires quickly.",
    "tour.start.step.3":
      "You return to the app signed in, with nothing to remember or reset.",
    "tour.start.step.4":
      "Use the buttons beside Sign in to change the theme, language and text size, or to share the page.",
    "tour.s1.title": "Find and open an entry",
    "tour.s1.summary": "Locate any entry in a site and open it for editing.",
    "tour.s1.step.1":
      "Open Entries and, under Choose a site, pick the site you work on.",
    "tour.s1.step.2":
      "Narrow the list with the Type drop-down, or type part of a name in the Key box.",
    "tour.s1.step.3":
      "Read the Key, Type, Source locale and Status columns; archived entries carry an Archived marker.",
    "tour.s1.step.4":
      "Select an entry's key to open its page, where you edit, review and publish it.",
    "tour.s2.title": "Edit content blocks and save a revision",
    "tour.s2.summary":
      "Entries are built from structured blocks rather than stored HTML, and every save becomes a revision.",
    "tour.s2.step.1":
      "On the entry page, pick a locale in the Locales table, which shows each locale's status, live revision and whether it is behind the source.",
    "tour.s2.step.2":
      "Under Content blocks, edit the blocks and use Add block to insert a heading, paragraph, list, quote, image or code block; Move up, Move down and Remove reorder or delete.",
    "tour.s2.step.3": "Change the Title if needed, then choose Save revision.",
    "tour.s2.step.4":
      "When the draft is ahead of what is live, the page says so, so you always know readers still see the older version.",
    "tour.s3.title": "Compare and restore revisions",
    "tour.s3.summary":
      "History is never rewritten, so you can look back and undo safely.",
    "tour.s3.step.1":
      "Scroll to Revision history, which lists each revision with its title, author and date; the live one is marked Published.",
    "tour.s3.step.2":
      "Choose Compare beside a revision to see how it differs from the one you are editing; identical revisions say so.",
    "tour.s3.step.3":
      "Choose Restore on an older revision; this writes a new revision and leaves every earlier one intact.",
    "tour.s3.step.4":
      "If someone else saved first, the page says so and shows the winning revision, so you can compare before overwriting anything.",
    "tour.s4.title": "Review and publish",
    "tour.s4.summary":
      "Move an entry through the editorial workflow, with blockers and reasons made explicit.",
    "tour.s4.step.1":
      "On the entry page, check the publish panel: Ready to publish, or Cannot publish yet with a table of the Rule, Title and What to do.",
    "tour.s4.step.2":
      "In the Workflow panel, choose an Action (submit, approve, reject, publish, unpublish or archive) and type a Reason; reject, unpublish and archive require one.",
    "tour.s4.step.3":
      "Choose Preview to see the selected locale as a server-side preview; a notice warns when it is not what readers see.",
    "tour.s4.step.4":
      "Open Workflow in the menu to see what is waiting: entries In review, open translation requests, and Scheduled items.",
    "tour.s5.title": "Keep translations current",
    "tour.s5.summary":
      "See which locales have fallen behind the source before readers notice.",
    "tour.s5.step.1":
      "Open Translations, pick a site, and read the Locales table: for each locale, its entries and how many are Draft and Published.",
    "tour.s5.step.2":
      "Read the rule printed on the page, so you know exactly what counts as behind the source, and the As of time it was worked out.",
    "tour.s5.step.3":
      "Under Open requests, find each entry Key with its Locale, Status, Author and last Updated time.",
    "tour.s5.step.4":
      "To fix one, open the entry from Entries, select the locale in its Locales table, and save a new revision in that language.",
    "tour.s6.title": "Check assets and alt text",
    "tour.s6.summary":
      "See every image and file in a site and catch missing alt text before it blocks publishing.",
    "tour.s6.step.1":
      "Open Assets and read the summary: Storage used, images with No alt text, and files Referenced by nothing.",
    "tour.s6.step.2":
      "In the table, check each asset's Title, Type, Size and alt text; an image without alt text carries a No alt text marker.",
    "tour.s6.step.3":
      "Remember the rule: an image without alt text stops the page publishing, so fix it before you move an entry to publish.",
    "tour.s6.step.4":
      "Files listed as Referenced by nothing are reported for your attention, but they are never deleted automatically.",
    "tour.intro":
      "A guided walkthrough of the CMS: how to find an entry, edit its content blocks, restore old revisions, publish through the workflow, keep translations current and manage assets.",
    "signin.sso": "Sign in with SSO",
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
      "Alemán, árabe, chino, español, francés, galés, hindi e inglés.",
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
    "nav.tour": "Recorrido",
    "splash.hero.tour": "Haz el recorrido",
    "tour.head": "Haz el recorrido",
    "tour.toc": "En esta página",
    "tour.open": "Abrir esta pantalla",
    "tour.top": "Volver arriba",
    "tour.start.title": "Antes de empezar",
    "tour.start.summary":
      "Necesitas una cuenta para trabajar con datos reales. Iniciar sesión lleva menos de un minuto y no requiere contraseña.",
    "tour.start.step.1":
      "Elige Iniciar sesión arriba a la derecha e introduce tu correo electrónico.",
    "tour.start.step.2":
      "Abre el enlace mágico que te enviamos por correo. Funciona una sola vez y caduca pronto.",
    "tour.start.step.3":
      "Vuelves a la aplicación con la sesión iniciada, sin nada que recordar ni restablecer.",
    "tour.start.step.4":
      "Usa los botones junto a Iniciar sesión para cambiar el tema, el idioma y el tamaño del texto, o para compartir la página.",
    "tour.s1.title": "Encontrar y abrir una entrada",
    "tour.s1.summary":
      "Localiza cualquier entrada de un sitio y ábrela para editarla.",
    "tour.s1.step.1":
      "Abre Entradas y, en Elegir un sitio, selecciona el sitio con el que trabajas.",
    "tour.s1.step.2":
      "Reduce la lista con el desplegable Tipo, o escribe parte de un nombre en el cuadro Clave.",
    "tour.s1.step.3":
      "Consulta las columnas Clave, Tipo, Idioma de origen y Estado; las entradas archivadas llevan la marca Archivada.",
    "tour.s1.step.4":
      "Selecciona la clave de una entrada para abrir su página, donde la editas, revisas y publicas.",
    "tour.s2.title": "Editar bloques de contenido y guardar una revisión",
    "tour.s2.summary":
      "Las entradas se componen de bloques estructurados, no de HTML almacenado, y cada guardado se convierte en una revisión.",
    "tour.s2.step.1":
      "En la página de la entrada, elige un idioma en la tabla Idiomas, que muestra el estado de cada uno, su revisión publicada y si va por detrás del origen.",
    "tour.s2.step.2":
      "En Bloques de contenido, edita los bloques y usa Añadir bloque para insertar un encabezado, párrafo, lista, cita, imagen o código; Subir, Bajar y Quitar reordenan o eliminan.",
    "tour.s2.step.3":
      "Cambia el Título si hace falta y elige Guardar revisión.",
    "tour.s2.step.4":
      "Cuando el borrador va por delante de lo publicado, la página lo indica, así sabes siempre que los lectores aún ven la versión anterior.",
    "tour.s3.title": "Comparar y restaurar revisiones",
    "tour.s3.summary":
      "El historial nunca se reescribe, así que puedes mirar atrás y deshacer con seguridad.",
    "tour.s3.step.1":
      "Baja hasta Historial de revisiones, que lista cada revisión con su título, autor y fecha; la publicada lleva la marca Publicada.",
    "tour.s3.step.2":
      "Elige Comparar junto a una revisión para ver en qué difiere de la que editas; si son idénticas, lo indica.",
    "tour.s3.step.3":
      "Elige Restaurar en una revisión anterior; se escribe una nueva revisión y todas las anteriores quedan intactas.",
    "tour.s3.step.4":
      "Si alguien guardó antes, la página lo indica y muestra la revisión ganadora, para que compares antes de sobrescribir nada.",
    "tour.s4.title": "Revisar y publicar",
    "tour.s4.summary":
      "Lleva una entrada por el flujo de trabajo editorial, con los bloqueos y los motivos a la vista.",
    "tour.s4.step.1":
      "En la página de la entrada, mira el panel de publicación: Listo para publicar, o No se puede publicar todavía, con una tabla de Regla, Título y Qué hacer.",
    "tour.s4.step.2":
      "En el panel Flujo de trabajo, elige una Acción (submit, approve, reject, publish, unpublish o archive) y escribe un Motivo; reject, unpublish y archive lo exigen.",
    "tour.s4.step.3":
      "Elige Vista previa para ver el idioma seleccionado como vista previa del servidor; un aviso indica cuándo no es lo que ven los lectores.",
    "tour.s4.step.4":
      "Abre Flujo de trabajo en el menú para ver lo que espera: entradas En revisión, solicitudes de traducción abiertas y elementos Programados.",
    "tour.s5.title": "Mantener al día las traducciones",
    "tour.s5.summary":
      "Descubre qué idiomas se han quedado por detrás del origen antes de que lo noten los lectores.",
    "tour.s5.step.1":
      "Abre Traducciones, elige un sitio y lee la tabla Idiomas: para cada idioma, sus entradas y cuántas están en Borrador y Publicadas.",
    "tour.s5.step.2":
      "Lee la regla impresa en la página para saber exactamente qué cuenta como ir por detrás del origen, y la hora Al momento en que se calculó.",
    "tour.s5.step.3":
      "En Solicitudes abiertas, encuentra cada Clave de entrada con su Idioma, Estado, Autor y hora de última Actualización.",
    "tour.s5.step.4":
      "Para corregir una, abre la entrada desde Entradas, selecciona el idioma en su tabla Idiomas y guarda una nueva revisión en ese idioma.",
    "tour.s6.title": "Revisar recursos y texto alternativo",
    "tour.s6.summary":
      "Consulta todas las imágenes y archivos de un sitio y detecta el texto alternativo que falta antes de que bloquee la publicación.",
    "tour.s6.step.1":
      "Abre Recursos y lee el resumen: Almacenamiento usado, imágenes Sin texto alternativo y archivos Sin referencias.",
    "tour.s6.step.2":
      "En la tabla, revisa el Título, Tipo, Tamaño y texto alternativo de cada recurso; una imagen sin texto alternativo lleva la marca Sin texto alternativo.",
    "tour.s6.step.3":
      "Recuerda la regla: una imagen sin texto alternativo impide publicar la página, así que corrígela antes de pasar una entrada a publicar.",
    "tour.s6.step.4":
      "Los archivos indicados como Sin referencias se comunican para tu atención, pero nunca se eliminan automáticamente.",
    "tour.intro":
      "Un recorrido guiado por el CMS: cómo encontrar una entrada, editar sus bloques de contenido, restaurar revisiones antiguas, publicar mediante el flujo de trabajo, mantener al día las traducciones y gestionar los recursos.",
    "signin.sso": "Iniciar sesión con SSO",
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
      "Allemand, anglais, arabe, chinois, espagnol, français, gallois et hindi.",
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
    "nav.tour": "Visite guidée",
    "splash.hero.tour": "Faire la visite guidée",
    "tour.head": "Faire la visite guidée",
    "tour.toc": "Sur cette page",
    "tour.open": "Ouvrir cet écran",
    "tour.top": "Retour en haut",
    "tour.start.title": "Avant de commencer",
    "tour.start.summary":
      "Un compte est nécessaire pour travailler avec des données réelles. La connexion prend moins d'une minute et n'exige aucun mot de passe.",
    "tour.start.step.1":
      "Choisissez Se connecter en haut à droite et saisissez votre adresse e-mail.",
    "tour.start.step.2":
      "Ouvrez le lien magique reçu par e-mail. Il ne fonctionne qu'une fois et expire vite.",
    "tour.start.step.3":
      "Vous revenez dans l'application connecté, sans rien à retenir ni à réinitialiser.",
    "tour.start.step.4":
      "Utilisez les boutons à côté de Se connecter pour changer le thème, la langue et la taille du texte, ou pour partager la page.",
    "tour.s1.title": "Trouver et ouvrir une entrée",
    "tour.s1.summary":
      "Repérez n'importe quelle entrée d'un site et ouvrez-la pour la modifier.",
    "tour.s1.step.1":
      "Ouvrez Entrées puis, sous Choisir un site, sélectionnez le site sur lequel vous travaillez.",
    "tour.s1.step.2":
      "Réduisez la liste avec le menu Type, ou saisissez une partie du nom dans le champ Clé.",
    "tour.s1.step.3":
      "Lisez les colonnes Clé, Type, Langue source et Statut ; les entrées archivées portent la mention Archivée.",
    "tour.s1.step.4":
      "Sélectionnez la clé d'une entrée pour ouvrir sa page, où vous la modifiez, la relisez et la publiez.",
    "tour.s2.title":
      "Modifier des blocs de contenu et enregistrer une révision",
    "tour.s2.summary":
      "Les entrées sont composées de blocs structurés plutôt que de HTML stocké, et chaque enregistrement devient une révision.",
    "tour.s2.step.1":
      "Sur la page de l'entrée, choisissez une langue dans le tableau Langues, qui indique le statut de chacune, sa révision en ligne et si elle est en retard sur la source.",
    "tour.s2.step.2":
      "Sous Blocs de contenu, modifiez les blocs et utilisez Ajouter un bloc pour insérer un titre, un paragraphe, une liste, une citation, une image ou du code ; Monter, Descendre et Retirer réordonnent ou suppriment.",
    "tour.s2.step.3":
      "Modifiez le Titre si nécessaire, puis choisissez Enregistrer la révision.",
    "tour.s2.step.4":
      "Lorsque le brouillon est en avance sur ce qui est en ligne, la page l'indique : vous savez toujours que les lecteurs voient encore l'ancienne version.",
    "tour.s3.title": "Comparer et restaurer des révisions",
    "tour.s3.summary":
      "L'historique n'est jamais réécrit : vous pouvez revenir en arrière et annuler sans risque.",
    "tour.s3.step.1":
      "Faites défiler jusqu'à Historique des révisions, qui liste chaque révision avec son titre, son auteur et sa date ; celle en ligne porte la mention Publiée.",
    "tour.s3.step.2":
      "Choisissez Comparer à côté d'une révision pour voir en quoi elle diffère de celle que vous modifiez ; des révisions identiques sont signalées.",
    "tour.s3.step.3":
      "Choisissez Restaurer sur une révision plus ancienne ; cela écrit une nouvelle révision et laisse intactes toutes les précédentes.",
    "tour.s3.step.4":
      "Si quelqu'un a enregistré avant vous, la page l'indique et affiche la révision gagnante, pour comparer avant d'écraser quoi que ce soit.",
    "tour.s4.title": "Relire et publier",
    "tour.s4.summary":
      "Faites avancer une entrée dans le flux éditorial, avec les blocages et les motifs rendus explicites.",
    "tour.s4.step.1":
      "Sur la page de l'entrée, consultez le panneau de publication : Prêt à publier, ou Publication impossible pour l'instant avec un tableau Règle, Titre et Que faire.",
    "tour.s4.step.2":
      "Dans le panneau Flux de travail, choisissez une Action (submit, approve, reject, publish, unpublish ou archive) et saisissez un Motif ; reject, unpublish et archive en exigent un.",
    "tour.s4.step.3":
      "Choisissez Aperçu pour voir la langue sélectionnée en aperçu côté serveur ; un avis signale quand ce n'est pas ce que voient les lecteurs.",
    "tour.s4.step.4":
      "Ouvrez Flux de travail dans le menu pour voir ce qui attend : entrées En révision, demandes de traduction ouvertes et éléments Planifiés.",
    "tour.s5.title": "Garder les traductions à jour",
    "tour.s5.summary":
      "Repérez les langues en retard sur la source avant que les lecteurs ne le remarquent.",
    "tour.s5.step.1":
      "Ouvrez Traductions, choisissez un site et lisez le tableau Langues : pour chaque langue, ses entrées et combien sont en Brouillon et Publiées.",
    "tour.s5.step.2":
      "Lisez la règle affichée sur la page pour savoir exactement ce qui compte comme un retard sur la source, ainsi que l'heure À la date de son calcul.",
    "tour.s5.step.3":
      "Sous Demandes ouvertes, retrouvez chaque Clé d'entrée avec sa Langue, son Statut, son Auteur et l'heure de dernière Mise à jour.",
    "tour.s5.step.4":
      "Pour en corriger une, ouvrez l'entrée depuis Entrées, sélectionnez la langue dans son tableau Langues et enregistrez une nouvelle révision dans cette langue.",
    "tour.s6.title": "Vérifier les ressources et le texte alternatif",
    "tour.s6.summary":
      "Consultez toutes les images et tous les fichiers d'un site et repérez le texte alternatif manquant avant qu'il ne bloque la publication.",
    "tour.s6.step.1":
      "Ouvrez Ressources et lisez le résumé : Stockage utilisé, images Sans texte alternatif et fichiers Référencés par rien.",
    "tour.s6.step.2":
      "Dans le tableau, vérifiez le Titre, le Type, la Taille et le texte alternatif de chaque ressource ; une image sans texte alternatif porte la mention Sans texte alternatif.",
    "tour.s6.step.3":
      "Retenez la règle : une image sans texte alternatif empêche la publication de la page ; corrigez-la avant de publier une entrée.",
    "tour.s6.step.4":
      "Les fichiers signalés comme Référencés par rien sont portés à votre attention, mais ne sont jamais supprimés automatiquement.",
    "tour.intro":
      "Une visite guidée du CMS : comment trouver une entrée, modifier ses blocs de contenu, restaurer d'anciennes révisions, publier via le flux de travail, garder les traductions à jour et gérer les ressources.",
    "signin.sso": "Se connecter avec SSO",
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
      "अरबी, चीनी, जर्मन, अंग्रेज़ी, फ़्रेंच, हिन्दी, स्पेनिश और वेल्श।",
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
    "nav.tour": "टूर",
    "splash.hero.tour": "टूर देखें",
    "tour.head": "टूर देखें",
    "tour.toc": "इस पृष्ठ पर",
    "tour.open": "यह स्क्रीन खोलें",
    "tour.top": "ऊपर लौटें",
    "tour.start.title": "शुरू करने से पहले",
    "tour.start.summary":
      "वास्तविक डेटा के साथ काम करने के लिए खाता चाहिए। साइन इन में एक मिनट से कम लगता है और पासवर्ड की ज़रूरत नहीं।",
    "tour.start.step.1":
      "ऊपर दाईं ओर साइन इन चुनें और अपना ईमेल पता दर्ज करें।",
    "tour.start.step.2":
      "हमारे भेजे मैजिक लिंक को खोलें। यह एक ही बार काम करता है और जल्दी समाप्त हो जाता है।",
    "tour.start.step.3":
      "आप साइन इन होकर ऐप में लौटते हैं, न कुछ याद रखना, न रीसेट करना।",
    "tour.start.step.4":
      "थीम, भाषा और टेक्स्ट का आकार बदलने या पृष्ठ साझा करने के लिए साइन इन के पास के बटन इस्तेमाल करें।",
    "tour.s1.title": "प्रविष्टि खोजें और खोलें",
    "tour.s1.summary":
      "किसी साइट की कोई भी प्रविष्टि खोजें और संपादन के लिए खोलें।",
    "tour.s1.step.1":
      "प्रविष्टियाँ खोलें और «साइट चुनें» में वह साइट चुनें जिस पर आप काम करते हैं।",
    "tour.s1.step.2":
      "प्रकार ड्रॉप-डाउन से सूची सीमित करें, या «कुंजी» बॉक्स में नाम का कोई हिस्सा लिखें।",
    "tour.s1.step.3":
      "कुंजी, प्रकार, स्रोत भाषा और स्थिति कॉलम पढ़ें; संग्रहीत प्रविष्टियों पर «संग्रहीत» चिह्न होता है।",
    "tour.s1.step.4":
      "किसी प्रविष्टि की कुंजी चुनकर उसका पेज खोलें, जहाँ आप उसे संपादित, समीक्षा और प्रकाशित करते हैं।",
    "tour.s2.title": "कंटेंट ब्लॉक संपादित करें और संस्करण सहेजें",
    "tour.s2.summary":
      "प्रविष्टियाँ संग्रहीत HTML के बजाय संरचित ब्लॉक से बनती हैं, और हर सहेजना एक संस्करण बन जाता है।",
    "tour.s2.step.1":
      "प्रविष्टि पेज पर भाषाएँ तालिका में एक भाषा चुनें, जो हर भाषा की स्थिति, लाइव संस्करण और स्रोत से पीछे होने की जानकारी दिखाती है।",
    "tour.s2.step.2":
      "«कंटेंट ब्लॉक» में ब्लॉक संपादित करें और «ब्लॉक जोड़ें» से शीर्षक, अनुच्छेद, सूची, उद्धरण, चित्र या कोड ब्लॉक डालें; ऊपर ले जाएँ, नीचे ले जाएँ और हटाएँ से क्रम बदलें या हटाएँ।",
    "tour.s2.step.3":
      "आवश्यकता हो तो शीर्षक बदलें, फिर «संस्करण सहेजें» चुनें।",
    "tour.s2.step.4":
      "जब ड्राफ़्ट लाइव संस्करण से आगे होता है, पेज यह बताता है, ताकि आपको पता रहे कि पाठक अब भी पुराना संस्करण देख रहे हैं।",
    "tour.s3.title": "संस्करणों की तुलना और बहाली",
    "tour.s3.summary":
      "इतिहास कभी दोबारा नहीं लिखा जाता, इसलिए आप पीछे देख सकते हैं और सुरक्षित रूप से पूर्ववत कर सकते हैं।",
    "tour.s3.step.1":
      "«संस्करण इतिहास» तक स्क्रॉल करें, जो हर संस्करण को शीर्षक, लेखक और तारीख के साथ सूचीबद्ध करता है; लाइव वाले पर «प्रकाशित» लिखा होता है।",
    "tour.s3.step.2":
      "किसी संस्करण के बगल में «तुलना करें» चुनें और देखें कि वह आपके संपादित संस्करण से कैसे अलग है; समान होने पर यह बताया जाता है।",
    "tour.s3.step.3":
      "किसी पुराने संस्करण पर «बहाल करें» चुनें; इससे नया संस्करण बनता है और पिछले सभी जस के तस रहते हैं।",
    "tour.s3.step.4":
      "अगर किसी और ने पहले सहेजा है, तो पेज बताता है और जीतने वाला संस्करण दिखाता है, ताकि कुछ भी अधिलेखित करने से पहले आप तुलना कर सकें।",
    "tour.s4.title": "समीक्षा और प्रकाशन",
    "tour.s4.summary":
      "किसी प्रविष्टि को संपादकीय वर्कफ़्लो से गुज़ारें, जहाँ बाधाएँ और कारण स्पष्ट दिखते हैं।",
    "tour.s4.step.1":
      "प्रविष्टि पेज पर प्रकाशन पैनल देखें: «प्रकाशन के लिए तैयार», या «अभी प्रकाशित नहीं हो सकता» और नियम, शीर्षक व क्या करें की तालिका।",
    "tour.s4.step.2":
      "वर्कफ़्लो पैनल में एक कार्रवाई (submit, approve, reject, publish, unpublish या archive) चुनें और कारण लिखें; reject, unpublish और archive के लिए कारण ज़रूरी है।",
    "tour.s4.step.3":
      "चुनी हुई भाषा को सर्वर-साइड पूर्वावलोकन में देखने के लिए «पूर्वावलोकन» चुनें; जब यह वह न हो जो पाठक देखते हैं, तो चेतावनी दिखती है।",
    "tour.s4.step.4":
      "मेनू में «वर्कफ़्लो» खोलें और देखें कि क्या प्रतीक्षा में है: «समीक्षा में» प्रविष्टियाँ, खुले अनुवाद अनुरोध और «निर्धारित» आइटम।",
    "tour.s5.title": "अनुवाद अद्यतन रखें",
    "tour.s5.summary":
      "पाठकों के ध्यान देने से पहले देखें कि कौन-सी भाषाएँ स्रोत से पीछे हो गई हैं।",
    "tour.s5.step.1":
      "«अनुवाद» खोलें, साइट चुनें और भाषाएँ तालिका पढ़ें: हर भाषा के लिए उसकी प्रविष्टियाँ और कितनी ड्राफ़्ट व प्रकाशित हैं।",
    "tour.s5.step.2":
      "पेज पर छपा नियम पढ़ें ताकि पता रहे कि स्रोत से पीछे होना ठीक-ठीक क्या माना जाता है, और वह «तक की स्थिति» समय जब इसकी गणना हुई।",
    "tour.s5.step.3":
      "«खुले अनुरोध» में हर प्रविष्टि की कुंजी उसकी भाषा, स्थिति, लेखक और अंतिम अद्यतन समय के साथ देखें।",
    "tour.s5.step.4":
      "किसी को ठीक करने के लिए «प्रविष्टियाँ» से प्रविष्टि खोलें, उसकी भाषाएँ तालिका में भाषा चुनें और उस भाषा में नया संस्करण सहेजें।",
    "tour.s6.title": "एसेट और वैकल्पिक टेक्स्ट जाँचें",
    "tour.s6.summary":
      "किसी साइट की हर छवि और फ़ाइल देखें और वैकल्पिक टेक्स्ट की कमी को प्रकाशन रोकने से पहले पकड़ें।",
    "tour.s6.step.1":
      "«एसेट» खोलें और सारांश पढ़ें: उपयोग किया गया स्टोरेज, बिना वैकल्पिक टेक्स्ट वाली छवियाँ, और वे फ़ाइलें जिन्हें कोई संदर्भित नहीं करता।",
    "tour.s6.step.2":
      "तालिका में हर एसेट का शीर्षक, प्रकार, आकार और वैकल्पिक टेक्स्ट देखें; बिना वैकल्पिक टेक्स्ट की छवि पर «कोई वैकल्पिक टेक्स्ट नहीं» चिह्न होता है।",
    "tour.s6.step.3":
      "नियम याद रखें: बिना वैकल्पिक टेक्स्ट की छवि पेज का प्रकाशन रोकती है, इसलिए प्रविष्टि प्रकाशित करने से पहले इसे ठीक करें।",
    "tour.s6.step.4":
      "«कोई संदर्भित नहीं करता» वाली फ़ाइलें आपके ध्यान के लिए बताई जाती हैं, पर कभी अपने-आप हटाई नहीं जातीं।",
    "tour.intro":
      "सीएमएस का निर्देशित परिचय: प्रविष्टि कैसे खोजें, उसके कंटेंट ब्लॉक कैसे संपादित करें, पुराने संस्करण कैसे बहाल करें, वर्कफ़्लो से कैसे प्रकाशित करें, अनुवाद कैसे अद्यतन रखें और एसेट कैसे प्रबंधित करें।",
    "signin.sso": "SSO से साइन इन करें",
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
      "阿拉伯语、中文、德语、英语、法语、印地语、西班牙语和威尔士语。",
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
    "nav.tour": "导览",
    "splash.hero.tour": "开始导览",
    "tour.head": "开始导览",
    "tour.toc": "本页内容",
    "tour.open": "打开此页面",
    "tour.top": "返回顶部",
    "tour.start.title": "开始之前",
    "tour.start.summary":
      "处理真实数据需要账号。登录不到一分钟，也不需要密码。",
    "tour.start.step.1": "点击右上角的“登录”，输入你的邮箱地址。",
    "tour.start.step.2":
      "打开我们发到邮箱的魔法链接。它只能使用一次，且很快过期。",
    "tour.start.step.3": "你会以已登录状态回到应用，无需记忆或重置任何内容。",
    "tour.start.step.4":
      "使用“登录”旁边的按钮切换主题、语言和文字大小，或分享此页面。",
    "tour.s1.title": "查找并打开条目",
    "tour.s1.summary": "在站点中找到任意条目并打开进行编辑。",
    "tour.s1.step.1": "打开“条目”，在“选择站点”中选择你要处理的站点。",
    "tour.s1.step.2":
      "用“类型”下拉框缩小列表，或在“键”输入框中输入名称的一部分。",
    "tour.s1.step.3":
      "查看“键”“类型”“源语言”和“状态”列；已归档的条目带有“已归档”标记。",
    "tour.s1.step.4": "选择条目的键即可打开其页面，在那里编辑、审阅并发布。",
    "tour.s2.title": "编辑内容块并保存修订",
    "tour.s2.summary":
      "条目由结构化内容块而非存储的 HTML 构成，每次保存都会生成一个修订。",
    "tour.s2.step.1":
      "在条目页面的“语言”表中选择一种语言，表中显示各语言的状态、线上修订以及是否落后于源语言。",
    "tour.s2.step.2":
      "在“内容块”下编辑各块，用“添加块”插入标题、段落、列表、引用、图片或代码块；用“上移”“下移”“移除”调整顺序或删除。",
    "tour.s2.step.3": "如有需要修改标题，然后选择“保存修订”。",
    "tour.s2.step.4":
      "当草稿领先于线上版本时，页面会提示，让你始终知道读者看到的仍是旧版本。",
    "tour.s3.title": "比较并恢复修订",
    "tour.s3.summary": "历史记录不会被改写，因此你可以放心回看并撤销。",
    "tour.s3.step.1":
      "滚动到“修订历史”，其中列出每个修订的标题、作者和日期；线上版本标有“已发布”。",
    "tour.s3.step.2":
      "在某个修订旁选择“比较”，查看它与你正在编辑的版本有何不同；完全相同时会有提示。",
    "tour.s3.step.3":
      "在较旧的修订上选择“恢复”；这会写入一个新修订，此前的所有修订保持不变。",
    "tour.s3.step.4":
      "如果别人先保存了，页面会提示并显示胜出的修订，方便你在覆盖前先比较。",
    "tour.s4.title": "审阅并发布",
    "tour.s4.summary": "让条目走完编辑工作流，阻碍与理由一目了然。",
    "tour.s4.step.1":
      "在条目页面查看发布面板：“可以发布”，或“暂时无法发布”并附有规则、标题和处理办法的表格。",
    "tour.s4.step.2":
      "在“工作流”面板选择一个操作（submit、approve、reject、publish、unpublish 或 archive）并填写理由；reject、unpublish 和 archive 必须填写理由。",
    "tour.s4.step.3":
      "选择“预览”以服务器端预览所选语言；当它不是读者所见内容时会有提示。",
    "tour.s4.step.4":
      "从菜单打开“工作流”，查看待处理事项：“审核中”的条目、未完成的翻译请求和“已排期”的项目。",
    "tour.s5.title": "保持译文最新",
    "tour.s5.summary": "在读者察觉之前，看清哪些语言已落后于源语言。",
    "tour.s5.step.1":
      "打开“翻译”，选择站点，查看“语言”表：每种语言的条目数，以及其中草稿和已发布的数量。",
    "tour.s5.step.2":
      "阅读页面上印出的规则，确切了解什么算“落后于源语言”，以及计算时的“截至”时间。",
    "tour.s5.step.3":
      "在“未完成的请求”下，找到每个条目的键，及其语言、状态、作者和最近更新时间。",
    "tour.s5.step.4":
      "要修复某一项，从“条目”打开该条目，在其“语言”表中选择该语言，并保存该语言的新修订。",
    "tour.s6.title": "检查资源与替代文本",
    "tour.s6.summary":
      "查看站点中的每张图片和文件，在缺失的替代文本阻止发布之前发现它。",
    "tour.s6.step.1":
      "打开“资源”，阅读摘要：已用存储、没有替代文本的图片，以及没有被引用的文件。",
    "tour.s6.step.2":
      "在表格中检查每个资源的标题、类型、大小和替代文本；没有替代文本的图片带有“无替代文本”标记。",
    "tour.s6.step.3":
      "牢记规则：没有替代文本的图片会阻止页面发布，请在发布条目之前修复。",
    "tour.s6.step.4":
      "列为“没有被引用”的文件会提醒你注意，但绝不会被自动删除。",
    "tour.intro":
      "内容管理系统的图文导览：如何查找条目、编辑内容块、恢复旧修订、通过工作流发布、保持译文最新，以及管理资源。",
    "signin.sso": "使用 SSO 登录",
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
