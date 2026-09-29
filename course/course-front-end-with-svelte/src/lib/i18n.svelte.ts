// Lightweight, dependency-free i18n for the Course SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is small and we keep the front-end dependency-light (drift
// across the family front-ends is accepted, see AGENTS.md).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). An unknown key/locale falls back to `en-001`, then to the
// key string itself. The chosen locale persists to localStorage and
// drives the UI strings, `<html lang>`, and `<html dir>` (right-to-left
// for `ar-001`).

import { browser } from "$app/environment";

/**
 * Locales for which the UI is translated, sorted alphabetically by code
 * (the LocalePicker shows them in this order). To add one, extend this
 * tuple AND add a matching entry to {@link LOCALE_LABELS} and `STRINGS`.
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

/**
 * Right-to-left locales. The layout mirrors `<html dir>` to `rtl` for
 * these (and `ltr` for everyone else) via {@link isRtl}.
 */
export const RTL_LOCALES = ["ar-001"] as const satisfies readonly Locale[];

/**
 * Whether `locale` is written right-to-left. Used by the layout to set
 * `<html dir>`; tolerant of region subtags via {@link normaliseLocale}.
 *
 * @param locale - A locale code (region subtag like `ar-EG` accepted).
 * @returns `true` for `ar-001`, otherwise `false`.
 */
export function isRtl(locale: string): boolean {
  const primary = normaliseLocale(locale);
  return (
    primary !== null && (RTL_LOCALES as readonly string[]).includes(primary)
  );
}

/**
 * localStorage key under which the chosen UI locale is persisted. The
 * chrome's `PickerBar` locale picker calls {@link i18n}'s `set` on
 * change (its own `applyDir={false}`, since this store already reflects
 * `lang`/`dir` onto `<html>` — see `+layout.svelte`); this key is what
 * {@link readStoredLocale} reads on boot, so a locale set by other means
 * (e.g. a query param a future route handles) still sticks across visits.
 */
export const LOCALE_KEY = "mxi.course.locale";

// Every translatable UI string, keyed by a stable dotted key. `en-001` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    "nav.calendar": "التقويم",
    "nav.board": "اللوحة",
    "brand.name": "دورة",
    "brand.tagline": "Main X Index",
    "nav.toggle": "تبديل التنقل",
    "nav.dashboard": "لوحة المعلومات",
    "nav.courses": "الدورات",
    "nav.newCourse": "دورة جديدة",
    "nav.matchCheck": "فحص التطابق",
    "nav.merge": "دمج",
    "chrome.theme": "السمة",
    "chrome.language": "اللغة",
    "chrome.share": "مشاركة",
    "chrome.textSize": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "dashboard.title": "لوحة المعلومات",
    "dashboard.servicePrefix": "الخدمة:",
    "dashboard.status.ok": "جيد",
    "dashboard.status.down": "متوقف",
    "dashboard.status.loading": "جارٍ التحميل",
    "dashboard.recentActivity": "النشاط الأخير",
    "dashboard.noRecent": "لا توجد إدخالات تدقيق حديثة.",
    "courses.title": "الدورات",
    "courses.new": "دورة جديدة",
    "courses.searchPlaceholder": "البحث بالاسم أو المعرف…",
    "courses.fuzzy": "تقريبي",
    "courses.loading": "جارٍ التحميل…",
    "courses.recordCount.one": "{n} سجل",
    "courses.recordCount.other": "{n} سجل",
    "detail.loading": "جارٍ التحميل…",
    "detail.edit": "تحرير",
    "detail.audit": "تدقيق",
    "detail.delete": "حذف",
    "detail.exportGdpr": "تصدير البيانات (GDPR)",
    "detail.exportingGdpr": "جارٍ التصدير…",
    "detail.confirmDelete":
      "حذف هذه الدورة بشكل مبدئي؟ لا يمكن التراجع عن ذلك عبر الواجهة.",
    "detail.identity": "الهوية",
    "detail.id": "المعرف",
    "detail.courseCode": "رمز الدورة",
    "detail.status": "الحالة",
    "detail.educationalLevel": "المستوى التعليمي",
    "detail.numberOfCredits": "عدد الساعات المعتمدة",
    "detail.timeRequired": "الوقت المطلوب",
    "detail.description": "الوصف",
    "detail.url": "الرابط",
    "detail.license": "الترخيص",
    "detail.free": "مجاني",
    "detail.yes": "نعم",
    "detail.no": "لا",
    "detail.empty": "—",
    "detail.identifiers": "المعرفات",
    "detail.teaches": "يُدرّس",
    "detail.keywords": "الكلمات المفتاحية",
    "detail.alternateNames": "الأسماء البديلة",
    "detail.sameAs": "مطابق لـ (روابط موثوقة)",
    "detail.instances": "النسخ",
    "detail.customPrefix": "مخصص:",
    "detail.noDate": "(بدون تاريخ)",
    "detail.capacity": "السعة",
    "edit.title": "تحرير الدورة",
    "edit.cancel": "إلغاء",
    "edit.loading": "جارٍ التحميل…",
    "edit.saveChanges": "حفظ التغييرات",
    "new.title": "دورة جديدة",
    "new.create": "إنشاء",
    "new.possibleDuplicates": "تكرارات محتملة",
    "new.duplicatesDetected":
      "تم اكتشاف تكرارات ({n}) — راجع أدناه قبل إعادة الإرسال.",
    "audit.title": "سجل التدقيق",
    "audit.backToCourse": "العودة إلى الدورة",
    "audit.loading": "جارٍ التحميل…",
    "audit.noEntries": "لا توجد إدخالات تدقيق.",
    "audit.by": "بواسطة",
    "audit.payload": "الحمولة",
    "match.title": "فحص التطابق",
    "match.name": "الاسم",
    "match.courseCode": "رمز الدورة",
    "match.providerId": "معرف المزود",
    "match.displayThreshold": "حد العرض",
    "match.thresholdHint": "مرشح من جانب العميل (0.0 – 1.0)",
    "match.educationalLevel": "المستوى التعليمي",
    "match.keywords": "الكلمات المفتاحية",
    "match.keywordsHint": "مفصولة بفواصل أو أسطر جديدة",
    "match.teaches": "يُدرّس (الكفاءات)",
    "match.teachesHint": "واحد لكل سطر",
    "match.sameAs": "روابط مطابق لـ",
    "match.sameAsHint": "واحد لكل سطر",
    "match.identifiers": "المعرفات",
    "match.matching": "جارٍ التطابق…",
    "match.findMatches": "البحث عن تطابقات",
    "merge.title": "دمج الدورات",
    "merge.mainId": "معرف الدورة الرئيسية",
    "merge.mainIdHint": "السجل الباقي",
    "merge.duplicateId": "معرف الدورة المكررة",
    "merge.duplicateIdHint": "سيتم حذفه بشكل مبدئي",
    "merge.reason": "السبب",
    "merge.reasonHint": "يُسجَّل في سجل تدقيق الدمج",
    "merge.reasonPlaceholder": "تكرار مؤكد",
    "merge.loadPreview": "تحميل المعاينة",
    "merge.merging": "جارٍ الدمج…",
    "merge.merge": "دمج",
    "merge.bothIdsRequired": "كلا المعرفين مطلوبان",
    "merge.mustDiffer": "يجب أن يختلف السجل الرئيسي عن المكرر",
    "merge.confirm":
      "دمج {dup}… في {main}…؟\nسيؤدي ذلك إلى حذف المكرر بشكل مبدئي.",
    "merge.preview": "معاينة",
    "merge.main": "رئيسي",
    "merge.duplicate": "مكرر",
    "merge.completed": "اكتمل الدمج",
    "merge.recordCreated": "تم إنشاء سجل الدمج {id} في {at}.",
    "merge.viewMerged": "عرض الدورة الرئيسية المدمجة",
    "form.name": "الاسم",
    "form.courseCode": "رمز الدورة",
    "form.courseCodeHint": "ضمن نطاق المزود (مثال CS101)",
    "form.status": "الحالة",
    "form.description": "الوصف",
    "form.url": "الرابط",
    "form.license": "الترخيص",
    "form.numberOfCredits": "عدد الساعات المعتمدة",
    "form.educationalLevel": "المستوى التعليمي",
    "form.typicalAgeRange": "النطاق العمري النموذجي",
    "form.typicalAgeRangeHint": "مثال 18-22",
    "form.timeRequired": "الوقت المطلوب",
    "form.timeRequiredHint": "ISO 8601 (مثال PT45H)",
    "form.alternateNames": "الأسماء البديلة",
    "form.alternateNamesHint": "واحد لكل سطر",
    "form.keywords": "الكلمات المفتاحية",
    "form.keywordsHint": "مفصولة بفواصل أو أسطر جديدة",
    "form.teaches": "يُدرّس (الكفاءات)",
    "form.teachesHint": "واحد لكل سطر",
    "form.availableLanguages": "اللغات المتاحة",
    "form.availableLanguagesHint": "رموز BCP-47 (مثال en fr de)",
    "form.sameAs": "روابط مطابق لـ",
    "form.sameAsHint": "ويكي بيانات، فهرس OER، إلخ — واحد لكل سطر",
    "form.identifiers": "المعرفات",
    "form.saving": "جارٍ الحفظ…",
    "form.save": "حفظ",
    "form.reset": "إعادة تعيين",
    "identifier.type": "النوع",
    "identifier.customOption": "مخصص…",
    "identifier.customLabel": "تسمية مخصصة",
    "identifier.value": "القيمة",
    "identifier.url": "الرابط",
    "identifier.remove": "إزالة",
    "identifier.add": "+ إضافة معرف",
    "search.placeholder": "بحث…",
    "search.submit": "بحث",
    "results.title": "نتائج التطابق",
    "results.count": "({n})",
    "results.none": "لا توجد نتائج.",
    "results.scoreBreakdown": "تفصيل النتيجة",
    "results.score.name": "الاسم",
    "results.score.courseCode": "رمز الدورة",
    "results.score.provider": "المزود",
    "results.score.level": "المستوى",
    "results.score.keywords": "الكلمات المفتاحية",
    "results.score.teaches": "يُدرّس",
    "results.score.deterministic": "حتمي (معرف / مزود+رمز / مطابق لـ)",
    "grid.id": "المعرف",
    "grid.name": "الاسم",
    "grid.courseCode": "رمز الدورة",
    "grid.level": "المستوى",
    "grid.status": "الحالة",
    "grid.primaryIdentifier": "المعرف الأساسي",
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
    "splash.hero.title": "سجل موثوق واحد لكل دورة",
    "splash.hero.subtitle":
      "سجّل الدورات مرة واحدة، واعثر عليها فورًا، وامنع التكرار قبل أن يبدأ، مع سجل تدقيق كامل مدمج.",
    "splash.benefits.1.title": "تكرار أقل",
    "splash.benefits.1.body":
      "تنبّه المطابقة إلى احتمال التكرار لحظة إنشاء الدورة.",
    "splash.benefits.2.title": "اعثر على أي دورة بسرعة",
    "splash.benefits.2.body":
      "يعثر البحث النصي والتقريبي على الدورات رغم الأخطاء الإملائية واختلاف الصيغ.",
    "splash.benefits.3.title": "قرارات واثقة",
    "splash.benefits.3.body":
      "تعرض كل مطابقة درجتها وتفصيلًا لكل حقل، ليرى المراجعون السبب بدقة.",
    "splash.benefits.4.title": "كتالوج واحد موثوق",
    "splash.benefits.4.body":
      "تُبقي رموز الدورات والمعرّفات سجلًا واحدًا فقط لكل دورة.",
    "splash.benefits.5.title": "لا شيء بلا تسجيل",
    "splash.benefits.5.body":
      "يُدقَّق كل تغيير، فيمكنك دائمًا معرفة من فعل ماذا ومتى.",
    "splash.benefits.6.title": "يناسب أنظمتك",
    "splash.benefits.6.body":
      "تتكامل واجهة REST الموثّقة مع الأدوات التي تستخدمها بالفعل.",
    "splash.features.1.title": "البحث والتصفح",
    "splash.features.1.body":
      "ابحث بالاسم أو المعرّف، مع خيار البحث التقريبي، في جدول قابل للفرز.",
    "splash.features.2.title": "سجلات الدورات",
    "splash.features.2.body":
      "رمز الدورة ومستواها وساعاتها ومعرّفاتها وكلماتها المفتاحية وما تُدرّسه، مع تحقق فوري.",
    "splash.features.3.title": "فحص التطابق",
    "splash.features.3.body":
      "قيّم دورة مرشحة مقابل الكتالوج واطلع على الأسباب.",
    "splash.features.4.title": "دمج الدورات",
    "splash.features.4.body":
      "ادمج التكرارات المؤكدة في سجل واحد دون فقدان السجل التاريخي.",
    "splash.features.5.title": "لوحة دورة الحياة",
    "splash.features.5.body":
      "اسحب الدورات بين مسودة ومنشورة ومؤرشفة ومتقاعدة.",
    "splash.features.6.title": "تقويم الجدول",
    "splash.features.6.body":
      "اطّلع على كل نسخة من الدورة ومواعيدها على تقويم.",
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
    "tour.intro":
      "جولة إرشادية في سجل الدورات: ما تفعله كل شاشة وخطوات استخدامها، من تسجيل دورة إلى دمج التكرارات ومتابعة دورة حياته.",
    "tour.s1.title": "تسجيل دورة",
    "tour.s1.summary":
      "أنشئ سجل دورة برمزه ومستواه وساعاته المعتمدة ومعرّفاته وما يُدرّسه. يُنبَّه إلى التكرارات المحتملة قبل الحفظ النهائي.",
    "tour.s1.step.1":
      "سجّل الدخول، وافتح القائمة واختر دورة جديدة، أو استخدم زر دورة جديدة في صفحة الدورات.",
    "tour.s1.step.2":
      "املأ الاسم ثم رمز الدورة (خاص بالجهة المزوِّدة، مثل CS101) والحالة والمستوى التعليمي وعدد الساعات المعتمدة.",
    "tour.s1.step.3":
      "أضف الكلمات المفتاحية وما يُدرّسه (الكفايات) والأسماء البديلة وروابط «هو نفسه» ومعرّفًا واحدًا أو أكثر بزر إضافة معرّف.",
    "tour.s1.step.4":
      "اضغط إنشاء. إذا ظهر «تم اكتشاف تكرارات» فراجع التكرارات المحتملة قبل إعادة الإرسال، وإلا فستنتقل إلى الدورة الجديد.",
    "tour.s2.title": "العثور على دورة وفتحه",
    "tour.s2.summary":
      "ابحث في الفهرس بالاسم أو المعرّف، ثم افتح دورة لترى كل ما سُجِّل عنه.",
    "tour.s2.step.1":
      "افتح الدورات من القائمة. تعرض الشبكة كل دورة مع المعرّف والاسم ورمز الدورة والمستوى والحالة والمعرّف الأساسي.",
    "tour.s2.step.2":
      "اكتب اسمًا أو معرّفًا في مربع البحث واضغط بحث؛ فعّل «تقريبي» لتحمّل الأخطاء الإملائية واختلافات الكتابة.",
    "tour.s2.step.3":
      "راجع عدد السجلات فوق الشبكة، ثم اختر صفًّا لفتح صفحة تفاصيل تلك الدورة.",
    "tour.s2.step.4":
      "في صفحة التفاصيل اقرأ الهوية والمعرّفات وما يُدرّسه والكلمات المفتاحية والنسخ، ثم استخدم تحرير للتغيير أو تدقيق لرؤية تاريخه.",
    "tour.s3.title": "التحقق من التكرارات",
    "tour.s3.summary":
      "صِف دورة واحسب درجة تطابقه مع الفهرس، مع تفصيل لكل حقل يوضح سبب تطابق كل مرشح.",
    "tour.s3.step.1": "افتح فحص التطابق من القائمة؛ هذه الشاشة لا تحفظ أي شيء.",
    "tour.s3.step.2":
      "أدخل ما تعرفه: الاسم ورمز الدورة ومعرّف الجهة المزوِّدة والمستوى التعليمي والكلمات المفتاحية وما يُدرّسه وروابط «هو نفسه» أو المعرّفات.",
    "tour.s3.step.3":
      "اضغط البحث عن تطابقات. تُعرض المرشحات تحت نتائج المطابقة وكل منها بدرجة؛ استخدم حد العرض (من 0.0 إلى 1.0) لإخفاء الضعيفة.",
    "tour.s3.step.4":
      "افتح تفصيل الدرجة لأي مرشح لترى درجات الاسم والرمز والجهة المزوِّدة والمستوى والكلمات المفتاحية وما يُدرّسه، أو تطابقًا حتميًا على معرّف أو جهة مع رمز أو رابط «هو نفسه».",
    "tour.s4.title": "دمج التكرارات المؤكدة",
    "tour.s4.summary":
      "ادمج دورة مكررًا في السجل الباقي، مع الاحتفاظ بسجل الدمج ودون حذف أي شيء حذفًا نهائيًا.",
    "tour.s4.step.1":
      "افتح دمج من القائمة (يتطلب تسجيل الدخول). أدخل معرّف الدورة الرئيسية (السجل الباقي) ومعرّف الدورة المكررة (الذي سيُحذف حذفًا ناعمًا).",
    "tour.s4.step.2":
      "يمكنك إضافة سبب مثل «تكرار مؤكد»؛ يُسجَّل في مسار تدقيق الدمج.",
    "tour.s4.step.3":
      "اضغط تحميل المعاينة لترى الدورة الرئيسية والمكرر جنبًا إلى جنب وتتأكد من صحة الزوج.",
    "tour.s4.step.4":
      "اضغط دمج وأكّد. تعرض «اكتمل الدمج» سجل الدمج الجديد مع رابط لعرض الدورة الرئيسية المدموج.",
    "tour.s5.title": "نقل الدورات عبر دورة حياتها",
    "tour.s5.summary":
      "شاهد كل دورة كبطاقة في عمود لكل حالة، وغيّر حالة الدورة بسحبه.",
    "tour.s5.step.1":
      "افتح اللوحة من القائمة. تمتد الأعمدة عبر دورة الحياة: draft وpublished وarchived وretired.",
    "tour.s5.step.2":
      "تعرض كل بطاقة اسم الدورة ورمزه، فتتعرّف على الدورة المطلوب بنظرة.",
    "tour.s5.step.3":
      "اسحب بطاقة إلى عمود آخر لتغيير حالة تلك الدورة؛ يُحفظ التغيير في سجل الدورة فورًا.",
    "tour.s5.step.4":
      "ثم تُعاد تحميل اللوحة من الخدمة، فتعود البطاقة التي تعذّر حفظ تغييرها إلى موضع السجل الفعلي ويظهر خطأ.",
    "tour.s6.title": "عرض نسخ الدورات على التقويم",
    "tour.s6.summary":
      "كل طرح مجدول لكل دورة مرتبًا حسب التاريخ، للقراءة فقط، مع نقرة تفتح الدورة المالك.",
    "tour.s6.step.1": "افتح التقويم من القائمة. يُفتح بعرض الشهر.",
    "tour.s6.step.2":
      "تظهر نافذة جدول كل نسخة من الدورة كفترة ليوم كامل بعنوان اسم النسخة، أو اسم الدورة إن لم يكن لها اسم.",
    "tour.s6.step.3":
      "تظهر الجلسات الفردية كأحداث محددة الوقت في أيامها، باستخدام تسمية الجلسة إن وُجدت.",
    "tour.s6.step.4":
      "اختر أي إدخال لفتح الدورة المالكة له، حيث يسرد قسم النسخ تواريخه ونمطه وسعته.",
    "signin.sso": "تسجيل الدخول عبر SSO",
  },
  "cy-001": {
    "nav.calendar": "Calendr",
    "nav.board": "Bwrdd",
    "brand.name": "Cwrs",
    "brand.tagline": "Main X Index",
    "nav.toggle": "Toglo'r llywio",
    "nav.dashboard": "Dangosfwrdd",
    "nav.courses": "Cyrsiau",
    "nav.newCourse": "Cwrs newydd",
    "nav.matchCheck": "Gwiriad cydweddu",
    "nav.merge": "Uno",
    "chrome.theme": "Thema",
    "chrome.language": "Iaith",
    "chrome.share": "Rhannu",
    "chrome.textSize": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "dashboard.title": "Dangosfwrdd",
    "dashboard.servicePrefix": "Gwasanaeth:",
    "dashboard.status.ok": "iawn",
    "dashboard.status.down": "i lawr",
    "dashboard.status.loading": "yn llwytho",
    "dashboard.recentActivity": "Gweithgaredd diweddar",
    "dashboard.noRecent": "Dim cofnodion archwilio diweddar.",
    "courses.title": "Cyrsiau",
    "courses.new": "Cwrs newydd",
    "courses.searchPlaceholder": "Chwilio yn ôl enw, dynodydd…",
    "courses.fuzzy": "Bras",
    "courses.loading": "Yn llwytho…",
    "courses.recordCount.one": "{n} cofnod",
    "courses.recordCount.other": "{n} cofnod",
    "detail.loading": "Yn llwytho…",
    "detail.edit": "Golygu",
    "detail.audit": "Archwilio",
    "detail.delete": "Dileu",
    "detail.exportGdpr": "Allforio data (GDPR)",
    "detail.exportingGdpr": "Wrthi'n allforio…",
    "detail.confirmDelete":
      "Meddal-ddileu'r cwrs hwn? Ni ellir dadwneud hyn drwy'r rhyngwyneb.",
    "detail.identity": "Hunaniaeth",
    "detail.id": "ID",
    "detail.courseCode": "Cod cwrs",
    "detail.status": "Statws",
    "detail.educationalLevel": "Lefel addysgol",
    "detail.numberOfCredits": "Nifer y credydau",
    "detail.timeRequired": "Amser gofynnol",
    "detail.description": "Disgrifiad",
    "detail.url": "URL",
    "detail.license": "Trwydded",
    "detail.free": "Am ddim",
    "detail.yes": "ie",
    "detail.no": "na",
    "detail.empty": "—",
    "detail.identifiers": "Dynodyddion",
    "detail.teaches": "Yn addysgu",
    "detail.keywords": "Allweddeiriau",
    "detail.alternateNames": "Enwau eraill",
    "detail.sameAs": "Yr un â (URLau awdurdodol)",
    "detail.instances": "Achosion",
    "detail.customPrefix": "Cwsmer:",
    "detail.noDate": "(dim dyddiad)",
    "detail.capacity": "cap",
    "edit.title": "Golygu cwrs",
    "edit.cancel": "Canslo",
    "edit.loading": "Yn llwytho…",
    "edit.saveChanges": "Cadw newidiadau",
    "new.title": "Cwrs newydd",
    "new.create": "Creu",
    "new.possibleDuplicates": "Dyblygiadau posibl",
    "new.duplicatesDetected":
      "Canfuwyd dyblygiadau ({n}) — adolygwch isod cyn ailgyflwyno.",
    "audit.title": "Cofnod archwilio",
    "audit.backToCourse": "Yn ôl i'r cwrs",
    "audit.loading": "Yn llwytho…",
    "audit.noEntries": "Dim cofnodion archwilio.",
    "audit.by": "gan",
    "audit.payload": "Llwyth",
    "match.title": "Gwiriad cydweddu",
    "match.name": "Enw",
    "match.courseCode": "Cod cwrs",
    "match.providerId": "ID darparwr",
    "match.displayThreshold": "Trothwy arddangos",
    "match.thresholdHint": "Hidlydd ochr-cleient (0.0 – 1.0)",
    "match.educationalLevel": "Lefel addysgol",
    "match.keywords": "Allweddeiriau",
    "match.keywordsHint": "Wedi'u gwahanu gan goma neu linell newydd",
    "match.teaches": "Yn addysgu (cymwyseddau)",
    "match.teachesHint": "Un fesul llinell",
    "match.sameAs": "URLau yr un â",
    "match.sameAsHint": "Un fesul llinell",
    "match.identifiers": "Dynodyddion",
    "match.matching": "Yn cydweddu…",
    "match.findMatches": "Canfod cydweddiadau",
    "merge.title": "Uno cyrsiau",
    "merge.mainId": "ID prif gwrs",
    "merge.mainIdHint": "Y cofnod sy'n goroesi",
    "merge.duplicateId": "ID cwrs dyblyg",
    "merge.duplicateIdHint": "Caiff ei feddal-ddileu",
    "merge.reason": "Rheswm",
    "merge.reasonHint": "Cofnodir yn y llwybr archwilio uno",
    "merge.reasonPlaceholder": "Dyblyg wedi'i gadarnhau",
    "merge.loadPreview": "Llwytho rhagolwg",
    "merge.merging": "Yn uno…",
    "merge.merge": "Uno",
    "merge.bothIdsRequired": "Mae angen y ddau ID",
    "merge.mustDiffer": "Rhaid i'r prif a'r dyblyg fod yn wahanol",
    "merge.confirm":
      "Uno {dup}… i mewn i {main}…?\nMae hyn yn meddal-ddileu'r dyblyg.",
    "merge.preview": "Rhagolwg",
    "merge.main": "Prif",
    "merge.duplicate": "Dyblyg",
    "merge.completed": "Uno wedi'i gwblhau",
    "merge.recordCreated": "Crëwyd cofnod uno {id} am {at}.",
    "merge.viewMerged": "Gweld y prif gwrs unedig",
    "form.name": "Enw",
    "form.courseCode": "Cod cwrs",
    "form.courseCodeHint": "Wedi'i gwmpasu gan ddarparwr (e.e. CS101)",
    "form.status": "Statws",
    "form.description": "Disgrifiad",
    "form.url": "URL",
    "form.license": "Trwydded",
    "form.numberOfCredits": "Nifer y credydau",
    "form.educationalLevel": "Lefel addysgol",
    "form.typicalAgeRange": "Ystod oedran nodweddiadol",
    "form.typicalAgeRangeHint": "e.e. 18-22",
    "form.timeRequired": "Amser gofynnol",
    "form.timeRequiredHint": "ISO 8601 (e.e. PT45H)",
    "form.alternateNames": "Enwau eraill",
    "form.alternateNamesHint": "Un fesul llinell",
    "form.keywords": "Allweddeiriau",
    "form.keywordsHint": "Wedi'u gwahanu gan goma neu linell newydd",
    "form.teaches": "Yn addysgu (cymwyseddau)",
    "form.teachesHint": "Un fesul llinell",
    "form.availableLanguages": "Ieithoedd ar gael",
    "form.availableLanguagesHint": "Codau BCP-47 (e.e. en fr de)",
    "form.sameAs": "URLau yr un â",
    "form.sameAsHint": "Wikidata, catalog OER, ac ati — un fesul llinell",
    "form.identifiers": "Dynodyddion",
    "form.saving": "Yn cadw…",
    "form.save": "Cadw",
    "form.reset": "Ailosod",
    "identifier.type": "Math",
    "identifier.customOption": "Cwsmer…",
    "identifier.customLabel": "Label cwsmer",
    "identifier.value": "Gwerth",
    "identifier.url": "URL",
    "identifier.remove": "Tynnu",
    "identifier.add": "+ Ychwanegu dynodydd",
    "search.placeholder": "Chwilio…",
    "search.submit": "Chwilio",
    "results.title": "Canlyniadau cydweddu",
    "results.count": "({n})",
    "results.none": "Dim ymgeiswyr.",
    "results.scoreBreakdown": "Dadansoddiad sgôr",
    "results.score.name": "enw",
    "results.score.courseCode": "cod cwrs",
    "results.score.provider": "darparwr",
    "results.score.level": "lefel",
    "results.score.keywords": "allweddeiriau",
    "results.score.teaches": "yn addysgu",
    "results.score.deterministic":
      "penderfyniaethol (dynodydd / darparwr+cod / yr un â)",
    "grid.id": "ID",
    "grid.name": "Enw",
    "grid.courseCode": "Cod cwrs",
    "grid.level": "Lefel",
    "grid.status": "Statws",
    "grid.primaryIdentifier": "Prif ddynodydd",
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
    "splash.hero.title": "Un cofnod dibynadwy ar gyfer pob cwrs",
    "splash.hero.subtitle":
      "Cofrestrwch gyrsiau unwaith, dewch o hyd iddynt ar unwaith, a rhwystrwch ddyblygu cyn iddo ddechrau, gyda llwybr archwilio llawn wedi'i gynnwys.",
    "splash.benefits.1.title": "Llai o ddyblygu",
    "splash.benefits.1.body":
      "Mae paru yn nodi dyblyg tebygol y funud y crëir cwrs.",
    "splash.benefits.2.title": "Dewch o hyd i unrhyw gwrs yn gyflym",
    "splash.benefits.2.body":
      "Mae chwilio testun llawn a bras yn dod o hyd i gyrsiau er gwaethaf camsillafu ac amrywiadau sillafu.",
    "splash.benefits.3.title": "Penderfyniadau hyderus",
    "splash.benefits.3.body":
      "Mae pob cydweddiad yn dangos ei sgôr a dadansoddiad fesul maes, fel bod adolygwyr yn gweld yn union pam.",
    "splash.benefits.4.title": "Un catalog dibynadwy",
    "splash.benefits.4.body":
      "Mae codau cyrsiau ac dynodwyr yn cadw un cofnod yn unig ar gyfer pob cwrs.",
    "splash.benefits.5.title": "Dim yn ddigofnod",
    "splash.benefits.5.body":
      "Archwilir pob newid, felly gallwch bob amser ateb pwy wnaeth beth, a phryd.",
    "splash.benefits.6.title": "Yn ffitio eich systemau",
    "splash.benefits.6.body":
      "Mae API REST wedi'i ddogfennu yn plygio i'r offer rydych eisoes yn eu rhedeg.",
    "splash.features.1.title": "Chwilio a phori",
    "splash.features.1.body":
      "Chwiliwch yn ôl enw neu ddynodwr, gydag opsiwn bras, mewn grid didoladwy.",
    "splash.features.2.title": "Cofnodion cyrsiau",
    "splash.features.2.body":
      "Cod y cwrs, lefel, credydau, dynodwyr, geiriau allweddol a'r hyn a addysgir, gyda dilysu byw.",
    "splash.features.3.title": "Gwiriad cydweddu",
    "splash.features.3.body":
      "Sgoriwch gwrs ymgeisiol yn erbyn y catalog a gweld y rhesymau.",
    "splash.features.4.title": "Uno cyrsiau",
    "splash.features.4.body":
      "Cyfunwch ddyblygiadau wedi'u cadarnhau yn un cofnod heb golli hanes.",
    "splash.features.5.title": "Bwrdd cylch bywyd",
    "splash.features.5.body":
      "Llusgwch gyrsiau rhwng drafft, cyhoeddedig, archifedig a wedi ymddeol.",
    "splash.features.6.title": "Calendr amserlen",
    "splash.features.6.body":
      "Gwelwch bob achos o gwrs a'i ddyddiadau ar galendr.",
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
    "tour.intro":
      "Taith dywys drwy'r gofrestr Cyrsiau: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o gofrestru cwrs i uno dyblygion a dilyn ei gylch bywyd.",
    "tour.s1.title": "Cofrestru cwrs",
    "tour.s1.summary":
      "Crëwch gofnod cwrs gyda'i god, ei lefel, ei gredydau, ei ddynodwyr a'r hyn y mae'n ei ddysgu. Caiff dyblygion tebygol eu nodi cyn i chi gadarnhau.",
    "tour.s1.step.1":
      "Mewngofnodwch, agorwch y ddewislen a dewis Cwrs newydd (neu defnyddiwch y botwm Cwrs newydd ar dudalen y Cyrsiau).",
    "tour.s1.step.2":
      "Llenwch yr Enw, yna Cod y cwrs (o fewn y darparwr, fel CS101), y Statws, y Lefel addysgol a Nifer y credydau.",
    "tour.s1.step.3":
      "Ychwanegwch Allweddeiriau, Yn addysgu (cymwyseddau), Enwau amgen, URLau yr un â a Dynodyddion gyda'r botwm + Ychwanegu dynodydd.",
    "tour.s1.step.4":
      "Pwyswch Creu. Os yw Canfuwyd dyblygiadau yn ymddangos, adolygwch y Dyblygiadau posibl cyn ailgyflwyno; fel arall byddwch yn cyrraedd y cwrs newydd.",
    "tour.s2.title": "Dod o hyd i gwrs a'i agor",
    "tour.s2.summary":
      "Chwiliwch y catalog yn ôl enw neu ddynodydd, yna agorwch gwrs i weld popeth a gofnodwyd amdano.",
    "tour.s2.step.1":
      "Agorwch Cyrsiau o'r ddewislen. Mae'r grid yn rhestru pob cwrs gyda'i ID, Enw, Cod y cwrs, Lefel, Statws a Dynodydd cynradd.",
    "tour.s2.step.2":
      "Teipiwch enw neu ddynodydd yn y blwch chwilio a phwyso Chwilio; ticiwch Bras i oddef camsillafu ac amrywiadau sillafu.",
    "tour.s2.step.3":
      "Edrychwch ar nifer y cofnodion uwchben y grid, yna dewiswch res i agor tudalen manylion y cwrs hwnnw.",
    "tour.s2.step.4":
      "Ar y dudalen manylion darllenwch Hunaniaeth, Dynodyddion, Yn addysgu, Allweddeiriau a Chyfarfodydd, yna defnyddiwch Golygu i'w newid neu Archwilio i weld ei hanes.",
    "tour.s3.title": "Gwirio am ddyblygion",
    "tour.s3.summary":
      "Disgrifiwch gwrs a'i sgorio yn erbyn y catalog, gyda dadansoddiad fesul maes sy'n dangos pam y cyfatebodd pob ymgeisydd.",
    "tour.s3.step.1":
      "Agorwch Gwiriad cydweddu o'r ddewislen; nid yw'r sgrin hon yn cadw dim.",
    "tour.s3.step.2":
      "Rhowch yr hyn a wyddoch: Enw, Cod y cwrs, ID y darparwr, Lefel addysgol, Allweddeiriau, Yn addysgu, URLau yr un â neu Ddynodwyr.",
    "tour.s3.step.3":
      "Pwyswch Canfod cydweddiadau. Rhestrir ymgeiswyr o dan Canlyniadau cydweddu, pob un â sgôr; defnyddiwch Trothwy arddangos (0.0 i 1.0) i guddio'r gwan.",
    "tour.s3.step.4":
      "Agorwch Dadansoddiad sgôr ar ymgeisydd i weld sgorau'r enw, cod y cwrs, y darparwr, y lefel, yr allweddeiriau a'r hyn a ddysgir, neu gyfatebiaeth bendant ar ddynodydd, darparwr a chod, neu URL yr un peth â.",
    "tour.s4.title": "Uno dyblygion wedi'u cadarnhau",
    "tour.s4.summary":
      "Plygwch gwrs dyblyg i'r cofnod sy'n goroesi, gan gadw cofnod uno heb ddileu dim yn barhaol.",
    "tour.s4.step.1":
      "Agorwch Uno o'r ddewislen (mae angen mewngofnodi). Rhowch ID y prif gwrs (y cofnod sy'n goroesi) ac ID y cwrs dyblyg (a gaiff ei feddal-ddileu).",
    "tour.s4.step.2":
      "Gallwch ychwanegu Rheswm fel Dyblyg wedi'i gadarnhau; caiff ei gofnodi yn llwybr archwilio'r uno.",
    "tour.s4.step.3":
      "Pwyswch Llwytho rhagolwg i weld y Prif gwrs a'r cwrs Dyblyg ochr yn ochr a gwirio bod gennych y pâr cywir.",
    "tour.s4.step.4":
      "Pwyswch Uno a chadarnhau. Mae Uno wedi'i gwblhau yn dangos y cofnod uno newydd, gyda dolen i Weld y prif gwrs unedig.",
    "tour.s5.title": "Symud cyrsiau drwy eu cylch bywyd",
    "tour.s5.summary":
      "Gwelwch bob cwrs fel cerdyn mewn un golofn ar gyfer pob statws, a newidiwch statws cwrs trwy ei lusgo.",
    "tour.s5.step.1":
      "Agorwch Bwrdd o'r ddewislen. Mae'r colofnau'n dilyn y cylch bywyd: draft, published, archived a retired.",
    "tour.s5.step.2":
      "Mae pob cerdyn yn dangos enw'r cwrs a chod y cwrs, felly gallwch adnabod y cwrs cywir ar unwaith.",
    "tour.s5.step.3":
      "Llusgwch gerdyn i golofn arall i newid statws y cwrs hwnnw; caiff y newid ei gadw yng nghofnod y cwrs ar unwaith.",
    "tour.s5.step.4":
      "Yna mae'r bwrdd yn ail-lwytho o'r gwasanaeth, felly mae cerdyn na chadwyd ei newid yn dychwelyd i'r lle mae'r cofnod mewn gwirionedd ac arddangosir gwall.",
    "tour.s6.title": "Gweld achosion cyrsiau ar y calendr",
    "tour.s6.summary":
      "Pob cynnig wedi'i amserlennu ar gyfer pob cwrs wedi'i osod yn ôl dyddiad, darllen yn unig, gyda chlic drwodd i'r cwrs perthnasol.",
    "tour.s6.step.1":
      "Agorwch Calendr o'r ddewislen. Mae'n agor yn y wedd fisol.",
    "tour.s6.step.2":
      "Mae ffenestr amserlen pob achos cwrs yn ymddangos fel cyfnod diwrnod cyfan wedi'i labelu ag enw'r achos, neu enw'r cwrs os nad oes un.",
    "tour.s6.step.3":
      "Mae sesiynau unigol yn ymddangos fel digwyddiadau amseredig ar eu diwrnodau eu hunain, gan ddefnyddio label y sesiwn lle mae un.",
    "tour.s6.step.4":
      "Dewiswch unrhyw gofnod i agor y cwrs sy'n berchen arno, lle mae'r adran Achosion yn rhestru ei ddyddiadau, ei fodd a'i gapasiti.",
    "signin.sso": "Mewngofnodi gydag SSO",
  },
  "en-001": {
    "nav.calendar": "Calendar",
    "nav.board": "Board",
    // Layout / chrome
    "brand.name": "Course",
    "brand.tagline": "Main X Index",
    "nav.toggle": "Toggle navigation",
    "nav.dashboard": "Dashboard",
    "nav.courses": "Courses",
    "nav.newCourse": "New course",
    "nav.matchCheck": "Match check",
    "nav.merge": "Merge",
    "chrome.theme": "Theme",
    "chrome.language": "Language",
    "chrome.share": "Share",
    "chrome.textSize": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    // Dashboard
    "dashboard.title": "Dashboard",
    "dashboard.servicePrefix": "Service:",
    "dashboard.status.ok": "ok",
    "dashboard.status.down": "down",
    "dashboard.status.loading": "loading",
    "dashboard.recentActivity": "Recent activity",
    "dashboard.noRecent": "No recent audit entries.",
    // Courses list
    "courses.title": "Courses",
    "courses.new": "New course",
    "courses.searchPlaceholder": "Search by name, identifier…",
    "courses.fuzzy": "Fuzzy",
    "courses.loading": "Loading…",
    "courses.recordCount.one": "{n} record",
    "courses.recordCount.other": "{n} records",
    // Course detail
    "detail.loading": "Loading…",
    "detail.edit": "Edit",
    "detail.audit": "Audit",
    "detail.delete": "Delete",
    "detail.exportGdpr": "Export data (GDPR)",
    "detail.exportingGdpr": "Exporting…",
    "detail.confirmDelete":
      "Soft-delete this course? This cannot be undone via the UI.",
    "detail.identity": "Identity",
    "detail.id": "ID",
    "detail.courseCode": "Course code",
    "detail.status": "Status",
    "detail.educationalLevel": "Educational level",
    "detail.numberOfCredits": "Number of credits",
    "detail.timeRequired": "Time required",
    "detail.description": "Description",
    "detail.url": "URL",
    "detail.license": "License",
    "detail.free": "Free",
    "detail.yes": "yes",
    "detail.no": "no",
    "detail.empty": "—",
    "detail.identifiers": "Identifiers",
    "detail.teaches": "Teaches",
    "detail.keywords": "Keywords",
    "detail.alternateNames": "Alternate names",
    "detail.sameAs": "Same-as (authoritative URLs)",
    "detail.instances": "Instances",
    "detail.customPrefix": "Custom:",
    "detail.noDate": "(no date)",
    "detail.capacity": "cap",
    // Edit course
    "edit.title": "Edit course",
    "edit.cancel": "Cancel",
    "edit.loading": "Loading…",
    "edit.saveChanges": "Save changes",
    // New course
    "new.title": "New course",
    "new.create": "Create",
    "new.possibleDuplicates": "Possible duplicates",
    "new.duplicatesDetected":
      "Duplicates detected ({n}) — review below before resubmitting.",
    // Audit log
    "audit.title": "Audit log",
    "audit.backToCourse": "Back to course",
    "audit.loading": "Loading…",
    "audit.noEntries": "No audit entries.",
    "audit.by": "by",
    "audit.payload": "Payload",
    // Match check
    "match.title": "Match check",
    "match.name": "Name",
    "match.courseCode": "Course code",
    "match.providerId": "Provider ID",
    "match.displayThreshold": "Display threshold",
    "match.thresholdHint": "Client-side filter (0.0 – 1.0)",
    "match.educationalLevel": "Educational level",
    "match.keywords": "Keywords",
    "match.keywordsHint": "Comma- or newline-separated",
    "match.teaches": "Teaches (competencies)",
    "match.teachesHint": "One per line",
    "match.sameAs": "Same-as URLs",
    "match.sameAsHint": "One per line",
    "match.identifiers": "Identifiers",
    "match.matching": "Matching…",
    "match.findMatches": "Find matches",
    // Merge
    "merge.title": "Merge courses",
    "merge.mainId": "Main course ID",
    "merge.mainIdHint": "The surviving record",
    "merge.duplicateId": "Duplicate course ID",
    "merge.duplicateIdHint": "Will be soft-deleted",
    "merge.reason": "Reason",
    "merge.reasonHint": "Recorded in the merge audit trail",
    "merge.reasonPlaceholder": "Confirmed duplicate",
    "merge.loadPreview": "Load preview",
    "merge.merging": "Merging…",
    "merge.merge": "Merge",
    "merge.bothIdsRequired": "Both IDs required",
    "merge.mustDiffer": "Main and duplicate must differ",
    "merge.confirm":
      "Merge {dup}… into {main}…?\nThis soft-deletes the duplicate.",
    "merge.preview": "Preview",
    "merge.main": "Main",
    "merge.duplicate": "Duplicate",
    "merge.completed": "Merge completed",
    "merge.recordCreated": "Merge record {id} created at {at}.",
    "merge.viewMerged": "View merged main course",
    // Course form
    "form.name": "Name",
    "form.courseCode": "Course code",
    "form.courseCodeHint": "Provider-scoped (e.g. CS101)",
    "form.status": "Status",
    "form.description": "Description",
    "form.url": "URL",
    "form.license": "License",
    "form.numberOfCredits": "Number of credits",
    "form.educationalLevel": "Educational level",
    "form.typicalAgeRange": "Typical age range",
    "form.typicalAgeRangeHint": "e.g. 18-22",
    "form.timeRequired": "Time required",
    "form.timeRequiredHint": "ISO 8601 (e.g. PT45H)",
    "form.alternateNames": "Alternate names",
    "form.alternateNamesHint": "One per line",
    "form.keywords": "Keywords",
    "form.keywordsHint": "Comma- or newline-separated",
    "form.teaches": "Teaches (competencies)",
    "form.teachesHint": "One per line",
    "form.availableLanguages": "Available languages",
    "form.availableLanguagesHint": "BCP-47 codes (e.g. en fr de)",
    "form.sameAs": "Same-as URLs",
    "form.sameAsHint": "Wikidata, OER catalog, etc. — one per line",
    "form.identifiers": "Identifiers",
    "form.saving": "Saving…",
    "form.save": "Save",
    "form.reset": "Reset",
    // Identifier input
    "identifier.type": "Type",
    "identifier.customOption": "Custom…",
    "identifier.customLabel": "Custom label",
    "identifier.value": "Value",
    "identifier.url": "URL",
    "identifier.remove": "Remove",
    "identifier.add": "+ Add identifier",
    // Search box
    "search.placeholder": "Search…",
    "search.submit": "Search",
    // Match results
    "results.title": "Match results",
    "results.count": "({n})",
    "results.none": "No candidates.",
    "results.scoreBreakdown": "Score breakdown",
    "results.score.name": "name",
    "results.score.courseCode": "course code",
    "results.score.provider": "provider",
    "results.score.level": "level",
    "results.score.keywords": "keywords",
    "results.score.teaches": "teaches",
    "results.score.deterministic":
      "deterministic (identifier / provider+code / same-as)",
    // Course grid column headers
    "grid.id": "ID",
    "grid.name": "Name",
    "grid.courseCode": "Course code",
    "grid.level": "Level",
    "grid.status": "Status",
    "grid.primaryIdentifier": "Primary identifier",
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
    "splash.hero.title": "One trusted record for every course",
    "splash.hero.subtitle":
      "Register courses once, find them instantly, and stop duplicates before they start, with a full audit trail built in.",
    "splash.benefits.1.title": "Fewer duplicates",
    "splash.benefits.1.body":
      "Matching flags a likely duplicate the moment a course is created.",
    "splash.benefits.2.title": "Find any course fast",
    "splash.benefits.2.body":
      "Full-text and fuzzy search finds courses despite typos and spelling variants.",
    "splash.benefits.3.title": "Confident decisions",
    "splash.benefits.3.body":
      "Every match shows its score and a per-field breakdown, so reviewers see exactly why.",
    "splash.benefits.4.title": "One trusted catalogue",
    "splash.benefits.4.body":
      "Course codes and identifiers keep exactly one record per course.",
    "splash.benefits.5.title": "Nothing goes unrecorded",
    "splash.benefits.5.body":
      "Each change is audited, so you can always answer who did what, and when.",
    "splash.benefits.6.title": "Fits your systems",
    "splash.benefits.6.body":
      "A documented REST API plugs into the tools you already run.",
    "splash.features.1.title": "Search and browse",
    "splash.features.1.body":
      "Search by name or identifier, with a fuzzy option, in a sortable grid.",
    "splash.features.2.title": "Course records",
    "splash.features.2.body":
      "Course code, level, credits, identifiers, keywords and what it teaches, with live validation.",
    "splash.features.3.title": "Match check",
    "splash.features.3.body":
      "Score a candidate course against the catalogue and see the reasons.",
    "splash.features.4.title": "Merge courses",
    "splash.features.4.body":
      "Combine confirmed duplicates into one record without losing history.",
    "splash.features.5.title": "Lifecycle board",
    "splash.features.5.body":
      "Drag courses between draft, published, archived and retired.",
    "splash.features.6.title": "Schedule calendar",
    "splash.features.6.body":
      "See every course instance and its dates laid out on a calendar.",
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
    "tour.intro":
      "A guided walkthrough of the Course registry: what each screen does and the steps to use it, from registering a course to merging duplicates and tracking its lifecycle.",
    "tour.s1.title": "Register a course",
    "tour.s1.summary":
      "Create a course record with its code, level, credits, identifiers and what it teaches. Likely duplicates are flagged before you commit.",
    "tour.s1.step.1":
      "Sign in, open the menu and choose New course (or use New course on the Courses page).",
    "tour.s1.step.2":
      "Fill in the Name, then the Course code (provider-scoped, such as CS101), Status, Educational level and Number of credits.",
    "tour.s1.step.3":
      "Add Keywords, Teaches (competencies), Alternate names, Same-as URLs and one or more Identifiers with the + Add identifier button.",
    "tour.s1.step.4":
      "Press Create. If Duplicates detected appears, review the Possible duplicates before resubmitting; otherwise you land on the new course.",
    "tour.s2.title": "Find and open a course",
    "tour.s2.summary":
      "Search the catalogue by name or identifier, then open a course to see everything recorded about it.",
    "tour.s2.step.1":
      "Open Courses from the menu. The grid lists each course with its ID, Name, Course code, Level, Status and Primary identifier.",
    "tour.s2.step.2":
      "Type a name or identifier in the search box and press Search; tick Fuzzy to tolerate typos and spelling variants.",
    "tour.s2.step.3":
      "Check the record count above the grid, then select a row to open that course's detail page.",
    "tour.s2.step.4":
      "On the detail page read Identity, Identifiers, Teaches, Keywords and Instances, then use Edit to change it or Audit to see its history.",
    "tour.s3.title": "Check for duplicates",
    "tour.s3.summary":
      "Describe a course and score it against the catalogue, with a per-field breakdown showing why each candidate matched.",
    "tour.s3.step.1":
      "Open Match check from the menu; nothing is saved by this screen.",
    "tour.s3.step.2":
      "Enter what you know: Name, Course code, Provider ID, Educational level, Keywords, Teaches, Same-as URLs or Identifiers.",
    "tour.s3.step.3":
      "Press Find matches. Candidates are listed under Match results, each with a score; use Display threshold (0.0 to 1.0) to hide weak ones.",
    "tour.s3.step.4":
      "Open Score breakdown on a candidate to see the name, course code, provider, level, keywords and teaches scores, or a deterministic hit on an identifier, provider plus code, or same-as URL.",
    "tour.s4.title": "Merge confirmed duplicates",
    "tour.s4.summary":
      "Fold a duplicate course into the record that survives, keeping a merge record and never hard-deleting anything.",
    "tour.s4.step.1":
      "Open Merge from the menu (sign-in required). Enter the Main course ID (the surviving record) and the Duplicate course ID (which will be soft-deleted).",
    "tour.s4.step.2":
      "Optionally add a Reason such as Confirmed duplicate; it is recorded in the merge audit trail.",
    "tour.s4.step.3":
      "Press Load preview to see the Main and Duplicate courses side by side and check you have the right pair.",
    "tour.s4.step.4":
      "Press Merge and confirm. Merge completed shows the new merge record, with a link to View merged main course.",
    "tour.s5.title": "Move courses through their lifecycle",
    "tour.s5.summary":
      "See every course as a card in one column per status, and change a course's status by dragging it.",
    "tour.s5.step.1":
      "Open Board from the menu. Columns run through the lifecycle: draft, published, archived and retired.",
    "tour.s5.step.2":
      "Each card shows the course name and its course code, so you can spot the right course at a glance.",
    "tour.s5.step.3":
      "Drag a card into another column to change that course's status; the change is saved to the course record straight away.",
    "tour.s5.step.4":
      "The board then reloads from the service, so a card whose change could not be saved returns to where the record really is and an error is shown.",
    "tour.s6.title": "See course instances on the calendar",
    "tour.s6.summary":
      "Every scheduled offering of every course laid out by date, read-only, with a click through to the owning course.",
    "tour.s6.step.1": "Open Calendar from the menu. It opens in month view.",
    "tour.s6.step.2":
      "Each course instance's schedule window appears as an all-day span labelled with the instance name, or the course name if it has none.",
    "tour.s6.step.3":
      "Individual sessions appear as timed events on their own days, using the session label where one is set.",
    "tour.s6.step.4":
      "Select any entry to open the course that owns it, where the Instances section lists its dates, mode and capacity.",
    "signin.sso": "Sign in with SSO",
  },
  "es-001": {
    "nav.calendar": "Calendario",
    "nav.board": "Tablero",
    "brand.name": "Curso",
    "brand.tagline": "Main X Index",
    "nav.toggle": "Alternar navegación",
    "nav.dashboard": "Panel",
    "nav.courses": "Cursos",
    "nav.newCourse": "Nuevo curso",
    "nav.matchCheck": "Comprobar coincidencias",
    "nav.merge": "Fusionar",
    "chrome.theme": "Tema",
    "chrome.language": "Idioma",
    "chrome.share": "Compartir",
    "chrome.textSize": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "dashboard.title": "Panel",
    "dashboard.servicePrefix": "Servicio:",
    "dashboard.status.ok": "correcto",
    "dashboard.status.down": "caído",
    "dashboard.status.loading": "cargando",
    "dashboard.recentActivity": "Actividad reciente",
    "dashboard.noRecent": "Sin entradas de auditoría recientes.",
    "courses.title": "Cursos",
    "courses.new": "Nuevo curso",
    "courses.searchPlaceholder": "Buscar por nombre, identificador…",
    "courses.fuzzy": "Difuso",
    "courses.loading": "Cargando…",
    "courses.recordCount.one": "{n} registro",
    "courses.recordCount.other": "{n} registros",
    "detail.loading": "Cargando…",
    "detail.edit": "Editar",
    "detail.audit": "Auditoría",
    "detail.delete": "Eliminar",
    "detail.exportGdpr": "Exportar datos (RGPD)",
    "detail.exportingGdpr": "Exportando…",
    "detail.confirmDelete":
      "¿Eliminar de forma reversible este curso? Esto no se puede deshacer desde la interfaz.",
    "detail.identity": "Identidad",
    "detail.id": "ID",
    "detail.courseCode": "Código del curso",
    "detail.status": "Estado",
    "detail.educationalLevel": "Nivel educativo",
    "detail.numberOfCredits": "Número de créditos",
    "detail.timeRequired": "Tiempo requerido",
    "detail.description": "Descripción",
    "detail.url": "URL",
    "detail.license": "Licencia",
    "detail.free": "Gratis",
    "detail.yes": "sí",
    "detail.no": "no",
    "detail.empty": "—",
    "detail.identifiers": "Identificadores",
    "detail.teaches": "Enseña",
    "detail.keywords": "Palabras clave",
    "detail.alternateNames": "Nombres alternativos",
    "detail.sameAs": "Igual que (URL autorizadas)",
    "detail.instances": "Instancias",
    "detail.customPrefix": "Personalizado:",
    "detail.noDate": "(sin fecha)",
    "detail.capacity": "cap",
    "edit.title": "Editar curso",
    "edit.cancel": "Cancelar",
    "edit.loading": "Cargando…",
    "edit.saveChanges": "Guardar cambios",
    "new.title": "Nuevo curso",
    "new.create": "Crear",
    "new.possibleDuplicates": "Posibles duplicados",
    "new.duplicatesDetected":
      "Duplicados detectados ({n}) — revise a continuación antes de reenviar.",
    "audit.title": "Registro de auditoría",
    "audit.backToCourse": "Volver al curso",
    "audit.loading": "Cargando…",
    "audit.noEntries": "Sin entradas de auditoría.",
    "audit.by": "por",
    "audit.payload": "Carga útil",
    "match.title": "Comprobar coincidencias",
    "match.name": "Nombre",
    "match.courseCode": "Código del curso",
    "match.providerId": "ID del proveedor",
    "match.displayThreshold": "Umbral de visualización",
    "match.thresholdHint": "Filtro del lado del cliente (0.0 – 1.0)",
    "match.educationalLevel": "Nivel educativo",
    "match.keywords": "Palabras clave",
    "match.keywordsHint": "Separadas por comas o saltos de línea",
    "match.teaches": "Enseña (competencias)",
    "match.teachesHint": "Una por línea",
    "match.sameAs": "URL de igual que",
    "match.sameAsHint": "Una por línea",
    "match.identifiers": "Identificadores",
    "match.matching": "Coincidiendo…",
    "match.findMatches": "Buscar coincidencias",
    "merge.title": "Fusionar cursos",
    "merge.mainId": "ID del curso principal",
    "merge.mainIdHint": "El registro superviviente",
    "merge.duplicateId": "ID del curso duplicado",
    "merge.duplicateIdHint": "Se eliminará de forma reversible",
    "merge.reason": "Motivo",
    "merge.reasonHint": "Registrado en el historial de auditoría de fusión",
    "merge.reasonPlaceholder": "Duplicado confirmado",
    "merge.loadPreview": "Cargar vista previa",
    "merge.merging": "Fusionando…",
    "merge.merge": "Fusionar",
    "merge.bothIdsRequired": "Se requieren ambos ID",
    "merge.mustDiffer": "El principal y el duplicado deben ser distintos",
    "merge.confirm":
      "¿Fusionar {dup}… en {main}…?\nEsto elimina de forma reversible el duplicado.",
    "merge.preview": "Vista previa",
    "merge.main": "Principal",
    "merge.duplicate": "Duplicado",
    "merge.completed": "Fusión completada",
    "merge.recordCreated": "Registro de fusión {id} creado el {at}.",
    "merge.viewMerged": "Ver curso principal fusionado",
    "form.name": "Nombre",
    "form.courseCode": "Código del curso",
    "form.courseCodeHint": "Ámbito del proveedor (p. ej. CS101)",
    "form.status": "Estado",
    "form.description": "Descripción",
    "form.url": "URL",
    "form.license": "Licencia",
    "form.numberOfCredits": "Número de créditos",
    "form.educationalLevel": "Nivel educativo",
    "form.typicalAgeRange": "Rango de edad típico",
    "form.typicalAgeRangeHint": "p. ej. 18-22",
    "form.timeRequired": "Tiempo requerido",
    "form.timeRequiredHint": "ISO 8601 (p. ej. PT45H)",
    "form.alternateNames": "Nombres alternativos",
    "form.alternateNamesHint": "Uno por línea",
    "form.keywords": "Palabras clave",
    "form.keywordsHint": "Separadas por comas o saltos de línea",
    "form.teaches": "Enseña (competencias)",
    "form.teachesHint": "Una por línea",
    "form.availableLanguages": "Idiomas disponibles",
    "form.availableLanguagesHint": "Códigos BCP-47 (p. ej. en fr de)",
    "form.sameAs": "URL de igual que",
    "form.sameAsHint": "Wikidata, catálogo OER, etc. — una por línea",
    "form.identifiers": "Identificadores",
    "form.saving": "Guardando…",
    "form.save": "Guardar",
    "form.reset": "Restablecer",
    "identifier.type": "Tipo",
    "identifier.customOption": "Personalizado…",
    "identifier.customLabel": "Etiqueta personalizada",
    "identifier.value": "Valor",
    "identifier.url": "URL",
    "identifier.remove": "Eliminar",
    "identifier.add": "+ Añadir identificador",
    "search.placeholder": "Buscar…",
    "search.submit": "Buscar",
    "results.title": "Resultados de coincidencia",
    "results.count": "({n})",
    "results.none": "Sin candidatos.",
    "results.scoreBreakdown": "Desglose de puntuación",
    "results.score.name": "nombre",
    "results.score.courseCode": "código del curso",
    "results.score.provider": "proveedor",
    "results.score.level": "nivel",
    "results.score.keywords": "palabras clave",
    "results.score.teaches": "enseña",
    "results.score.deterministic":
      "determinista (identificador / proveedor+código / igual que)",
    "grid.id": "ID",
    "grid.name": "Nombre",
    "grid.courseCode": "Código del curso",
    "grid.level": "Nivel",
    "grid.status": "Estado",
    "grid.primaryIdentifier": "Identificador principal",
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
    "splash.hero.title": "Un registro fiable para cada curso",
    "splash.hero.subtitle":
      "Registra los cursos una sola vez, encuéntralos al instante y evita los duplicados antes de que aparezcan, con auditoría completa integrada.",
    "splash.benefits.1.title": "Menos duplicados",
    "splash.benefits.1.body":
      "La coincidencia señala un posible duplicado en el momento de crear el curso.",
    "splash.benefits.2.title": "Encuentra cualquier curso",
    "splash.benefits.2.body":
      "La búsqueda de texto completo y difusa encuentra cursos pese a erratas y variantes de escritura.",
    "splash.benefits.3.title": "Decisiones seguras",
    "splash.benefits.3.body":
      "Cada coincidencia muestra su puntuación y un desglose por campo, para que quien revisa vea exactamente por qué.",
    "splash.benefits.4.title": "Un catálogo fiable",
    "splash.benefits.4.body":
      "Los códigos de curso y los identificadores mantienen un único registro por curso.",
    "splash.benefits.5.title": "Todo queda registrado",
    "splash.benefits.5.body":
      "Cada cambio se audita, así que siempre sabrás quién hizo qué y cuándo.",
    "splash.benefits.6.title": "Encaja con tus sistemas",
    "splash.benefits.6.body":
      "Una API REST documentada se integra con las herramientas que ya usas.",
    "splash.features.1.title": "Buscar y explorar",
    "splash.features.1.body":
      "Busca por nombre o identificador, con opción difusa, en una tabla ordenable.",
    "splash.features.2.title": "Fichas de curso",
    "splash.features.2.body":
      "Código, nivel, créditos, identificadores, palabras clave y lo que enseña, con validación al instante.",
    "splash.features.3.title": "Comprobar coincidencias",
    "splash.features.3.body":
      "Puntúa un curso candidato frente al catálogo y consulta los motivos.",
    "splash.features.4.title": "Fusionar cursos",
    "splash.features.4.body":
      "Combina duplicados confirmados en un solo registro sin perder el historial.",
    "splash.features.5.title": "Tablero de ciclo de vida",
    "splash.features.5.body":
      "Arrastra los cursos entre borrador, publicado, archivado y retirado.",
    "splash.features.6.title": "Calendario de horarios",
    "splash.features.6.body":
      "Consulta cada edición del curso y sus fechas en un calendario.",
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
    "tour.intro":
      "Un recorrido guiado por el registro de cursos: qué hace cada pantalla y los pasos para usarla, desde registrar un curso hasta fusionar duplicados y seguir su ciclo de vida.",
    "tour.s1.title": "Registrar un curso",
    "tour.s1.summary":
      "Crea el registro de un curso con su código, nivel, créditos, identificadores y lo que enseña. Los posibles duplicados se señalan antes de confirmar.",
    "tour.s1.step.1":
      "Inicia sesión, abre el menú y elige Nuevo curso (o usa el botón Nuevo curso de la página Cursos).",
    "tour.s1.step.2":
      "Completa el Nombre, luego el Código del curso (propio del proveedor, como CS101), el Estado, el Nivel educativo y el Número de créditos.",
    "tour.s1.step.3":
      "Añade Palabras clave, Enseña (competencias), Nombres alternativos, URL de igual que y uno o más Identificadores con el botón + Añadir identificador.",
    "tour.s1.step.4":
      "Pulsa Crear. Si aparece Duplicados detectados, revisa los Posibles duplicados antes de reenviar; si no, llegarás al curso nuevo.",
    "tour.s2.title": "Buscar y abrir un curso",
    "tour.s2.summary":
      "Busca en el catálogo por nombre o identificador y abre un curso para ver todo lo registrado sobre él.",
    "tour.s2.step.1":
      "Abre Cursos desde el menú. La cuadrícula lista cada curso con su ID, Nombre, Código del curso, Nivel, Estado e Identificador principal.",
    "tour.s2.step.2":
      "Escribe un nombre o identificador en el cuadro de búsqueda y pulsa Buscar; marca Difusa para tolerar erratas y variantes ortográficas.",
    "tour.s2.step.3":
      "Consulta el recuento de registros sobre la cuadrícula y selecciona una fila para abrir la página de detalle de ese curso.",
    "tour.s2.step.4":
      "En la página de detalle lee Identidad, Identificadores, Enseña, Palabras clave e Instancias; usa Editar para modificarlo o Auditoría para ver su historial.",
    "tour.s3.title": "Comprobar duplicados",
    "tour.s3.summary":
      "Describe un curso y puntúalo frente al catálogo, con un desglose por campo que muestra por qué coincidió cada candidato.",
    "tour.s3.step.1":
      "Abre Comprobar coincidencias desde el menú; esta pantalla no guarda nada.",
    "tour.s3.step.2":
      "Introduce lo que sepas: Nombre, Código del curso, ID del proveedor, Nivel educativo, Palabras clave, Enseña, URL de igual que o Identificadores.",
    "tour.s3.step.3":
      "Pulsa Buscar coincidencias. Los candidatos aparecen en Resultados de coincidencia, cada uno con su puntuación; usa Umbral de visualización (0.0 a 1.0) para ocultar los débiles.",
    "tour.s3.step.4":
      "Abre Desglose de puntuación en un candidato para ver las puntuaciones de nombre, código, proveedor, nivel, palabras clave y enseña, o un acierto determinista por identificador, proveedor más código o URL equivalente.",
    "tour.s4.title": "Fusionar duplicados confirmados",
    "tour.s4.summary":
      "Integra un curso duplicado en el registro que sobrevive, conservando un registro de fusión y sin borrar nada de forma definitiva.",
    "tour.s4.step.1":
      "Abre Fusionar desde el menú (requiere iniciar sesión). Introduce el ID del curso principal (el registro que sobrevive) y el ID del curso duplicado (que se eliminará de forma lógica).",
    "tour.s4.step.2":
      "Si quieres, añade un Motivo como Duplicado confirmado; queda en el rastro de auditoría de la fusión.",
    "tour.s4.step.3":
      "Pulsa Cargar vista previa para ver el curso principal y el duplicado lado a lado y comprobar que es el par correcto.",
    "tour.s4.step.4":
      "Pulsa Fusionar y confirma. Fusión completada muestra el nuevo registro de fusión, con un enlace para Ver el curso principal fusionado.",
    "tour.s5.title": "Mover cursos por su ciclo de vida",
    "tour.s5.summary":
      "Ve cada curso como una tarjeta en una columna por estado y cambia su estado arrastrándolo.",
    "tour.s5.step.1":
      "Abre Tablero desde el menú. Las columnas recorren el ciclo de vida: draft, published, archived y retired.",
    "tour.s5.step.2":
      "Cada tarjeta muestra el nombre y el código del curso, así que reconoces el curso correcto de un vistazo.",
    "tour.s5.step.3":
      "Arrastra una tarjeta a otra columna para cambiar el estado de ese curso; el cambio se guarda enseguida en su registro.",
    "tour.s5.step.4":
      "Después el tablero se recarga desde el servicio, de modo que una tarjeta cuyo cambio no pudo guardarse vuelve a donde está realmente el registro y se muestra un error.",
    "tour.s6.title": "Ver las instancias de los cursos en el calendario",
    "tour.s6.summary":
      "Cada oferta programada de cada curso ordenada por fecha, de solo lectura, con un clic para abrir el curso al que pertenece.",
    "tour.s6.step.1":
      "Abre Calendario desde el menú. Se abre en vista mensual.",
    "tour.s6.step.2":
      "La ventana de programación de cada instancia de curso aparece como un tramo de día completo con el nombre de la instancia, o el del curso si no tiene.",
    "tour.s6.step.3":
      "Las sesiones individuales aparecen como eventos con hora en sus días, con la etiqueta de la sesión cuando la hay.",
    "tour.s6.step.4":
      "Selecciona cualquier entrada para abrir el curso al que pertenece, donde la sección Instancias lista sus fechas, modalidad y capacidad.",
    "signin.sso": "Iniciar sesión con SSO",
  },
  "fr-001": {
    "nav.calendar": "Calendrier",
    "nav.board": "Tableau",
    "brand.name": "Cours",
    "brand.tagline": "Main X Index",
    "nav.toggle": "Basculer la navigation",
    "nav.dashboard": "Tableau de bord",
    "nav.courses": "Cours",
    "nav.newCourse": "Nouveau cours",
    "nav.matchCheck": "Vérifier les correspondances",
    "nav.merge": "Fusionner",
    "chrome.theme": "Thème",
    "chrome.language": "Langue",
    "chrome.share": "Partager",
    "chrome.textSize": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "dashboard.title": "Tableau de bord",
    "dashboard.servicePrefix": "Service :",
    "dashboard.status.ok": "ok",
    "dashboard.status.down": "hors service",
    "dashboard.status.loading": "chargement",
    "dashboard.recentActivity": "Activité récente",
    "dashboard.noRecent": "Aucune entrée d'audit récente.",
    "courses.title": "Cours",
    "courses.new": "Nouveau cours",
    "courses.searchPlaceholder": "Rechercher par nom, identifiant…",
    "courses.fuzzy": "Approximatif",
    "courses.loading": "Chargement…",
    "courses.recordCount.one": "{n} enregistrement",
    "courses.recordCount.other": "{n} enregistrements",
    "detail.loading": "Chargement…",
    "detail.edit": "Modifier",
    "detail.audit": "Audit",
    "detail.delete": "Supprimer",
    "detail.exportGdpr": "Exporter les données (RGPD)",
    "detail.exportingGdpr": "Exportation…",
    "detail.confirmDelete":
      "Supprimer logiquement ce cours ? Cette action est irréversible depuis l'interface.",
    "detail.identity": "Identité",
    "detail.id": "ID",
    "detail.courseCode": "Code du cours",
    "detail.status": "Statut",
    "detail.educationalLevel": "Niveau d'enseignement",
    "detail.numberOfCredits": "Nombre de crédits",
    "detail.timeRequired": "Temps requis",
    "detail.description": "Description",
    "detail.url": "URL",
    "detail.license": "Licence",
    "detail.free": "Gratuit",
    "detail.yes": "oui",
    "detail.no": "non",
    "detail.empty": "—",
    "detail.identifiers": "Identifiants",
    "detail.teaches": "Enseigne",
    "detail.keywords": "Mots-clés",
    "detail.alternateNames": "Noms alternatifs",
    "detail.sameAs": "Identique à (URL faisant autorité)",
    "detail.instances": "Sessions",
    "detail.customPrefix": "Personnalisé :",
    "detail.noDate": "(pas de date)",
    "detail.capacity": "cap",
    "edit.title": "Modifier le cours",
    "edit.cancel": "Annuler",
    "edit.loading": "Chargement…",
    "edit.saveChanges": "Enregistrer les modifications",
    "new.title": "Nouveau cours",
    "new.create": "Créer",
    "new.possibleDuplicates": "Doublons possibles",
    "new.duplicatesDetected":
      "Doublons détectés ({n}) — vérifiez ci-dessous avant de resoumettre.",
    "audit.title": "Journal d'audit",
    "audit.backToCourse": "Retour au cours",
    "audit.loading": "Chargement…",
    "audit.noEntries": "Aucune entrée d'audit.",
    "audit.by": "par",
    "audit.payload": "Charge utile",
    "match.title": "Vérifier les correspondances",
    "match.name": "Nom",
    "match.courseCode": "Code du cours",
    "match.providerId": "ID du fournisseur",
    "match.displayThreshold": "Seuil d'affichage",
    "match.thresholdHint": "Filtre côté client (0.0 – 1.0)",
    "match.educationalLevel": "Niveau d'enseignement",
    "match.keywords": "Mots-clés",
    "match.keywordsHint": "Séparés par des virgules ou des retours à la ligne",
    "match.teaches": "Enseigne (compétences)",
    "match.teachesHint": "Un par ligne",
    "match.sameAs": "URL identiques à",
    "match.sameAsHint": "Un par ligne",
    "match.identifiers": "Identifiants",
    "match.matching": "Correspondance…",
    "match.findMatches": "Trouver des correspondances",
    "merge.title": "Fusionner les cours",
    "merge.mainId": "ID du cours principal",
    "merge.mainIdHint": "L'enregistrement survivant",
    "merge.duplicateId": "ID du cours en double",
    "merge.duplicateIdHint": "Sera supprimé logiquement",
    "merge.reason": "Motif",
    "merge.reasonHint": "Enregistré dans le journal d'audit de fusion",
    "merge.reasonPlaceholder": "Doublon confirmé",
    "merge.loadPreview": "Charger l'aperçu",
    "merge.merging": "Fusion…",
    "merge.merge": "Fusionner",
    "merge.bothIdsRequired": "Les deux ID sont requis",
    "merge.mustDiffer": "Le principal et le doublon doivent être différents",
    "merge.confirm":
      "Fusionner {dup}… dans {main}… ?\nCela supprime logiquement le doublon.",
    "merge.preview": "Aperçu",
    "merge.main": "Principal",
    "merge.duplicate": "Doublon",
    "merge.completed": "Fusion terminée",
    "merge.recordCreated": "Enregistrement de fusion {id} créé le {at}.",
    "merge.viewMerged": "Voir le cours principal fusionné",
    "form.name": "Nom",
    "form.courseCode": "Code du cours",
    "form.courseCodeHint": "Limité au fournisseur (p. ex. CS101)",
    "form.status": "Statut",
    "form.description": "Description",
    "form.url": "URL",
    "form.license": "Licence",
    "form.numberOfCredits": "Nombre de crédits",
    "form.educationalLevel": "Niveau d'enseignement",
    "form.typicalAgeRange": "Tranche d'âge typique",
    "form.typicalAgeRangeHint": "p. ex. 18-22",
    "form.timeRequired": "Temps requis",
    "form.timeRequiredHint": "ISO 8601 (p. ex. PT45H)",
    "form.alternateNames": "Noms alternatifs",
    "form.alternateNamesHint": "Un par ligne",
    "form.keywords": "Mots-clés",
    "form.keywordsHint": "Séparés par des virgules ou des retours à la ligne",
    "form.teaches": "Enseigne (compétences)",
    "form.teachesHint": "Un par ligne",
    "form.availableLanguages": "Langues disponibles",
    "form.availableLanguagesHint": "Codes BCP-47 (p. ex. en fr de)",
    "form.sameAs": "URL identiques à",
    "form.sameAsHint": "Wikidata, catalogue OER, etc. — un par ligne",
    "form.identifiers": "Identifiants",
    "form.saving": "Enregistrement…",
    "form.save": "Enregistrer",
    "form.reset": "Réinitialiser",
    "identifier.type": "Type",
    "identifier.customOption": "Personnalisé…",
    "identifier.customLabel": "Libellé personnalisé",
    "identifier.value": "Valeur",
    "identifier.url": "URL",
    "identifier.remove": "Supprimer",
    "identifier.add": "+ Ajouter un identifiant",
    "search.placeholder": "Rechercher…",
    "search.submit": "Rechercher",
    "results.title": "Résultats de correspondance",
    "results.count": "({n})",
    "results.none": "Aucun candidat.",
    "results.scoreBreakdown": "Détail du score",
    "results.score.name": "nom",
    "results.score.courseCode": "code du cours",
    "results.score.provider": "fournisseur",
    "results.score.level": "niveau",
    "results.score.keywords": "mots-clés",
    "results.score.teaches": "enseigne",
    "results.score.deterministic":
      "déterministe (identifiant / fournisseur+code / identique à)",
    "grid.id": "ID",
    "grid.name": "Nom",
    "grid.courseCode": "Code du cours",
    "grid.level": "Niveau",
    "grid.status": "Statut",
    "grid.primaryIdentifier": "Identifiant principal",
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
    "splash.hero.title": "Une fiche fiable pour chaque cours",
    "splash.hero.subtitle":
      "Enregistrez chaque cours une seule fois, retrouvez-le instantanément et évitez les doublons avant qu'ils n'apparaissent, avec piste d'audit intégrée.",
    "splash.benefits.1.title": "Moins de doublons",
    "splash.benefits.1.body":
      "Le rapprochement signale un doublon probable dès la création du cours.",
    "splash.benefits.2.title": "Trouvez vite n'importe quel cours",
    "splash.benefits.2.body":
      "La recherche plein texte et approximative retrouve les cours malgré les fautes de frappe et les variantes d'orthographe.",
    "splash.benefits.3.title": "Décisions éclairées",
    "splash.benefits.3.body":
      "Chaque correspondance affiche son score et le détail par champ, pour que les relecteurs voient exactement pourquoi.",
    "splash.benefits.4.title": "Un catalogue fiable",
    "splash.benefits.4.body":
      "Les codes de cours et les identifiants garantissent une seule fiche par cours.",
    "splash.benefits.5.title": "Tout est consigné",
    "splash.benefits.5.body":
      "Chaque modification est auditée : vous savez toujours qui a fait quoi, et quand.",
    "splash.benefits.6.title": "S'intègre à vos systèmes",
    "splash.benefits.6.body":
      "Une API REST documentée se branche sur les outils que vous utilisez déjà.",
    "splash.features.1.title": "Rechercher et parcourir",
    "splash.features.1.body":
      "Recherchez par nom ou identifiant, avec option approximative, dans une grille triable.",
    "splash.features.2.title": "Fiches de cours",
    "splash.features.2.body":
      "Code, niveau, crédits, identifiants, mots-clés et contenu enseigné, avec validation instantanée.",
    "splash.features.3.title": "Vérifier les correspondances",
    "splash.features.3.body":
      "Évaluez un cours candidat par rapport au catalogue et consultez les raisons.",
    "splash.features.4.title": "Fusionner des cours",
    "splash.features.4.body":
      "Regroupez les doublons confirmés en une seule fiche sans perdre l'historique.",
    "splash.features.5.title": "Tableau de cycle de vie",
    "splash.features.5.body":
      "Faites glisser les cours entre brouillon, publié, archivé et retiré.",
    "splash.features.6.title": "Calendrier des sessions",
    "splash.features.6.body":
      "Visualisez chaque session de cours et ses dates dans un calendrier.",
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
    "tour.intro":
      "Une visite guidée du registre des cours : le rôle de chaque écran et les étapes pour l'utiliser, de l'enregistrement d'un cours à la fusion des doublons et au suivi de son cycle de vie.",
    "tour.s1.title": "Enregistrer un cours",
    "tour.s1.summary":
      "Créez la fiche d'un cours avec son code, son niveau, ses crédits, ses identifiants et ce qu'il enseigne. Les doublons probables sont signalés avant validation.",
    "tour.s1.step.1":
      "Connectez-vous, ouvrez le menu et choisissez Nouveau cours (ou utilisez le bouton Nouveau cours de la page Cours).",
    "tour.s1.step.2":
      "Renseignez le Nom, puis le Code du cours (propre au fournisseur, comme CS101), le Statut, le Niveau d'études et le Nombre de crédits.",
    "tour.s1.step.3":
      "Ajoutez des Mots-clés, ce qu'il Enseigne (compétences), des Noms alternatifs, des URL équivalentes et un ou plusieurs Identifiants avec le bouton + Ajouter un identifiant.",
    "tour.s1.step.4":
      "Cliquez sur Créer. Si Doublons détectés apparaît, examinez les Doublons possibles avant de renvoyer ; sinon vous arrivez sur le nouveau cours.",
    "tour.s2.title": "Trouver et ouvrir un cours",
    "tour.s2.summary":
      "Recherchez dans le catalogue par nom ou identifiant, puis ouvrez un cours pour voir tout ce qui le concerne.",
    "tour.s2.step.1":
      "Ouvrez Cours depuis le menu. La grille liste chaque cours avec son ID, Nom, Code du cours, Niveau, Statut et Identifiant principal.",
    "tour.s2.step.2":
      "Saisissez un nom ou un identifiant dans la zone de recherche et cliquez sur Rechercher ; cochez Approximative pour tolérer les fautes et variantes d'orthographe.",
    "tour.s2.step.3":
      "Consultez le nombre d'enregistrements au-dessus de la grille, puis sélectionnez une ligne pour ouvrir la page de détail du cours.",
    "tour.s2.step.4":
      "Sur la page de détail, lisez Identité, Identifiants, Enseigne, Mots-clés et Sessions, puis utilisez Modifier pour le changer ou Audit pour voir son historique.",
    "tour.s3.title": "Vérifier les doublons",
    "tour.s3.summary":
      "Décrivez un cours et évaluez-le par rapport au catalogue, avec le détail par champ expliquant pourquoi chaque candidat correspond.",
    "tour.s3.step.1":
      "Ouvrez Vérification des correspondances depuis le menu ; cet écran n'enregistre rien.",
    "tour.s3.step.2":
      "Saisissez ce que vous savez : Nom, Code du cours, ID du fournisseur, Niveau d'études, Mots-clés, Enseigne, URL équivalentes ou Identifiants.",
    "tour.s3.step.3":
      "Cliquez sur Trouver des correspondances. Les candidats apparaissent sous Résultats de correspondance, chacun avec un score ; utilisez Seuil d'affichage (0.0 à 1.0) pour masquer les plus faibles.",
    "tour.s3.step.4":
      "Ouvrez Détail du score sur un candidat pour voir les scores du nom, du code, du fournisseur, du niveau, des mots-clés et de ce qu'il enseigne, ou une correspondance déterministe sur un identifiant, fournisseur plus code, ou URL équivalente.",
    "tour.s4.title": "Fusionner les doublons confirmés",
    "tour.s4.summary":
      "Intégrez un cours en double dans la fiche conservée, avec une trace de fusion et sans jamais rien supprimer définitivement.",
    "tour.s4.step.1":
      "Ouvrez Fusionner depuis le menu (connexion requise). Saisissez l'ID du cours principal (la fiche conservée) et l'ID du cours en double (qui sera supprimé logiquement).",
    "tour.s4.step.2":
      "Vous pouvez ajouter un Motif comme Doublon confirmé ; il est consigné dans la piste d'audit de la fusion.",
    "tour.s4.step.3":
      "Cliquez sur Charger l'aperçu pour voir le cours principal et le doublon côte à côte et vérifier que la paire est la bonne.",
    "tour.s4.step.4":
      "Cliquez sur Fusionner et confirmez. Fusion terminée affiche la nouvelle trace de fusion, avec un lien pour Voir le cours principal fusionné.",
    "tour.s5.title": "Faire avancer les cours dans leur cycle de vie",
    "tour.s5.summary":
      "Voyez chaque cours comme une carte dans une colonne par statut, et changez son statut en le faisant glisser.",
    "tour.s5.step.1":
      "Ouvrez Tableau depuis le menu. Les colonnes suivent le cycle de vie : draft, published, archived et retired.",
    "tour.s5.step.2":
      "Chaque carte affiche le nom et le code du cours, pour repérer d'un coup d'œil le bon cours.",
    "tour.s5.step.3":
      "Faites glisser une carte vers une autre colonne pour changer le statut de ce cours ; la modification est aussitôt enregistrée dans sa fiche.",
    "tour.s5.step.4":
      "Le tableau se recharge ensuite depuis le service : une carte dont la modification n'a pas pu être enregistrée revient là où se trouve réellement la fiche, et une erreur s'affiche.",
    "tour.s6.title": "Voir les sessions de cours dans le calendrier",
    "tour.s6.summary":
      "Chaque session programmée de chaque cours placée par date, en lecture seule, avec un clic pour ouvrir le cours concerné.",
    "tour.s6.step.1":
      "Ouvrez Calendrier depuis le menu. Il s'ouvre en vue mensuelle.",
    "tour.s6.step.2":
      "La période programmée de chaque session de cours apparaît comme une plage sur journée entière, étiquetée du nom de la session, ou du nom du cours à défaut.",
    "tour.s6.step.3":
      "Les séances individuelles apparaissent comme des événements horaires à leur date, avec le libellé de la séance quand il existe.",
    "tour.s6.step.4":
      "Sélectionnez une entrée pour ouvrir le cours concerné, où la section Sessions liste ses dates, son mode et sa capacité.",
    "signin.sso": "Se connecter avec SSO",
  },
  "hi-001": {
    "nav.calendar": "कैलेंडर",
    "nav.board": "बोर्ड",
    "brand.name": "पाठ्यक्रम",
    "brand.tagline": "Main X Index",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "nav.dashboard": "डैशबोर्ड",
    "nav.courses": "पाठ्यक्रम",
    "nav.newCourse": "नया पाठ्यक्रम",
    "nav.matchCheck": "मिलान जाँच",
    "nav.merge": "मर्ज करें",
    "chrome.theme": "थीम",
    "chrome.language": "भाषा",
    "chrome.share": "साझा करें",
    "chrome.textSize": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "dashboard.title": "डैशबोर्ड",
    "dashboard.servicePrefix": "सेवा:",
    "dashboard.status.ok": "ठीक",
    "dashboard.status.down": "बंद",
    "dashboard.status.loading": "लोड हो रहा है",
    "dashboard.recentActivity": "हाल की गतिविधि",
    "dashboard.noRecent": "कोई हाल की ऑडिट प्रविष्टियाँ नहीं।",
    "courses.title": "पाठ्यक्रम",
    "courses.new": "नया पाठ्यक्रम",
    "courses.searchPlaceholder": "नाम, पहचानकर्ता से खोजें…",
    "courses.fuzzy": "अस्पष्ट",
    "courses.loading": "लोड हो रहा है…",
    "courses.recordCount.one": "{n} रिकॉर्ड",
    "courses.recordCount.other": "{n} रिकॉर्ड",
    "detail.loading": "लोड हो रहा है…",
    "detail.edit": "संपादित करें",
    "detail.audit": "ऑडिट",
    "detail.delete": "हटाएँ",
    "detail.exportGdpr": "डेटा निर्यात करें (GDPR)",
    "detail.exportingGdpr": "निर्यात हो रहा है…",
    "detail.confirmDelete":
      "इस पाठ्यक्रम को सॉफ़्ट-डिलीट करें? इसे इंटरफ़ेस के माध्यम से पूर्ववत नहीं किया जा सकता।",
    "detail.identity": "पहचान",
    "detail.id": "ID",
    "detail.courseCode": "पाठ्यक्रम कोड",
    "detail.status": "स्थिति",
    "detail.educationalLevel": "शैक्षिक स्तर",
    "detail.numberOfCredits": "क्रेडिट की संख्या",
    "detail.timeRequired": "आवश्यक समय",
    "detail.description": "विवरण",
    "detail.url": "URL",
    "detail.license": "लाइसेंस",
    "detail.free": "निःशुल्क",
    "detail.yes": "हाँ",
    "detail.no": "नहीं",
    "detail.empty": "—",
    "detail.identifiers": "पहचानकर्ता",
    "detail.teaches": "सिखाता है",
    "detail.keywords": "कीवर्ड",
    "detail.alternateNames": "वैकल्पिक नाम",
    "detail.sameAs": "के समान (आधिकारिक URL)",
    "detail.instances": "उदाहरण",
    "detail.customPrefix": "कस्टम:",
    "detail.noDate": "(कोई तिथि नहीं)",
    "detail.capacity": "क्षमता",
    "edit.title": "पाठ्यक्रम संपादित करें",
    "edit.cancel": "रद्द करें",
    "edit.loading": "लोड हो रहा है…",
    "edit.saveChanges": "परिवर्तन सहेजें",
    "new.title": "नया पाठ्यक्रम",
    "new.create": "बनाएँ",
    "new.possibleDuplicates": "संभावित डुप्लिकेट",
    "new.duplicatesDetected":
      "डुप्लिकेट पाए गए ({n}) — पुनः सबमिट करने से पहले नीचे समीक्षा करें।",
    "audit.title": "ऑडिट लॉग",
    "audit.backToCourse": "पाठ्यक्रम पर वापस जाएँ",
    "audit.loading": "लोड हो रहा है…",
    "audit.noEntries": "कोई ऑडिट प्रविष्टियाँ नहीं।",
    "audit.by": "द्वारा",
    "audit.payload": "पेलोड",
    "match.title": "मिलान जाँच",
    "match.name": "नाम",
    "match.courseCode": "पाठ्यक्रम कोड",
    "match.providerId": "प्रदाता ID",
    "match.displayThreshold": "प्रदर्शन सीमा",
    "match.thresholdHint": "क्लाइंट-साइड फ़िल्टर (0.0 – 1.0)",
    "match.educationalLevel": "शैक्षिक स्तर",
    "match.keywords": "कीवर्ड",
    "match.keywordsHint": "अल्पविराम या नई पंक्ति से अलग",
    "match.teaches": "सिखाता है (दक्षताएँ)",
    "match.teachesHint": "प्रति पंक्ति एक",
    "match.sameAs": "के समान URL",
    "match.sameAsHint": "प्रति पंक्ति एक",
    "match.identifiers": "पहचानकर्ता",
    "match.matching": "मिलान हो रहा है…",
    "match.findMatches": "मिलान खोजें",
    "merge.title": "पाठ्यक्रम मर्ज करें",
    "merge.mainId": "मुख्य पाठ्यक्रम ID",
    "merge.mainIdHint": "बचा रहने वाला रिकॉर्ड",
    "merge.duplicateId": "डुप्लिकेट पाठ्यक्रम ID",
    "merge.duplicateIdHint": "सॉफ़्ट-डिलीट किया जाएगा",
    "merge.reason": "कारण",
    "merge.reasonHint": "मर्ज ऑडिट ट्रेल में दर्ज",
    "merge.reasonPlaceholder": "पुष्ट डुप्लिकेट",
    "merge.loadPreview": "पूर्वावलोकन लोड करें",
    "merge.merging": "मर्ज हो रहा है…",
    "merge.merge": "मर्ज करें",
    "merge.bothIdsRequired": "दोनों ID आवश्यक हैं",
    "merge.mustDiffer": "मुख्य और डुप्लिकेट भिन्न होने चाहिए",
    "merge.confirm":
      "{dup}… को {main}… में मर्ज करें?\nयह डुप्लिकेट को सॉफ़्ट-डिलीट कर देगा।",
    "merge.preview": "पूर्वावलोकन",
    "merge.main": "मुख्य",
    "merge.duplicate": "डुप्लिकेट",
    "merge.completed": "मर्ज पूर्ण हुआ",
    "merge.recordCreated": "मर्ज रिकॉर्ड {id} {at} पर बनाया गया।",
    "merge.viewMerged": "मर्ज किया गया मुख्य पाठ्यक्रम देखें",
    "form.name": "नाम",
    "form.courseCode": "पाठ्यक्रम कोड",
    "form.courseCodeHint": "प्रदाता-स्कोप (जैसे CS101)",
    "form.status": "स्थिति",
    "form.description": "विवरण",
    "form.url": "URL",
    "form.license": "लाइसेंस",
    "form.numberOfCredits": "क्रेडिट की संख्या",
    "form.educationalLevel": "शैक्षिक स्तर",
    "form.typicalAgeRange": "सामान्य आयु सीमा",
    "form.typicalAgeRangeHint": "जैसे 18-22",
    "form.timeRequired": "आवश्यक समय",
    "form.timeRequiredHint": "ISO 8601 (जैसे PT45H)",
    "form.alternateNames": "वैकल्पिक नाम",
    "form.alternateNamesHint": "प्रति पंक्ति एक",
    "form.keywords": "कीवर्ड",
    "form.keywordsHint": "अल्पविराम या नई पंक्ति से अलग",
    "form.teaches": "सिखाता है (दक्षताएँ)",
    "form.teachesHint": "प्रति पंक्ति एक",
    "form.availableLanguages": "उपलब्ध भाषाएँ",
    "form.availableLanguagesHint": "BCP-47 कोड (जैसे en fr de)",
    "form.sameAs": "के समान URL",
    "form.sameAsHint": "Wikidata, OER कैटलॉग, आदि — प्रति पंक्ति एक",
    "form.identifiers": "पहचानकर्ता",
    "form.saving": "सहेजा जा रहा है…",
    "form.save": "सहेजें",
    "form.reset": "रीसेट करें",
    "identifier.type": "प्रकार",
    "identifier.customOption": "कस्टम…",
    "identifier.customLabel": "कस्टम लेबल",
    "identifier.value": "मान",
    "identifier.url": "URL",
    "identifier.remove": "हटाएँ",
    "identifier.add": "+ पहचानकर्ता जोड़ें",
    "search.placeholder": "खोजें…",
    "search.submit": "खोज",
    "results.title": "मिलान परिणाम",
    "results.count": "({n})",
    "results.none": "कोई उम्मीदवार नहीं।",
    "results.scoreBreakdown": "स्कोर विवरण",
    "results.score.name": "नाम",
    "results.score.courseCode": "पाठ्यक्रम कोड",
    "results.score.provider": "प्रदाता",
    "results.score.level": "स्तर",
    "results.score.keywords": "कीवर्ड",
    "results.score.teaches": "सिखाता है",
    "results.score.deterministic":
      "नियतात्मक (पहचानकर्ता / प्रदाता+कोड / के समान)",
    "grid.id": "ID",
    "grid.name": "नाम",
    "grid.courseCode": "पाठ्यक्रम कोड",
    "grid.level": "स्तर",
    "grid.status": "स्थिति",
    "grid.primaryIdentifier": "प्राथमिक पहचानकर्ता",
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
    "splash.hero.title": "हर पाठ्यक्रम के लिए एक भरोसेमंद रिकॉर्ड",
    "splash.hero.subtitle":
      "पाठ्यक्रमों को एक बार दर्ज करें, उन्हें तुरंत खोजें, और डुप्लिकेट बनने से पहले ही रोकें, पूरे ऑडिट ट्रेल के साथ।",
    "splash.benefits.1.title": "कम डुप्लिकेट",
    "splash.benefits.1.body":
      "कोर्स बनते ही मैचिंग संभावित डुप्लिकेट को चिह्नित कर देती है।",
    "splash.benefits.2.title": "कोई भी पाठ्यक्रम तेज़ी से खोजें",
    "splash.benefits.2.body":
      "फ़ुल-टेक्स्ट और अस्पष्ट खोज टाइपो और वर्तनी के अंतर के बावजूद पाठ्यक्रम ढूँढ लेती है।",
    "splash.benefits.3.title": "भरोसेमंद निर्णय",
    "splash.benefits.3.body":
      "हर मैच अपना स्कोर और फ़ील्ड-वार विवरण दिखाता है, ताकि समीक्षक ठीक-ठीक कारण देख सकें।",
    "splash.benefits.4.title": "एक भरोसेमंद सूची",
    "splash.benefits.4.body":
      "पाठ्यक्रम कोड और पहचानकर्ता हर पाठ्यक्रम के लिए केवल एक रिकॉर्ड रखते हैं।",
    "splash.benefits.5.title": "कुछ भी दर्ज होने से नहीं छूटता",
    "splash.benefits.5.body":
      "हर बदलाव का ऑडिट होता है, इसलिए आप हमेशा बता सकते हैं कि किसने क्या और कब किया।",
    "splash.benefits.6.title": "आपके सिस्टम के अनुकूल",
    "splash.benefits.6.body":
      "दस्तावेज़ीकृत REST API आपके मौजूदा टूल से जुड़ जाती है।",
    "splash.features.1.title": "खोजें और ब्राउज़ करें",
    "splash.features.1.body":
      "नाम या पहचानकर्ता से खोजें, अस्पष्ट विकल्प के साथ, क्रमबद्ध ग्रिड में।",
    "splash.features.2.title": "पाठ्यक्रम रिकॉर्ड",
    "splash.features.2.body":
      "पाठ्यक्रम कोड, स्तर, क्रेडिट, पहचानकर्ता, कीवर्ड और विषय-वस्तु, तुरंत सत्यापन के साथ।",
    "splash.features.3.title": "मिलान जाँच",
    "splash.features.3.body":
      "किसी उम्मीदवार पाठ्यक्रम को सूची से मिलाकर स्कोर करें और कारण देखें।",
    "splash.features.4.title": "पाठ्यक्रम मर्ज करें",
    "splash.features.4.body":
      "पुष्ट डुप्लिकेट को इतिहास खोए बिना एक रिकॉर्ड में मिलाएँ।",
    "splash.features.5.title": "जीवनचक्र बोर्ड",
    "splash.features.5.body":
      "पाठ्यक्रमों को ड्राफ़्ट, प्रकाशित, संग्रहीत और सेवानिवृत्त के बीच खींचें।",
    "splash.features.6.title": "शेड्यूल कैलेंडर",
    "splash.features.6.body":
      "हर पाठ्यक्रम इंस्टेंस और उसकी तिथियाँ कैलेंडर पर देखें।",
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
    "tour.intro":
      "पाठ्यक्रम रजिस्ट्री का निर्देशित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, पाठ्यक्रम दर्ज करने से लेकर डुप्लिकेट मर्ज करने और उसके जीवनचक्र को ट्रैक करने तक।",
    "tour.s1.title": "पाठ्यक्रम दर्ज करना",
    "tour.s1.summary":
      "पाठ्यक्रम का रिकॉर्ड उसके कोड, स्तर, क्रेडिट, पहचानकर्ताओं और वह जो सिखाता है उसके साथ बनाएँ। संभावित डुप्लिकेट सहेजने से पहले चिह्नित हो जाते हैं।",
    "tour.s1.step.1":
      "साइन इन करें, मेनू खोलें और नया पाठ्यक्रम चुनें (या पाठ्यक्रम पेज पर नया पाठ्यक्रम बटन इस्तेमाल करें)।",
    "tour.s1.step.2":
      "नाम भरें, फिर पाठ्यक्रम कोड (प्रदाता-विशिष्ट, जैसे CS101), स्थिति, शैक्षिक स्तर और क्रेडिट की संख्या।",
    "tour.s1.step.3":
      "कीवर्ड, सिखाता है (दक्षताएँ), वैकल्पिक नाम, समान-URL और + पहचानकर्ता जोड़ें बटन से एक या अधिक पहचानकर्ता जोड़ें।",
    "tour.s1.step.4":
      "बनाएँ दबाएँ। यदि डुप्लिकेट मिले दिखे तो दोबारा भेजने से पहले संभावित डुप्लिकेट देखें; वरना आप नए पाठ्यक्रम पर पहुँचेंगे।",
    "tour.s2.title": "पाठ्यक्रम खोजना और खोलना",
    "tour.s2.summary":
      "नाम या पहचानकर्ता से सूची खोजें, फिर किसी पाठ्यक्रम को खोलकर उसके बारे में दर्ज सब कुछ देखें।",
    "tour.s2.step.1":
      "मेनू से पाठ्यक्रम खोलें। ग्रिड हर पाठ्यक्रम को उसकी आईडी, नाम, पाठ्यक्रम कोड, स्तर, स्थिति और प्राथमिक पहचानकर्ता के साथ दिखाता है।",
    "tour.s2.step.2":
      "खोज बॉक्स में नाम या पहचानकर्ता लिखकर खोजें दबाएँ; टाइपो और वर्तनी के अंतर सहने के लिए अस्पष्ट (Fuzzy) चुनें।",
    "tour.s2.step.3":
      "ग्रिड के ऊपर रिकॉर्ड की संख्या देखें, फिर किसी पंक्ति को चुनकर उस पाठ्यक्रम का विवरण पेज खोलें।",
    "tour.s2.step.4":
      "विवरण पेज पर पहचान, पहचानकर्ता, सिखाता है, कीवर्ड और उदाहरण पढ़ें; बदलने के लिए संपादित करें या इतिहास देखने के लिए ऑडिट इस्तेमाल करें।",
    "tour.s3.title": "डुप्लिकेट की जाँच",
    "tour.s3.summary":
      "किसी पाठ्यक्रम का वर्णन करें और सूची के विरुद्ध उसका स्कोर देखें; हर फ़ील्ड का ब्योरा बताता है कि प्रत्याशी क्यों मेल खाया।",
    "tour.s3.step.1":
      "मेनू से मिलान जाँच खोलें; यह स्क्रीन कुछ भी सहेजती नहीं है।",
    "tour.s3.step.2":
      "जो जानते हैं वह भरें: नाम, पाठ्यक्रम कोड, प्रदाता आईडी, शैक्षिक स्तर, कीवर्ड, सिखाता है, समान-URL या पहचानकर्ता।",
    "tour.s3.step.3":
      "मिलान खोजें दबाएँ। प्रत्याशी मिलान परिणाम के नीचे स्कोर के साथ सूचीबद्ध होते हैं; कमज़ोर को छिपाने के लिए प्रदर्शन सीमा (0.0 से 1.0) इस्तेमाल करें।",
    "tour.s3.step.4":
      "किसी प्रत्याशी पर स्कोर ब्योरा खोलें और नाम, कोर्स कोड, प्रदाता, स्तर, कीवर्ड और सिखाता है के स्कोर देखें, या पहचानकर्ता, प्रदाता-सहित-कोड, या समान-URL पर निर्धारक मिलान देखें।",
    "tour.s4.title": "पुष्ट डुप्लिकेट मर्ज करना",
    "tour.s4.summary":
      "किसी डुप्लिकेट पाठ्यक्रम को बचने वाले रिकॉर्ड में मिलाएँ; मर्ज रिकॉर्ड बना रहता है और कुछ भी स्थायी रूप से नहीं मिटता।",
    "tour.s4.step.1":
      "मेनू से मर्ज खोलें (साइन-इन आवश्यक)। मुख्य पाठ्यक्रम आईडी (बचने वाला रिकॉर्ड) और डुप्लिकेट पाठ्यक्रम आईडी (जो सॉफ़्ट-डिलीट होगा) दर्ज करें।",
    "tour.s4.step.2":
      "चाहें तो कारण जोड़ें, जैसे पुष्ट डुप्लिकेट; यह मर्ज ऑडिट ट्रेल में दर्ज होता है।",
    "tour.s4.step.3":
      "पूर्वावलोकन लोड करें दबाएँ ताकि मुख्य और डुप्लिकेट पाठ्यक्रम साथ-साथ दिखें और आप सही जोड़ी जाँच सकें।",
    "tour.s4.step.4":
      "मर्ज करें दबाकर पुष्टि करें। मर्ज पूरा हुआ नया मर्ज रिकॉर्ड दिखाता है, साथ में मर्ज किया गया मुख्य पाठ्यक्रम देखें लिंक।",
    "tour.s5.title": "पाठ्यक्रमों को उनके जीवनचक्र में आगे बढ़ाना",
    "tour.s5.summary":
      "हर पाठ्यक्रम को स्थिति के अनुसार एक स्तंभ में कार्ड के रूप में देखें और खींचकर उसकी स्थिति बदलें।",
    "tour.s5.step.1":
      "मेनू से बोर्ड खोलें। स्तंभ जीवनचक्र के अनुसार हैं: draft, published, archived और retired।",
    "tour.s5.step.2":
      "हर कार्ड पाठ्यक्रम का नाम और कोड दिखाता है, ताकि सही पाठ्यक्रम एक नज़र में पहचाना जा सके।",
    "tour.s5.step.3":
      "किसी कार्ड को दूसरे स्तंभ में खींचें तो उस पाठ्यक्रम की स्थिति बदल जाती है; बदलाव तुरंत पाठ्यक्रम के रिकॉर्ड में सहेजा जाता है।",
    "tour.s5.step.4":
      "इसके बाद बोर्ड सेवा से दोबारा लोड होता है, इसलिए जिस कार्ड का बदलाव सहेजा न जा सका वह रिकॉर्ड की असली जगह लौट आता है और त्रुटि दिखती है।",
    "tour.s6.title": "कैलेंडर पर पाठ्यक्रम उदाहरण देखना",
    "tour.s6.summary":
      "हर पाठ्यक्रम की हर निर्धारित पेशकश तारीख़ के अनुसार, केवल-पढ़ने योग्य, और एक क्लिक से संबंधित पाठ्यक्रम तक।",
    "tour.s6.step.1": "मेनू से कैलेंडर खोलें। यह माह दृश्य में खुलता है।",
    "tour.s6.step.2":
      "हर पाठ्यक्रम उदाहरण की समय-सारणी अवधि पूरे दिन की पट्टी के रूप में उदाहरण के नाम से दिखती है, नाम न हो तो पाठ्यक्रम के नाम से।",
    "tour.s6.step.3":
      "अलग-अलग सत्र अपने दिनों पर समयबद्ध घटनाओं के रूप में दिखते हैं, जहाँ सेट हो वहाँ सत्र के लेबल के साथ।",
    "tour.s6.step.4":
      "किसी भी प्रविष्टि को चुनकर उसका पाठ्यक्रम खोलें, जहाँ उदाहरण अनुभाग उसकी तारीखें, मोड और क्षमता सूचीबद्ध करता है।",
    "signin.sso": "SSO से साइन इन करें",
  },
  "zh-cn": {
    "nav.calendar": "日历",
    "nav.board": "看板",
    "brand.name": "课程",
    "brand.tagline": "Main X Index",
    "nav.toggle": "切换导航",
    "nav.dashboard": "仪表板",
    "nav.courses": "课程",
    "nav.newCourse": "新建课程",
    "nav.matchCheck": "匹配检查",
    "nav.merge": "合并",
    "chrome.theme": "主题",
    "chrome.language": "语言",
    "chrome.share": "分享",
    "chrome.textSize": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "dashboard.title": "仪表板",
    "dashboard.servicePrefix": "服务：",
    "dashboard.status.ok": "正常",
    "dashboard.status.down": "中断",
    "dashboard.status.loading": "加载中",
    "dashboard.recentActivity": "近期活动",
    "dashboard.noRecent": "没有近期审计记录。",
    "courses.title": "课程",
    "courses.new": "新建课程",
    "courses.searchPlaceholder": "按名称、标识符搜索…",
    "courses.fuzzy": "模糊",
    "courses.loading": "加载中…",
    "courses.recordCount.one": "{n} 条记录",
    "courses.recordCount.other": "{n} 条记录",
    "detail.loading": "加载中…",
    "detail.edit": "编辑",
    "detail.audit": "审计",
    "detail.delete": "删除",
    "detail.exportGdpr": "导出数据（GDPR）",
    "detail.exportingGdpr": "导出中…",
    "detail.confirmDelete": "软删除此课程？此操作无法通过界面撤销。",
    "detail.identity": "身份",
    "detail.id": "ID",
    "detail.courseCode": "课程代码",
    "detail.status": "状态",
    "detail.educationalLevel": "教育级别",
    "detail.numberOfCredits": "学分数",
    "detail.timeRequired": "所需时间",
    "detail.description": "描述",
    "detail.url": "URL",
    "detail.license": "许可",
    "detail.free": "免费",
    "detail.yes": "是",
    "detail.no": "否",
    "detail.empty": "—",
    "detail.identifiers": "标识符",
    "detail.teaches": "教授内容",
    "detail.keywords": "关键词",
    "detail.alternateNames": "替代名称",
    "detail.sameAs": "等同于（权威 URL）",
    "detail.instances": "实例",
    "detail.customPrefix": "自定义：",
    "detail.noDate": "（无日期）",
    "detail.capacity": "容量",
    "edit.title": "编辑课程",
    "edit.cancel": "取消",
    "edit.loading": "加载中…",
    "edit.saveChanges": "保存更改",
    "new.title": "新建课程",
    "new.create": "创建",
    "new.possibleDuplicates": "可能的重复项",
    "new.duplicatesDetected": "检测到重复项（{n}）— 重新提交前请在下方查看。",
    "audit.title": "审计日志",
    "audit.backToCourse": "返回课程",
    "audit.loading": "加载中…",
    "audit.noEntries": "没有审计记录。",
    "audit.by": "由",
    "audit.payload": "负载",
    "match.title": "匹配检查",
    "match.name": "名称",
    "match.courseCode": "课程代码",
    "match.providerId": "提供方 ID",
    "match.displayThreshold": "显示阈值",
    "match.thresholdHint": "客户端过滤（0.0 – 1.0）",
    "match.educationalLevel": "教育级别",
    "match.keywords": "关键词",
    "match.keywordsHint": "以逗号或换行分隔",
    "match.teaches": "教授内容（能力）",
    "match.teachesHint": "每行一个",
    "match.sameAs": "等同于 URL",
    "match.sameAsHint": "每行一个",
    "match.identifiers": "标识符",
    "match.matching": "匹配中…",
    "match.findMatches": "查找匹配项",
    "merge.title": "合并课程",
    "merge.mainId": "主课程 ID",
    "merge.mainIdHint": "保留的记录",
    "merge.duplicateId": "重复课程 ID",
    "merge.duplicateIdHint": "将被软删除",
    "merge.reason": "原因",
    "merge.reasonHint": "记录在合并审计跟踪中",
    "merge.reasonPlaceholder": "已确认的重复项",
    "merge.loadPreview": "加载预览",
    "merge.merging": "合并中…",
    "merge.merge": "合并",
    "merge.bothIdsRequired": "需要两个 ID",
    "merge.mustDiffer": "主记录与重复记录必须不同",
    "merge.confirm": "将 {dup}… 合并到 {main}…？\n这将软删除重复项。",
    "merge.preview": "预览",
    "merge.main": "主",
    "merge.duplicate": "重复",
    "merge.completed": "合并完成",
    "merge.recordCreated": "合并记录 {id} 已于 {at} 创建。",
    "merge.viewMerged": "查看合并后的主课程",
    "form.name": "名称",
    "form.courseCode": "课程代码",
    "form.courseCodeHint": "提供方范围内（如 CS101）",
    "form.status": "状态",
    "form.description": "描述",
    "form.url": "URL",
    "form.license": "许可",
    "form.numberOfCredits": "学分数",
    "form.educationalLevel": "教育级别",
    "form.typicalAgeRange": "典型年龄范围",
    "form.typicalAgeRangeHint": "如 18-22",
    "form.timeRequired": "所需时间",
    "form.timeRequiredHint": "ISO 8601（如 PT45H）",
    "form.alternateNames": "替代名称",
    "form.alternateNamesHint": "每行一个",
    "form.keywords": "关键词",
    "form.keywordsHint": "以逗号或换行分隔",
    "form.teaches": "教授内容（能力）",
    "form.teachesHint": "每行一个",
    "form.availableLanguages": "可用语言",
    "form.availableLanguagesHint": "BCP-47 代码（如 en fr de）",
    "form.sameAs": "等同于 URL",
    "form.sameAsHint": "Wikidata、OER 目录等 — 每行一个",
    "form.identifiers": "标识符",
    "form.saving": "保存中…",
    "form.save": "保存",
    "form.reset": "重置",
    "identifier.type": "类型",
    "identifier.customOption": "自定义…",
    "identifier.customLabel": "自定义标签",
    "identifier.value": "值",
    "identifier.url": "URL",
    "identifier.remove": "移除",
    "identifier.add": "+ 添加标识符",
    "search.placeholder": "搜索…",
    "search.submit": "搜索",
    "results.title": "匹配结果",
    "results.count": "({n})",
    "results.none": "无候选项。",
    "results.scoreBreakdown": "分数明细",
    "results.score.name": "名称",
    "results.score.courseCode": "课程代码",
    "results.score.provider": "提供方",
    "results.score.level": "级别",
    "results.score.keywords": "关键词",
    "results.score.teaches": "教授内容",
    "results.score.deterministic": "确定性（标识符 / 提供方+代码 / 等同于）",
    "grid.id": "ID",
    "grid.name": "名称",
    "grid.courseCode": "课程代码",
    "grid.level": "级别",
    "grid.status": "状态",
    "grid.primaryIdentifier": "主标识符",
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
    "splash.hero.title": "每门课程，一份可信记录",
    "splash.hero.subtitle":
      "一次登记，即刻查找，在重复出现之前就将其拦截，并内置完整审计记录。",
    "splash.benefits.1.title": "更少重复",
    "splash.benefits.1.body": "创建课程的瞬间，匹配功能就会标出疑似重复。",
    "splash.benefits.2.title": "快速找到任何课程",
    "splash.benefits.2.body":
      "全文和模糊搜索即使有拼写错误或写法差异，也能找到课程。",
    "splash.benefits.3.title": "有把握的决定",
    "splash.benefits.3.body":
      "每次匹配都显示得分和逐字段明细，审核人员一目了然。",
    "splash.benefits.4.title": "统一可信的目录",
    "splash.benefits.4.body": "课程代码和标识符确保每门课程只有一条记录。",
    "splash.benefits.5.title": "一切皆有记录",
    "splash.benefits.5.body":
      "每次更改都会被审计，随时可以查明谁在何时做了什么。",
    "splash.benefits.6.title": "适配您的系统",
    "splash.benefits.6.body": "文档完善的 REST API 可接入您现有的工具。",
    "splash.features.1.title": "搜索与浏览",
    "splash.features.1.body":
      "按名称或标识符搜索，可选模糊匹配，结果显示在可排序表格中。",
    "splash.features.2.title": "课程记录",
    "splash.features.2.body":
      "课程代码、级别、学分、标识符、关键词和教学内容，并即时校验。",
    "splash.features.3.title": "匹配检查",
    "splash.features.3.body": "将候选课程与目录比对打分，并查看原因。",
    "splash.features.4.title": "合并课程",
    "splash.features.4.body": "将确认的重复记录合并为一条，且不丢失历史。",
    "splash.features.5.title": "生命周期看板",
    "splash.features.5.body": "在草稿、已发布、已归档和已停用之间拖动课程。",
    "splash.features.6.title": "日程日历",
    "splash.features.6.body": "在日历上查看每个课程班次及其日期。",
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
    "tour.intro":
      "课程登记库的图文导览：每个页面的作用和使用步骤，从登记课程到合并重复记录、跟踪课程生命周期。",
    "tour.s1.title": "登记课程",
    "tour.s1.summary":
      "创建课程记录，填写课程代码、级别、学分、标识符和教授内容。提交前会提示可能的重复项。",
    "tour.s1.step.1":
      "登录后打开菜单并选择“新建课程”（或使用“课程”页面上的“新建课程”按钮）。",
    "tour.s1.step.2":
      "填写名称，然后填写课程代码（按提供方区分，如 CS101）、状态、教育级别和学分数。",
    "tour.s1.step.3":
      "添加关键词、教授内容（能力）、别名、等同于 URL，并用“+ 添加标识符”按钮添加一个或多个标识符。",
    "tour.s1.step.4":
      "点击“创建”。若出现“检测到重复项”，请先查看“可能的重复项”再重新提交；否则将跳转到新课程。",
    "tour.s2.title": "查找并打开课程",
    "tour.s2.summary": "按名称或标识符搜索目录，然后打开课程查看其全部记录。",
    "tour.s2.step.1":
      "从菜单打开“课程”。表格列出每门课程的 ID、名称、课程代码、级别、状态和主要标识符。",
    "tour.s2.step.2":
      "在搜索框输入名称或标识符并点击“搜索”；勾选“模糊”可容忍拼写错误和写法差异。",
    "tour.s2.step.3": "查看表格上方的记录数，然后选择一行打开该课程的详情页。",
    "tour.s2.step.4":
      "在详情页查看身份、标识符、教授内容、关键词和实例，用“编辑”修改，或用“审计”查看历史。",
    "tour.s3.title": "检查重复项",
    "tour.s3.summary":
      "描述一门课程并与目录比对打分，逐字段明细说明每个候选项匹配的原因。",
    "tour.s3.step.1": "从菜单打开“匹配检查”；此页面不会保存任何内容。",
    "tour.s3.step.2":
      "输入你所知道的信息：名称、课程代码、提供方 ID、教育级别、关键词、教授内容、等同于 URL或标识符。",
    "tour.s3.step.3":
      "点击“查找匹配项”。候选项会在“匹配结果”下列出并附带分数；用“显示阈值”（0.0 到 1.0）隐藏较弱的候选项。",
    "tour.s3.step.4":
      "在候选项上展开“得分明细”，可看到名称、课程代码、提供方、级别、关键词和教授内容的得分，或标识符、提供方加代码、等同于 URL上的确定性命中。",
    "tour.s4.title": "合并已确认的重复项",
    "tour.s4.summary":
      "将重复课程并入保留的记录，同时保留合并记录，不做任何硬删除。",
    "tour.s4.step.1":
      "从菜单打开“合并”（需要登录）。输入主课程 ID（保留的记录）和重复课程 ID（将被软删除）。",
    "tour.s4.step.2":
      "可选填写原因，例如“已确认重复”；它会记录在合并审计轨迹中。",
    "tour.s4.step.3":
      "点击“加载预览”，并排查看主课程和重复课程，确认这是正确的一对。",
    "tour.s4.step.4":
      "点击“合并”并确认。“合并完成”会显示新的合并记录，并附“查看合并后的主课程”链接。",
    "tour.s5.title": "推进课程的生命周期",
    "tour.s5.summary":
      "每门课程以卡片形式按状态分列显示，拖动卡片即可更改状态。",
    "tour.s5.step.1":
      "从菜单打开“看板”。各列对应生命周期：draft、published、archived 和 retired。",
    "tour.s5.step.2": "每张卡片显示课程名称和课程代码，一眼即可认出目标课程。",
    "tour.s5.step.3":
      "将卡片拖到另一列即可更改该课程的状态；更改会立即保存到课程记录。",
    "tour.s5.step.4":
      "随后看板会从服务重新加载，因此保存失败的卡片会回到记录的实际位置，并显示错误。",
    "tour.s6.title": "在日历上查看课程实例",
    "tour.s6.summary":
      "每门课程的每个排期按日期排布，只读，点击即可进入所属课程。",
    "tour.s6.step.1": "从菜单打开“日历”。默认为月视图。",
    "tour.s6.step.2":
      "每个课程实例的排期窗口显示为全天区间，标签为实例名称，无名称时为课程名称。",
    "tour.s6.step.3":
      "单次课节以带时间的事件显示在各自日期上，有课节标签时使用该标签。",
    "tour.s6.step.4":
      "选择任一条目即可打开所属课程，其“实例”部分列出日期、授课方式和容量。",
    "signin.sso": "使用 SSO 登录",
  },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)["en-001"];

/**
 * Every translatable key (the English catalog's key set). Exported so a
 * coverage test can assert that each locale defines every key.
 */
export const STRING_KEYS = Object.keys(STRINGS["en-001"]) as StringKey[];

/** Raw per-locale strings table, exposed for coverage testing. */
export const STRINGS_BY_LOCALE: Record<
  Locale,
  Record<string, string>
> = STRINGS;

// Normalise raw input to a supported locale, or null if unsupported.
// Case-insensitive and hyphen/underscore-insensitive. A region or legacy
// code resolves by primary language (`es-MX`, `en_US`, `zh`, `en` all map
// to the one supported locale of that language).
function normaliseLocale(raw: string | null | undefined): Locale | null {
  if (!raw) return null;
  const normalized = raw.trim().replace(/_/g, "-").toLowerCase();
  const exact = LOCALES.find((l) => l === normalized);
  if (exact) return exact;
  const primary = normalized.split("-")[0] ?? "";
  return LOCALES.find((l) => l.split("-")[0] === primary) ?? null;
}

// Seed the reactive locale from localStorage (default off the browser).
function readStoredLocale(): Locale {
  // Guard on `localStorage` availability, not just `browser`: a jsdom test
  // run can set the browser resolve condition while leaving `localStorage`
  // undefined.
  if (!browser || typeof localStorage === "undefined") return DEFAULT_LOCALE;
  return normaliseLocale(localStorage.getItem(LOCALE_KEY)) ?? DEFAULT_LOCALE;
}

// Reactive current-locale state; mutating it re-renders every `t(...)` call.
let current = $state<Locale>(readStoredLocale());

/**
 * Reactive current-locale store with persistence.
 *
 * Reading `i18n.locale` (or calling {@link t}) inside a component subscribes
 * that component to locale changes, so switching the locale re-renders the UI.
 */
export const i18n = {
  /** The currently selected locale. */
  get locale(): Locale {
    return current;
  },
  /**
   * Switch the active locale and persist the choice.
   *
   * @param next - Desired locale (a region subtag like `es-MX` is
   *   accepted); unsupported input falls back to {@link DEFAULT_LOCALE}.
   */
  set(next: string): void {
    const locale = normaliseLocale(next) ?? DEFAULT_LOCALE;
    current = locale;
    if (browser && typeof localStorage !== "undefined")
      localStorage.setItem(LOCALE_KEY, locale);
  },
  /** The list of supported locales (for rendering the switcher). */
  get locales(): readonly Locale[] {
    return LOCALES;
  },
};

/**
 * Translate `key` in `locale` with graceful fallbacks.
 *
 * Falls back to the English translation when the target locale lacks the
 * key, then to the key string itself if even English lacks it. Pure — safe
 * to unit-test without a Svelte component (does not read reactive state
 * unless `locale` is defaulted to the current locale).
 *
 * @param key - A valid {@link StringKey}.
 * @param locale - Target locale; defaults to the current reactive locale.
 * @returns The translated string (or the key as a last resort).
 */
export function translate(key: StringKey, locale: Locale = current): string {
  const table = STRINGS[locale] ?? STRINGS[DEFAULT_LOCALE];
  return table[key] ?? STRINGS[DEFAULT_LOCALE][key] ?? key;
}

/**
 * Reactive translation accessor for components: `t("courses.title")`.
 *
 * Reads the current reactive locale, so a locale switch re-renders every
 * string rendered through it.
 *
 * @param key - A valid {@link StringKey}.
 * @returns The translated string in the current locale.
 */
export function t(key: StringKey): string {
  return translate(key, current);
}
