// Lightweight, dependency-free i18n for the Organization SPA. A
// per-locale strings map plus a reactive `$state` current-locale (Svelte
// 5 runes), exposed via a `t(key)` accessor. Deliberately no i18n
// library: the surface is small and we keep the front-end dependency-light
// (drift across the family front-ends is accepted, see AGENTS.md).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// German (`de-de`), English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. An unknown key/locale falls back to `en-001`, then to
// the key string itself. The chosen locale persists to localStorage and
// drives the UI strings, `<html lang>`, and `<html dir>` (right-to-left
// for `ar-001`).

import { browser } from "$app/env";

/**
 * Locales for which the UI is translated, sorted alphabetically by code
 * (the LocalePicker shows them in this order). To add one, extend this
 * tuple AND add a matching entry to {@link LOCALE_LABELS} and `STRINGS`.
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
 * @returns `true` for Arabic, otherwise `false`.
 */
export function isRtl(locale: string): boolean {
  const resolved = normaliseLocale(locale);

  return (
    resolved !== null && (RTL_LOCALES as readonly string[]).includes(resolved)
  );
}

/**
 * localStorage key under which the chosen UI locale is persisted. Exported
 * so a locale control can share it — the i18n store is the single source of
 * truth for the chosen locale, and the control persists to the same key.
 */
export const LOCALE_KEY = "mxi.organization.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    "nav.review": "المراجعة",
    "review.run": "تشغيل الفحص",
    "brand.name": "Main X · المنظمات",
    "nav.toggle": "تبديل التنقل",
    "nav.organizations": "المنظمات",
    "nav.newOrganization": "منظمة جديدة",
    "chrome.language": "اللغة",
    "nav.share": "مشاركة",
    "nav.text_size": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "chrome.theme": "السمة",
    "session.title": "الجلسة",
    "session.signedIn": "تم تسجيل الدخول (الرمز مرفق)",
    "session.signOut": "تسجيل الخروج",
    "session.signIn": "تسجيل الدخول",
    "session.pasteToken": "لصق رمز",
    "session.accessToken": "رمز الوصول",
    "session.pastePlaceholder": "الصق رمز الحامل",
    "session.useToken": "استخدام الرمز",
    "list.title": "المنظمات",
    "list.new": "منظمة جديدة",
    "list.loading": "جارٍ التحميل…",
    "list.empty": "لا توجد منظمات بعد.",
    "list.createOne": "أنشئ واحدة",
    "detail.organization": "منظمة",
    "detail.loading": "جارٍ التحميل…",
    "detail.notFound": "غير موجود",
    "detail.legalName": "الاسم القانوني:",
    "detail.url": "الرابط:",
    "detail.jurisdiction": "الاختصاص القضائي:",
    "detail.founded": "تأسست:",
    "detail.identifiers": "المعرفات:",
    "detail.keywords": "الكلمات المفتاحية:",
    "detail.id": "المعرف:",
    "detail.edit": "تحرير",
    "detail.checkDuplicates": "فحص التكرارات",
    "detail.checking": "جارٍ الفحص…",
    "detail.delete": "حذف",
    "detail.showMasked": "إظهار المُقنَّع",
    "detail.showFull": "إظهار الكامل",
    "detail.maskedNotice": "يتم عرض العرض المُقنَّع — بعض الحقول مخفية.",
    "detail.exportGdpr": "تصدير البيانات (GDPR)",
    "detail.exportingGdpr": "جارٍ التصدير…",
    "detail.showAudit": "عرض سجل التدقيق",
    "detail.hideAudit": "إخفاء سجل التدقيق",
    "detail.auditTrail": "سجل التدقيق",
    "detail.loadingAudit": "جارٍ تحميل سجل التدقيق…",
    "detail.noAuditEntries": "لا توجد إدخالات تدقيق.",
    "detail.auditLoadFailed": "فشل تحميل التدقيق",
    "detail.checkFailed": "فشل الفحص",
    "detail.potentialDuplicates": "تكرارات محتملة",
    "detail.noneAboveThreshold": "لا شيء فوق حد التطابق.",
    "new.title": "منظمة جديدة",
    "new.create": "إنشاء",
    "edit.title": "تحرير المنظمة",
    "edit.organizationFallback": "منظمة",
    "edit.loading": "جارٍ التحميل…",
    "edit.notFound": "غير موجود",
    "edit.saveChanges": "حفظ التغييرات",
    "form.save": "حفظ",
    "form.saving": "جارٍ الحفظ…",
    "form.nameRequired": "الاسم مطلوب.",
    "form.saveFailed": "فشل الحفظ",
    "form.name": "الاسم",
    "form.legalName": "الاسم القانوني",
    "form.url": "رابط الموقع",
    "form.jurisdiction": "الاختصاص القضائي (ISO 3166)",
    "form.foundingDate": "تاريخ التأسيس",
    "form.alternateNames": "الأسماء البديلة",
    "form.commaSeparated": "(مفصولة بفواصل)",
    "form.keywords": "الكلمات المفتاحية",
    "form.sameAs": "روابط مطابق لـ",
    "form.address": "العنوان",
    "form.street": "الشارع",
    "form.locality": "المنطقة",
    "form.region": "الإقليم",
    "form.postalCode": "الرمز البريدي",
    "form.country": "البلد",
    "form.identifiers": "المعرفات",
    "form.value": "القيمة",
    "form.remove": "إزالة",
    "form.addIdentifier": "+ إضافة معرف",
    "nav.merge": "دمج",
    "merge.title": "دمج المنظمات",
    "merge.mainId": "معرف المنظمة الرئيسية",
    "merge.mainIdHint": "السجل الذي يبقى بعد الدمج.",
    "merge.dupId": "معرف المنظمة المكررة",
    "merge.dupIdHint": "السجل الذي يُدمج ثم يُحذف حذفًا مبدئيًا.",
    "merge.reason": "السبب",
    "merge.reasonHint": "اختياري؛ يُحفظ في سجل الدمج.",
    "merge.reasonPlaceholder": "الشركة نفسها، تسجيل مكرر",
    "merge.loadPreview": "تحميل المعاينة",
    "merge.merging": "جارٍ الدمج…",
    "merge.merge": "دمج",
    "merge.bothIdsRequired": "كلا المعرفين مطلوبان.",
    "merge.mustDiffer": "يجب أن يختلف المعرف الرئيسي عن معرف النسخة المكررة.",
    "merge.preview": "معاينة",
    "merge.main": "الرئيسية",
    "merge.duplicate": "المكررة",
    "merge.completed": "اكتمل الدمج",
    "merge.completedDetail": "تم دمج {dup} في {main}.",
    "merge.viewMain": "عرض المنظمة الرئيسية",
    "merge.recent": "عمليات الدمج الأخيرة",
    "merge.recentEmpty": "لم تُسجَّل أي عمليات دمج بعد.",
    "merge.recentRefresh": "تحديث",
    "merge.colMergedAt": "تاريخ الدمج",
    "merge.colReason": "السبب",
    "merge.colActor": "المنفِّذ",
    "merge.confirm":
      "هل تريد دمج {dup} في {main}؟ سيتم حذف النسخة المكررة حذفًا مبدئيًا.",
    "review.intro":
      "قائمة الانتظار المخزنة للتكرارات المحتملة من عمليات الفحص الدُفعي. راجع كل زوج وأكّده أو ارفضه.",
    "review.filter.status": "الحالة",
    "review.filter.statusAll": "كل الحالات",
    "review.filter.limit": "حجم الصفحة",
    "review.filter.limitHint":
      "لا يوجد إزاحة للصفحة — تُعرض فقط أحدث العناصر المطابقة حتى هذا الحد.",
    "review.board.title": "اللوحة",
    "review.list.title": "قائمة الانتظار",
    "review.empty": "قائمة المراجعة فارغة.",
    "review.loading": "جارٍ التحميل…",
    "review.col.pair": "الزوج",
    "review.col.score": "الدرجة",
    "review.col.quality": "الجودة",
    "review.col.provenance": "المصدر",
    "review.col.status": "الحالة",
    "review.col.actions": "الإجراءات",
    "review.compare.open": "مقارنة",
    "review.compare.title": "المقارنة",
    "review.compare.close": "إغلاق",
    "review.compare.loading": "جارٍ تحميل كلا السجلين…",
    "review.compare.partial":
      "تعذّر تحميل أحد السجلين (ربما تم حذفه)؛ يُعرض المتاح.",
    "review.compare.field": "الحقل",
    "review.compare.a": "أ",
    "review.compare.b": "ب",
    "review.compare.none": "غير مسجَّل",
    "review.field.score": "الدرجة:",
    "review.field.quality": "الجودة:",
    "review.field.method": "طريقة الاكتشاف:",
    "review.field.provenance": "المصدر:",
    "review.field.status": "الحالة:",
    "review.field.legalName": "الاسم القانوني",
    "review.breakdown.title": "تفصيل الدرجة",
    "review.breakdown.loading": "جارٍ تقييم الزوج…",
    "review.breakdown.none": "لا يوجد تفصيل للدرجة متاح.",
    "review.breakdown.component": "المكوّن",
    "review.breakdown.weight": "الوزن",
    "review.breakdown.score": "الدرجة",
    "review.decide.confirm": "تأكيد التكرار",
    "review.decide.reject": "رفض",
    "review.decide.deciding": "جارٍ التسجيل…",
    "review.decide.locked":
      "تم اتخاذ قرار بشأن هذا العنصر مسبقًا ولا يمكن تغييره هنا.",
    "review.merge.title": "دمج",
    "review.merge.note": "التأكيد لا يدمج السجلات — اختر أيهما يبقى.",
    "review.merge.keepA": "دمج، الإبقاء على أ",
    "review.merge.keepB": "دمج، الإبقاء على ب",
    "review.status.pending": "قيد الانتظار",
    "review.status.confirmed": "مؤكَّد",
    "review.status.rejected": "مرفوض",
    "review.status.automerged": "مدموج تلقائيًا",
    "review.provenance.operator": "المشغّل",
    "review.provenance.import": "استيراد",
    "review.provenance.matcherSuggested": "مقترح من المطابق",
    "review.component.name": "الاسم",
    "review.component.address": "العنوان",
    "review.component.url": "الرابط",
    "review.component.jurisdiction": "الاختصاص القضائي",
    "review.component.foundingDate": "تاريخ التأسيس",
    "review.component.keywords": "الكلمات المفتاحية",
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
    "splash.hero.title": "سجل موثوق واحد لكل منظمة",
    "splash.hero.subtitle":
      "سجّل الشركات والجمعيات والجهات مرة واحدة، واكتشف التكرار مبكرًا، وادمجها بأمان، مع خصوصية وسجل تدقيق كامل مدمجين.",
    "splash.benefits.1.title": "تكرار أقل",
    "splash.benefits.1.body":
      "تنبّه فحوصات التكرار إلى احتمال التكرار قبل إنشاء سجل ثانٍ.",
    "splash.benefits.2.title": "معرّفات ثابتة",
    "splash.benefits.2.body":
      "احتفظ برمز LEI وDUNS وغيرهما من المعرّفات لكل منظمة.",
    "splash.benefits.3.title": "قرارات مبنية على أدلة",
    "splash.benefits.3.body":
      "اطّلع على تفصيل النتيجة لكل زوج مرشَّح قبل تأكيده أو رفضه.",
    "splash.benefits.4.title": "دمج آمن",
    "splash.benefits.4.body": "الدمج خطوة مقصودة تحتفظ بسجل لما جرى دمجه.",
    "splash.benefits.5.title": "الخصوصية عند المشاركة",
    "splash.benefits.5.body":
      "اعرض سجلًا مُقنَّعًا أو صدّر البيانات دون كشف أكثر مما ينبغي.",
    "splash.benefits.6.title": "تغييرات خاضعة للمساءلة",
    "splash.benefits.6.body": "كل تغيير مسجَّل، فتعرف ما حدث ومتى.",
    "splash.features.1.title": "تصفّح المنظمات",
    "splash.features.1.body": "تصفّح السجل كقائمة، أو صفِّ شبكة بيانات كاملة.",
    "splash.features.2.title": "إنشاء وتعديل",
    "splash.features.2.body":
      "سجّل الأسماء والاختصاص القضائي وتاريخ التأسيس والكلمات المفتاحية والمعرّفات.",
    "splash.features.3.title": "التحقق من التكرار",
    "splash.features.3.body":
      "اعثر على منظمات مخزَّنة تشبه هذه المنظمة، مع نتائجها.",
    "splash.features.4.title": "لوحة المراجعة",
    "splash.features.4.body":
      "اسحب الأزواج المرشَّحة للتأكيد أو الرفض، أو قارنها جنبًا إلى جنب.",
    "splash.features.5.title": "دمج السجلات",
    "splash.features.5.body":
      "ادمج التكرار المؤكَّد في السجل الذي تحتفظ به، مع سجل للدمج.",
    "splash.features.6.title": "العرض المُقنَّع والتصدير",
    "splash.features.6.body":
      "بدّل إلى العرض المُقنَّع، أو نزّل تصدير GDPR، أو افتح سجل التدقيق.",
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
      "جولة إرشادية في سجل المنظمات: ما تفعله كل شاشة وخطوات استخدامها، من تسجيل منظمة إلى دمج التكرارات.",
    "tour.s1.title": "تسجيل منظمة",
    "tour.s1.summary":
      "أنشئ سجلًا بأسماء ومعرّفات وعنوان وكلمات مفتاحية. الاسم مطلوب، وتنبّه تلميحات المعرّفات إلى القيم غير الصحيحة قبل الحفظ.",
    "tour.s1.step.1":
      "اختر منظمة جديدة من القائمة، أو أنشئ واحدة في قائمة فارغة، لفتح النموذج.",
    "tour.s1.step.2":
      "أدخل الاسم (مطلوب)، ثم اختياريًا الاسم القانوني وعنوان URL للموقع والولاية القضائية وتاريخ التأسيس والأسماء البديلة والكلمات المفتاحية وعناوين URL المكافئة.",
    "tour.s1.step.3":
      "املأ العنوان، واستخدم + إضافة معرف لتسجيل LEI أو DUNS أو معرّف آخر مع قيمته؛ يظهر تلميح إذا بدا التنسيق خاطئًا.",
    "tour.s1.step.4":
      "اختر إنشاء. إذا كان الاسم ناقصًا أو رفضت الخدمة قيمة ما، يعرض النموذج الخطأ ولا يُحفظ شيء.",
    "tour.s2.title": "تصفّح السجل وتصفيته",
    "tour.s2.summary":
      "شاهد كل منظمة في السجل كقائمة بسيطة أو كشبكة بيانات قابلة للتصفية، وانتقل من أيٍّ منهما إلى السجل.",
    "tour.s2.step.1":
      "سجّل الدخول وافتح المنظمات من القائمة: تعرض الصفحة الرئيسية كل منظمة باسمها، مع زر منظمة جديدة.",
    "tour.s2.step.2":
      "افتح عرض الشبكة عبر الرابط أدناه: تعرض شبكة البيانات الاسم وpid لكل منظمة، إضافةً إلى عدد الصفوف المحمّلة من الإجمالي.",
    "tour.s2.step.3":
      "استخدم شريط التصفية فوق الشبكة لتضييق الصفوف حسب الاسم؛ تعمل التصفية على الصفوف المحمّلة بالفعل.",
    "tour.s2.step.4":
      "اختر صفًا في الشبكة، أو اسمًا في القائمة الرئيسية، لفتح سجل تلك المنظمة.",
    "tour.s3.title": "فتح سجل وفحص التكرارات",
    "tour.s3.summary":
      "تعرض صفحة السجل تفاصيله والإجراءات المتاحة عليه: فحص التكرارات والتحرير والحذف.",
    "tour.s3.step.1":
      "افتح منظمة لترى اسمها القانوني وعنوان URL والولاية القضائية وتاريخ التأسيس والمعرّفات والكلمات المفتاحية والمعرّف.",
    "tour.s3.step.2":
      "اختر فحص التكرارات لمقارنتها بالمنظمات المخزَّنة؛ تُدرج التطابقات المحتملة ضمن التكرارات المحتملة، أو تذكر الصفحة أنه لا يوجد ما يتجاوز عتبة التطابق.",
    "tour.s3.step.3":
      "اختر اسمًا في النتائج لفتح السجل الآخر ومقارنة التفاصيل.",
    "tour.s3.step.4":
      "لتغيير سجل، اختر تحرير وعدّل النموذج ثم اختر حفظ التغييرات؛ يحذف حذف السجل حذفًا منطقيًا ويعيدك إلى القائمة.",
    "tour.s4.title": "الخصوصية والتصدير والتدقيق",
    "tour.s4.summary":
      "اطّلع على العرض المُقنَّع، أو نزّل تصديرًا وفق GDPR، أو اقرأ سجل التدقيق، كل ذلك من صفحة السجل.",
    "tour.s4.step.1":
      "في صفحة السجل، اختر إظهار المُقنَّع لإعادة تحميله عبر العرض المُقنَّع؛ يوضح إشعار أن بعض الحقول محجوبة.",
    "tour.s4.step.2": "اختر إظهار الكامل للعودة إلى السجل الكامل.",
    "tour.s4.step.3":
      "اختر تصدير البيانات (GDPR) لتنزيل بيانات السجل كملف JSON.",
    "tour.s4.step.4":
      "اختر عرض سجل التدقيق لتحميل تاريخ التغييرات، وإخفاء سجل التدقيق لإغلاقه؛ والسجل الذي لا تاريخ له يعرض: لا توجد مدخلات تدقيق.",
    "tour.s5.title": "مراجعة المرشّحين للتكرار",
    "tour.s5.summary":
      "تحتوي صفحة المراجعة على قائمة انتظار الأزواج المحتمل تكرارها التي وجدتها الفحوصات، وفيها تؤكد كل زوج أو ترفضه.",
    "tour.s5.step.1":
      "افتح المراجعة واختر تشغيل الفحص لفحص السجل، ثم صفِّ القائمة حسب الحالة وحجم الصفحة.",
    "tour.s5.step.2":
      "تظهر الأزواج المرشّحة على لوحة وفي جدول قائمة الانتظار يعرض الزوج والدرجة والجودة والمصدر والحالة.",
    "tour.s5.step.3":
      "اختر مقارنة (أو بطاقة) لرؤية السجلين جنبًا إلى جنب مع تفصيل الدرجة حسب المكوّن.",
    "tour.s5.step.4":
      "اختر تأكيد التكرار أو رفض، أو اسحب بطاقة معلّقة إلى عمود المؤكَّد أو المرفوض؛ العنصر الذي جرى البتّ فيه يُقفل.",
    "tour.s6.title": "دمج سجلين",
    "tour.s6.summary":
      "يدمج الدمج تكرارًا مؤكَّدًا في السجل الذي تحتفظ به، ويسجّل سجل الدمج ما جرى دمجه.",
    "tour.s6.step.1":
      "من عنصر مؤكَّد في المراجعة، اختر دمج، الإبقاء على أ أو دمج، الإبقاء على ب لفتح الدمج مع تعبئة المعرّفين، أو افتح الدمج واكتب المعرّفين بنفسك.",
    "tour.s6.step.2":
      "تحقق من معرّف المنظمة الرئيسية (السجل الذي يبقى) ومعرّف المنظمة المكررة؛ ويجب أن يختلفا. أضف سببًا اختياريًا.",
    "tour.s6.step.3":
      "اختر تحميل المعاينة لمقارنة الرئيسية والمكررة، ثم اختر دمج وأكّد الرسالة، لأن المكررة تُحذف حذفًا منطقيًا.",
    "tour.s6.step.4":
      "تعرض الصفحة اكتمال الدمج مع رابط إلى المنظمة الرئيسية، وتسرد عمليات الدمج الأخيرة متى ولماذا ومن قام بها.",
    "signin.sso": "تسجيل الدخول عبر SSO",
  },
  "cy-001": {
    "nav.review": "Adolygu",
    "review.run": "Rhedeg sgan",
    "brand.name": "Main X · Sefydliadau",
    "nav.toggle": "Toglo'r llywio",
    "nav.organizations": "Sefydliadau",
    "nav.newOrganization": "Sefydliad newydd",
    "chrome.language": "Iaith",
    "nav.share": "Rhannu",
    "nav.text_size": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "chrome.theme": "Thema",
    "session.title": "Sesiwn",
    "session.signedIn": "Wedi mewngofnodi (tocyn ynghlwm)",
    "session.signOut": "Allgofnodi",
    "session.signIn": "Mewngofnodi",
    "session.pasteToken": "Gludo tocyn",
    "session.accessToken": "Tocyn mynediad",
    "session.pastePlaceholder": "Gludo tocyn cludwr",
    "session.useToken": "Defnyddio tocyn",
    "list.title": "Sefydliadau",
    "list.new": "Sefydliad newydd",
    "list.loading": "Yn llwytho…",
    "list.empty": "Dim sefydliadau eto.",
    "list.createOne": "Creu un",
    "detail.organization": "Sefydliad",
    "detail.loading": "Yn llwytho…",
    "detail.notFound": "Heb ei ganfod",
    "detail.legalName": "Enw cyfreithiol:",
    "detail.url": "URL:",
    "detail.jurisdiction": "Awdurdodaeth:",
    "detail.founded": "Sefydlwyd:",
    "detail.identifiers": "Dynodyddion:",
    "detail.keywords": "Allweddeiriau:",
    "detail.id": "ID:",
    "detail.edit": "Golygu",
    "detail.checkDuplicates": "Gwirio dyblygiadau",
    "detail.checking": "Yn gwirio…",
    "detail.delete": "Dileu",
    "detail.showMasked": "Dangos wedi'i guddio",
    "detail.showFull": "Dangos yn llawn",
    "detail.maskedNotice":
      "Yn dangos y golwg guddiedig — mae rhai meysydd wedi'u cuddio.",
    "detail.exportGdpr": "Allforio data (GDPR)",
    "detail.exportingGdpr": "Wrthi'n allforio…",
    "detail.showAudit": "Dangos llwybr archwilio",
    "detail.hideAudit": "Cuddio llwybr archwilio",
    "detail.auditTrail": "Llwybr archwilio",
    "detail.loadingAudit": "Yn llwytho llwybr archwilio…",
    "detail.noAuditEntries": "Dim cofnodion archwilio.",
    "detail.auditLoadFailed": "Methodd llwytho'r archwiliad",
    "detail.checkFailed": "Methodd y gwiriad",
    "detail.potentialDuplicates": "Dyblygiadau posibl",
    "detail.noneAboveThreshold": "Dim uwchlaw'r trothwy cydweddu.",
    "new.title": "Sefydliad newydd",
    "new.create": "Creu",
    "edit.title": "Golygu sefydliad",
    "edit.organizationFallback": "sefydliad",
    "edit.loading": "Yn llwytho…",
    "edit.notFound": "Heb ei ganfod",
    "edit.saveChanges": "Cadw newidiadau",
    "form.save": "Cadw",
    "form.saving": "Yn cadw…",
    "form.nameRequired": "Mae angen enw.",
    "form.saveFailed": "Methodd y cadw",
    "form.name": "Enw",
    "form.legalName": "Enw cyfreithiol",
    "form.url": "URL y wefan",
    "form.jurisdiction": "Awdurdodaeth (ISO 3166)",
    "form.foundingDate": "Dyddiad sefydlu",
    "form.alternateNames": "Enwau eraill",
    "form.commaSeparated": "(wedi'u gwahanu gan goma)",
    "form.keywords": "Allweddeiriau",
    "form.sameAs": "URLau yr un â",
    "form.address": "Cyfeiriad",
    "form.street": "Stryd",
    "form.locality": "Ardal",
    "form.region": "Rhanbarth",
    "form.postalCode": "Cod post",
    "form.country": "Gwlad",
    "form.identifiers": "Dynodyddion",
    "form.value": "gwerth",
    "form.remove": "Tynnu",
    "form.addIdentifier": "+ Ychwanegu dynodydd",
    "nav.merge": "Uno",
    "merge.title": "Uno sefydliadau",
    "merge.mainId": "ID y prif sefydliad",
    "merge.mainIdHint": "Y cofnod sy'n goroesi'r uno.",
    "merge.dupId": "ID y sefydliad dyblyg",
    "merge.dupIdHint": "Y cofnod sy'n cael ei uno a'i ddileu'n feddal.",
    "merge.reason": "Rheswm",
    "merge.reasonHint": "Dewisol; cedwir yn hanes yr uno.",
    "merge.reasonPlaceholder": "Yr un cwmni, cofrestriad dyblyg",
    "merge.loadPreview": "Llwytho rhagolwg",
    "merge.merging": "Yn uno…",
    "merge.merge": "Uno",
    "merge.bothIdsRequired": "Mae angen y ddau ID.",
    "merge.mustDiffer": "Rhaid i'r prif ID a'r ID dyblyg fod yn wahanol.",
    "merge.preview": "Rhagolwg",
    "merge.main": "Prif",
    "merge.duplicate": "Dyblyg",
    "merge.completed": "Uno wedi'i gwblhau",
    "merge.completedDetail": "Unwyd {dup} i mewn i {main}.",
    "merge.viewMain": "Gweld y prif sefydliad",
    "merge.recent": "Unoau diweddar",
    "merge.recentEmpty": "Dim unoau wedi'u cofnodi eto.",
    "merge.recentRefresh": "Adnewyddu",
    "merge.colMergedAt": "Unwyd ar",
    "merge.colReason": "Rheswm",
    "merge.colActor": "Gweithredwr",
    "merge.confirm":
      "Uno {dup} i mewn i {main}? Caiff y dyblyg ei ddileu'n feddal.",
    "review.intro":
      "Y ciw ymgeiswyr dyblyg wedi'i storio o sganiau bwrpasol. Adolygwch bob pâr a'i gadarnhau neu ei wrthod.",
    "review.filter.status": "Statws",
    "review.filter.statusAll": "Pob statws",
    "review.filter.limit": "Maint tudalen",
    "review.filter.limitHint":
      "Nid oes atred tudalen — dim ond yr eitemau cyfatebol diweddaraf hyd at y terfyn hwn a ddangosir.",
    "review.board.title": "Bwrdd",
    "review.list.title": "Ciw",
    "review.empty": "Mae'r ciw adolygu'n wag.",
    "review.loading": "Yn llwytho…",
    "review.col.pair": "Pâr",
    "review.col.score": "Sgôr",
    "review.col.quality": "Ansawdd",
    "review.col.provenance": "Ffynhonnell",
    "review.col.status": "Statws",
    "review.col.actions": "Gweithredoedd",
    "review.compare.open": "Cymharu",
    "review.compare.title": "Cymhariaeth",
    "review.compare.close": "Cau",
    "review.compare.loading": "Yn llwytho'r ddau gofnod…",
    "review.compare.partial":
      "Ni ellid llwytho un cofnod (efallai iddo gael ei ddileu); dangos yr hyn sydd ar gael.",
    "review.compare.field": "Maes",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "Heb ei gofnodi",
    "review.field.score": "Sgôr:",
    "review.field.quality": "Ansawdd:",
    "review.field.method": "Dull canfod:",
    "review.field.provenance": "Ffynhonnell:",
    "review.field.status": "Statws:",
    "review.field.legalName": "Enw cyfreithiol",
    "review.breakdown.title": "Dadansoddiad sgôr",
    "review.breakdown.loading": "Yn sgorio'r pâr…",
    "review.breakdown.none": "Dim dadansoddiad sgôr ar gael.",
    "review.breakdown.component": "Cydran",
    "review.breakdown.weight": "Pwysau",
    "review.breakdown.score": "Sgôr",
    "review.decide.confirm": "Cadarnhau dyblyg",
    "review.decide.reject": "Gwrthod",
    "review.decide.deciding": "Yn cofnodi…",
    "review.decide.locked":
      "Penderfynwyd ar yr eitem hon eisoes ac ni ellir ei newid yma.",
    "review.merge.title": "Uno",
    "review.merge.note":
      "Nid yw cadarnhau yn uno'r cofnodion — dewiswch pa un sy'n goroesi.",
    "review.merge.keepA": "Uno, cadw A",
    "review.merge.keepB": "Uno, cadw B",
    "review.status.pending": "Yn aros",
    "review.status.confirmed": "Wedi'i gadarnhau",
    "review.status.rejected": "Wedi'i wrthod",
    "review.status.automerged": "Wedi'i awto-uno",
    "review.provenance.operator": "Gweithredwr",
    "review.provenance.import": "Mewnforio",
    "review.provenance.matcherSuggested": "Awgrym paru",
    "review.component.name": "Enw",
    "review.component.address": "Cyfeiriad",
    "review.component.url": "URL",
    "review.component.jurisdiction": "Awdurdodaeth",
    "review.component.foundingDate": "Dyddiad sefydlu",
    "review.component.keywords": "Allweddeiriau",
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
    "splash.hero.title": "Un cofnod dibynadwy ar gyfer pob sefydliad",
    "splash.hero.subtitle":
      "Cofrestrwch gwmnïau, elusennau ac asiantaethau unwaith, canfyddwch ddyblygu'n gynnar, ac unwch nhw'n ddiogel, gyda phreifatrwydd a llwybr archwilio llawn wedi'u cynnwys.",
    "splash.benefits.1.title": "Llai o ddyblygu",
    "splash.benefits.1.body":
      "Mae gwiriadau dyblygu yn nodi ailadrodd tebygol cyn creu ail gofnod.",
    "splash.benefits.2.title": "Dynodwyr sy'n para",
    "splash.benefits.2.body":
      "Cadwch LEI, DUNS a dynodwyr eraill ar gyfer pob sefydliad.",
    "splash.benefits.3.title": "Penderfyniadau â thystiolaeth",
    "splash.benefits.3.body":
      "Gwelwch ddadansoddiad sgôr ar gyfer pob pâr ymgeisiol cyn ei gadarnhau neu ei wrthod.",
    "splash.benefits.4.title": "Uno diogel",
    "splash.benefits.4.body":
      "Mae uno yn gam bwriadol sy'n cadw hanes o'r hyn a gyfunwyd.",
    "splash.benefits.5.title": "Preifatrwydd wrth rannu",
    "splash.benefits.5.body":
      "Gwelwch gofnod wedi'i guddio neu allforiwch ddata heb ddatgelu mwy nag y dylech.",
    "splash.benefits.6.title": "Newidiadau atebol",
    "splash.benefits.6.body":
      "Mae pob newid ar y cofnod, felly gwelwch beth ddigwyddodd a phryd.",
    "splash.features.1.title": "Pori sefydliadau",
    "splash.features.1.body":
      "Sganiwch y gofrestr fel rhestr, neu hidlwch grid data llawn.",
    "splash.features.2.title": "Creu a golygu",
    "splash.features.2.body":
      "Cofnodwch enwau, awdurdodaeth, dyddiad sefydlu, allweddeiriau a dynodwyr.",
    "splash.features.3.title": "Gwirio dyblygiadau",
    "splash.features.3.body":
      "Dewch o hyd i sefydliadau sydd wedi'u storio sy'n debyg i hwn, gyda sgorau.",
    "splash.features.4.title": "Bwrdd adolygu",
    "splash.features.4.body":
      "Llusgwch barau ymgeisiol i gadarnhau neu wrthod, neu cymharwch nhw ochr yn ochr.",
    "splash.features.5.title": "Uno cofnodion",
    "splash.features.5.body":
      "Cyfunwch ddyblyg cadarn â'r cofnod a gadwch, gyda hanes uno.",
    "splash.features.6.title": "Golwg wedi'i guddio ac allforio",
    "splash.features.6.body":
      "Newidiwch i olwg wedi'i guddio, lawrlwythwch allforiad GDPR, neu agorwch y llwybr archwilio.",
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
      "Taith dywys drwy'r gofrestr Sefydliadau: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o gofrestru sefydliad i uno dyblygion.",
    "tour.s1.title": "Cofrestru sefydliad",
    "tour.s1.summary":
      "Crëwch gofnod gydag enwau, dynodyddion, cyfeiriad a geiriau allweddol. Mae enw yn ofynnol, ac mae awgrymiadau dynodyddion yn nodi gwerthoedd anghywir cyn i chi gadw.",
    "tour.s1.step.1":
      "Dewiswch Sefydliad newydd yn y ddewislen, neu Creu un ar restr wag, i agor y ffurflen.",
    "tour.s1.step.2":
      "Rhowch yr Enw (gofynnol), yna, os dymunwch, yr enw cyfreithiol, URL y wefan, awdurdodaeth, dyddiad sefydlu, enwau eraill, geiriau allweddol ac URLau same-as.",
    "tour.s1.step.3":
      "Llenwch y cyfeiriad, a defnyddiwch + Ychwanegu dynodydd i gofnodi LEI, DUNS neu ddynodydd arall gyda'i werth; mae awgrym yn ymddangos os yw'r fformat yn edrych yn anghywir.",
    "tour.s1.step.4":
      "Dewiswch Creu. Os yw'r enw ar goll neu os yw'r gwasanaeth yn gwrthod gwerth, mae'r ffurflen yn dangos y gwall ac ni chaiff dim ei gadw.",
    "tour.s2.title": "Pori a hidlo'r gofrestr",
    "tour.s2.summary":
      "Gwelwch bob sefydliad yn y gofrestr fel rhestr syml neu fel grid data hidladwy, a neidiwch o'r naill neu'r llall i gofnod.",
    "tour.s2.step.1":
      "Mewngofnodwch ac agorwch Sefydliadau o'r ddewislen: mae'r hafan yn rhestru pob sefydliad yn ôl enw, gyda botwm Sefydliad newydd.",
    "tour.s2.step.2":
      "Agorwch y golwg grid gyda'r ddolen isod: mae grid data yn dangos Enw a pid pob sefydliad, ynghyd â chyfrif o'r rhesi a lwythwyd allan o'r cyfanswm.",
    "tour.s2.step.3":
      "Defnyddiwch y bar hidlo uwchben y grid i gyfyngu'r rhesi yn ôl enw; mae hidlo'n gweithio ar y rhesi sydd eisoes wedi'u llwytho.",
    "tour.s2.step.4":
      "Dewiswch res yn y grid, neu enw ar restr yr hafan, i agor cofnod y sefydliad hwnnw.",
    "tour.s3.title": "Agor cofnod a gwirio am ddyblygiadau",
    "tour.s3.summary":
      "Mae tudalen cofnod yn dangos ei fanylion a'r camau gweithredu sydd ar gael arno: gwirio dyblygiadau, golygu a dileu.",
    "tour.s3.step.1":
      "Agorwch sefydliad i weld ei enw cyfreithiol, URL, awdurdodaeth, dyddiad sefydlu, dynodyddion, geiriau allweddol ac ID.",
    "tour.s3.step.2":
      "Dewiswch Gwirio dyblygiadau i'w gymharu â'r sefydliadau sydd wedi'u storio; caiff cyfatebiaethau tebygol eu rhestru dan Dyblygiadau posibl, neu mae'r dudalen yn dweud nad oes yr un uwchlaw'r trothwy cyfatebiaeth.",
    "tour.s3.step.3":
      "Dewiswch enw yn y canlyniadau i agor y cofnod arall a chymharu'r manylion.",
    "tour.s3.step.4":
      "I newid cofnod, dewiswch Golygu, addaswch y ffurflen a dewiswch Cadw newidiadau; mae Dileu yn dileu'r cofnod yn feddal ac yn eich dychwelyd i'r rhestr.",
    "tour.s4.title": "Preifatrwydd, allforio ac archwilio",
    "tour.s4.summary":
      "Edrychwch ar olwg wedi'i guddio, lawrlwythwch allforio GDPR, neu darllenwch y llwybr archwilio, i gyd o dudalen cofnod.",
    "tour.s4.step.1":
      "Ar dudalen cofnod, dewiswch Dangos wedi'i guddio i'w ail-lwytho drwy'r olwg wedi'i guddio; mae hysbysiad yn dweud bod rhai meysydd wedi'u cuddio.",
    "tour.s4.step.2": "Dewiswch Dangos llawn i fynd yn ôl at y cofnod llawn.",
    "tour.s4.step.3":
      "Dewiswch Allforio data (GDPR) i lawrlwytho data'r cofnod fel ffeil JSON.",
    "tour.s4.step.4":
      "Dewiswch Dangos llwybr archwilio i lwytho hanes y newidiadau, a Cuddio llwybr archwilio i'w gau; mae cofnod heb hanes yn dweud Dim cofnodion archwilio.",
    "tour.s5.title": "Adolygu ymgeiswyr dyblyg",
    "tour.s5.summary":
      "Mae'r dudalen Adolygu yn dal y ciw o barau dyblyg tebygol a ganfu sganiau, lle rydych yn cadarnhau neu'n gwrthod pob un.",
    "tour.s5.step.1":
      "Agorwch Adolygu a dewiswch Rhedeg sgan i wirio'r gofrestr, yna hidlwch y ciw yn ôl Statws a Maint tudalen.",
    "tour.s5.step.2":
      "Mae parau ymgeisiol yn ymddangos ar Fwrdd ac mewn tabl Ciw sy'n dangos y pâr, sgôr, ansawdd, ffynhonnell a statws.",
    "tour.s5.step.3":
      "Dewiswch Cymharu (neu gerdyn) i weld y ddau gofnod ochr yn ochr, gyda'r Dadansoddiad sgôr yn ôl cydran.",
    "tour.s5.step.4":
      "Dewiswch Cadarnhau dyblyg neu Gwrthod, neu llusgwch gerdyn sy'n aros i'r golofn wedi'i chadarnhau neu wedi'i gwrthod; mae eitem a benderfynwyd yn cael ei chloi.",
    "tour.s6.title": "Uno dau gofnod",
    "tour.s6.summary":
      "Mae uno yn plygu dyblyg wedi'i gadarnhau i'r cofnod rydych yn ei gadw, ac mae'r hanes uno yn cofnodi'r hyn a gyfunwyd.",
    "tour.s6.step.1":
      "O eitem wedi'i chadarnhau ar Adolygu, dewiswch Uno, cadw A neu Uno, cadw B i agor Uno gyda'r ddau ID wedi'u llenwi, neu agorwch Uno a theipiwch yr IDau eich hun.",
    "tour.s6.step.2":
      "Gwiriwch ID y prif sefydliad (y cofnod sy'n goroesi) ac ID y sefydliad dyblyg; rhaid iddynt fod yn wahanol. Ychwanegwch Reswm dewisol.",
    "tour.s6.step.3":
      "Dewiswch Llwytho rhagolwg i gymharu'r Prif a'r Dyblyg, yna dewiswch Uno a chadarnhewch yr anogwr, gan fod y dyblyg yn cael ei ddileu'n feddal.",
    "tour.s6.step.4":
      "Mae'r dudalen yn dangos Uno wedi'i gwblhau gyda dolen i'r prif sefydliad, ac mae Unoau diweddar yn rhestru pryd, pam a chan bwy.",
    "signin.sso": "Mewngofnodi gydag SSO",
  },
  "de-de": {
    "nav.review": "Überprüfung",
    "review.run": "Scan starten",
    "brand.name": "Main X · Organisationen",
    "nav.toggle": "Navigation umschalten",
    "nav.organizations": "Organisationen",
    "nav.newOrganization": "Neue Organisation",
    "chrome.language": "Sprache",
    "nav.share": "Teilen",
    "nav.text_size": "Textgröße",
    "share.copy_link": "Link kopieren",
    "share.copied": "Link kopiert",
    "share.copy_failed":
      "Kopieren fehlgeschlagen — bitte aus der Adressleiste kopieren",
    "chrome.theme": "Thema",
    "session.title": "Sitzung",
    "session.signedIn": "Angemeldet (Token angehängt)",
    "session.signOut": "Abmelden",
    "session.signIn": "Anmelden",
    "session.pasteToken": "Token einfügen",
    "session.accessToken": "Zugriffstoken",
    "session.pastePlaceholder": "Bearer-Token einfügen",
    "session.useToken": "Token verwenden",
    "list.title": "Organisationen",
    "list.new": "Neue Organisation",
    "list.loading": "Wird geladen…",
    "list.empty": "Noch keine Organisationen.",
    "list.createOne": "Eine erstellen",
    "detail.organization": "Organisation",
    "detail.loading": "Wird geladen…",
    "detail.notFound": "Nicht gefunden",
    "detail.legalName": "Rechtlicher Name:",
    "detail.url": "URL:",
    "detail.jurisdiction": "Zuständigkeit:",
    "detail.founded": "Gegründet:",
    "detail.identifiers": "Bezeichner:",
    "detail.keywords": "Schlüsselwörter:",
    "detail.id": "ID:",
    "detail.edit": "Bearbeiten",
    "detail.checkDuplicates": "Duplikate prüfen",
    "detail.checking": "Wird geprüft…",
    "detail.delete": "Löschen",
    "detail.showMasked": "Maskiert anzeigen",
    "detail.showFull": "Vollständig anzeigen",
    "detail.maskedNotice":
      "Maskierte Ansicht wird angezeigt — einige Felder sind geschwärzt.",
    "detail.exportGdpr": "Daten exportieren (DSGVO)",
    "detail.exportingGdpr": "Exportiere…",
    "detail.showAudit": "Audit-Protokoll anzeigen",
    "detail.hideAudit": "Audit-Protokoll ausblenden",
    "detail.auditTrail": "Audit-Protokoll",
    "detail.loadingAudit": "Audit-Protokoll wird geladen…",
    "detail.noAuditEntries": "Keine Audit-Einträge.",
    "detail.auditLoadFailed": "Laden des Audits fehlgeschlagen",
    "detail.checkFailed": "Prüfung fehlgeschlagen",
    "detail.potentialDuplicates": "Mögliche Duplikate",
    "detail.noneAboveThreshold": "Keine über der Abgleichschwelle.",
    "new.title": "Neue Organisation",
    "new.create": "Erstellen",
    "edit.title": "Organisation bearbeiten",
    "edit.organizationFallback": "Organisation",
    "edit.loading": "Wird geladen…",
    "edit.notFound": "Nicht gefunden",
    "edit.saveChanges": "Änderungen speichern",
    "form.save": "Speichern",
    "form.saving": "Wird gespeichert…",
    "form.nameRequired": "Name ist erforderlich.",
    "form.saveFailed": "Speichern fehlgeschlagen",
    "form.name": "Name",
    "form.legalName": "Rechtlicher Name",
    "form.url": "Website-URL",
    "form.jurisdiction": "Zuständigkeit (ISO 3166)",
    "form.foundingDate": "Gründungsdatum",
    "form.alternateNames": "Alternative Namen",
    "form.commaSeparated": "(durch Komma getrennt)",
    "form.keywords": "Schlüsselwörter",
    "form.sameAs": "Identisch-mit-URLs",
    "form.address": "Adresse",
    "form.street": "Straße",
    "form.locality": "Ort",
    "form.region": "Region",
    "form.postalCode": "Postleitzahl",
    "form.country": "Land",
    "form.identifiers": "Bezeichner",
    "form.value": "Wert",
    "form.remove": "Entfernen",
    "form.addIdentifier": "+ Bezeichner hinzufügen",
    "nav.merge": "Zusammenführen",
    "merge.title": "Organisationen zusammenführen",
    "merge.mainId": "ID der Hauptorganisation",
    "merge.mainIdHint": "Der Datensatz, der die Zusammenführung überlebt.",
    "merge.dupId": "ID der doppelten Organisation",
    "merge.dupIdHint":
      "Der Datensatz, der übernommen und logisch gelöscht wird.",
    "merge.reason": "Grund",
    "merge.reasonHint":
      "Optional; wird im Zusammenführungsverlauf gespeichert.",
    "merge.reasonPlaceholder": "Gleiches Unternehmen, doppelte Registrierung",
    "merge.loadPreview": "Vorschau laden",
    "merge.merging": "Wird zusammengeführt…",
    "merge.merge": "Zusammenführen",
    "merge.bothIdsRequired": "Beide IDs sind erforderlich.",
    "merge.mustDiffer": "Haupt-ID und Duplikat-ID müssen sich unterscheiden.",
    "merge.preview": "Vorschau",
    "merge.main": "Haupt",
    "merge.duplicate": "Duplikat",
    "merge.completed": "Zusammenführung abgeschlossen",
    "merge.completedDetail": "{dup} wurde in {main} zusammengeführt.",
    "merge.viewMain": "Hauptorganisation anzeigen",
    "merge.recent": "Letzte Zusammenführungen",
    "merge.recentEmpty": "Noch keine Zusammenführungen aufgezeichnet.",
    "merge.recentRefresh": "Aktualisieren",
    "merge.colMergedAt": "Zusammengeführt am",
    "merge.colReason": "Grund",
    "merge.colActor": "Akteur",
    "merge.confirm":
      "{dup} in {main} zusammenführen? Das Duplikat wird logisch gelöscht.",
    "review.intro":
      "Die gespeicherte Warteschlange möglicher Duplikate aus Stapelscans. Prüfen Sie jedes Paar und bestätigen oder lehnen Sie es ab.",
    "review.filter.status": "Status",
    "review.filter.statusAll": "Alle Status",
    "review.filter.limit": "Seitengröße",
    "review.filter.limitHint":
      "Es gibt keinen Seitenversatz — es werden nur die neuesten passenden Einträge bis zu diesem Limit angezeigt.",
    "review.board.title": "Board",
    "review.list.title": "Warteschlange",
    "review.empty": "Die Prüfwarteschlange ist leer.",
    "review.loading": "Wird geladen…",
    "review.col.pair": "Paar",
    "review.col.score": "Punktzahl",
    "review.col.quality": "Qualität",
    "review.col.provenance": "Quelle",
    "review.col.status": "Status",
    "review.col.actions": "Aktionen",
    "review.compare.open": "Vergleichen",
    "review.compare.title": "Vergleich",
    "review.compare.close": "Schließen",
    "review.compare.loading": "Beide Datensätze werden geladen…",
    "review.compare.partial":
      "Ein Datensatz konnte nicht geladen werden (möglicherweise gelöscht); es wird gezeigt, was verfügbar ist.",
    "review.compare.field": "Feld",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "Nicht erfasst",
    "review.field.score": "Punktzahl:",
    "review.field.quality": "Qualität:",
    "review.field.method": "Erkennungsmethode:",
    "review.field.provenance": "Quelle:",
    "review.field.status": "Status:",
    "review.field.legalName": "Rechtlicher Name",
    "review.breakdown.title": "Punktzahl-Aufschlüsselung",
    "review.breakdown.loading": "Paar wird bewertet…",
    "review.breakdown.none": "Keine Punktzahl-Aufschlüsselung verfügbar.",
    "review.breakdown.component": "Komponente",
    "review.breakdown.weight": "Gewichtung",
    "review.breakdown.score": "Punktzahl",
    "review.decide.confirm": "Duplikat bestätigen",
    "review.decide.reject": "Ablehnen",
    "review.decide.deciding": "Wird erfasst…",
    "review.decide.locked":
      "Über diesen Eintrag wurde bereits entschieden; er kann hier nicht geändert werden.",
    "review.merge.title": "Zusammenführen",
    "review.merge.note":
      "Bestätigen führt die Datensätze nicht zusammen — wählen Sie, welcher erhalten bleibt.",
    "review.merge.keepA": "Zusammenführen, A behalten",
    "review.merge.keepB": "Zusammenführen, B behalten",
    "review.status.pending": "Ausstehend",
    "review.status.confirmed": "Bestätigt",
    "review.status.rejected": "Abgelehnt",
    "review.status.automerged": "Automatisch zusammengeführt",
    "review.provenance.operator": "Bediener",
    "review.provenance.import": "Import",
    "review.provenance.matcherSuggested": "Vom Abgleich vorgeschlagen",
    "review.component.name": "Name",
    "review.component.address": "Adresse",
    "review.component.url": "URL",
    "review.component.jurisdiction": "Zuständigkeit",
    "review.component.foundingDate": "Gründungsdatum",
    "review.component.keywords": "Schlüsselwörter",
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
    "splash.benefits.1.title": "Weniger Dubletten",
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
    "splash.hero.title": "Ein verlässlicher Datensatz für jede Organisation",
    "splash.hero.subtitle":
      "Erfassen Sie Unternehmen, gemeinnützige Organisationen und Behörden einmal, erkennen Sie Duplikate früh und führen Sie sie sicher zusammen, mit integriertem Datenschutz und lückenlosem Audit-Protokoll.",
    "splash.benefits.1.body":
      "Duplikatprüfungen melden eine wahrscheinliche Wiederholung, bevor ein zweiter Datensatz angelegt wird.",
    "splash.benefits.2.title": "Kennungen, die haften bleiben",
    "splash.benefits.2.body":
      "Halten Sie LEI, DUNS und andere Kennungen bei jeder Organisation fest.",
    "splash.benefits.3.title": "Entscheidungen mit Belegen",
    "splash.benefits.3.body":
      "Sehen Sie für jedes Kandidatenpaar eine Aufschlüsselung der Bewertung, bevor Sie es bestätigen oder ablehnen.",
    "splash.benefits.4.title": "Sicheres Zusammenführen",
    "splash.benefits.4.body":
      "Das Zusammenführen ist ein bewusster Schritt, der einen Verlauf dessen festhält, was vereint wurde.",
    "splash.benefits.5.title": "Datenschutz beim Teilen",
    "splash.benefits.5.body":
      "Zeigen Sie einen maskierten Datensatz an oder exportieren Sie Daten, ohne mehr preiszugeben als nötig.",
    "splash.benefits.6.title": "Nachvollziehbare Änderungen",
    "splash.benefits.6.body":
      "Jede Änderung ist festgehalten, sodass Sie sehen, was wann geschehen ist.",
    "splash.features.1.title": "Organisationen durchsuchen",
    "splash.features.1.body":
      "Überblicken Sie das Register als Liste oder filtern Sie eine vollständige Datentabelle.",
    "splash.features.2.title": "Erstellen und bearbeiten",
    "splash.features.2.body":
      "Erfassen Sie Namen, Zuständigkeit, Gründungsdatum, Schlüsselwörter und Kennungen.",
    "splash.features.3.title": "Duplikate prüfen",
    "splash.features.3.body":
      "Finden Sie gespeicherte Organisationen, die dieser ähneln, mit Bewertungen.",
    "splash.features.4.title": "Überprüfungsboard",
    "splash.features.4.body":
      "Ziehen Sie Kandidatenpaare, um sie zu bestätigen oder abzulehnen, oder vergleichen Sie sie nebeneinander.",
    "splash.features.5.title": "Datensätze zusammenführen",
    "splash.features.5.body":
      "Führen Sie ein bestätigtes Duplikat in den Datensatz ein, den Sie behalten, mit Zusammenführungsverlauf.",
    "splash.features.6.title": "Maskierte Ansicht und Export",
    "splash.features.6.body":
      "Wechseln Sie zur maskierten Ansicht, laden Sie einen DSGVO-Export herunter oder öffnen Sie das Audit-Protokoll.",
    "tour.intro":
      "Ein geführter Rundgang durch das Organisationsregister: was jede Ansicht leistet und wie Sie sie nutzen, von der Erfassung einer Organisation bis zum Zusammenführen von Duplikaten.",
    "tour.s1.title": "Eine Organisation erfassen",
    "tour.s1.summary":
      "Legen Sie einen Datensatz mit Namen, Kennungen, Adresse und Schlüsselwörtern an. Ein Name ist erforderlich, und Hinweise zu Kennungen markieren fehlerhafte Werte vor dem Speichern.",
    "tour.s1.step.1":
      "Wählen Sie im Menü „Neue Organisation“ oder bei einer leeren Liste „Eine erstellen“, um das Formular zu öffnen.",
    "tour.s1.step.2":
      "Geben Sie den Namen ein (erforderlich), dazu optional rechtlicher Name, Website-URL, Zuständigkeit, Gründungsdatum, alternative Namen, Schlüsselwörter und „Identisch mit“-URLs.",
    "tour.s1.step.3":
      "Füllen Sie die Adresse aus und erfassen Sie mit „+ Bezeichner hinzufügen“ eine LEI, DUNS oder eine andere Kennung samt Wert; ein Hinweis erscheint, wenn das Format falsch aussieht.",
    "tour.s1.step.4":
      "Wählen Sie „Erstellen“. Fehlt der Name oder weist der Dienst einen Wert zurück, zeigt das Formular den Fehler, und nichts wird gespeichert.",
    "tour.s2.title": "Das Register durchsuchen und filtern",
    "tour.s2.summary":
      "Sehen Sie jede Organisation im Register als einfache Liste oder als filterbare Datentabelle und springen Sie von beiden zu einem Datensatz.",
    "tour.s2.step.1":
      "Melden Sie sich an und öffnen Sie im Menü „Organisationen“: Die Startseite listet jede Organisation nach Namen auf, mit einer Schaltfläche „Neue Organisation“.",
    "tour.s2.step.2":
      "Öffnen Sie die Tabellenansicht über den Link darunter: Eine Datentabelle zeigt Name und pid jeder Organisation sowie die Zahl der geladenen Zeilen von der Gesamtzahl.",
    "tour.s2.step.3":
      "Grenzen Sie die Zeilen mit der Filterleiste über der Tabelle nach Namen ein; das Filtern wirkt auf die bereits geladenen Zeilen.",
    "tour.s2.step.4":
      "Wählen Sie eine Zeile in der Tabelle oder einen Namen in der Startliste, um den Datensatz dieser Organisation zu öffnen.",
    "tour.s3.title": "Einen Datensatz öffnen und auf Duplikate prüfen",
    "tour.s3.summary":
      "Die Seite eines Datensatzes zeigt seine Details und die verfügbaren Aktionen: Duplikate prüfen, bearbeiten und löschen.",
    "tour.s3.step.1":
      "Öffnen Sie eine Organisation, um rechtlichen Namen, URL, Zuständigkeit, Gründungsdatum, Kennungen, Schlüsselwörter und ID zu sehen.",
    "tour.s3.step.2":
      "Wählen Sie „Duplikate prüfen“, um sie mit den gespeicherten Organisationen zu vergleichen; wahrscheinliche Treffer stehen unter „Mögliche Duplikate“, oder die Seite meldet, dass keiner über der Abgleichschwelle liegt.",
    "tour.s3.step.3":
      "Wählen Sie im Ergebnis einen Namen, um den anderen Datensatz zu öffnen und die Details zu vergleichen.",
    "tour.s3.step.4":
      "Zum Ändern eines Datensatzes wählen Sie „Bearbeiten“, passen das Formular an und wählen „Speichern“; „Löschen“ löscht den Datensatz weich und führt Sie zur Liste zurück.",
    "tour.s4.title": "Datenschutz, Export und Audit",
    "tour.s4.summary":
      "Sehen Sie eine maskierte Ansicht, laden Sie einen DSGVO-Export herunter oder lesen Sie das Audit-Protokoll, alles von der Seite eines Datensatzes aus.",
    "tour.s4.step.1":
      "Wählen Sie auf der Seite eines Datensatzes „Maskiert anzeigen“, um ihn über die maskierte Ansicht neu zu laden; ein Hinweis nennt, dass einige Felder geschwärzt sind.",
    "tour.s4.step.2":
      "Wählen Sie „Vollständig anzeigen“, um zum vollständigen Datensatz zurückzukehren.",
    "tour.s4.step.3":
      "Wählen Sie „Daten exportieren (DSGVO)“, um die Daten des Datensatzes als JSON-Datei herunterzuladen.",
    "tour.s4.step.4":
      "Wählen Sie „Audit-Protokoll anzeigen“, um den Änderungsverlauf zu laden, und „Audit-Protokoll ausblenden“, um ihn zu schließen; bei einem Datensatz ohne Verlauf steht „Keine Audit-Einträge.“",
    "tour.s5.title": "Duplikatkandidaten prüfen",
    "tour.s5.summary":
      "Die Seite „Überprüfung“ enthält die Warteschlange wahrscheinlicher Duplikatpaare aus Scans, in der Sie jedes bestätigen oder ablehnen.",
    "tour.s5.step.1":
      "Öffnen Sie „Überprüfung“ und wählen Sie „Scan starten“, um das Register zu prüfen; filtern Sie die Warteschlange dann nach Status und Seitengröße.",
    "tour.s5.step.2":
      "Kandidatenpaare erscheinen auf einem „Board“ und in einer Tabelle „Warteschlange“ mit Paar, Bewertung, Qualität, Quelle und Status.",
    "tour.s5.step.3":
      "Wählen Sie „Vergleichen“ (oder eine Karte), um beide Datensätze nebeneinander zu sehen, mit der Punktzahl-Aufschlüsselung nach Komponenten.",
    "tour.s5.step.4":
      "Wählen Sie „Duplikat bestätigen“ oder „Ablehnen“, oder ziehen Sie eine offene Karte in die Spalte „Bestätigt“ oder „Abgelehnt“; ein entschiedener Eintrag ist gesperrt.",
    "tour.s6.title": "Zwei Datensätze zusammenführen",
    "tour.s6.summary":
      "Beim Zusammenführen wird ein bestätigtes Duplikat in den Datensatz eingeführt, den Sie behalten, und der Zusammenführungsverlauf hält fest, was vereint wurde.",
    "tour.s6.step.1":
      "Wählen Sie bei einem bestätigten Eintrag in „Überprüfung“ „Zusammenführen, A behalten“ oder „Zusammenführen, B behalten“, um „Zusammenführen“ mit beiden IDs ausgefüllt zu öffnen, oder öffnen Sie „Zusammenführen“ und tippen Sie die IDs selbst ein.",
    "tour.s6.step.2":
      "Prüfen Sie die „ID der Hauptorganisation“ (der bleibende Datensatz) und die „ID der doppelten Organisation“; beide müssen verschieden sein. Ergänzen Sie optional einen „Grund“.",
    "tour.s6.step.3":
      "Wählen Sie „Vorschau laden“, um Haupt- und Duplikat-Datensatz zu vergleichen, dann „Zusammenführen“, und bestätigen Sie die Rückfrage, denn das Duplikat wird weich gelöscht.",
    "tour.s6.step.4":
      "Die Seite zeigt „Zusammenführung abgeschlossen“ mit einem Link zur Hauptorganisation, und „Letzte Zusammenführungen“ listet auf, wann, warum und von wem.",
  },
  "en-001": {
    "nav.review": "Review",
    "review.run": "Run scan",
    // Layout / chrome
    "brand.name": "Main X · Organizations",
    "nav.toggle": "Toggle navigation",
    "nav.organizations": "Organizations",
    "nav.newOrganization": "New organization",
    "chrome.language": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    "chrome.theme": "Theme",
    // Session panel
    "session.title": "Session",
    "session.signedIn": "Signed in (token attached)",
    "session.signOut": "Sign out",
    "session.signIn": "Sign in",
    "session.pasteToken": "Paste a token",
    "session.accessToken": "Access token",
    "session.pastePlaceholder": "Paste bearer token",
    "session.useToken": "Use token",
    // List page
    "list.title": "Organizations",
    "list.new": "New organization",
    "list.loading": "Loading…",
    "list.empty": "No organizations yet.",
    "list.createOne": "Create one",
    // Detail page
    "detail.organization": "Organization",
    "detail.loading": "Loading…",
    "detail.notFound": "Not found",
    "detail.legalName": "Legal name:",
    "detail.url": "URL:",
    "detail.jurisdiction": "Jurisdiction:",
    "detail.founded": "Founded:",
    "detail.identifiers": "Identifiers:",
    "detail.keywords": "Keywords:",
    "detail.id": "ID:",
    "detail.edit": "Edit",
    "detail.checkDuplicates": "Check duplicates",
    "detail.checking": "Checking…",
    "detail.delete": "Delete",
    "detail.showMasked": "Show masked",
    "detail.showFull": "Show full",
    "detail.maskedNotice":
      "Showing the masked view — some fields are redacted.",
    "detail.exportGdpr": "Export data (GDPR)",
    "detail.exportingGdpr": "Exporting…",
    "detail.showAudit": "Show audit trail",
    "detail.hideAudit": "Hide audit trail",
    "detail.auditTrail": "Audit trail",
    "detail.loadingAudit": "Loading audit trail…",
    "detail.noAuditEntries": "No audit entries.",
    "detail.auditLoadFailed": "Audit load failed",
    "detail.checkFailed": "Check failed",
    "detail.potentialDuplicates": "Potential duplicates",
    "detail.noneAboveThreshold": "None above the match threshold.",
    // New page
    "new.title": "New organization",
    "new.create": "Create",
    // Edit page
    "edit.title": "Edit organization",
    "edit.organizationFallback": "organization",
    "edit.loading": "Loading…",
    "edit.notFound": "Not found",
    "edit.saveChanges": "Save changes",
    // Form
    "form.save": "Save",
    "form.saving": "Saving…",
    "form.nameRequired": "Name is required.",
    "form.saveFailed": "Save failed",
    "form.name": "Name",
    "form.legalName": "Legal name",
    "form.url": "Website URL",
    "form.jurisdiction": "Jurisdiction (ISO 3166)",
    "form.foundingDate": "Founding date",
    "form.alternateNames": "Alternate names",
    "form.commaSeparated": "(comma-separated)",
    "form.keywords": "Keywords",
    "form.sameAs": "Same-as URLs",
    "form.address": "Address",
    "form.street": "Street",
    "form.locality": "Locality",
    "form.region": "Region",
    "form.postalCode": "Postal code",
    "form.country": "Country",
    "form.identifiers": "Identifiers",
    "form.value": "value",
    "form.remove": "Remove",
    "form.addIdentifier": "+ Add identifier",
    // Merge page
    "nav.merge": "Merge",
    "merge.title": "Merge organizations",
    "merge.mainId": "Main organization ID",
    "merge.mainIdHint": "The record that survives the merge.",
    "merge.dupId": "Duplicate organization ID",
    "merge.dupIdHint": "The record folded in and soft-deleted.",
    "merge.reason": "Reason",
    "merge.reasonHint": "Optional; kept in the merge history.",
    "merge.reasonPlaceholder": "Same company, duplicate registration",
    "merge.loadPreview": "Load preview",
    "merge.merging": "Merging…",
    "merge.merge": "Merge",
    "merge.bothIdsRequired": "Both IDs are required.",
    "merge.mustDiffer": "The main and duplicate IDs must differ.",
    "merge.preview": "Preview",
    "merge.main": "Main",
    "merge.duplicate": "Duplicate",
    "merge.completed": "Merge completed",
    "merge.completedDetail": "{dup} was merged into {main}.",
    "merge.viewMain": "View the main organization",
    "merge.recent": "Recent merges",
    "merge.recentEmpty": "No merges recorded yet.",
    "merge.recentRefresh": "Refresh",
    "merge.colMergedAt": "Merged at",
    "merge.colReason": "Reason",
    "merge.colActor": "Actor",
    "merge.confirm":
      "Merge {dup} into {main}? The duplicate will be soft-deleted.",
    "review.intro":
      "The stored duplicate-candidate queue from batch scans. Review each pair and confirm or reject it.",
    "review.filter.status": "Status",
    "review.filter.statusAll": "All statuses",
    "review.filter.limit": "Page size",
    "review.filter.limitHint":
      "There is no page offset — only the newest matching items up to this limit are shown.",
    "review.board.title": "Board",
    "review.list.title": "Queue",
    "review.empty": "The review queue is empty.",
    "review.loading": "Loading…",
    "review.col.pair": "Pair",
    "review.col.score": "Score",
    "review.col.quality": "Quality",
    "review.col.provenance": "Source",
    "review.col.status": "Status",
    "review.col.actions": "Actions",
    "review.compare.open": "Compare",
    "review.compare.title": "Comparison",
    "review.compare.close": "Close",
    "review.compare.loading": "Loading both records…",
    "review.compare.partial":
      "One record could not be loaded (it may have been deleted); showing what is available.",
    "review.compare.field": "Field",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "Not recorded",
    "review.field.score": "Score:",
    "review.field.quality": "Quality:",
    "review.field.method": "Detection method:",
    "review.field.provenance": "Source:",
    "review.field.status": "Status:",
    "review.field.legalName": "Legal name",
    "review.breakdown.title": "Score breakdown",
    "review.breakdown.loading": "Scoring the pair…",
    "review.breakdown.none": "No score breakdown is available.",
    "review.breakdown.component": "Component",
    "review.breakdown.weight": "Weight",
    "review.breakdown.score": "Score",
    "review.decide.confirm": "Confirm duplicate",
    "review.decide.reject": "Reject",
    "review.decide.deciding": "Recording…",
    "review.decide.locked":
      "This item was already decided and cannot be changed here.",
    "review.merge.title": "Merge",
    "review.merge.note":
      "Confirming does not merge the records — choose which one survives.",
    "review.merge.keepA": "Merge, keep A",
    "review.merge.keepB": "Merge, keep B",
    "review.status.pending": "Pending",
    "review.status.confirmed": "Confirmed",
    "review.status.rejected": "Rejected",
    "review.status.automerged": "Auto-merged",
    "review.provenance.operator": "Operator",
    "review.provenance.import": "Import",
    "review.provenance.matcherSuggested": "Matcher-suggested",
    "review.component.name": "Name",
    "review.component.address": "Address",
    "review.component.url": "URL",
    "review.component.jurisdiction": "Jurisdiction",
    "review.component.foundingDate": "Founding date",
    "review.component.keywords": "Keywords",
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
    "splash.hero.title": "One trusted record for every organization",
    "splash.hero.subtitle":
      "Register companies, charities and agencies once, catch duplicates early, and merge them safely, with privacy and a full audit trail built in.",
    "splash.benefits.1.title": "Fewer duplicates",
    "splash.benefits.1.body":
      "Duplicate checks flag a likely repeat before a second record is created.",
    "splash.benefits.2.title": "Identifiers that stick",
    "splash.benefits.2.body":
      "Keep LEI, DUNS and other identifiers against each organization.",
    "splash.benefits.3.title": "Decisions with evidence",
    "splash.benefits.3.body":
      "See a score breakdown for each candidate pair before you confirm or reject it.",
    "splash.benefits.4.title": "Safe merging",
    "splash.benefits.4.body":
      "Merging is a deliberate step that keeps a history of what was combined.",
    "splash.benefits.5.title": "Privacy when sharing",
    "splash.benefits.5.body":
      "View a masked record or export data without exposing more than you should.",
    "splash.benefits.6.title": "Accountable changes",
    "splash.benefits.6.body":
      "Every change is on the record, so you can see what happened and when.",
    "splash.features.1.title": "Browse organizations",
    "splash.features.1.body":
      "Scan the registry as a list, or filter a full data grid.",
    "splash.features.2.title": "Create and edit",
    "splash.features.2.body":
      "Capture names, jurisdiction, founding date, keywords and identifiers.",
    "splash.features.3.title": "Check duplicates",
    "splash.features.3.body":
      "Find stored organizations that look like this one, with scores.",
    "splash.features.4.title": "Review board",
    "splash.features.4.body":
      "Drag candidate pairs to confirm or reject, or compare them side by side.",
    "splash.features.5.title": "Merge records",
    "splash.features.5.body":
      "Combine a confirmed duplicate into the record you keep, with a merge history.",
    "splash.features.6.title": "Masked view and export",
    "splash.features.6.body":
      "Switch to a masked view, download a GDPR export, or open the audit trail.",
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
      "A guided walkthrough of the Organizations registry: what each screen does and the steps to use it, from registering an organization to merging duplicates.",
    "tour.s1.title": "Register an organization",
    "tour.s1.summary":
      "Create a record with names, identifiers, an address and keywords. A name is required, and identifier hints flag malformed values before you save.",
    "tour.s1.step.1":
      "Choose New organization in the menu, or Create one on an empty list, to open the form.",
    "tour.s1.step.2":
      "Enter the Name (required), then optionally the legal name, website URL, jurisdiction, founding date, alternate names, keywords and same-as URLs.",
    "tour.s1.step.3":
      "Fill in the address, and use + Add identifier to record an LEI, DUNS or other identifier with its value; a hint appears if the format looks wrong.",
    "tour.s1.step.4":
      "Select Create. If the name is missing or the service rejects a value, the form shows the error and nothing is saved.",
    "tour.s2.title": "Browse and filter the registry",
    "tour.s2.summary":
      "See every organization in the registry as a simple list or as a filterable data grid, and jump from either to a record.",
    "tour.s2.step.1":
      "Sign in and open Organizations from the menu: the home page lists every organization by name, with a New organization button.",
    "tour.s2.step.2":
      "Open the grid view with the link below: a data grid shows each organization's Name and pid, plus a count of rows loaded out of the total.",
    "tour.s2.step.3":
      "Use the filter bar above the grid to narrow the rows by name; filtering works on the rows already loaded.",
    "tour.s2.step.4":
      "Select a row in the grid, or a name on the home list, to open that organization's record.",
    "tour.s3.title": "Open a record and check for duplicates",
    "tour.s3.summary":
      "A record's page shows its details and the actions available on it: check duplicates, edit and delete.",
    "tour.s3.step.1":
      "Open an organization to see its legal name, URL, jurisdiction, founding date, identifiers, keywords and ID.",
    "tour.s3.step.2":
      "Select Check duplicates to compare it with the stored organizations; likely matches are listed under Potential duplicates, or the page says none are above the match threshold.",
    "tour.s3.step.3":
      "Select a name in the results to open that other record and compare the details.",
    "tour.s3.step.4":
      "To change a record, select Edit, adjust the form and choose Save changes; Delete soft-deletes the record and returns you to the list.",
    "tour.s4.title": "Privacy, export and audit",
    "tour.s4.summary":
      "Look at a masked view, download a GDPR export, or read the audit trail, all from a record's page.",
    "tour.s4.step.1":
      "On a record's page, select Show masked to reload it through the masked view; a notice says some fields are redacted.",
    "tour.s4.step.2": "Select Show full to go back to the full record.",
    "tour.s4.step.3":
      "Select Export data (GDPR) to download the record's data as a JSON file.",
    "tour.s4.step.4":
      "Select Show audit trail to load the history of changes, and Hide audit trail to close it; a record with no history reads No audit entries.",
    "tour.s5.title": "Review duplicate candidates",
    "tour.s5.summary":
      "The Review page holds the queue of likely duplicate pairs found by scans, where you confirm or reject each one.",
    "tour.s5.step.1":
      "Open Review and select Run scan to check the registry, then filter the queue by Status and Page size.",
    "tour.s5.step.2":
      "Candidate pairs appear on a Board and in a Queue table showing the pair, score, quality, source and status.",
    "tour.s5.step.3":
      "Select Compare (or a card) to see both records side by side, with the Score breakdown by component.",
    "tour.s5.step.4":
      "Choose Confirm duplicate or Reject, or drag a pending card to the confirmed or rejected column; a decided item is locked.",
    "tour.s6.title": "Merge two records",
    "tour.s6.summary":
      "Merging folds a confirmed duplicate into the record you keep, and the merge history records what was combined.",
    "tour.s6.step.1":
      "From a confirmed item on Review, choose Merge, keep A or Merge, keep B to open Merge with both IDs filled in, or open Merge and type the IDs yourself.",
    "tour.s6.step.2":
      "Check the Main organization ID (the record that survives) and the Duplicate organization ID; they must differ. Add an optional Reason.",
    "tour.s6.step.3":
      "Select Load preview to compare Main and Duplicate, then select Merge and confirm the prompt, because the duplicate is soft-deleted.",
    "tour.s6.step.4":
      "The page shows Merge completed with a link to the main organization, and Recent merges lists when, why and by whom.",
    "signin.sso": "Sign in with SSO",
  },
  "es-001": {
    "nav.review": "Revisión",
    "review.run": "Ejecutar análisis",
    "brand.name": "Main X · Organizaciones",
    "nav.toggle": "Alternar navegación",
    "nav.organizations": "Organizaciones",
    "nav.newOrganization": "Nueva organización",
    "chrome.language": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "chrome.theme": "Tema",
    "session.title": "Sesión",
    "session.signedIn": "Sesión iniciada (token adjunto)",
    "session.signOut": "Cerrar sesión",
    "session.signIn": "Iniciar sesión",
    "session.pasteToken": "Pegar un token",
    "session.accessToken": "Token de acceso",
    "session.pastePlaceholder": "Pegar token de portador",
    "session.useToken": "Usar token",
    "list.title": "Organizaciones",
    "list.new": "Nueva organización",
    "list.loading": "Cargando…",
    "list.empty": "Aún no hay organizaciones.",
    "list.createOne": "Crear una",
    "detail.organization": "Organización",
    "detail.loading": "Cargando…",
    "detail.notFound": "No encontrado",
    "detail.legalName": "Nombre legal:",
    "detail.url": "URL:",
    "detail.jurisdiction": "Jurisdicción:",
    "detail.founded": "Fundada:",
    "detail.identifiers": "Identificadores:",
    "detail.keywords": "Palabras clave:",
    "detail.id": "ID:",
    "detail.edit": "Editar",
    "detail.checkDuplicates": "Comprobar duplicados",
    "detail.checking": "Comprobando…",
    "detail.delete": "Eliminar",
    "detail.showMasked": "Mostrar enmascarado",
    "detail.showFull": "Mostrar completo",
    "detail.maskedNotice":
      "Mostrando la vista enmascarada: algunos campos están ocultos.",
    "detail.exportGdpr": "Exportar datos (RGPD)",
    "detail.exportingGdpr": "Exportando…",
    "detail.showAudit": "Mostrar registro de auditoría",
    "detail.hideAudit": "Ocultar registro de auditoría",
    "detail.auditTrail": "Registro de auditoría",
    "detail.loadingAudit": "Cargando registro de auditoría…",
    "detail.noAuditEntries": "Sin entradas de auditoría.",
    "detail.auditLoadFailed": "Error al cargar la auditoría",
    "detail.checkFailed": "La comprobación falló",
    "detail.potentialDuplicates": "Posibles duplicados",
    "detail.noneAboveThreshold":
      "Ninguno por encima del umbral de coincidencia.",
    "new.title": "Nueva organización",
    "new.create": "Crear",
    "edit.title": "Editar organización",
    "edit.organizationFallback": "organización",
    "edit.loading": "Cargando…",
    "edit.notFound": "No encontrado",
    "edit.saveChanges": "Guardar cambios",
    "form.save": "Guardar",
    "form.saving": "Guardando…",
    "form.nameRequired": "El nombre es obligatorio.",
    "form.saveFailed": "Error al guardar",
    "form.name": "Nombre",
    "form.legalName": "Nombre legal",
    "form.url": "URL del sitio web",
    "form.jurisdiction": "Jurisdicción (ISO 3166)",
    "form.foundingDate": "Fecha de fundación",
    "form.alternateNames": "Nombres alternativos",
    "form.commaSeparated": "(separados por comas)",
    "form.keywords": "Palabras clave",
    "form.sameAs": "URL de igual que",
    "form.address": "Dirección",
    "form.street": "Calle",
    "form.locality": "Localidad",
    "form.region": "Región",
    "form.postalCode": "Código postal",
    "form.country": "País",
    "form.identifiers": "Identificadores",
    "form.value": "valor",
    "form.remove": "Eliminar",
    "form.addIdentifier": "+ Añadir identificador",
    "nav.merge": "Fusionar",
    "merge.title": "Fusionar organizaciones",
    "merge.mainId": "ID de la organización principal",
    "merge.mainIdHint": "El registro que sobrevive a la fusión.",
    "merge.dupId": "ID de la organización duplicada",
    "merge.dupIdHint":
      "El registro que se incorpora y se elimina de forma lógica.",
    "merge.reason": "Motivo",
    "merge.reasonHint": "Opcional; se conserva en el historial de fusiones.",
    "merge.reasonPlaceholder": "Misma empresa, registro duplicado",
    "merge.loadPreview": "Cargar vista previa",
    "merge.merging": "Fusionando…",
    "merge.merge": "Fusionar",
    "merge.bothIdsRequired": "Ambos ID son obligatorios.",
    "merge.mustDiffer": "El ID principal y el duplicado deben ser distintos.",
    "merge.preview": "Vista previa",
    "merge.main": "Principal",
    "merge.duplicate": "Duplicada",
    "merge.completed": "Fusión completada",
    "merge.completedDetail": "{dup} se fusionó en {main}.",
    "merge.viewMain": "Ver la organización principal",
    "merge.recent": "Fusiones recientes",
    "merge.recentEmpty": "Aún no hay fusiones registradas.",
    "merge.recentRefresh": "Actualizar",
    "merge.colMergedAt": "Fusionado el",
    "merge.colReason": "Motivo",
    "merge.colActor": "Autor",
    "merge.confirm":
      "¿Fusionar {dup} en {main}? La duplicada se eliminará de forma lógica.",
    "review.intro":
      "La cola almacenada de posibles duplicados de los análisis por lotes. Revise cada par y confírmelo o recházelo.",
    "review.filter.status": "Estado",
    "review.filter.statusAll": "Todos los estados",
    "review.filter.limit": "Tamaño de página",
    "review.filter.limitHint":
      "No hay desplazamiento de página — solo se muestran los elementos coincidentes más recientes hasta este límite.",
    "review.board.title": "Tablero",
    "review.list.title": "Cola",
    "review.empty": "La cola de revisión está vacía.",
    "review.loading": "Cargando…",
    "review.col.pair": "Par",
    "review.col.score": "Puntuación",
    "review.col.quality": "Calidad",
    "review.col.provenance": "Origen",
    "review.col.status": "Estado",
    "review.col.actions": "Acciones",
    "review.compare.open": "Comparar",
    "review.compare.title": "Comparación",
    "review.compare.close": "Cerrar",
    "review.compare.loading": "Cargando ambos registros…",
    "review.compare.partial":
      "No se pudo cargar un registro (puede haber sido eliminado); se muestra lo disponible.",
    "review.compare.field": "Campo",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "No registrado",
    "review.field.score": "Puntuación:",
    "review.field.quality": "Calidad:",
    "review.field.method": "Método de detección:",
    "review.field.provenance": "Origen:",
    "review.field.status": "Estado:",
    "review.field.legalName": "Nombre legal",
    "review.breakdown.title": "Desglose de la puntuación",
    "review.breakdown.loading": "Puntuando el par…",
    "review.breakdown.none": "No hay desglose de puntuación disponible.",
    "review.breakdown.component": "Componente",
    "review.breakdown.weight": "Peso",
    "review.breakdown.score": "Puntuación",
    "review.decide.confirm": "Confirmar duplicado",
    "review.decide.reject": "Rechazar",
    "review.decide.deciding": "Registrando…",
    "review.decide.locked":
      "Este elemento ya fue decidido y no se puede cambiar aquí.",
    "review.merge.title": "Fusionar",
    "review.merge.note":
      "Confirmar no fusiona los registros — elija cuál sobrevive.",
    "review.merge.keepA": "Fusionar, conservar A",
    "review.merge.keepB": "Fusionar, conservar B",
    "review.status.pending": "Pendiente",
    "review.status.confirmed": "Confirmado",
    "review.status.rejected": "Rechazado",
    "review.status.automerged": "Fusionado automáticamente",
    "review.provenance.operator": "Operador",
    "review.provenance.import": "Importación",
    "review.provenance.matcherSuggested": "Sugerido por el comparador",
    "review.component.name": "Nombre",
    "review.component.address": "Dirección",
    "review.component.url": "URL",
    "review.component.jurisdiction": "Jurisdicción",
    "review.component.foundingDate": "Fecha de fundación",
    "review.component.keywords": "Palabras clave",
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
    "splash.hero.title": "Un registro fiable para cada organización",
    "splash.hero.subtitle":
      "Registra empresas, entidades benéficas y organismos una sola vez, detecta duplicados a tiempo y fusiónalos con seguridad, con privacidad y auditoría completa integradas.",
    "splash.benefits.1.title": "Menos duplicados",
    "splash.benefits.1.body":
      "Las comprobaciones de duplicados señalan una posible repetición antes de crear un segundo registro.",
    "splash.benefits.2.title": "Identificadores fiables",
    "splash.benefits.2.body":
      "Guarda el LEI, el DUNS y otros identificadores de cada organización.",
    "splash.benefits.3.title": "Decisiones con evidencia",
    "splash.benefits.3.body":
      "Consulta el desglose de puntuación de cada par candidato antes de confirmarlo o rechazarlo.",
    "splash.benefits.4.title": "Fusión segura",
    "splash.benefits.4.body":
      "La fusión es un paso deliberado que conserva el historial de lo combinado.",
    "splash.benefits.5.title": "Privacidad al compartir",
    "splash.benefits.5.body":
      "Consulta un registro enmascarado o exporta datos sin revelar más de lo debido.",
    "splash.benefits.6.title": "Cambios con trazabilidad",
    "splash.benefits.6.body":
      "Cada cambio queda registrado, así que sabes qué pasó y cuándo.",
    "splash.features.1.title": "Explorar organizaciones",
    "splash.features.1.body":
      "Recorre el registro como lista o filtra una tabla de datos completa.",
    "splash.features.2.title": "Crear y editar",
    "splash.features.2.body":
      "Registra nombres, jurisdicción, fecha de fundación, palabras clave e identificadores.",
    "splash.features.3.title": "Comprobar duplicados",
    "splash.features.3.body":
      "Encuentra organizaciones almacenadas parecidas a esta, con sus puntuaciones.",
    "splash.features.4.title": "Tablero de revisión",
    "splash.features.4.body":
      "Arrastra los pares candidatos para confirmar o rechazar, o compáralos lado a lado.",
    "splash.features.5.title": "Fusionar registros",
    "splash.features.5.body":
      "Combina un duplicado confirmado con el registro que conservas, con historial de fusiones.",
    "splash.features.6.title": "Vista enmascarada y exportación",
    "splash.features.6.body":
      "Cambia a la vista enmascarada, descarga una exportación RGPD o abre el registro de auditoría.",
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
      "Un recorrido guiado por el registro de organizaciones: qué hace cada pantalla y los pasos para usarla, desde registrar una organización hasta fusionar duplicados.",
    "tour.s1.title": "Registrar una organización",
    "tour.s1.summary":
      "Crea un registro con nombres, identificadores, dirección y palabras clave. El nombre es obligatorio y las sugerencias de identificador señalan valores mal formados antes de guardar.",
    "tour.s1.step.1":
      "Elige Nueva organización en el menú, o Crear una en una lista vacía, para abrir el formulario.",
    "tour.s1.step.2":
      "Introduce el Nombre (obligatorio) y, si quieres, el nombre legal, la URL del sitio web, la jurisdicción, la fecha de fundación, los nombres alternativos, las palabras clave y las URL de igual que.",
    "tour.s1.step.3":
      "Rellena la dirección y usa + Añadir identificador para registrar un LEI, DUNS u otro identificador con su valor; aparece una sugerencia si el formato parece incorrecto.",
    "tour.s1.step.4":
      "Selecciona Crear. Si falta el nombre o el servicio rechaza un valor, el formulario muestra el error y no se guarda nada.",
    "tour.s2.title": "Explorar y filtrar el registro",
    "tour.s2.summary":
      "Consulta todas las organizaciones del registro como una lista sencilla o como una cuadrícula de datos filtrable, y salta desde cualquiera de ellas a un registro.",
    "tour.s2.step.1":
      "Inicia sesión y abre Organizaciones desde el menú: la página de inicio lista cada organización por nombre, con un botón Nueva organización.",
    "tour.s2.step.2":
      "Abre la vista de cuadrícula con el enlace de abajo: una cuadrícula de datos muestra el Nombre y el pid de cada organización, además del número de filas cargadas sobre el total.",
    "tour.s2.step.3":
      "Usa la barra de filtros sobre la cuadrícula para acotar las filas por nombre; el filtrado actúa sobre las filas ya cargadas.",
    "tour.s2.step.4":
      "Selecciona una fila de la cuadrícula, o un nombre de la lista de inicio, para abrir el registro de esa organización.",
    "tour.s3.title": "Abrir un registro y comprobar duplicados",
    "tour.s3.summary":
      "La página de un registro muestra sus datos y las acciones disponibles: comprobar duplicados, editar y eliminar.",
    "tour.s3.step.1":
      "Abre una organización para ver su nombre legal, URL, jurisdicción, fecha de fundación, identificadores, palabras clave e ID.",
    "tour.s3.step.2":
      "Selecciona Comprobar duplicados para compararla con las organizaciones almacenadas; las posibles coincidencias aparecen en Posibles duplicados, o la página indica que ninguna supera el umbral de coincidencia.",
    "tour.s3.step.3":
      "Selecciona un nombre en los resultados para abrir ese otro registro y comparar los datos.",
    "tour.s3.step.4":
      "Para cambiar un registro, selecciona Editar, ajusta el formulario y elige Guardar cambios; Eliminar lo elimina de forma lógica y te devuelve a la lista.",
    "tour.s4.title": "Privacidad, exportación y auditoría",
    "tour.s4.summary":
      "Consulta una vista enmascarada, descarga una exportación RGPD o lee el registro de auditoría, todo desde la página de un registro.",
    "tour.s4.step.1":
      "En la página de un registro, selecciona Mostrar enmascarado para recargarlo con la vista enmascarada; un aviso indica que algunos campos están ocultos.",
    "tour.s4.step.2":
      "Selecciona Mostrar completo para volver al registro completo.",
    "tour.s4.step.3":
      "Selecciona Exportar datos (RGPD) para descargar los datos del registro como archivo JSON.",
    "tour.s4.step.4":
      "Selecciona Mostrar registro de auditoría para cargar el historial de cambios y Ocultar registro de auditoría para cerrarlo; un registro sin historial indica Sin entradas de auditoría.",
    "tour.s5.title": "Revisar posibles duplicados",
    "tour.s5.summary":
      "La página Revisión contiene la cola de pares probablemente duplicados que hallaron los análisis, donde confirmas o rechazas cada uno.",
    "tour.s5.step.1":
      "Abre Revisión y selecciona Ejecutar análisis para comprobar el registro; después filtra la cola por Estado y Tamaño de página.",
    "tour.s5.step.2":
      "Los pares candidatos aparecen en un Tablero y en una tabla Cola que muestra el par, la puntuación, la calidad, el origen y el estado.",
    "tour.s5.step.3":
      "Selecciona Comparar (o una tarjeta) para ver ambos registros lado a lado, con el Desglose de la puntuación por componente.",
    "tour.s5.step.4":
      "Elige Confirmar duplicado o Rechazar, o arrastra una tarjeta pendiente a la columna de confirmado o rechazado; un elemento ya decidido queda bloqueado.",
    "tour.s6.title": "Fusionar dos registros",
    "tour.s6.summary":
      "Fusionar incorpora un duplicado confirmado al registro que conservas, y el historial de fusiones registra lo que se combinó.",
    "tour.s6.step.1":
      "Desde un elemento confirmado en Revisión, elige Fusionar, conservar A o Fusionar, conservar B para abrir Fusionar con ambos ID rellenados, o abre Fusionar y escribe tú los ID.",
    "tour.s6.step.2":
      "Comprueba el ID de la organización principal (el registro que sobrevive) y el ID de la organización duplicada; deben ser distintos. Añade un Motivo opcional.",
    "tour.s6.step.3":
      "Selecciona Cargar vista previa para comparar Principal y Duplicada, luego selecciona Fusionar y confirma el aviso, porque la duplicada se elimina de forma lógica.",
    "tour.s6.step.4":
      "La página muestra Fusión completada con un enlace a la organización principal, y Fusiones recientes indica cuándo, por qué y quién.",
    "signin.sso": "Iniciar sesión con SSO",
  },
  "fr-001": {
    "nav.review": "Révision",
    "review.run": "Lancer l'analyse",
    "brand.name": "Main X · Organisations",
    "nav.toggle": "Basculer la navigation",
    "nav.organizations": "Organisations",
    "nav.newOrganization": "Nouvelle organisation",
    "chrome.language": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "chrome.theme": "Thème",
    "session.title": "Session",
    "session.signedIn": "Connecté (jeton attaché)",
    "session.signOut": "Se déconnecter",
    "session.signIn": "Se connecter",
    "session.pasteToken": "Coller un jeton",
    "session.accessToken": "Jeton d'accès",
    "session.pastePlaceholder": "Coller le jeton porteur",
    "session.useToken": "Utiliser le jeton",
    "list.title": "Organisations",
    "list.new": "Nouvelle organisation",
    "list.loading": "Chargement…",
    "list.empty": "Aucune organisation pour le moment.",
    "list.createOne": "En créer une",
    "detail.organization": "Organisation",
    "detail.loading": "Chargement…",
    "detail.notFound": "Introuvable",
    "detail.legalName": "Nom légal :",
    "detail.url": "URL :",
    "detail.jurisdiction": "Juridiction :",
    "detail.founded": "Fondée :",
    "detail.identifiers": "Identifiants :",
    "detail.keywords": "Mots-clés :",
    "detail.id": "ID :",
    "detail.edit": "Modifier",
    "detail.checkDuplicates": "Vérifier les doublons",
    "detail.checking": "Vérification…",
    "detail.delete": "Supprimer",
    "detail.showMasked": "Afficher masqué",
    "detail.showFull": "Afficher complet",
    "detail.maskedNotice":
      "Affichage de la vue masquée — certains champs sont masqués.",
    "detail.exportGdpr": "Exporter les données (RGPD)",
    "detail.exportingGdpr": "Exportation…",
    "detail.showAudit": "Afficher le journal d'audit",
    "detail.hideAudit": "Masquer le journal d'audit",
    "detail.auditTrail": "Journal d'audit",
    "detail.loadingAudit": "Chargement du journal d'audit…",
    "detail.noAuditEntries": "Aucune entrée d'audit.",
    "detail.auditLoadFailed": "Échec du chargement de l'audit",
    "detail.checkFailed": "Échec de la vérification",
    "detail.potentialDuplicates": "Doublons possibles",
    "detail.noneAboveThreshold": "Aucun au-dessus du seuil de correspondance.",
    "new.title": "Nouvelle organisation",
    "new.create": "Créer",
    "edit.title": "Modifier l'organisation",
    "edit.organizationFallback": "organisation",
    "edit.loading": "Chargement…",
    "edit.notFound": "Introuvable",
    "edit.saveChanges": "Enregistrer les modifications",
    "form.save": "Enregistrer",
    "form.saving": "Enregistrement…",
    "form.nameRequired": "Le nom est requis.",
    "form.saveFailed": "Échec de l'enregistrement",
    "form.name": "Nom",
    "form.legalName": "Nom légal",
    "form.url": "URL du site web",
    "form.jurisdiction": "Juridiction (ISO 3166)",
    "form.foundingDate": "Date de fondation",
    "form.alternateNames": "Noms alternatifs",
    "form.commaSeparated": "(séparés par des virgules)",
    "form.keywords": "Mots-clés",
    "form.sameAs": "URL identiques à",
    "form.address": "Adresse",
    "form.street": "Rue",
    "form.locality": "Localité",
    "form.region": "Région",
    "form.postalCode": "Code postal",
    "form.country": "Pays",
    "form.identifiers": "Identifiants",
    "form.value": "valeur",
    "form.remove": "Supprimer",
    "form.addIdentifier": "+ Ajouter un identifiant",
    "nav.merge": "Fusionner",
    "merge.title": "Fusionner des organisations",
    "merge.mainId": "ID de l'organisation principale",
    "merge.mainIdHint": "L'enregistrement conservé après la fusion.",
    "merge.dupId": "ID de l'organisation en double",
    "merge.dupIdHint": "L'enregistrement intégré puis supprimé logiquement.",
    "merge.reason": "Motif",
    "merge.reasonHint": "Facultatif ; conservé dans l'historique des fusions.",
    "merge.reasonPlaceholder": "Même société, enregistrement en double",
    "merge.loadPreview": "Charger l'aperçu",
    "merge.merging": "Fusion en cours…",
    "merge.merge": "Fusionner",
    "merge.bothIdsRequired": "Les deux identifiants sont requis.",
    "merge.mustDiffer":
      "L'identifiant principal et celui du doublon doivent différer.",
    "merge.preview": "Aperçu",
    "merge.main": "Principale",
    "merge.duplicate": "Doublon",
    "merge.completed": "Fusion terminée",
    "merge.completedDetail": "{dup} a été fusionnée dans {main}.",
    "merge.viewMain": "Voir l'organisation principale",
    "merge.recent": "Fusions récentes",
    "merge.recentEmpty": "Aucune fusion enregistrée pour le moment.",
    "merge.recentRefresh": "Actualiser",
    "merge.colMergedAt": "Fusionné le",
    "merge.colReason": "Motif",
    "merge.colActor": "Auteur",
    "merge.confirm":
      "Fusionner {dup} dans {main} ? Le doublon sera supprimé logiquement.",
    "review.intro":
      "La file des doublons potentiels stockée à partir des analyses par lot. Examinez chaque paire et confirmez-la ou rejetez-la.",
    "review.filter.status": "Statut",
    "review.filter.statusAll": "Tous les statuts",
    "review.filter.limit": "Taille de page",
    "review.filter.limitHint":
      "Il n'y a pas de décalage de page — seuls les éléments correspondants les plus récents jusqu'à cette limite sont affichés.",
    "review.board.title": "Tableau",
    "review.list.title": "File",
    "review.empty": "La file de révision est vide.",
    "review.loading": "Chargement…",
    "review.col.pair": "Paire",
    "review.col.score": "Score",
    "review.col.quality": "Qualité",
    "review.col.provenance": "Origine",
    "review.col.status": "Statut",
    "review.col.actions": "Actions",
    "review.compare.open": "Comparer",
    "review.compare.title": "Comparaison",
    "review.compare.close": "Fermer",
    "review.compare.loading": "Chargement des deux enregistrements…",
    "review.compare.partial":
      "Un enregistrement n'a pas pu être chargé (il a peut-être été supprimé) ; affichage de ce qui est disponible.",
    "review.compare.field": "Champ",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "Non renseigné",
    "review.field.score": "Score :",
    "review.field.quality": "Qualité :",
    "review.field.method": "Méthode de détection :",
    "review.field.provenance": "Origine :",
    "review.field.status": "Statut :",
    "review.field.legalName": "Nom légal",
    "review.breakdown.title": "Détail du score",
    "review.breakdown.loading": "Calcul du score de la paire…",
    "review.breakdown.none": "Aucun détail de score disponible.",
    "review.breakdown.component": "Composant",
    "review.breakdown.weight": "Poids",
    "review.breakdown.score": "Score",
    "review.decide.confirm": "Confirmer le doublon",
    "review.decide.reject": "Rejeter",
    "review.decide.deciding": "Enregistrement…",
    "review.decide.locked":
      "Cet élément a déjà été décidé et ne peut pas être modifié ici.",
    "review.merge.title": "Fusionner",
    "review.merge.note":
      "Confirmer ne fusionne pas les enregistrements — choisissez celui qui survit.",
    "review.merge.keepA": "Fusionner, conserver A",
    "review.merge.keepB": "Fusionner, conserver B",
    "review.status.pending": "En attente",
    "review.status.confirmed": "Confirmé",
    "review.status.rejected": "Rejeté",
    "review.status.automerged": "Fusionné automatiquement",
    "review.provenance.operator": "Opérateur",
    "review.provenance.import": "Importation",
    "review.provenance.matcherSuggested": "Suggéré par le comparateur",
    "review.component.name": "Nom",
    "review.component.address": "Adresse",
    "review.component.url": "URL",
    "review.component.jurisdiction": "Juridiction",
    "review.component.foundingDate": "Date de fondation",
    "review.component.keywords": "Mots-clés",
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
    "splash.hero.title": "Une fiche fiable pour chaque organisation",
    "splash.hero.subtitle":
      "Enregistrez entreprises, associations et administrations une seule fois, repérez les doublons tôt et fusionnez-les en toute sécurité, avec confidentialité et piste d'audit intégrées.",
    "splash.benefits.1.title": "Moins de doublons",
    "splash.benefits.1.body":
      "Les vérifications de doublons signalent une répétition probable avant la création d'une seconde fiche.",
    "splash.benefits.2.title": "Identifiants durables",
    "splash.benefits.2.body":
      "Conservez le LEI, le DUNS et d'autres identifiants pour chaque organisation.",
    "splash.benefits.3.title": "Des décisions documentées",
    "splash.benefits.3.body":
      "Consultez le détail du score de chaque paire candidate avant de la confirmer ou de la rejeter.",
    "splash.benefits.4.title": "Fusion sûre",
    "splash.benefits.4.body":
      "La fusion est une étape délibérée qui conserve l'historique de ce qui a été regroupé.",
    "splash.benefits.5.title": "Confidentialité au partage",
    "splash.benefits.5.body":
      "Affichez une fiche masquée ou exportez des données sans en révéler plus que nécessaire.",
    "splash.benefits.6.title": "Modifications traçables",
    "splash.benefits.6.body":
      "Chaque modification est consignée, vous savez donc ce qui s'est passé et quand.",
    "splash.features.1.title": "Parcourir les organisations",
    "splash.features.1.body":
      "Parcourez le registre sous forme de liste ou filtrez une grille de données complète.",
    "splash.features.2.title": "Créer et modifier",
    "splash.features.2.body":
      "Saisissez noms, juridiction, date de création, mots-clés et identifiants.",
    "splash.features.3.title": "Vérifier les doublons",
    "splash.features.3.body":
      "Trouvez les organisations enregistrées qui ressemblent à celle-ci, avec leurs scores.",
    "splash.features.4.title": "Tableau de révision",
    "splash.features.4.body":
      "Faites glisser les paires candidates pour confirmer ou rejeter, ou comparez-les côte à côte.",
    "splash.features.5.title": "Fusionner les fiches",
    "splash.features.5.body":
      "Regroupez un doublon confirmé dans la fiche conservée, avec un historique des fusions.",
    "splash.features.6.title": "Vue masquée et export",
    "splash.features.6.body":
      "Passez à la vue masquée, téléchargez un export RGPD ou ouvrez la piste d'audit.",
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
      "Une visite guidée du registre des organisations : le rôle de chaque écran et les étapes pour l'utiliser, de l'enregistrement d'une organisation à la fusion des doublons.",
    "tour.s1.title": "Enregistrer une organisation",
    "tour.s1.summary":
      "Créez une fiche avec noms, identifiants, adresse et mots-clés. Le nom est obligatoire, et des indications signalent les identifiants mal formés avant l'enregistrement.",
    "tour.s1.step.1":
      "Choisissez Nouvelle organisation dans le menu, ou En créer une sur une liste vide, pour ouvrir le formulaire.",
    "tour.s1.step.2":
      "Saisissez le Nom (obligatoire), puis si vous le souhaitez la dénomination légale, l'URL du site, la juridiction, la date de fondation, les autres noms, les mots-clés et les URL « même que ».",
    "tour.s1.step.3":
      "Renseignez l'adresse et utilisez + Ajouter un identifiant pour saisir un LEI, un DUNS ou un autre identifiant avec sa valeur ; une indication apparaît si le format semble incorrect.",
    "tour.s1.step.4":
      "Sélectionnez Créer. Si le nom manque ou si le service refuse une valeur, le formulaire affiche l'erreur et rien n'est enregistré.",
    "tour.s2.title": "Parcourir et filtrer le registre",
    "tour.s2.summary":
      "Consultez toutes les organisations du registre sous forme de liste simple ou de grille de données filtrable, et passez de l'une ou l'autre à une fiche.",
    "tour.s2.step.1":
      "Connectez-vous et ouvrez Organisations depuis le menu : la page d'accueil liste chaque organisation par nom, avec un bouton Nouvelle organisation.",
    "tour.s2.step.2":
      "Ouvrez la vue en grille avec le lien ci-dessous : une grille de données affiche le Nom et le pid de chaque organisation, ainsi que le nombre de lignes chargées sur le total.",
    "tour.s2.step.3":
      "Utilisez la barre de filtre au-dessus de la grille pour restreindre les lignes par nom ; le filtre porte sur les lignes déjà chargées.",
    "tour.s2.step.4":
      "Sélectionnez une ligne de la grille, ou un nom dans la liste d'accueil, pour ouvrir la fiche de cette organisation.",
    "tour.s3.title": "Ouvrir une fiche et vérifier les doublons",
    "tour.s3.summary":
      "La page d'une fiche affiche ses détails et les actions possibles : vérifier les doublons, modifier et supprimer.",
    "tour.s3.step.1":
      "Ouvrez une organisation pour voir sa dénomination légale, son URL, sa juridiction, sa date de fondation, ses identifiants, ses mots-clés et son ID.",
    "tour.s3.step.2":
      "Sélectionnez Vérifier les doublons pour la comparer aux organisations enregistrées ; les correspondances probables sont listées sous Doublons potentiels, ou la page indique qu'aucune ne dépasse le seuil de correspondance.",
    "tour.s3.step.3":
      "Sélectionnez un nom dans les résultats pour ouvrir cette autre fiche et comparer les détails.",
    "tour.s3.step.4":
      "Pour modifier une fiche, sélectionnez Modifier, ajustez le formulaire et choisissez Enregistrer les modifications ; Supprimer effectue une suppression logique et vous ramène à la liste.",
    "tour.s4.title": "Confidentialité, export et audit",
    "tour.s4.summary":
      "Consultez une vue masquée, téléchargez un export RGPD ou lisez le journal d'audit, le tout depuis la page d'une fiche.",
    "tour.s4.step.1":
      "Sur la page d'une fiche, sélectionnez Afficher masqué pour la recharger via la vue masquée ; un avis indique que certains champs sont occultés.",
    "tour.s4.step.2":
      "Sélectionnez Afficher complet pour revenir à la fiche complète.",
    "tour.s4.step.3":
      "Sélectionnez Exporter les données (RGPD) pour télécharger les données de la fiche au format JSON.",
    "tour.s4.step.4":
      "Sélectionnez Afficher le journal d'audit pour charger l'historique des modifications, et Masquer le journal d'audit pour le fermer ; une fiche sans historique indique Aucune entrée d'audit.",
    "tour.s5.title": "Examiner les doublons candidats",
    "tour.s5.summary":
      "La page Révision contient la file des paires probablement en double trouvées par les analyses, où vous confirmez ou rejetez chacune.",
    "tour.s5.step.1":
      "Ouvrez Révision et sélectionnez Lancer l'analyse pour contrôler le registre, puis filtrez la file par Statut et Taille de page.",
    "tour.s5.step.2":
      "Les paires candidates apparaissent sur un Tableau et dans une table File affichant la paire, le score, la qualité, la source et le statut.",
    "tour.s5.step.3":
      "Sélectionnez Comparer (ou une carte) pour voir les deux fiches côte à côte, avec le Détail du score par composant.",
    "tour.s5.step.4":
      "Choisissez Confirmer le doublon ou Rejeter, ou faites glisser une carte en attente vers la colonne confirmé ou rejeté ; un élément tranché est verrouillé.",
    "tour.s6.title": "Fusionner deux fiches",
    "tour.s6.summary":
      "La fusion intègre un doublon confirmé dans la fiche que vous conservez, et l'historique des fusions garde trace de ce qui a été combiné.",
    "tour.s6.step.1":
      "Depuis un élément confirmé dans Révision, choisissez Fusionner, conserver A ou Fusionner, conserver B pour ouvrir Fusionner avec les deux ID renseignés, ou ouvrez Fusionner et saisissez les ID vous-même.",
    "tour.s6.step.2":
      "Vérifiez l'ID de l'organisation principale (la fiche qui subsiste) et l'ID de l'organisation en double ; ils doivent différer. Ajoutez éventuellement un Motif.",
    "tour.s6.step.3":
      "Sélectionnez Charger l'aperçu pour comparer Principale et Doublon, puis sélectionnez Fusionner et confirmez l'invite, car le doublon est supprimé logiquement.",
    "tour.s6.step.4":
      "La page affiche Fusion terminée avec un lien vers l'organisation principale, et Fusions récentes indique quand, pourquoi et par qui.",
    "signin.sso": "Se connecter avec SSO",
  },
  "hi-001": {
    "nav.review": "समीक्षा",
    "review.run": "स्कैन चलाएं",
    "brand.name": "Main X · संगठन",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "nav.organizations": "संगठन",
    "nav.newOrganization": "नया संगठन",
    "chrome.language": "भाषा",
    "nav.share": "साझा करें",
    "nav.text_size": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "chrome.theme": "थीम",
    "session.title": "सत्र",
    "session.signedIn": "साइन इन (टोकन संलग्न)",
    "session.signOut": "साइन आउट",
    "session.signIn": "साइन इन",
    "session.pasteToken": "टोकन चिपकाएँ",
    "session.accessToken": "एक्सेस टोकन",
    "session.pastePlaceholder": "बियरर टोकन चिपकाएँ",
    "session.useToken": "टोकन उपयोग करें",
    "list.title": "संगठन",
    "list.new": "नया संगठन",
    "list.loading": "लोड हो रहा है…",
    "list.empty": "अभी तक कोई संगठन नहीं।",
    "list.createOne": "एक बनाएँ",
    "detail.organization": "संगठन",
    "detail.loading": "लोड हो रहा है…",
    "detail.notFound": "नहीं मिला",
    "detail.legalName": "कानूनी नाम:",
    "detail.url": "URL:",
    "detail.jurisdiction": "क्षेत्राधिकार:",
    "detail.founded": "स्थापित:",
    "detail.identifiers": "पहचानकर्ता:",
    "detail.keywords": "कीवर्ड:",
    "detail.id": "आईडी:",
    "detail.edit": "संपादित करें",
    "detail.checkDuplicates": "डुप्लिकेट जाँचें",
    "detail.checking": "जाँच हो रही है…",
    "detail.delete": "हटाएँ",
    "detail.showMasked": "मास्क्ड दिखाएँ",
    "detail.showFull": "पूर्ण दिखाएँ",
    "detail.maskedNotice":
      "मास्क्ड दृश्य दिखाया जा रहा है — कुछ फ़ील्ड छिपे हुए हैं।",
    "detail.exportGdpr": "डेटा निर्यात करें (GDPR)",
    "detail.exportingGdpr": "निर्यात हो रहा है…",
    "detail.showAudit": "ऑडिट ट्रेल दिखाएँ",
    "detail.hideAudit": "ऑडिट ट्रेल छिपाएँ",
    "detail.auditTrail": "ऑडिट ट्रेल",
    "detail.loadingAudit": "ऑडिट ट्रेल लोड हो रहा है…",
    "detail.noAuditEntries": "कोई ऑडिट प्रविष्टियाँ नहीं।",
    "detail.auditLoadFailed": "ऑडिट लोड विफल",
    "detail.checkFailed": "जाँच विफल",
    "detail.potentialDuplicates": "संभावित डुप्लिकेट",
    "detail.noneAboveThreshold": "मिलान सीमा से ऊपर कोई नहीं।",
    "new.title": "नया संगठन",
    "new.create": "बनाएँ",
    "edit.title": "संगठन संपादित करें",
    "edit.organizationFallback": "संगठन",
    "edit.loading": "लोड हो रहा है…",
    "edit.notFound": "नहीं मिला",
    "edit.saveChanges": "परिवर्तन सहेजें",
    "form.save": "सहेजें",
    "form.saving": "सहेजा जा रहा है…",
    "form.nameRequired": "नाम आवश्यक है।",
    "form.saveFailed": "सहेजना विफल",
    "form.name": "नाम",
    "form.legalName": "कानूनी नाम",
    "form.url": "वेबसाइट URL",
    "form.jurisdiction": "क्षेत्राधिकार (ISO 3166)",
    "form.foundingDate": "स्थापना तिथि",
    "form.alternateNames": "वैकल्पिक नाम",
    "form.commaSeparated": "(अल्पविराम से अलग)",
    "form.keywords": "कीवर्ड",
    "form.sameAs": "समान-रूप URL",
    "form.address": "पता",
    "form.street": "गली",
    "form.locality": "इलाका",
    "form.region": "क्षेत्र",
    "form.postalCode": "पिन कोड",
    "form.country": "देश",
    "form.identifiers": "पहचानकर्ता",
    "form.value": "मान",
    "form.remove": "हटाएँ",
    "form.addIdentifier": "+ पहचानकर्ता जोड़ें",
    "nav.merge": "विलय",
    "merge.title": "संगठनों का विलय",
    "merge.mainId": "मुख्य संगठन आईडी",
    "merge.mainIdHint": "वह रिकॉर्ड जो विलय के बाद बना रहता है।",
    "merge.dupId": "डुप्लिकेट संगठन आईडी",
    "merge.dupIdHint": "वह रिकॉर्ड जो समाहित कर सॉफ़्ट-डिलीट किया जाएगा।",
    "merge.reason": "कारण",
    "merge.reasonHint": "वैकल्पिक; विलय इतिहास में सहेजा जाता है।",
    "merge.reasonPlaceholder": "वही कंपनी, दोहरा पंजीकरण",
    "merge.loadPreview": "पूर्वावलोकन लोड करें",
    "merge.merging": "विलय हो रहा है…",
    "merge.merge": "विलय करें",
    "merge.bothIdsRequired": "दोनों आईडी आवश्यक हैं।",
    "merge.mustDiffer": "मुख्य और डुप्लिकेट आईडी अलग होनी चाहिए।",
    "merge.preview": "पूर्वावलोकन",
    "merge.main": "मुख्य",
    "merge.duplicate": "डुप्लिकेट",
    "merge.completed": "विलय पूर्ण हुआ",
    "merge.completedDetail": "{dup} को {main} में मिला दिया गया।",
    "merge.viewMain": "मुख्य संगठन देखें",
    "merge.recent": "हाल के विलय",
    "merge.recentEmpty": "अभी तक कोई विलय दर्ज नहीं।",
    "merge.recentRefresh": "ताज़ा करें",
    "merge.colMergedAt": "विलय का समय",
    "merge.colReason": "कारण",
    "merge.colActor": "कर्ता",
    "merge.confirm":
      "क्या {dup} को {main} में मिलाएँ? डुप्लिकेट सॉफ़्ट-डिलीट कर दिया जाएगा।",
    "review.intro":
      "बैच स्कैन से संग्रहीत संभावित डुप्लिकेट कतार। प्रत्येक जोड़ी की समीक्षा करें और उसे स्वीकृत या अस्वीकृत करें।",
    "review.filter.status": "स्थिति",
    "review.filter.statusAll": "सभी स्थितियाँ",
    "review.filter.limit": "पृष्ठ आकार",
    "review.filter.limitHint":
      "कोई पृष्ठ ऑफ़सेट नहीं है — इस सीमा तक केवल नवीनतम मेल खाने वाली प्रविष्टियाँ दिखाई जाती हैं।",
    "review.board.title": "बोर्ड",
    "review.list.title": "कतार",
    "review.empty": "समीक्षा कतार खाली है।",
    "review.loading": "लोड हो रहा है…",
    "review.col.pair": "जोड़ी",
    "review.col.score": "स्कोर",
    "review.col.quality": "गुणवत्ता",
    "review.col.provenance": "स्रोत",
    "review.col.status": "स्थिति",
    "review.col.actions": "क्रियाएँ",
    "review.compare.open": "तुलना करें",
    "review.compare.title": "तुलना",
    "review.compare.close": "बंद करें",
    "review.compare.loading": "दोनों रिकॉर्ड लोड हो रहे हैं…",
    "review.compare.partial":
      "एक रिकॉर्ड लोड नहीं हो सका (हो सकता है वह हटा दिया गया हो); जो उपलब्ध है वह दिखाया जा रहा है।",
    "review.compare.field": "फ़ील्ड",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "दर्ज नहीं",
    "review.field.score": "स्कोर:",
    "review.field.quality": "गुणवत्ता:",
    "review.field.method": "पहचान विधि:",
    "review.field.provenance": "स्रोत:",
    "review.field.status": "स्थिति:",
    "review.field.legalName": "कानूनी नाम",
    "review.breakdown.title": "स्कोर विवरण",
    "review.breakdown.loading": "जोड़ी का स्कोर निकाला जा रहा है…",
    "review.breakdown.none": "कोई स्कोर विवरण उपलब्ध नहीं है।",
    "review.breakdown.component": "घटक",
    "review.breakdown.weight": "भार",
    "review.breakdown.score": "स्कोर",
    "review.decide.confirm": "डुप्लिकेट स्वीकृत करें",
    "review.decide.reject": "अस्वीकृत करें",
    "review.decide.deciding": "सहेजा जा रहा है…",
    "review.decide.locked":
      "इस प्रविष्टि पर पहले ही निर्णय लिया जा चुका है और इसे यहाँ बदला नहीं जा सकता।",
    "review.merge.title": "विलय",
    "review.merge.note":
      "स्वीकृत करने से रिकॉर्ड का विलय नहीं होता — चुनें कि कौन-सा बना रहेगा।",
    "review.merge.keepA": "विलय करें, A रखें",
    "review.merge.keepB": "विलय करें, B रखें",
    "review.status.pending": "लंबित",
    "review.status.confirmed": "स्वीकृत",
    "review.status.rejected": "अस्वीकृत",
    "review.status.automerged": "स्वतः विलय",
    "review.provenance.operator": "संचालक",
    "review.provenance.import": "आयात",
    "review.provenance.matcherSuggested": "मिलानकर्ता द्वारा सुझाया गया",
    "review.component.name": "नाम",
    "review.component.address": "पता",
    "review.component.url": "URL",
    "review.component.jurisdiction": "क्षेत्राधिकार",
    "review.component.foundingDate": "स्थापना तिथि",
    "review.component.keywords": "कीवर्ड",
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
    "splash.hero.title": "हर संगठन के लिए एक भरोसेमंद रिकॉर्ड",
    "splash.hero.subtitle":
      "कंपनियों, चैरिटी और एजेंसियों को एक बार दर्ज करें, डुप्लिकेट जल्दी पकड़ें और उन्हें सुरक्षित रूप से मर्ज करें, गोपनीयता और पूरे ऑडिट ट्रेल के साथ।",
    "splash.benefits.1.title": "कम डुप्लिकेट",
    "splash.benefits.1.body":
      "डुप्लिकेट जाँच दूसरा रिकॉर्ड बनने से पहले ही संभावित दोहराव को चिह्नित करती है।",
    "splash.benefits.2.title": "टिकाऊ पहचानकर्ता",
    "splash.benefits.2.body":
      "हर संगठन के साथ LEI, DUNS और अन्य पहचानकर्ता सहेजें।",
    "splash.benefits.3.title": "प्रमाण के साथ निर्णय",
    "splash.benefits.3.body":
      "हर उम्मीदवार जोड़ी को पुष्ट या अस्वीकार करने से पहले उसका स्कोर विवरण देखें।",
    "splash.benefits.4.title": "सुरक्षित मर्ज",
    "splash.benefits.4.body":
      "मर्ज एक सोच-समझकर उठाया गया कदम है जो मिलाई गई चीज़ों का इतिहास रखता है।",
    "splash.benefits.5.title": "साझा करते समय गोपनीयता",
    "splash.benefits.5.body":
      "मास्क किया गया रिकॉर्ड देखें या डेटा निर्यात करें, बिना ज़रूरत से ज़्यादा उजागर किए।",
    "splash.benefits.6.title": "जवाबदेह बदलाव",
    "splash.benefits.6.body":
      "हर बदलाव दर्ज रहता है, इसलिए आप देख सकते हैं कि क्या हुआ और कब।",
    "splash.features.1.title": "संगठन ब्राउज़ करें",
    "splash.features.1.body":
      "रजिस्ट्री को सूची के रूप में देखें, या पूरे डेटा ग्रिड को फ़िल्टर करें।",
    "splash.features.2.title": "बनाएँ और संपादित करें",
    "splash.features.2.body":
      "नाम, अधिकार क्षेत्र, स्थापना तिथि, कीवर्ड और पहचानकर्ता दर्ज करें।",
    "splash.features.3.title": "डुप्लिकेट जाँचें",
    "splash.features.3.body":
      "इससे मिलते-जुलते संग्रहीत संगठन स्कोर के साथ खोजें।",
    "splash.features.4.title": "समीक्षा बोर्ड",
    "splash.features.4.body":
      "उम्मीदवार जोड़ियों को पुष्ट या अस्वीकार करने के लिए खींचें, या उनकी साथ-साथ तुलना करें।",
    "splash.features.5.title": "रिकॉर्ड मर्ज करें",
    "splash.features.5.body":
      "पुष्ट डुप्लिकेट को अपने रखे रिकॉर्ड में मिलाएँ, मर्ज इतिहास के साथ।",
    "splash.features.6.title": "मास्क्ड व्यू और निर्यात",
    "splash.features.6.body":
      "मास्क्ड व्यू पर जाएँ, GDPR निर्यात डाउनलोड करें, या ऑडिट ट्रेल खोलें।",
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
      "संगठन रजिस्ट्री का निर्देशित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, किसी संगठन को दर्ज करने से लेकर डुप्लिकेट विलय करने तक।",
    "tour.s1.title": "संगठन दर्ज करना",
    "tour.s1.summary":
      "नाम, पहचानकर्ता, पता और कीवर्ड के साथ रिकॉर्ड बनाएँ। नाम अनिवार्य है, और पहचानकर्ता के संकेत सहेजने से पहले गलत मान बता देते हैं।",
    "tour.s1.step.1":
      "फ़ॉर्म खोलने के लिए मेनू में नया संगठन चुनें, या खाली सूची में एक बनाएँ चुनें।",
    "tour.s1.step.2":
      "नाम (अनिवार्य) दर्ज करें, फिर चाहें तो कानूनी नाम, वेबसाइट URL, अधिकार क्षेत्र, स्थापना तिथि, वैकल्पिक नाम, कीवर्ड और समकक्ष URL भरें।",
    "tour.s1.step.3":
      "पता भरें और LEI, DUNS या कोई अन्य पहचानकर्ता उसके मान के साथ दर्ज करने के लिए + पहचानकर्ता जोड़ें का उपयोग करें; प्रारूप गलत लगे तो संकेत दिखता है।",
    "tour.s1.step.4":
      "बनाएँ चुनें। यदि नाम छूटा है या सेवा किसी मान को अस्वीकार करती है, तो फ़ॉर्म त्रुटि दिखाता है और कुछ सहेजा नहीं जाता।",
    "tour.s2.title": "रजिस्ट्री ब्राउज़ करना और फ़िल्टर करना",
    "tour.s2.summary":
      "रजिस्ट्री के हर संगठन को सरल सूची या फ़िल्टर योग्य डेटा ग्रिड में देखें, और दोनों में से किसी से भी रिकॉर्ड पर जाएँ।",
    "tour.s2.step.1":
      "साइन इन करें और मेनू से संगठन खोलें: होम पेज हर संगठन को उसके नाम से सूचीबद्ध करता है, साथ में नया संगठन बटन होता है।",
    "tour.s2.step.2":
      "नीचे दिए लिंक से ग्रिड दृश्य खोलें: डेटा ग्रिड हर संगठन का नाम और pid दिखाता है, साथ में कुल में से लोड की गई पंक्तियों की गिनती।",
    "tour.s2.step.3":
      "ग्रिड के ऊपर फ़िल्टर बार से पंक्तियों को नाम के आधार पर सीमित करें; फ़िल्टर पहले से लोड पंक्तियों पर काम करता है।",
    "tour.s2.step.4":
      "उस संगठन का रिकॉर्ड खोलने के लिए ग्रिड में कोई पंक्ति, या होम सूची में कोई नाम चुनें।",
    "tour.s3.title": "रिकॉर्ड खोलना और डुप्लिकेट जाँचना",
    "tour.s3.summary":
      "रिकॉर्ड का पेज उसका विवरण और उस पर उपलब्ध क्रियाएँ दिखाता है: डुप्लिकेट जाँचें, संपादित करें और हटाएँ।",
    "tour.s3.step.1":
      "किसी संगठन को खोलकर उसका कानूनी नाम, URL, अधिकार क्षेत्र, स्थापना तिथि, पहचानकर्ता, कीवर्ड और ID देखें।",
    "tour.s3.step.2":
      "संग्रहीत संगठनों से तुलना करने के लिए डुप्लिकेट जाँचें चुनें; संभावित मिलान संभावित डुप्लिकेट के अंतर्गत सूचीबद्ध होते हैं, या पेज बताता है कि कोई भी मिलान सीमा से ऊपर नहीं है।",
    "tour.s3.step.3":
      "परिणामों में कोई नाम चुनकर वह दूसरा रिकॉर्ड खोलें और विवरण की तुलना करें।",
    "tour.s3.step.4":
      "रिकॉर्ड बदलने के लिए संपादित करें चुनें, फ़ॉर्म में बदलाव करें और परिवर्तन सहेजें चुनें; हटाएँ रिकॉर्ड को सॉफ़्ट-डिलीट करता है और आपको सूची पर लौटाता है।",
    "tour.s4.title": "गोपनीयता, निर्यात और ऑडिट",
    "tour.s4.summary":
      "रिकॉर्ड के पेज से ही मास्क्ड दृश्य देखें, GDPR निर्यात डाउनलोड करें या ऑडिट ट्रेल पढ़ें।",
    "tour.s4.step.1":
      "रिकॉर्ड के पेज पर, उसे मास्क्ड दृश्य से दोबारा लोड करने के लिए मास्क्ड दिखाएँ चुनें; एक सूचना बताती है कि कुछ फ़ील्ड छिपाए गए हैं।",
    "tour.s4.step.2": "पूरे रिकॉर्ड पर लौटने के लिए पूरा दिखाएँ चुनें।",
    "tour.s4.step.3":
      "रिकॉर्ड का डेटा JSON फ़ाइल के रूप में डाउनलोड करने के लिए डेटा निर्यात करें (GDPR) चुनें।",
    "tour.s4.step.4":
      "परिवर्तनों का इतिहास लोड करने के लिए ऑडिट ट्रेल दिखाएँ और उसे बंद करने के लिए ऑडिट ट्रेल छिपाएँ चुनें; जिस रिकॉर्ड का इतिहास नहीं है वह कोई ऑडिट प्रविष्टियाँ नहीं। दिखाता।",
    "tour.s5.title": "डुप्लिकेट उम्मीदवारों की समीक्षा",
    "tour.s5.summary":
      "समीक्षा पेज में स्कैन से मिले संभावित डुप्लिकेट जोड़ों की कतार होती है, जहाँ आप हर एक को स्वीकृत या अस्वीकृत करते हैं।",
    "tour.s5.step.1":
      "समीक्षा खोलें और रजिस्ट्री जाँचने के लिए स्कैन चलाएं चुनें, फिर कतार को स्थिति और पेज आकार से फ़िल्टर करें।",
    "tour.s5.step.2":
      "उम्मीदवार जोड़े बोर्ड पर और कतार तालिका में दिखते हैं, जो जोड़ा, स्कोर, गुणवत्ता, स्रोत और स्थिति दिखाती है।",
    "tour.s5.step.3":
      "दोनों रिकॉर्ड आमने-सामने और घटक के अनुसार स्कोर विवरण देखने के लिए तुलना करें (या कोई कार्ड) चुनें।",
    "tour.s5.step.4":
      "डुप्लिकेट स्वीकृत करें या अस्वीकृत करें चुनें, या लंबित कार्ड को स्वीकृत या अस्वीकृत कॉलम में खींचें; तय हो चुका आइटम लॉक हो जाता है।",
    "tour.s6.title": "दो रिकॉर्ड का विलय",
    "tour.s6.summary":
      "विलय पुष्ट किए गए डुप्लिकेट को आपके रखे रिकॉर्ड में मिला देता है, और विलय इतिहास दर्ज करता है कि क्या जोड़ा गया।",
    "tour.s6.step.1":
      "समीक्षा में किसी स्वीकृत आइटम से विलय करें, A रखें या विलय करें, B रखें चुनें, जिससे दोनों ID भरे हुए विलय पेज खुलता है; या विलय खोलकर ID स्वयं टाइप करें।",
    "tour.s6.step.2":
      "मुख्य संगठन ID (जो रिकॉर्ड बचेगा) और डुप्लिकेट संगठन ID जाँचें; उनका अलग होना ज़रूरी है। चाहें तो कारण जोड़ें।",
    "tour.s6.step.3":
      "मुख्य और डुप्लिकेट की तुलना के लिए पूर्वावलोकन लोड करें चुनें, फिर विलय करें चुनकर संकेत की पुष्टि करें, क्योंकि डुप्लिकेट सॉफ़्ट-डिलीट हो जाता है।",
    "tour.s6.step.4":
      "पेज विलय पूर्ण हुआ के साथ मुख्य संगठन का लिंक दिखाता है, और हाल के विलय बताते हैं कि कब, क्यों और किसने।",
    "signin.sso": "SSO से साइन इन करें",
  },
  "zh-cn": {
    "nav.review": "审核",
    "review.run": "运行扫描",
    "brand.name": "Main X · 组织",
    "nav.toggle": "切换导航",
    "nav.organizations": "组织",
    "nav.newOrganization": "新建组织",
    "chrome.language": "语言",
    "nav.share": "分享",
    "nav.text_size": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "chrome.theme": "主题",
    "session.title": "会话",
    "session.signedIn": "已登录（已附加令牌）",
    "session.signOut": "退出登录",
    "session.signIn": "登录",
    "session.pasteToken": "粘贴令牌",
    "session.accessToken": "访问令牌",
    "session.pastePlaceholder": "粘贴持有者令牌",
    "session.useToken": "使用令牌",
    "list.title": "组织",
    "list.new": "新建组织",
    "list.loading": "加载中…",
    "list.empty": "暂无组织。",
    "list.createOne": "创建一个",
    "detail.organization": "组织",
    "detail.loading": "加载中…",
    "detail.notFound": "未找到",
    "detail.legalName": "法定名称：",
    "detail.url": "网址：",
    "detail.jurisdiction": "管辖区：",
    "detail.founded": "成立于：",
    "detail.identifiers": "标识符：",
    "detail.keywords": "关键词：",
    "detail.id": "ID：",
    "detail.edit": "编辑",
    "detail.checkDuplicates": "检查重复项",
    "detail.checking": "检查中…",
    "detail.delete": "删除",
    "detail.showMasked": "显示脱敏视图",
    "detail.showFull": "显示完整视图",
    "detail.maskedNotice": "正在显示脱敏视图——部分字段已隐藏。",
    "detail.exportGdpr": "导出数据（GDPR）",
    "detail.exportingGdpr": "导出中…",
    "detail.showAudit": "显示审计跟踪",
    "detail.hideAudit": "隐藏审计跟踪",
    "detail.auditTrail": "审计跟踪",
    "detail.loadingAudit": "正在加载审计跟踪…",
    "detail.noAuditEntries": "没有审计记录。",
    "detail.auditLoadFailed": "审计加载失败",
    "detail.checkFailed": "检查失败",
    "detail.potentialDuplicates": "潜在重复项",
    "detail.noneAboveThreshold": "没有超过匹配阈值的项。",
    "new.title": "新建组织",
    "new.create": "创建",
    "edit.title": "编辑组织",
    "edit.organizationFallback": "组织",
    "edit.loading": "加载中…",
    "edit.notFound": "未找到",
    "edit.saveChanges": "保存更改",
    "form.save": "保存",
    "form.saving": "保存中…",
    "form.nameRequired": "名称为必填项。",
    "form.saveFailed": "保存失败",
    "form.name": "名称",
    "form.legalName": "法定名称",
    "form.url": "网站网址",
    "form.jurisdiction": "管辖区 (ISO 3166)",
    "form.foundingDate": "成立日期",
    "form.alternateNames": "别名",
    "form.commaSeparated": "（以逗号分隔）",
    "form.keywords": "关键词",
    "form.sameAs": "相同对象网址",
    "form.address": "地址",
    "form.street": "街道",
    "form.locality": "地区",
    "form.region": "区域",
    "form.postalCode": "邮政编码",
    "form.country": "国家/地区",
    "form.identifiers": "标识符",
    "form.value": "值",
    "form.remove": "移除",
    "form.addIdentifier": "+ 添加标识符",
    "nav.merge": "合并",
    "merge.title": "合并组织",
    "merge.mainId": "主组织 ID",
    "merge.mainIdHint": "合并后保留的记录。",
    "merge.dupId": "重复组织 ID",
    "merge.dupIdHint": "被并入并软删除的记录。",
    "merge.reason": "原因",
    "merge.reasonHint": "可选；保存在合并历史中。",
    "merge.reasonPlaceholder": "同一公司，重复登记",
    "merge.loadPreview": "加载预览",
    "merge.merging": "合并中…",
    "merge.merge": "合并",
    "merge.bothIdsRequired": "两个 ID 均为必填项。",
    "merge.mustDiffer": "主 ID 与重复 ID 必须不同。",
    "merge.preview": "预览",
    "merge.main": "主记录",
    "merge.duplicate": "重复记录",
    "merge.completed": "合并完成",
    "merge.completedDetail": "{dup} 已并入 {main}。",
    "merge.viewMain": "查看主组织",
    "merge.recent": "最近的合并",
    "merge.recentEmpty": "尚无合并记录。",
    "merge.recentRefresh": "刷新",
    "merge.colMergedAt": "合并时间",
    "merge.colReason": "原因",
    "merge.colActor": "操作者",
    "merge.confirm": "确定将 {dup} 并入 {main} 吗？重复记录将被软删除。",
    "review.intro":
      "来自批量扫描的已存储潜在重复项队列。请逐对审核并确认或拒绝。",
    "review.filter.status": "状态",
    "review.filter.statusAll": "所有状态",
    "review.filter.limit": "页面大小",
    "review.filter.limitHint": "没有页面偏移量——仅显示此限制内最新的匹配项。",
    "review.board.title": "看板",
    "review.list.title": "队列",
    "review.empty": "审核队列为空。",
    "review.loading": "加载中…",
    "review.col.pair": "配对",
    "review.col.score": "得分",
    "review.col.quality": "质量",
    "review.col.provenance": "来源",
    "review.col.status": "状态",
    "review.col.actions": "操作",
    "review.compare.open": "比较",
    "review.compare.title": "比较",
    "review.compare.close": "关闭",
    "review.compare.loading": "正在加载两条记录…",
    "review.compare.partial":
      "有一条记录无法加载（可能已被删除）；显示可用的部分。",
    "review.compare.field": "字段",
    "review.compare.a": "A",
    "review.compare.b": "B",
    "review.compare.none": "未记录",
    "review.field.score": "得分：",
    "review.field.quality": "质量：",
    "review.field.method": "检测方法：",
    "review.field.provenance": "来源：",
    "review.field.status": "状态：",
    "review.field.legalName": "法定名称",
    "review.breakdown.title": "得分明细",
    "review.breakdown.loading": "正在为该配对评分…",
    "review.breakdown.none": "没有可用的得分明细。",
    "review.breakdown.component": "组成部分",
    "review.breakdown.weight": "权重",
    "review.breakdown.score": "得分",
    "review.decide.confirm": "确认为重复项",
    "review.decide.reject": "拒绝",
    "review.decide.deciding": "正在记录…",
    "review.decide.locked": "该项目已被处理，无法在此更改。",
    "review.merge.title": "合并",
    "review.merge.note": "确认并不会合并记录——请选择保留哪一条。",
    "review.merge.keepA": "合并，保留 A",
    "review.merge.keepB": "合并，保留 B",
    "review.status.pending": "待处理",
    "review.status.confirmed": "已确认",
    "review.status.rejected": "已拒绝",
    "review.status.automerged": "自动合并",
    "review.provenance.operator": "操作员",
    "review.provenance.import": "导入",
    "review.provenance.matcherSuggested": "匹配器建议",
    "review.component.name": "名称",
    "review.component.address": "地址",
    "review.component.url": "网址",
    "review.component.jurisdiction": "管辖区",
    "review.component.foundingDate": "成立日期",
    "review.component.keywords": "关键词",
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
    "splash.hero.title": "每个组织，一份可信记录",
    "splash.hero.subtitle":
      "一次登记公司、慈善机构和政府机构，及早发现重复项并安全合并，内置隐私保护和完整审计记录。",
    "splash.benefits.1.title": "更少重复",
    "splash.benefits.1.body": "在创建第二条记录之前，查重就会标出疑似重复。",
    "splash.benefits.2.title": "可靠的标识符",
    "splash.benefits.2.body": "为每个组织保存 LEI、DUNS 及其他标识符。",
    "splash.benefits.3.title": "有据可依的决定",
    "splash.benefits.3.body": "确认或驳回前，先查看每个候选配对的得分明细。",
    "splash.benefits.4.title": "安全合并",
    "splash.benefits.4.body": "合并是经过确认的步骤，并会保留合并历史。",
    "splash.benefits.5.title": "分享时保护隐私",
    "splash.benefits.5.body":
      "查看已屏蔽的记录或导出数据，不会泄露超出所需的信息。",
    "splash.benefits.6.title": "可追溯的更改",
    "splash.benefits.6.body":
      "每次更改都有记录，可随时查看发生了什么、何时发生。",
    "splash.features.1.title": "浏览组织",
    "splash.features.1.body": "以列表方式浏览注册库，或筛选完整的数据表格。",
    "splash.features.2.title": "创建与编辑",
    "splash.features.2.body": "录入名称、司法辖区、成立日期、关键词和标识符。",
    "splash.features.3.title": "检查重复",
    "splash.features.3.body": "查找与该组织相似的已存组织及其得分。",
    "splash.features.4.title": "审核看板",
    "splash.features.4.body": "拖动候选配对以确认或驳回，或并排比较。",
    "splash.features.5.title": "合并记录",
    "splash.features.5.body":
      "将确认的重复项合并到保留的记录中，并附合并历史。",
    "splash.features.6.title": "屏蔽视图与导出",
    "splash.features.6.body":
      "切换到屏蔽视图、下载 GDPR 导出文件，或打开审计记录。",
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
      "组织登记库的图文导览：每个页面的作用和使用步骤，从登记一个组织到合并重复记录。",
    "tour.s1.title": "登记组织",
    "tour.s1.summary":
      "创建包含名称、标识符、地址和关键词的记录。名称必填，标识符提示会在保存前指出格式有误的值。",
    "tour.s1.step.1":
      "在菜单中选择“新建组织”，或在空列表中选择“创建一个”，打开表单。",
    "tour.s1.step.2":
      "填写“名称”（必填），然后可选填法定名称、网站 URL、司法管辖区、成立日期、别名、关键词和同一实体 URL。",
    "tour.s1.step.3":
      "填写地址，并用“+ 添加标识符”记录 LEI、DUNS 或其他标识符及其值；格式看起来有误时会出现提示。",
    "tour.s1.step.4":
      "选择“创建”。如果缺少名称或服务拒绝了某个值，表单会显示错误，且不会保存任何内容。",
    "tour.s2.title": "浏览和筛选登记库",
    "tour.s2.summary":
      "以简单列表或可筛选的数据网格查看登记库中的所有组织，并从任一视图跳转到记录。",
    "tour.s2.step.1":
      "登录后从菜单打开“组织”：首页按名称列出每个组织，并带有“新建组织”按钮。",
    "tour.s2.step.2":
      "通过下方链接打开网格视图：数据网格显示每个组织的名称和 pid，以及已加载行数与总数。",
    "tour.s2.step.3":
      "使用网格上方的筛选栏按名称缩小行范围；筛选作用于已加载的行。",
    "tour.s2.step.4":
      "选择网格中的一行，或首页列表中的名称，打开该组织的记录。",
    "tour.s3.title": "打开记录并检查重复项",
    "tour.s3.summary": "记录页面显示其详情和可用操作：检查重复项、编辑和删除。",
    "tour.s3.step.1":
      "打开一个组织，查看其法定名称、URL、司法管辖区、成立日期、标识符、关键词和 ID。",
    "tour.s3.step.2":
      "选择“检查重复项”，与已存储的组织比较；可能的匹配项列在“潜在重复项”下，或页面提示没有超过匹配阈值的项。",
    "tour.s3.step.3": "选择结果中的名称，打开另一条记录并比较详情。",
    "tour.s3.step.4":
      "要修改记录，选择“编辑”，调整表单并选择“保存更改”；“删除”会软删除该记录并返回列表。",
    "tour.s4.title": "隐私、导出与审计",
    "tour.s4.summary":
      "在记录页面即可查看脱敏视图、下载 GDPR 导出，或阅读审计跟踪。",
    "tour.s4.step.1":
      "在记录页面选择“显示脱敏视图”，通过脱敏视图重新加载；提示会说明部分字段已被隐去。",
    "tour.s4.step.2": "选择“显示完整视图”返回完整记录。",
    "tour.s4.step.3":
      "选择“导出数据（GDPR）”，将该记录的数据下载为 JSON 文件。",
    "tour.s4.step.4":
      "选择“显示审计跟踪”加载变更历史，选择“隐藏审计跟踪”关闭；没有历史的记录会显示“没有审计记录”。",
    "tour.s5.title": "审核重复候选项",
    "tour.s5.summary":
      "“审核”页面保存扫描发现的疑似重复配对队列，你可以逐个确认或拒绝。",
    "tour.s5.step.1":
      "打开“审核”，选择“运行扫描”检查登记库，然后按“状态”和“每页条数”筛选队列。",
    "tour.s5.step.2":
      "候选配对显示在“看板”和“队列”表中，表中列出配对、得分、质量、来源和状态。",
    "tour.s5.step.3":
      "选择“比较”（或某张卡片），并排查看两条记录以及按组成部分划分的“得分明细”。",
    "tour.s5.step.4":
      "选择“确认为重复项”或“拒绝”，或将待处理卡片拖到已确认或已拒绝列；已决定的条目会被锁定。",
    "tour.s6.title": "合并两条记录",
    "tour.s6.summary":
      "合并会把已确认的重复项并入你保留的记录，合并历史会记录合并了什么。",
    "tour.s6.step.1":
      "在“审核”中对已确认的条目选择“合并，保留 A”或“合并，保留 B”，即可打开已填好两个 ID 的“合并”页面；也可以打开“合并”自行输入 ID。",
    "tour.s6.step.2":
      "核对“主组织 ID”（保留的记录）和“重复组织 ID”；两者必须不同。可选填“原因”。",
    "tour.s6.step.3":
      "选择“加载预览”比较主记录和重复记录，然后选择“合并”并确认提示，因为重复记录会被软删除。",
    "tour.s6.step.4":
      "页面显示“合并完成”并附主组织链接，“最近的合并”则列出时间、原因和操作人。",
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
// Accepts a region subtag (es-MX → es) and is case-insensitive.
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
 * Reactive translation accessor for components: `t("list.title")`.
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
