// Lightweight, dependency-free i18n for the Thing front-end SPA. A
// per-locale strings map plus a reactive `$state` current-locale (Svelte 5
// runes), exposed via a `t(key)` accessor. Deliberately no i18n library:
// the surface is small and the front-end family is kept dependency-light.
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// German (`de-de`), English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. An unknown key/locale falls back to `en-001`. The
// chosen locale persists to localStorage under `mxi.thing.locale`.

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
 * Right-to-left locales. The layout mirrors these onto `<html dir="rtl">`.
 */
export const RTL_LOCALES = ["ar-001"] as const;

/**
 * Whether `locale` is written right-to-left (Arabic).
 *
 * @param locale - A locale code (region subtags are accepted; only the
 *   primary subtag is considered).
 * @returns `true` for RTL locales, `false` otherwise.
 */
export function isRtl(locale: string): boolean {
  const primary = locale.trim().split(/[-_]/)[0]?.toLowerCase() ?? "";
  return (RTL_LOCALES as readonly string[]).some(
    (l) => l.split("-")[0] === primary,
  );
}

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
 * localStorage key under which the chosen UI locale is persisted. Exported
 * so the layout's LocalePicker can share the same key — making the i18n
 * store the single source of truth for the chosen locale (no drift between
 * the store's persisted value and the picker's).
 */
export const LOCALE_KEY = "mxi.thing.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
//
// Exported (read-only) so tests can assert per-locale key coverage without
// going through the `en`-fallback path of `translate`.
export const STRINGS = {
  "ar-001": {
    "auth.sessionExpired":
      "انتهت صلاحية جلستك. جارٍ إعادة التوجيه لتسجيل الدخول…",
    "auth.accessDenied": "ليس لديك إذن للقيام بذلك.",
    "nav.review": "المراجعة",
    "review.run": "تشغيل الفحص",
    "review.intro":
      "أزواج مكررة محتملة من الفحص الدفعي. اسحب بطاقة معلّقة، أو افتح زوجًا لمقارنة كلا السجلين جنبًا إلى جنب قبل اتخاذ القرار.",
    "review.gap.provenance":
      "لا تسجل هذه الخدمة مصدرًا منفصلاً لكل زوج — فقط طريقة الاكتشاف.",
    "review.loading": "جارٍ تحميل قائمة انتظار المراجعة…",
    "review.empty": "لا توجد عناصر مراجعة لهذا الفلتر.",
    "review.filter.status": "الحالة",
    "review.filter.statusAll": "الكل",
    "review.filter.limit": "حجم الصفحة",
    "review.filter.limitHint":
      "تُعيد الخدمة 500 عنصر كحد أقصى ولا توفر ترقيمًا للصفحات بعد ذلك.",
    "review.status.pending": "قيد الانتظار",
    "review.status.confirmed": "مؤكَّد",
    "review.status.rejected": "مرفوض",
    "review.status.automerged": "مدمج تلقائيًا",
    "review.board.title": "اللوحة",
    "review.list.title": "القائمة",
    "review.col.pair": "الزوج",
    "review.col.score": "النتيجة",
    "review.col.quality": "الجودة",
    "review.col.method": "طريقة الاكتشاف",
    "review.col.status": "الحالة",
    "review.col.actions": "الإجراءات",
    "review.compare.open": "مقارنة",
    "review.compare.title": "مقارنة الزوج",
    "review.compare.close": "إغلاق",
    "review.compare.loading": "جارٍ تحميل كلا السجلين…",
    "review.compare.field": "الحقل",
    "review.compare.a": "السجل أ",
    "review.compare.b": "السجل ب",
    "review.compare.none": "غير مسجَّل",
    "review.compare.partial": "تعذّر تحميل أحد السجلين — ربما تم دمجه أو حذفه.",
    "review.field.score": "نتيجة المطابقة",
    "review.field.quality": "جودة المطابقة",
    "review.field.method": "طريقة الاكتشاف",
    "review.field.status": "الحالة",
    "review.breakdown.title": "تفصيل النتيجة",
    "review.breakdown.none": "لم يُسجَّل تفصيل للنتيجة لهذا الزوج.",
    "review.breakdown.component": "المكوّن",
    "review.breakdown.weight": "الوزن",
    "review.breakdown.score": "النتيجة",
    "review.decide.confirm": "تأكيد التكرار",
    "review.decide.reject": "رفض",
    "review.decide.deciding": "جارٍ الحفظ…",
    "review.decide.locked":
      "تم اتخاذ القرار بالفعل — يمكن اتخاذ القرار فقط للعناصر المعلّقة.",
    "review.merge.title": "دمج هذا الزوج",
    "review.merge.note":
      "التأكيد يسجل الحكم فقط؛ ولا يقوم بالدمج. اختر السجل الذي سيبقى.",
    "review.merge.keepA": "الاحتفاظ بـ أ، ودمج ب فيه",
    "review.merge.keepB": "الاحتفاظ بـ ب، ودمج أ فيه",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "لوحة المعلومات",
    "nav.things": "الأشياء",
    "nav.newThing": "شيء جديد",
    "nav.matchCheck": "فحص التطابق",
    "nav.merge": "دمج",
    "nav.toggle": "تبديل التنقل",
    "chrome.theme": "السمة",
    "chrome.language": "اللغة",
    "nav.share": "مشاركة",
    "nav.text_size": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "dashboard.title": "لوحة المعلومات",
    "dashboard.service": "الخدمة:",
    "dashboard.recentActivity": "النشاط الأخير",
    "dashboard.noRecent": "لا توجد إدخالات تدقيق أخيرة.",
    "things.title": "الأشياء",
    "things.new": "شيء جديد",
    "things.searchPlaceholder": "البحث بالاسم أو المعرّف…",
    "things.fuzzy": "غامض",
    "things.phonetic": "صوتي (Soundex)",
    "things.maskSensitive": "إخفاء الحساس",
    "things.previousPage": "السابق",
    "things.nextPage": "التالي",
    "things.pageRange": "{from}–{to} من {total}",
    "things.loading": "جارٍ التحميل…",
    "things.recordCount": "{count} سجل",
    "things.recordCountPlural": "{count} سجل",
    "search.action": "بحث",
    "grid.id": "المعرّف",
    "grid.name": "الاسم",
    "grid.type": "النوع (schema.org)",
    "grid.primaryId": "المعرّف الأساسي",
    "grid.url": "الرابط",
    "detail.loading": "جارٍ التحميل…",
    "detail.edit": "تحرير",
    "detail.audit": "تدقيق",
    "detail.delete": "حذف",
    "detail.exportGdpr": "تصدير البيانات (GDPR)",
    "detail.exportingGdpr": "جارٍ التصدير…",
    "detail.showMasked": "إظهار المُقنَّع",
    "detail.showFull": "إظهار الكامل",
    "detail.maskedNotice": "يتم عرض العرض المُقنَّع — بعض الحقول مخفية.",
    "detail.confirmDelete":
      "حذف هذا الشيء حذفًا مبدئيًا؟ لا يمكن التراجع عنه عبر الواجهة.",
    "detail.identity": "الهوية",
    "detail.id": "المعرّف",
    "detail.additionalType": "نوع إضافي",
    "detail.description": "الوصف",
    "detail.disambiguating": "إزالة الالتباس",
    "detail.url": "الرابط",
    "detail.owner": "المالك",
    "detail.mainEntityOfPage": "الكيان الرئيسي للصفحة",
    "detail.identifiers": "المعرفات",
    "detail.alternateNames": "أسماء بديلة",
    "detail.sameAs": "مطابق لـ (روابط موثوقة)",
    "detail.images": "الصور",
    "detail.customPrefix": "مخصص: ",
    "audit.title": "سجل التدقيق",
    "audit.backToThing": "العودة إلى الشيء",
    "audit.loading": "جارٍ التحميل…",
    "audit.none": "لا توجد إدخالات تدقيق.",
    "audit.by": "بواسطة",
    "audit.payload": "الحمولة",
    "edit.title": "تحرير الشيء",
    "edit.cancel": "إلغاء",
    "edit.loading": "جارٍ التحميل…",
    "edit.submitLabel": "حفظ التغييرات",
    "new.title": "شيء جديد",
    "new.submitLabel": "إنشاء",
    "new.possibleDuplicates": "نسخ مكررة محتملة",
    "new.duplicatesDetected":
      "تم اكتشاف نسخ مكررة ({count}) — راجع أدناه قبل إعادة الإرسال.",
    "match.title": "فحص التطابق",
    "match.name": "الاسم",
    "match.threshold": "العتبة",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "الوصف",
    "match.url": "الرابط",
    "match.sameAs": "روابط «مطابق لـ»",
    "match.sameAsHint": "واحد في كل سطر",
    "match.identifiers": "المعرفات",
    "match.matching": "جارٍ المطابقة…",
    "match.findMatches": "البحث عن تطابقات",
    "merge.title": "دمج الأشياء",
    "merge.mainId": "معرّف الشيء الرئيسي",
    "merge.mainIdHint": "السجل الباقي",
    "merge.dupId": "معرّف الشيء المكرر",
    "merge.dupIdHint": "سيُحذف حذفًا مبدئيًا",
    "merge.reason": "السبب",
    "merge.reasonHint": "مُسجَّل في سجل تدقيق الدمج",
    "merge.reasonPlaceholder": "نسخة مكررة مؤكدة",
    "merge.loadPreview": "تحميل المعاينة",
    "merge.merging": "جارٍ الدمج…",
    "merge.merge": "دمج",
    "merge.preview": "معاينة",
    "merge.main": "رئيسي",
    "merge.duplicate": "مكرر",
    "merge.completed": "اكتمل الدمج",
    "merge.recordCreated": "تم إنشاء سجل الدمج {id} في {at}.",
    "merge.viewMain": "عرض الشيء الرئيسي المدمج",
    "merge.confirm":
      "دمج {dup}… في {main}…؟\nسيؤدي ذلك إلى الحذف المبدئي للنسخة المكررة.",
    "results.title": "نتائج المطابقة",
    "results.noCandidates": "لا يوجد مرشحون.",
    "results.scoreBreakdown": "تفصيل النتيجة",
    "results.nameScore": "الاسم",
    "results.identifierScore": "المعرّف",
    "results.descriptionScore": "الوصف",
    "results.urlScore": "الرابط",
    "results.sameAsScore": "مطابق لـ",
    "results.phoneticMatch": "تطابق صوتي",
    "results.deterministicMatch": "حتمي (DOI/ISBN/…)",
    "form.name": "الاسم",
    "form.additionalType": "نوع إضافي",
    "form.additionalTypeHint": "رابط النوع الفرعي في schema.org",
    "form.description": "الوصف",
    "form.disambiguating": "وصف إزالة الالتباس",
    "form.disambiguatingHint": "تفصيل مميِّز قصير",
    "form.url": "الرابط",
    "form.owner": "المالك",
    "form.alternateNames": "أسماء بديلة",
    "form.alternateNamesHint": "واحد في كل سطر",
    "form.sameAs": "روابط «مطابق لـ»",
    "form.sameAsHint": "Wikidata وWikipedia وما إلى ذلك — واحد في كل سطر",
    "form.identifiers": "المعرفات",
    "form.saving": "جارٍ الحفظ…",
    "form.reset": "إعادة تعيين",
    "identifier.type": "النوع",
    "identifier.customOption": "مخصص…",
    "identifier.customLabel": "تسمية مخصصة",
    "identifier.value": "القيمة",
    "identifier.url": "الرابط",
    "identifier.remove": "حذف",
    "identifier.add": "+ إضافة معرّف",
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
    "splash.hero.title": "سجل موثوق واحد لكل شيء",
    "splash.hero.subtitle":
      "سجّل المنتجات والأصول والأعمال مرة واحدة، واعثر عليها فورًا، وامنع التكرار قبل أن يبدأ، مع سجل تدقيق كامل مدمج.",
    "splash.benefits.1.title": "تكرار أقل",
    "splash.benefits.1.body":
      "تنبّه المطابقة الفورية إلى احتمال التكرار لحظة إنشاء الشيء.",
    "splash.benefits.2.title": "اعثر على الأشياء بسرعة",
    "splash.benefits.2.body":
      "يعثر البحث التقريبي والصوتي على الأشياء رغم الأخطاء الإملائية واختلاف الهجاء.",
    "splash.benefits.3.title": "سجل واحد، معرّفات متعددة",
    "splash.benefits.3.body":
      "تجتمع معرّفات DOI وISBN وGTIN وغيرها في سجل واحد.",
    "splash.benefits.4.title": "دمج نظيف",
    "splash.benefits.4.body":
      "ادمج التكرار المؤكَّد في سجل واحد باقٍ دون فقدان ما كان معروفًا.",
    "splash.benefits.5.title": "قرارات مُراجَعة",
    "splash.benefits.5.body":
      "تنتظر أزواج التكرار المرشَّحة في قائمة ليؤكدها شخص أو يرفضها.",
    "splash.benefits.6.title": "سجل تاريخي كامل",
    "splash.benefits.6.body":
      "يُسجَّل كل تغيير في سجل تدقيق يمكنك الاطلاع عليه لأي شيء.",
    "splash.features.1.title": "البحث والتصفح",
    "splash.features.1.body":
      "تعرض شبكة بيانات قابلة للفرز والتصفية كل شيء مع بحث نصي كامل.",
    "splash.features.2.title": "سجلات غنية",
    "splash.features.2.body":
      "احتفظ بالمعرّفات والأسماء البديلة وروابط «هو نفسه» والصور لكل شيء.",
    "splash.features.3.title": "فحص التطابق",
    "splash.features.3.body": "قيّم سجلًا افتراضيًا مقابل الفهرس قبل إنشائه.",
    "splash.features.4.title": "دمج جنبًا إلى جنب",
    "splash.features.4.body":
      "اختر سجلًا رئيسيًا وآخر مكررًا ثم ادمجهما في خطوة واحدة.",
    "splash.features.5.title": "لوحة المراجعة",
    "splash.features.5.body":
      "اسحب الأزواج المعلّقة لتأكيدها أو رفضها، أو افتح زوجًا لمقارنة السجلين.",
    "splash.features.6.title": "خاص وسهل الوصول",
    "splash.features.6.body":
      "أخفِ الحقول الحساسة، وصدِّر البيانات، وبدّل اللغة والسمة وحجم النص.",
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
      "جولة إرشادية في سجل الأشياء: ما تفعله كل شاشة وخطوات استخدامها، من تسجيل منتج أو أصل إلى دمج التكرارات.",
    "tour.s1.title": "تسجيل شيء",
    "tour.s1.summary":
      "أنشئ سجلًا لمنتج أو أصل أو عمل مع معرّفاته وروابطه. يفحصه النموذج ويحذّر من التكرارات المحتملة قبل إنشاء أي شيء.",
    "tour.s1.step.1": "افتح «شيء جديد» من القائمة.",
    "tour.s1.step.2":
      "أدخل الاسم، وهو الحقل الوحيد المطلوب، ثم أضف الوصف والرابط والمالك والأسماء البديلة بحسب ما تعرف.",
    "tour.s1.step.3":
      "أضف معرّفات مثل DOI أو ISBN أو GTIN عبر «+ إضافة معرّف»، وأدرج روابط «نفس الشيء» رابطًا في كل سطر.",
    "tour.s1.step.4":
      "اختر «إنشاء». إذا ظهرت «نسخ مكررة محتملة» فراجعها قبل إعادة الإرسال؛ وإلا فستنتقل إلى صفحة الشيء الجديد.",
    "tour.s2.title": "العثور على شيء",
    "tour.s2.summary":
      "ابحث في الفهرس كله بالاسم أو المعرّف، ثم افتح أي نتيجة من شبكة البيانات.",
    "tour.s2.step.1": "افتح «الأشياء» من القائمة.",
    "tour.s2.step.2": "اكتب اسمًا أو معرّفًا في مربع البحث ثم اختر «بحث».",
    "tour.s2.step.3":
      "فعّل «غامض» أو «صوتي (Soundex)» لالتقاط الأخطاء الإملائية واختلافات التهجئة، أو «إخفاء الحساس» لإخفاء التفاصيل المحمية.",
    "tour.s2.step.4":
      "استخدم «السابق» و«التالي» للتنقل بين النتائج، واختر صفًا لفتح ذلك الشيء.",
    "tour.s3.title": "فحص التطابقات",
    "tour.s3.summary":
      "قيّم سجلًا افتراضيًا مقابل الفهرس قبل إنشائه، واطّلع على سبب تطابق كل مرشح.",
    "tour.s3.step.1": "افتح «فحص التطابق» من القائمة.",
    "tour.s3.step.2":
      "املأ الاسم، واختياريًا الوصف والرابط وروابط «نفس الشيء» والمعرّفات.",
    "tour.s3.step.3": "اضبط العتبة بين 0.0 و1.0 ثم اختر «البحث عن تطابقات».",
    "tour.s3.step.4":
      "اقرأ «نتائج المطابقة»: لكل مرشح «تفصيل النتيجة» حسب الاسم والمعرّف والوصف والرابط و«نفس الشيء»، وتُوسَم التطابقات الصوتية أو القطعية.",
    "tour.s4.title": "معالجة قائمة المراجعة",
    "tour.s4.summary":
      "تنتظر هنا أزواج التكرارات المرشحة من الفحص الدفعي ليؤكدها شخص أو يرفضها.",
    "tour.s4.step.1":
      "افتح «المراجعة» من القائمة واختر «تشغيل الفحص» للبحث عن أزواج مرشحة.",
    "tour.s4.step.2":
      "صفِّ حسب الحالة وحدّد حجم الصفحة؛ تظهر الأزواج بطاقات في اللوحة وصفوفًا في القائمة.",
    "tour.s4.step.3":
      "اختر «مقارنة» على زوج لرؤية السجل أ والسجل ب جنبًا إلى جنب مع تفصيل النتيجة.",
    "tour.s4.step.4":
      "اختر «تأكيد التكرار» أو «رفض». التأكيد يسجّل الحكم فقط؛ وللدمج اختر «الاحتفاظ بـ أ» أو «الاحتفاظ بـ ب».",
    "tour.s5.title": "دمج التكرارات",
    "tour.s5.summary":
      "ادمج تكرارًا مؤكدًا في سجل واحد باقٍ. يُحذف التكرار حذفًا مبدئيًا ويُسجَّل الدمج.",
    "tour.s5.step.1": "افتح «دمج» من القائمة.",
    "tour.s5.step.2":
      "أدخل معرّف الشيء الرئيسي (السجل الباقي) ومعرّف الشيء المكرر والسبب، وهو يُسجَّل في سجل تدقيق الدمج.",
    "tour.s5.step.3":
      "اختر «تحميل المعاينة» لمقارنة السجلين الرئيسي والمكرر قبل تغيير أي شيء.",
    "tour.s5.step.4":
      "اختر «دمج» وأكّد. تظهر بعدها رسالة «اكتمل الدمج» مع رابط «عرض الشيء الرئيسي المدمج».",
    "tour.s6.title": "فحص سجل وحمايته وتدقيقه",
    "tour.s6.summary": "لكل شيء صفحة تفاصيل لقراءته وتحريره وتصديره وتدقيقه.",
    "tour.s6.step.1":
      "في «الأشياء»، ابحث ثم اختر صفًا لفتح صفحة تفاصيل ذلك الشيء.",
    "tour.s6.step.2":
      "استخدم «إظهار المُقنَّع» للعرض المُنقَّح، أو «إظهار الكامل» لرؤية كل الحقول.",
    "tour.s6.step.3":
      "اختر «تحرير» للتعديل، أو «تدقيق» لقراءة سجل كل تغيير ومن أجراه، أو «تصدير البيانات (GDPR)» لتنزيل بياناته.",
    "tour.s6.step.4":
      "اختر «حذف» لحذف الشيء حذفًا مبدئيًا بعد التأكيد؛ ولا يمكن التراجع عن ذلك من الشاشة.",
    "signin.sso": "تسجيل الدخول عبر SSO",
  },
  "cy-001": {
    "auth.sessionExpired":
      "Mae eich sesiwn wedi dod i ben. Yn ailgyfeirio i fewngofnodi…",
    "auth.accessDenied": "Nid oes gennych ganiatâd i wneud hynny.",
    "nav.review": "Adolygu",
    "review.run": "Rhedeg sgan",
    "review.intro":
      "Parau dyblyg posibl o'r sgan swp. Llusgwch gerdyn sy'n aros, neu agorwch bâr i gymharu'r ddau gofnod ochr yn ochr cyn penderfynu.",
    "review.gap.provenance":
      "Nid yw'r gwasanaeth hwn yn cofnodi tarddiad ar wahân i bob pâr — dim ond sut y'i canfuwyd.",
    "review.loading": "Yn llwytho'r ciw adolygu…",
    "review.empty": "Dim eitemau adolygu ar gyfer yr hidl hon.",
    "review.filter.status": "Statws",
    "review.filter.statusAll": "Pob un",
    "review.filter.limit": "Maint tudalen",
    "review.filter.limitHint":
      "Mae'r gwasanaeth yn dychwelyd 500 eitem ar y mwyaf ac nid yw'n cynnig tudalennu y tu hwnt i hyn.",
    "review.status.pending": "Yn aros",
    "review.status.confirmed": "Wedi'i gadarnhau",
    "review.status.rejected": "Wedi'i wrthod",
    "review.status.automerged": "Wedi'i uno'n awtomatig",
    "review.board.title": "Bwrdd",
    "review.list.title": "Ciw",
    "review.col.pair": "Pâr",
    "review.col.score": "Sgôr",
    "review.col.quality": "Ansawdd",
    "review.col.method": "Dull canfod",
    "review.col.status": "Statws",
    "review.col.actions": "Gweithredoedd",
    "review.compare.open": "Cymharu",
    "review.compare.title": "Cymharu'r pâr",
    "review.compare.close": "Cau",
    "review.compare.loading": "Yn llwytho'r ddau gofnod…",
    "review.compare.field": "Maes",
    "review.compare.a": "Cofnod A",
    "review.compare.b": "Cofnod B",
    "review.compare.none": "Heb ei gofnodi",
    "review.compare.partial":
      "Ni ellid llwytho un cofnod — efallai ei fod wedi'i uno neu ei ddileu.",
    "review.field.score": "Sgôr cydweddu",
    "review.field.quality": "Ansawdd cydweddu",
    "review.field.method": "Dull canfod",
    "review.field.status": "Statws",
    "review.breakdown.title": "Dadansoddiad sgôr",
    "review.breakdown.none":
      "Ni chofnodwyd dadansoddiad sgôr ar gyfer y pâr hwn.",
    "review.breakdown.component": "Cydran",
    "review.breakdown.weight": "Pwysau",
    "review.breakdown.score": "Sgôr",
    "review.decide.confirm": "Cadarnhau dyblyg",
    "review.decide.reject": "Gwrthod",
    "review.decide.deciding": "Yn cadw…",
    "review.decide.locked":
      "Eisoes wedi penderfynu — dim ond eitemau sy'n aros y gellir eu penderfynu.",
    "review.merge.title": "Uno'r pâr hwn",
    "review.merge.note":
      "Mae cadarnhau ond yn cofnodi'r dyfarniad; nid yw'n uno. Dewiswch pa gofnod sy'n goroesi.",
    "review.merge.keepA": "Cadw A, uno B iddo",
    "review.merge.keepB": "Cadw B, uno A iddo",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Dangosfwrdd",
    "nav.things": "Pethau",
    "nav.newThing": "Peth newydd",
    "nav.matchCheck": "Gwiriad cydweddu",
    "nav.merge": "Uno",
    "nav.toggle": "Toglo'r llywio",
    "chrome.theme": "Thema",
    "chrome.language": "Iaith",
    "nav.share": "Rhannu",
    "nav.text_size": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "dashboard.title": "Dangosfwrdd",
    "dashboard.service": "Gwasanaeth:",
    "dashboard.recentActivity": "Gweithgaredd diweddar",
    "dashboard.noRecent": "Dim cofnodion archwilio diweddar.",
    "things.title": "Pethau",
    "things.new": "Peth newydd",
    "things.searchPlaceholder": "Chwilio yn ôl enw, dynodydd…",
    "things.fuzzy": "Bras",
    "things.phonetic": "Ffonetig (Soundex)",
    "things.maskSensitive": "Cuddio sensitif",
    "things.previousPage": "Blaenorol",
    "things.nextPage": "Nesaf",
    "things.pageRange": "{from}–{to} o {total}",
    "things.loading": "Yn llwytho…",
    "things.recordCount": "{count} cofnod",
    "things.recordCountPlural": "{count} cofnod",
    "search.action": "Chwilio",
    "grid.id": "ID",
    "grid.name": "Enw",
    "grid.type": "Math (schema.org)",
    "grid.primaryId": "Prif ddynodydd",
    "grid.url": "URL",
    "detail.loading": "Yn llwytho…",
    "detail.edit": "Golygu",
    "detail.audit": "Archwilio",
    "detail.delete": "Dileu",
    "detail.exportGdpr": "Allforio data (GDPR)",
    "detail.exportingGdpr": "Wrthi'n allforio…",
    "detail.showMasked": "Dangos wedi'i guddio",
    "detail.showFull": "Dangos yn llawn",
    "detail.maskedNotice":
      "Yn dangos y golwg guddiedig — mae rhai meysydd wedi'u cuddio.",
    "detail.confirmDelete":
      "Meddal-ddileu'r peth hwn? Ni ellir ei ddadwneud drwy'r rhyngwyneb.",
    "detail.identity": "Hunaniaeth",
    "detail.id": "ID",
    "detail.additionalType": "Math ychwanegol",
    "detail.description": "Disgrifiad",
    "detail.disambiguating": "Dadamwysol",
    "detail.url": "URL",
    "detail.owner": "Perchennog",
    "detail.mainEntityOfPage": "Prif endid y dudalen",
    "detail.identifiers": "Dynodyddion",
    "detail.alternateNames": "Enwau eraill",
    "detail.sameAs": "Yr un fath â (URLau awdurdodol)",
    "detail.images": "Delweddau",
    "detail.customPrefix": "Personol: ",
    "audit.title": "Cofnod archwilio",
    "audit.backToThing": "Yn ôl i'r peth",
    "audit.loading": "Yn llwytho…",
    "audit.none": "Dim cofnodion archwilio.",
    "audit.by": "gan",
    "audit.payload": "Llwyth",
    "edit.title": "Golygu peth",
    "edit.cancel": "Canslo",
    "edit.loading": "Yn llwytho…",
    "edit.submitLabel": "Cadw newidiadau",
    "new.title": "Peth newydd",
    "new.submitLabel": "Creu",
    "new.possibleDuplicates": "Dyblygiadau posibl",
    "new.duplicatesDetected":
      "Canfuwyd dyblygiadau ({count}) — adolygwch isod cyn ailgyflwyno.",
    "match.title": "Gwiriad cydweddu",
    "match.name": "Enw",
    "match.threshold": "Trothwy",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "Disgrifiad",
    "match.url": "URL",
    "match.sameAs": "URLau yr un fath â",
    "match.sameAsHint": "Un fesul llinell",
    "match.identifiers": "Dynodyddion",
    "match.matching": "Yn cydweddu…",
    "match.findMatches": "Canfod cydweddiadau",
    "merge.title": "Uno pethau",
    "merge.mainId": "ID y prif beth",
    "merge.mainIdHint": "Y cofnod sy'n goroesi",
    "merge.dupId": "ID y peth dyblyg",
    "merge.dupIdHint": "Caiff ei feddal-ddileu",
    "merge.reason": "Rheswm",
    "merge.reasonHint": "Wedi'i gofnodi yn llwybr archwilio'r uno",
    "merge.reasonPlaceholder": "Dyblyg wedi'i gadarnhau",
    "merge.loadPreview": "Llwytho rhagolwg",
    "merge.merging": "Yn uno…",
    "merge.merge": "Uno",
    "merge.preview": "Rhagolwg",
    "merge.main": "Prif",
    "merge.duplicate": "Dyblyg",
    "merge.completed": "Uno wedi'i gwblhau",
    "merge.recordCreated": "Crëwyd cofnod uno {id} am {at}.",
    "merge.viewMain": "Gweld y prif beth a unwyd",
    "merge.confirm":
      "Uno {dup}… i mewn i {main}…?\nMae hyn yn meddal-ddileu'r dyblyg.",
    "results.title": "Canlyniadau cydweddu",
    "results.noCandidates": "Dim ymgeiswyr.",
    "results.scoreBreakdown": "Dadansoddiad sgôr",
    "results.nameScore": "enw",
    "results.identifierScore": "dynodydd",
    "results.descriptionScore": "disgrifiad",
    "results.urlScore": "URL",
    "results.sameAsScore": "yr un fath â",
    "results.phoneticMatch": "cydweddiad ffonetig",
    "results.deterministicMatch": "penderfyniadol (DOI/ISBN/…)",
    "form.name": "Enw",
    "form.additionalType": "Math ychwanegol",
    "form.additionalTypeHint": "URL is-fath schema.org",
    "form.description": "Disgrifiad",
    "form.disambiguating": "Disgrifiad dadamwysol",
    "form.disambiguatingHint": "Manylyn gwahaniaethol byr",
    "form.url": "URL",
    "form.owner": "Perchennog",
    "form.alternateNames": "Enwau eraill",
    "form.alternateNamesHint": "Un fesul llinell",
    "form.sameAs": "URLau yr un fath â",
    "form.sameAsHint": "Wikidata, Wikipedia, ac ati — un fesul llinell",
    "form.identifiers": "Dynodyddion",
    "form.saving": "Yn cadw…",
    "form.reset": "Ailosod",
    "identifier.type": "Math",
    "identifier.customOption": "Personol…",
    "identifier.customLabel": "Label personol",
    "identifier.value": "Gwerth",
    "identifier.url": "URL",
    "identifier.remove": "Dileu",
    "identifier.add": "+ Ychwanegu dynodydd",
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
    "splash.hero.title": "Un cofnod dibynadwy ar gyfer pob peth",
    "splash.hero.subtitle":
      "Cofrestrwch gynhyrchion, asedau a gweithiau unwaith, dewch o hyd iddynt ar unwaith, a rhwystrwch ddyblygu cyn iddo ddechrau, gyda llwybr archwilio llawn wedi'i gynnwys.",
    "splash.benefits.1.title": "Llai o ddyblygu",
    "splash.benefits.1.body":
      "Mae paru amser real yn nodi dyblyg tebygol y funud y crëir peth.",
    "splash.benefits.2.title": "Dewch o hyd i bethau'n gyflym",
    "splash.benefits.2.body":
      "Mae chwilio niwlog a ffonetig yn dod o hyd i bethau er gwaethaf camsillafu ac amrywiadau sillafu.",
    "splash.benefits.3.title": "Un cofnod, llawer o ddynodyddion",
    "splash.benefits.3.body":
      "Mae DOI, ISBN, GTIN a dynodyddion eraill i gyd ar un cofnod.",
    "splash.benefits.4.title": "Uno glân",
    "splash.benefits.4.body":
      "Unwch ddyblyg wedi'i gadarnhau yn un cofnod sy'n goroesi heb golli'r hyn a wyddid.",
    "splash.benefits.5.title": "Penderfyniadau wedi'u hadolygu",
    "splash.benefits.5.body":
      "Mae parau dyblyg posibl yn aros mewn ciw i rywun eu cadarnhau neu eu gwrthod.",
    "splash.benefits.6.title": "Hanes cyflawn",
    "splash.benefits.6.body":
      "Cofnodir pob newid mewn cofnod archwilio y gallwch ei ddarllen ar gyfer unrhyw beth.",
    "splash.features.1.title": "Chwilio a phori",
    "splash.features.1.body":
      "Mae grid data y gellir ei ddidoli a'i hidlo yn rhestru pob peth, gyda chwilio testun llawn ar y brig.",
    "splash.features.2.title": "Cofnodion cyfoethog",
    "splash.features.2.body":
      "Cadwch ddynodyddion, enwau amgen, dolenni 'yr un fath â' a delweddau ar bob peth.",
    "splash.features.3.title": "Gwiriad cydweddu",
    "splash.features.3.body":
      "Sgoriwch gofnod damcaniaethol yn erbyn y mynegai cyn ei greu.",
    "splash.features.4.title": "Uno ochr yn ochr",
    "splash.features.4.body":
      "Dewiswch brif gofnod a dyblyg, yna unwch nhw mewn un cam.",
    "splash.features.5.title": "Bwrdd adolygu",
    "splash.features.5.body":
      "Llusgwch barau sydd ar y gweill i'w cadarnhau neu eu gwrthod, neu agorwch un i gymharu'r ddau gofnod.",
    "splash.features.6.title": "Preifat a hygyrch",
    "splash.features.6.body":
      "Cuddiwch feysydd sensitif, allforiwch ddata, a newidiwch iaith, thema a maint testun.",
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
      "Taith dywys drwy'r gofrestr Pethau: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o gofrestru cynnyrch neu ased i uno dyblygion.",
    "tour.s1.title": "Cofrestru peth",
    "tour.s1.summary":
      "Crëwch gofnod ar gyfer cynnyrch, ased neu waith gyda'i ddynodwyr a'i gysylltiadau. Mae'r ffurflen yn ei wirio ac yn rhybuddio am ddyblygion tebygol cyn creu dim.",
    "tour.s1.step.1": "Agorwch Peth newydd o'r ddewislen.",
    "tour.s1.step.2":
      "Rhowch yr Enw, yr unig faes gorfodol, yna ychwanegwch y disgrifiad, yr URL, y perchennog a'r enwau eraill fel y gwyddoch.",
    "tour.s1.step.3":
      "Ychwanegwch ddynodwyr fel DOI, ISBN neu GTIN gyda + Ychwanegu dynodydd, a rhestrwch URLau «yr un peth â» un ar bob llinell.",
    "tour.s1.step.4":
      "Dewiswch Creu. Os bydd Dyblygiadau posibl yn ymddangos, adolygwch nhw cyn ailgyflwyno; fel arall fe'ch cymerir i dudalen y peth newydd.",
    "tour.s2.title": "Dod o hyd i beth",
    "tour.s2.summary":
      "Chwiliwch y mynegai cyfan yn ôl enw neu ddynodydd, yna agorwch unrhyw ganlyniad o'r grid data.",
    "tour.s2.step.1": "Agorwch Pethau o'r ddewislen.",
    "tour.s2.step.2":
      "Teipiwch enw neu ddynodydd yn y blwch chwilio a dewiswch Chwilio.",
    "tour.s2.step.3":
      "Trowch Bras neu Ffonetig (Soundex) ymlaen i ddal camsillafu ac amrywiadau sillafu, neu Cuddio sensitif i guddio manylion gwarchodedig.",
    "tour.s2.step.4":
      "Defnyddiwch Blaenorol a Nesaf i droi trwy'r canlyniadau, a dewiswch res i agor y peth hwnnw.",
    "tour.s3.title": "Gwirio am gydweddiadau",
    "tour.s3.summary":
      "Sgoriwch cofnod damcaniaethol yn erbyn y mynegai cyn ei greu, a gweld pam y cydweddodd pob ymgeisydd.",
    "tour.s3.step.1": "Agorwch Gwiriad cydweddu o'r ddewislen.",
    "tour.s3.step.2":
      "Llenwch yr Enw, ac yn ddewisol y disgrifiad, yr URL, URLau «yr un peth â» a'r dynodwyr.",
    "tour.s3.step.3":
      "Gosodwch y Trothwy rhwng 0.0 ac 1.0, yna dewiswch Canfod cydweddiadau.",
    "tour.s3.step.4":
      "Darllenwch y Canlyniadau cydweddu: mae gan bob ymgeisydd Ddadansoddiad sgôr yn ôl enw, dynodydd, disgrifiad, URL ac yr un peth â, a nodir cydweddiadau ffonetig neu benderfynol.",
    "tour.s4.title": "Gweithio'r ciw adolygu",
    "tour.s4.summary":
      "Mae parau dyblyg ymgeisiol o'r sgan swp yn aros yma i berson eu cadarnhau neu eu gwrthod.",
    "tour.s4.step.1":
      "Agorwch Adolygu o'r ddewislen a dewiswch Rhedeg sgan i chwilio am barau ymgeisiol.",
    "tour.s4.step.2":
      "Hidlwch yn ôl Statws a gosodwch faint y dudalen; mae parau'n ymddangos fel cardiau ar y Bwrdd ac fel rhesi yn y Ciw.",
    "tour.s4.step.3":
      "Dewiswch Cymharu ar bâr i weld Cofnod A a Chofnod B ochr yn ochr, gyda'r Dadansoddiad sgôr.",
    "tour.s4.step.4":
      "Dewiswch Cadarnhau dyblyg neu Gwrthod. Dim ond y dyfarniad y mae cadarnhau yn ei gofnodi; i uno, dewiswch Cadw A neu Cadw B.",
    "tour.s5.title": "Uno dyblygion",
    "tour.s5.summary":
      "Plygwch ddyblyg wedi'i gadarnhau i mewn i un cofnod sy'n goroesi. Caiff y dyblyg ei feddal-ddileu a chofnodir yr uno.",
    "tour.s5.step.1": "Agorwch Uno o'r ddewislen.",
    "tour.s5.step.2":
      "Rhowch ID y prif beth (y cofnod sy'n goroesi), ID y peth dyblyg a Rheswm, sy'n cael ei gofnodi yn llwybr archwilio'r uno.",
    "tour.s5.step.3":
      "Dewiswch Llwytho rhagolwg i gymharu'r cofnodion Prif a Dyblyg cyn i ddim newid.",
    "tour.s5.step.4":
      "Dewiswch Uno a chadarnhewch. Daw neges Uno wedi'i gwblhau nesaf, gyda dolen i Gweld y prif beth a unwyd.",
    "tour.s6.title": "Archwilio, gwarchod ac archwilio cofnod",
    "tour.s6.summary":
      "Mae gan bob peth dudalen fanylion i'w ddarllen, ei olygu, ei allforio a'i archwilio.",
    "tour.s6.step.1":
      "Ar Pethau, chwiliwch a dewiswch res i agor tudalen fanylion y peth hwnnw.",
    "tour.s6.step.2":
      "Defnyddiwch Dangos wedi'i guddio am y golwg wedi'i olygu, neu Dangos yn llawn i weld pob maes.",
    "tour.s6.step.3":
      "Dewiswch Golygu i'w newid, Archwilio i ddarllen y log o bob newid a phwy a'i gwnaeth, neu Allforio data (GDPR) i lawrlwytho ei ddata.",
    "tour.s6.step.4":
      "Dewiswch Dileu i feddal-ddileu'r peth ar ôl i chi gadarnhau; ni ellir dadwneud hyn o'r sgrin.",
    "signin.sso": "Mewngofnodi gydag SSO",
  },
  "de-de": {
    "auth.sessionExpired":
      "Ihre Sitzung ist abgelaufen. Weiterleitung zur Anmeldung…",
    "auth.accessDenied": "Sie haben keine Berechtigung dazu.",
    "nav.review": "Überprüfung",
    "review.run": "Scan starten",
    "review.intro":
      "Mögliche Duplikatpaare aus dem Stapel-Scan. Ziehen Sie eine ausstehende Karte oder öffnen Sie ein Paar, um beide Datensätze vor der Entscheidung nebeneinander zu vergleichen.",
    "review.gap.provenance":
      "Dieser Dienst erfasst keine eigene Herkunft je Paar — nur die Erkennungsmethode.",
    "review.loading": "Überprüfungswarteschlange wird geladen…",
    "review.empty": "Keine Überprüfungseinträge für diesen Filter.",
    "review.filter.status": "Status",
    "review.filter.statusAll": "Alle",
    "review.filter.limit": "Seitengröße",
    "review.filter.limitHint":
      "Der Dienst liefert höchstens 500 Einträge und bietet darüber hinaus keine Seitennummerierung.",
    "review.status.pending": "Ausstehend",
    "review.status.confirmed": "Bestätigt",
    "review.status.rejected": "Abgelehnt",
    "review.status.automerged": "Automatisch zusammengeführt",
    "review.board.title": "Board",
    "review.list.title": "Warteschlange",
    "review.col.pair": "Paar",
    "review.col.score": "Punktzahl",
    "review.col.quality": "Qualität",
    "review.col.method": "Erkennungsmethode",
    "review.col.status": "Status",
    "review.col.actions": "Aktionen",
    "review.compare.open": "Vergleichen",
    "review.compare.title": "Paar vergleichen",
    "review.compare.close": "Schließen",
    "review.compare.loading": "Beide Datensätze werden geladen…",
    "review.compare.field": "Feld",
    "review.compare.a": "Datensatz A",
    "review.compare.b": "Datensatz B",
    "review.compare.none": "Nicht erfasst",
    "review.compare.partial":
      "Ein Datensatz konnte nicht geladen werden — er wurde möglicherweise zusammengeführt oder gelöscht.",
    "review.field.score": "Übereinstimmungswert",
    "review.field.quality": "Übereinstimmungsqualität",
    "review.field.method": "Erkennungsmethode",
    "review.field.status": "Status",
    "review.breakdown.title": "Bewertungsaufschlüsselung",
    "review.breakdown.none":
      "Für dieses Paar wurde keine Bewertungsaufschlüsselung erfasst.",
    "review.breakdown.component": "Komponente",
    "review.breakdown.weight": "Gewichtung",
    "review.breakdown.score": "Punktzahl",
    "review.decide.confirm": "Duplikat bestätigen",
    "review.decide.reject": "Ablehnen",
    "review.decide.deciding": "Wird gespeichert…",
    "review.decide.locked":
      "Bereits entschieden — nur ausstehende Einträge können entschieden werden.",
    "review.merge.title": "Dieses Paar zusammenführen",
    "review.merge.note":
      "Bestätigen erfasst nur die Entscheidung; es führt nicht zusammen. Wählen Sie, welcher Datensatz erhalten bleibt.",
    "review.merge.keepA": "A behalten, B damit zusammenführen",
    "review.merge.keepB": "B behalten, A damit zusammenführen",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Übersicht",
    "nav.things": "Dinge",
    "nav.newThing": "Neues Ding",
    "nav.matchCheck": "Abgleich prüfen",
    "nav.merge": "Zusammenführen",
    "nav.toggle": "Navigation umschalten",
    "chrome.theme": "Thema",
    "chrome.language": "Sprache",
    "nav.share": "Teilen",
    "nav.text_size": "Textgröße",
    "share.copy_link": "Link kopieren",
    "share.copied": "Link kopiert",
    "share.copy_failed":
      "Kopieren fehlgeschlagen — bitte aus der Adressleiste kopieren",
    "dashboard.title": "Übersicht",
    "dashboard.service": "Dienst:",
    "dashboard.recentActivity": "Letzte Aktivität",
    "dashboard.noRecent": "Keine aktuellen Audit-Einträge.",
    "things.title": "Dinge",
    "things.new": "Neues Ding",
    "things.searchPlaceholder": "Nach Name, Bezeichner suchen…",
    "things.fuzzy": "Unscharf",
    "things.phonetic": "Phonetisch (Soundex)",
    "things.maskSensitive": "Sensibel maskieren",
    "things.previousPage": "Zurück",
    "things.nextPage": "Weiter",
    "things.pageRange": "{from}–{to} von {total}",
    "things.loading": "Wird geladen…",
    "things.recordCount": "{count} Datensatz",
    "things.recordCountPlural": "{count} Datensätze",
    "search.action": "Suchen",
    "grid.id": "ID",
    "grid.name": "Name",
    "grid.type": "Typ (schema.org)",
    "grid.primaryId": "Primärer Bezeichner",
    "grid.url": "URL",
    "detail.loading": "Wird geladen…",
    "detail.edit": "Bearbeiten",
    "detail.audit": "Audit",
    "detail.delete": "Löschen",
    "detail.exportGdpr": "Daten exportieren (DSGVO)",
    "detail.exportingGdpr": "Exportiere…",
    "detail.showMasked": "Maskiert anzeigen",
    "detail.showFull": "Vollständig anzeigen",
    "detail.maskedNotice":
      "Maskierte Ansicht wird angezeigt — einige Felder sind geschwärzt.",
    "detail.confirmDelete":
      "Dieses Ding logisch löschen? Über die Oberfläche nicht rückgängig zu machen.",
    "detail.identity": "Identität",
    "detail.id": "ID",
    "detail.additionalType": "Zusätzlicher Typ",
    "detail.description": "Beschreibung",
    "detail.disambiguating": "Begriffsklärung",
    "detail.url": "URL",
    "detail.owner": "Eigentümer",
    "detail.mainEntityOfPage": "Hauptentität der Seite",
    "detail.identifiers": "Bezeichner",
    "detail.alternateNames": "Alternative Namen",
    "detail.sameAs": "Identisch mit (maßgebliche URLs)",
    "detail.images": "Bilder",
    "detail.customPrefix": "Benutzerdefiniert: ",
    "audit.title": "Audit-Protokoll",
    "audit.backToThing": "Zurück zum Ding",
    "audit.loading": "Wird geladen…",
    "audit.none": "Keine Audit-Einträge.",
    "audit.by": "von",
    "audit.payload": "Nutzlast",
    "edit.title": "Ding bearbeiten",
    "edit.cancel": "Abbrechen",
    "edit.loading": "Wird geladen…",
    "edit.submitLabel": "Änderungen speichern",
    "new.title": "Neues Ding",
    "new.submitLabel": "Erstellen",
    "new.possibleDuplicates": "Mögliche Duplikate",
    "new.duplicatesDetected":
      "Duplikate erkannt ({count}) — bitte unten prüfen, bevor Sie erneut senden.",
    "match.title": "Abgleich prüfen",
    "match.name": "Name",
    "match.threshold": "Schwellenwert",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "Beschreibung",
    "match.url": "URL",
    "match.sameAs": "„Identisch mit“-URLs",
    "match.sameAsHint": "Eine pro Zeile",
    "match.identifiers": "Bezeichner",
    "match.matching": "Abgleich läuft…",
    "match.findMatches": "Treffer finden",
    "merge.title": "Dinge zusammenführen",
    "merge.mainId": "ID des Hauptdings",
    "merge.mainIdHint": "Der erhaltene Datensatz",
    "merge.dupId": "ID des Duplikats",
    "merge.dupIdHint": "Wird logisch gelöscht",
    "merge.reason": "Grund",
    "merge.reasonHint": "Im Audit-Protokoll der Zusammenführung erfasst",
    "merge.reasonPlaceholder": "Bestätigtes Duplikat",
    "merge.loadPreview": "Vorschau laden",
    "merge.merging": "Wird zusammengeführt…",
    "merge.merge": "Zusammenführen",
    "merge.preview": "Vorschau",
    "merge.main": "Haupt",
    "merge.duplicate": "Duplikat",
    "merge.completed": "Zusammenführung abgeschlossen",
    "merge.recordCreated": "Zusammenführungsdatensatz {id} erstellt am {at}.",
    "merge.viewMain": "Zusammengeführtes Hauptding anzeigen",
    "merge.confirm":
      "{dup}… in {main}… zusammenführen?\nDies löscht das Duplikat logisch.",
    "results.title": "Abgleichsergebnisse",
    "results.noCandidates": "Keine Kandidaten.",
    "results.scoreBreakdown": "Bewertungsaufschlüsselung",
    "results.nameScore": "Name",
    "results.identifierScore": "Bezeichner",
    "results.descriptionScore": "Beschreibung",
    "results.urlScore": "URL",
    "results.sameAsScore": "identisch mit",
    "results.phoneticMatch": "phonetischer Treffer",
    "results.deterministicMatch": "deterministisch (DOI/ISBN/…)",
    "form.name": "Name",
    "form.additionalType": "Zusätzlicher Typ",
    "form.additionalTypeHint": "schema.org-Subtyp-URL",
    "form.description": "Beschreibung",
    "form.disambiguating": "Begriffsklärungsbeschreibung",
    "form.disambiguatingHint": "Kurzes unterscheidendes Detail",
    "form.url": "URL",
    "form.owner": "Eigentümer",
    "form.alternateNames": "Alternative Namen",
    "form.alternateNamesHint": "Eine pro Zeile",
    "form.sameAs": "„Identisch mit“-URLs",
    "form.sameAsHint": "Wikidata, Wikipedia usw. — eine pro Zeile",
    "form.identifiers": "Bezeichner",
    "form.saving": "Wird gespeichert…",
    "form.reset": "Zurücksetzen",
    "identifier.type": "Typ",
    "identifier.customOption": "Benutzerdefiniert…",
    "identifier.customLabel": "Benutzerdefinierte Bezeichnung",
    "identifier.value": "Wert",
    "identifier.url": "URL",
    "identifier.remove": "Löschen",
    "identifier.add": "+ Bezeichner hinzufügen",
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
    "splash.hero.title": "Ein verlässlicher Datensatz für jedes Ding",
    "splash.hero.subtitle":
      "Erfassen Sie Produkte, Vermögenswerte und Werke einmal, finden Sie sie sofort und verhindern Sie Dubletten, bevor sie entstehen, mit integriertem, lückenlosem Audit-Protokoll.",
    "splash.benefits.1.body":
      "Der Echtzeit-Abgleich meldet eine wahrscheinliche Dublette, sobald ein Ding angelegt wird.",
    "splash.benefits.2.title": "Dinge schnell finden",
    "splash.benefits.2.body":
      "Unscharfe und phonetische Suche findet Dinge trotz Tippfehlern und Schreibvarianten.",
    "splash.benefits.3.title": "Ein Datensatz, viele Kennungen",
    "splash.benefits.3.body":
      "DOI, ISBN, GTIN und andere Kennungen liegen alle in einem einzigen Datensatz.",
    "splash.benefits.4.title": "Saubere Zusammenführungen",
    "splash.benefits.4.body":
      "Führen Sie eine bestätigte Dublette in einen bleibenden Datensatz ein, ohne Bekanntes zu verlieren.",
    "splash.benefits.5.title": "Geprüfte Entscheidungen",
    "splash.benefits.5.body":
      "Kandidatenpaare für Dubletten warten in einer Warteschlange darauf, dass eine Person sie bestätigt oder ablehnt.",
    "splash.benefits.6.title": "Ein vollständiger Verlauf",
    "splash.benefits.6.body":
      "Jede Änderung wird in einem Audit-Protokoll festgehalten, das Sie für jedes Ding lesen können.",
    "splash.features.1.title": "Suchen und durchstöbern",
    "splash.features.1.body":
      "Eine sortier- und filterbare Datentabelle listet jedes Ding, darüber eine Volltextsuche.",
    "splash.features.2.title": "Reichhaltige Datensätze",
    "splash.features.2.body":
      "Halten Sie Kennungen, alternative Namen, „Identisch mit“-Links und Bilder an jedem Ding fest.",
    "splash.features.3.title": "Abgleich prüfen",
    "splash.features.3.body":
      "Bewerten Sie einen hypothetischen Datensatz anhand des Index, bevor Sie ihn anlegen.",
    "splash.features.4.title": "Zusammenführen im Vergleich",
    "splash.features.4.body":
      "Wählen Sie einen Hauptdatensatz und ein Duplikat und führen Sie beide in einem Schritt zusammen.",
    "splash.features.5.title": "Überprüfungsboard",
    "splash.features.5.body":
      "Ziehen Sie offene Paare, um sie zu bestätigen oder abzulehnen, oder öffnen Sie eines, um beide Datensätze zu vergleichen.",
    "splash.features.6.title": "Privat und barrierefrei",
    "splash.features.6.body":
      "Maskieren Sie sensible Felder, exportieren Sie Daten und wechseln Sie Sprache, Thema und Textgröße.",
    "tour.intro":
      "Ein geführter Rundgang durch das Register der Dinge: was jede Ansicht leistet und wie Sie sie nutzen, von der Erfassung eines Produkts oder Vermögenswerts bis zum Zusammenführen von Dubletten.",
    "tour.s1.title": "Ein Ding erfassen",
    "tour.s1.summary":
      "Legen Sie einen Datensatz für ein Produkt, einen Vermögenswert oder ein Werk mit Kennungen und Verknüpfungen an. Das Formular prüft ihn und warnt vor wahrscheinlichen Dubletten, bevor etwas angelegt wird.",
    "tour.s1.step.1": "Öffnen Sie im Menü „Neues Ding“.",
    "tour.s1.step.2":
      "Geben Sie den Namen ein, das einzige Pflichtfeld, und ergänzen Sie nach Kenntnis Beschreibung, URL, Eigentümer und alternative Namen.",
    "tour.s1.step.3":
      "Fügen Sie Kennungen wie DOI, ISBN oder GTIN mit „+ Bezeichner hinzufügen“ hinzu und tragen Sie „Identisch mit“-URLs eine pro Zeile ein.",
    "tour.s1.step.4":
      "Wählen Sie „Erstellen“. Erscheinen „Mögliche Duplikate“, prüfen Sie sie, bevor Sie erneut absenden; andernfalls gelangen Sie zur Seite des neuen Dings.",
    "tour.s2.title": "Ein Ding finden",
    "tour.s2.summary":
      "Durchsuchen Sie den gesamten Index nach Name oder Kennung und öffnen Sie ein Ergebnis aus der Datentabelle.",
    "tour.s2.step.1": "Öffnen Sie im Menü „Dinge“.",
    "tour.s2.step.2":
      "Geben Sie einen Namen oder eine Kennung in das Suchfeld ein und wählen Sie „Suchen“.",
    "tour.s2.step.3":
      "Schalten Sie „Unscharf“ oder „Phonetisch (Soundex)“ ein, um Tippfehler und Schreibvarianten zu finden, oder „Sensibel maskieren“, um geschützte Angaben auszublenden.",
    "tour.s2.step.4":
      "Mit „Zurück“ und „Weiter“ blättern Sie durch die Ergebnisse; wählen Sie eine Zeile, um dieses Ding zu öffnen.",
    "tour.s3.title": "Auf Übereinstimmungen prüfen",
    "tour.s3.summary":
      "Bewerten Sie einen hypothetischen Datensatz anhand des Index, bevor Sie ihn anlegen, und sehen Sie, warum jeder Kandidat passt.",
    "tour.s3.step.1": "Öffnen Sie im Menü „Abgleich prüfen“.",
    "tour.s3.step.2":
      "Tragen Sie den Namen ein und optional Beschreibung, URL, „Identisch mit“-URLs und Kennungen.",
    "tour.s3.step.3":
      "Legen Sie den „Schwellenwert“ zwischen 0,0 und 1,0 fest und wählen Sie „Treffer finden“.",
    "tour.s3.step.4":
      "Lesen Sie die „Abgleichergebnisse“: Jeder Kandidat hat eine Bewertungsaufschlüsselung nach Name, Kennung, Beschreibung, URL und „Identisch mit“, und phonetische oder deterministische Treffer sind gekennzeichnet.",
    "tour.s4.title": "Die Überprüfungswarteschlange bearbeiten",
    "tour.s4.summary":
      "Kandidatenpaare aus dem Stapel-Scan warten hier darauf, dass eine Person sie bestätigt oder ablehnt.",
    "tour.s4.step.1":
      "Öffnen Sie im Menü „Überprüfung“ und wählen Sie „Scan starten“, um nach Kandidatenpaaren zu suchen.",
    "tour.s4.step.2":
      "Filtern Sie nach Status und legen Sie die Seitengröße fest; Paare erscheinen als Karten auf dem „Board“ und als Zeilen in der „Warteschlange“.",
    "tour.s4.step.3":
      "Wählen Sie bei einem Paar „Vergleichen“, um Datensatz A und Datensatz B nebeneinander zu sehen, mit der Bewertungsaufschlüsselung.",
    "tour.s4.step.4":
      "Wählen Sie „Dublette bestätigen“ oder „Ablehnen“. Bestätigen hält nur das Urteil fest; zum Zusammenführen wählen Sie „A behalten“ oder „B behalten“.",
    "tour.s5.title": "Dubletten zusammenführen",
    "tour.s5.summary":
      "Führen Sie eine bestätigte Dublette in einen bleibenden Datensatz ein. Die Dublette wird weich gelöscht und die Zusammenführung protokolliert.",
    "tour.s5.step.1": "Öffnen Sie im Menü „Zusammenführen“.",
    "tour.s5.step.2":
      "Geben Sie die „ID des Hauptdings“ (der bleibende Datensatz), die „ID des Duplikats“ und einen „Grund“ ein, der im Audit-Protokoll der Zusammenführung festgehalten wird.",
    "tour.s5.step.3":
      "Wählen Sie „Vorschau laden“, um Haupt- und Duplikat-Datensatz zu vergleichen, bevor sich etwas ändert.",
    "tour.s5.step.4":
      "Wählen Sie „Zusammenführen“ und bestätigen Sie. Es folgt die Meldung „Zusammenführung abgeschlossen“ mit einem Link zu „Zusammengeführtes Hauptding anzeigen“.",
    "tour.s6.title": "Einen Datensatz prüfen, schützen und nachverfolgen",
    "tour.s6.summary":
      "Jedes Ding hat eine Detailseite zum Lesen, Bearbeiten, Exportieren und Nachverfolgen.",
    "tour.s6.step.1":
      "Suchen Sie unter „Dinge“ und wählen Sie eine Zeile, um die Detailseite dieses Dings zu öffnen.",
    "tour.s6.step.2":
      "Mit „Maskiert anzeigen“ sehen Sie die geschwärzte Ansicht, mit „Vollständig anzeigen“ jedes Feld.",
    "tour.s6.step.3":
      "Wählen Sie „Bearbeiten“, um es zu ändern, „Audit“, um das Protokoll aller Änderungen samt Urheber zu lesen, oder „Daten exportieren (DSGVO)“, um seine Daten herunterzuladen.",
    "tour.s6.step.4":
      "Wählen Sie „Löschen“, um das Ding nach Bestätigung weich zu löschen; das lässt sich über die Oberfläche nicht rückgängig machen.",
  },
  "en-001": {
    "auth.sessionExpired": "Your session has expired. Redirecting to sign in…",
    "auth.accessDenied": "You don't have permission to do that.",
    "nav.review": "Review",
    "review.run": "Run scan",
    "review.intro":
      "Candidate duplicate pairs from the batch scan. Drag a pending card, or open a pair to compare both records side by side before deciding.",
    "review.gap.provenance":
      "This service records how each pair was detected but not a separate provenance value.",
    "review.loading": "Loading the review queue…",
    "review.empty": "No review items for this filter.",
    "review.filter.status": "Status",
    "review.filter.statusAll": "All",
    "review.filter.limit": "Page size",
    "review.filter.limitHint":
      "The service returns at most 500 items and offers no paging beyond this.",
    "review.status.pending": "Pending",
    "review.status.confirmed": "Confirmed",
    "review.status.rejected": "Rejected",
    "review.status.automerged": "Auto-merged",
    "review.board.title": "Board",
    "review.list.title": "Queue",
    "review.col.pair": "Pair",
    "review.col.score": "Score",
    "review.col.quality": "Quality",
    "review.col.method": "Detection method",
    "review.col.status": "Status",
    "review.col.actions": "Actions",
    "review.compare.open": "Compare",
    "review.compare.title": "Compare the pair",
    "review.compare.close": "Close",
    "review.compare.loading": "Loading both records…",
    "review.compare.field": "Field",
    "review.compare.a": "Record A",
    "review.compare.b": "Record B",
    "review.compare.none": "Not recorded",
    "review.compare.partial":
      "One record could not be loaded — it may have been merged away or deleted.",
    "review.field.score": "Match score",
    "review.field.quality": "Match quality",
    "review.field.method": "Detection method",
    "review.field.status": "Status",
    "review.breakdown.title": "Score breakdown",
    "review.breakdown.none": "No score breakdown was recorded for this pair.",
    "review.breakdown.component": "Component",
    "review.breakdown.weight": "Weight",
    "review.breakdown.score": "Score",
    "review.decide.confirm": "Confirm duplicate",
    "review.decide.reject": "Reject",
    "review.decide.deciding": "Saving…",
    "review.decide.locked":
      "Already decided — only pending items can be decided.",
    "review.merge.title": "Merge this pair",
    "review.merge.note":
      "Confirming records the verdict only; it does not merge. Choose which record survives.",
    "review.merge.keepA": "Keep A, merge B into it",
    "review.merge.keepB": "Keep B, merge A into it",
    // Layout / chrome
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Dashboard",
    "nav.things": "Things",
    "nav.newThing": "New thing",
    "nav.matchCheck": "Match check",
    "nav.merge": "Merge",
    "nav.toggle": "Toggle navigation",
    "chrome.theme": "Theme",
    "chrome.language": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    // Dashboard
    "dashboard.title": "Dashboard",
    "dashboard.service": "Service:",
    "dashboard.recentActivity": "Recent activity",
    "dashboard.noRecent": "No recent audit entries.",
    // Things list
    "things.title": "Things",
    "things.new": "New thing",
    "things.searchPlaceholder": "Search by name, identifier…",
    "things.fuzzy": "Fuzzy",
    "things.phonetic": "Phonetic (Soundex)",
    "things.maskSensitive": "Mask sensitive",
    "things.previousPage": "Previous",
    "things.nextPage": "Next",
    "things.pageRange": "{from}–{to} of {total}",
    "things.loading": "Loading…",
    "things.recordCount": "{count} record",
    "things.recordCountPlural": "{count} records",
    // Search box
    "search.action": "Search",
    // Grid headers
    "grid.id": "ID",
    "grid.name": "Name",
    "grid.type": "Type (schema.org)",
    "grid.primaryId": "Primary identifier",
    "grid.url": "URL",
    // Thing detail
    "detail.loading": "Loading…",
    "detail.edit": "Edit",
    "detail.audit": "Audit",
    "detail.delete": "Delete",
    "detail.exportGdpr": "Export data (GDPR)",
    "detail.exportingGdpr": "Exporting…",
    "detail.showMasked": "Show masked",
    "detail.showFull": "Show full",
    "detail.maskedNotice":
      "Showing the masked view — some fields are redacted.",
    "detail.confirmDelete":
      "Soft-delete this thing? This cannot be undone via the UI.",
    "detail.identity": "Identity",
    "detail.id": "ID",
    "detail.additionalType": "Additional type",
    "detail.description": "Description",
    "detail.disambiguating": "Disambiguating",
    "detail.url": "URL",
    "detail.owner": "Owner",
    "detail.mainEntityOfPage": "Main entity of page",
    "detail.identifiers": "Identifiers",
    "detail.alternateNames": "Alternate names",
    "detail.sameAs": "Same-as (authoritative URLs)",
    "detail.images": "Images",
    "detail.customPrefix": "Custom: ",
    // Audit log
    "audit.title": "Audit log",
    "audit.backToThing": "Back to thing",
    "audit.loading": "Loading…",
    "audit.none": "No audit entries.",
    "audit.by": "by",
    "audit.payload": "Payload",
    // Edit
    "edit.title": "Edit thing",
    "edit.cancel": "Cancel",
    "edit.loading": "Loading…",
    "edit.submitLabel": "Save changes",
    // New
    "new.title": "New thing",
    "new.submitLabel": "Create",
    "new.possibleDuplicates": "Possible duplicates",
    "new.duplicatesDetected":
      "Duplicates detected ({count}) — review below before resubmitting.",
    // Match check
    "match.title": "Match check",
    "match.name": "Name",
    "match.threshold": "Threshold",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "Description",
    "match.url": "URL",
    "match.sameAs": "Same-as URLs",
    "match.sameAsHint": "One per line",
    "match.identifiers": "Identifiers",
    "match.matching": "Matching…",
    "match.findMatches": "Find matches",
    // Merge
    "merge.title": "Merge things",
    "merge.mainId": "Main thing ID",
    "merge.mainIdHint": "The surviving record",
    "merge.dupId": "Duplicate thing ID",
    "merge.dupIdHint": "Will be soft-deleted",
    "merge.reason": "Reason",
    "merge.reasonHint": "Recorded in the merge audit trail",
    "merge.reasonPlaceholder": "Confirmed duplicate",
    "merge.loadPreview": "Load preview",
    "merge.merging": "Merging…",
    "merge.merge": "Merge",
    "merge.preview": "Preview",
    "merge.main": "Main",
    "merge.duplicate": "Duplicate",
    "merge.completed": "Merge completed",
    "merge.recordCreated": "Merge record {id} created at {at}.",
    "merge.viewMain": "View merged main thing",
    "merge.confirm":
      "Merge {dup}… into {main}…?\nThis soft-deletes the duplicate.",
    // Match results
    "results.title": "Match results",
    "results.noCandidates": "No candidates.",
    "results.scoreBreakdown": "Score breakdown",
    "results.nameScore": "name",
    "results.identifierScore": "identifier",
    "results.descriptionScore": "description",
    "results.urlScore": "URL",
    "results.sameAsScore": "same-as",
    "results.phoneticMatch": "phonetic match",
    "results.deterministicMatch": "deterministic (DOI/ISBN/…)",
    // Thing form
    "form.name": "Name",
    "form.additionalType": "Additional type",
    "form.additionalTypeHint": "schema.org subtype URL",
    "form.description": "Description",
    "form.disambiguating": "Disambiguating description",
    "form.disambiguatingHint": "Short distinguishing detail",
    "form.url": "URL",
    "form.owner": "Owner",
    "form.alternateNames": "Alternate names",
    "form.alternateNamesHint": "One per line",
    "form.sameAs": "Same-as URLs",
    "form.sameAsHint": "Wikidata, Wikipedia, etc. — one per line",
    "form.identifiers": "Identifiers",
    "form.saving": "Saving…",
    "form.reset": "Reset",
    // Identifier input
    "identifier.type": "Type",
    "identifier.customOption": "Custom…",
    "identifier.customLabel": "Custom label",
    "identifier.value": "Value",
    "identifier.url": "URL",
    "identifier.remove": "Delete",
    "identifier.add": "+ Add identifier",
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
    "splash.hero.title": "One trusted record for every thing",
    "splash.hero.subtitle":
      "Register products, assets and works once, find them instantly, and stop duplicates before they start, with a full audit trail built in.",
    "splash.benefits.1.title": "Fewer duplicates",
    "splash.benefits.1.body":
      "Real-time matching flags a likely duplicate the moment a thing is created.",
    "splash.benefits.2.title": "Find things fast",
    "splash.benefits.2.body":
      "Fuzzy and phonetic search finds things despite typos and spelling variants.",
    "splash.benefits.3.title": "One record, many IDs",
    "splash.benefits.3.body":
      "DOI, ISBN, GTIN and other identifiers all live on a single record.",
    "splash.benefits.4.title": "Clean merges",
    "splash.benefits.4.body":
      "Fold a confirmed duplicate into one surviving record without losing what was known.",
    "splash.benefits.5.title": "Reviewed decisions",
    "splash.benefits.5.body":
      "Candidate duplicate pairs wait in a queue for a person to confirm or reject.",
    "splash.benefits.6.title": "A complete history",
    "splash.benefits.6.body":
      "Every change is recorded in an audit log you can read for any thing.",
    "splash.features.1.title": "Search and browse",
    "splash.features.1.body":
      "A sortable, filterable data grid lists every thing with full-text search on top.",
    "splash.features.2.title": "Rich records",
    "splash.features.2.body":
      "Keep identifiers, alternate names, same-as links and images on each thing.",
    "splash.features.3.title": "Match check",
    "splash.features.3.body":
      "Score a hypothetical record against the index before you create it.",
    "splash.features.4.title": "Side-by-side merge",
    "splash.features.4.body":
      "Pick a main record and a duplicate, then merge them in one step.",
    "splash.features.5.title": "Review board",
    "splash.features.5.body":
      "Drag pending pairs to confirm or reject them, or open one to compare both records.",
    "splash.features.6.title": "Private and accessible",
    "splash.features.6.body":
      "Mask sensitive fields, export data, and switch language, theme and text size.",
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
      "A guided walkthrough of the Thing registry: what each screen does and the steps to use it, from registering a product or asset to merging duplicates.",
    "tour.s1.title": "Register a thing",
    "tour.s1.summary":
      "Create a record for a product, asset or work with its identifiers and links. The form checks it and warns about likely duplicates before anything is created.",
    "tour.s1.step.1": "Open New thing from the menu.",
    "tour.s1.step.2":
      "Enter the Name, the only required field, then add the description, URL, owner and alternate names as you know them.",
    "tour.s1.step.3":
      "Add identifiers such as DOI, ISBN or GTIN with + Add identifier, and list same-as URLs one per line.",
    "tour.s1.step.4":
      "Choose Create. If Possible duplicates appear, review them before you resubmit; otherwise you land on the new thing's page.",
    "tour.s2.title": "Find a thing",
    "tour.s2.summary":
      "Search the whole index by name or identifier, then open any result from the data grid.",
    "tour.s2.step.1": "Open Things from the menu.",
    "tour.s2.step.2":
      "Type a name or identifier in the search box and choose Search.",
    "tour.s2.step.3":
      "Switch on Fuzzy or Phonetic (Soundex) to catch typos and spelling variants, or Mask sensitive to hide protected details.",
    "tour.s2.step.4":
      "Use Previous and Next to page through the results, and select a row to open that thing.",
    "tour.s3.title": "Check for matches",
    "tour.s3.summary":
      "Score a hypothetical record against the index before you create it, and see why each candidate matched.",
    "tour.s3.step.1": "Open Match check from the menu.",
    "tour.s3.step.2":
      "Fill in the Name, and optionally the description, URL, same-as URLs and identifiers.",
    "tour.s3.step.3":
      "Set the Threshold between 0.0 and 1.0, then choose Find matches.",
    "tour.s3.step.4":
      "Read the Match results: each candidate has a Score breakdown by name, identifier, description, URL and same-as, and phonetic or deterministic matches are flagged.",
    "tour.s4.title": "Work the review queue",
    "tour.s4.summary":
      "Candidate duplicate pairs from the batch scan wait here for a person to confirm or reject.",
    "tour.s4.step.1":
      "Open Review from the menu and choose Run scan to look for candidate pairs.",
    "tour.s4.step.2":
      "Filter by Status and set the page size; pairs appear as cards on the Board and as rows in the Queue.",
    "tour.s4.step.3":
      "Choose Compare on a pair to see Record A and Record B side by side, with the Score breakdown.",
    "tour.s4.step.4":
      "Choose Confirm duplicate or Reject. Confirming records the verdict only; to merge, choose Keep A or Keep B.",
    "tour.s5.title": "Merge duplicates",
    "tour.s5.summary":
      "Fold a confirmed duplicate into one surviving record. The duplicate is soft-deleted and the merge is recorded.",
    "tour.s5.step.1": "Open Merge from the menu.",
    "tour.s5.step.2":
      "Enter the Main thing ID (the surviving record), the Duplicate thing ID and a Reason, which is recorded in the merge audit trail.",
    "tour.s5.step.3":
      "Choose Load preview to compare the Main and Duplicate records before anything changes.",
    "tour.s5.step.4":
      "Choose Merge and confirm. A Merge completed message follows, with a link to View merged main thing.",
    "tour.s6.title": "Inspect, protect and audit a record",
    "tour.s6.summary":
      "Every thing has a detail page for reading, editing, exporting and auditing it.",
    "tour.s6.step.1":
      "On Things, search and select a row to open that thing's detail page.",
    "tour.s6.step.2":
      "Use Show masked for the redacted view, or Show full to see every field.",
    "tour.s6.step.3":
      "Choose Edit to change it, Audit to read the log of every change and who made it, or Export data (GDPR) to download its data.",
    "tour.s6.step.4":
      "Choose Delete to soft-delete the thing after you confirm; this cannot be undone from the screen.",
    "signin.sso": "Sign in with SSO",
  },
  "es-001": {
    "auth.sessionExpired":
      "Tu sesión ha caducado. Redirigiendo para iniciar sesión…",
    "auth.accessDenied": "No tienes permiso para hacer eso.",
    "nav.review": "Revisión",
    "review.run": "Ejecutar análisis",
    "review.intro":
      "Pares duplicados candidatos del análisis por lotes. Arrastre una tarjeta pendiente o abra un par para comparar ambos registros uno al lado del otro antes de decidir.",
    "review.gap.provenance":
      "Este servicio no registra una procedencia independiente para cada par, solo cómo se detectó.",
    "review.loading": "Cargando la cola de revisión…",
    "review.empty": "No hay elementos de revisión para este filtro.",
    "review.filter.status": "Estado",
    "review.filter.statusAll": "Todos",
    "review.filter.limit": "Tamaño de página",
    "review.filter.limitHint":
      "El servicio devuelve como máximo 500 elementos y no ofrece paginación más allá de esto.",
    "review.status.pending": "Pendiente",
    "review.status.confirmed": "Confirmado",
    "review.status.rejected": "Rechazado",
    "review.status.automerged": "Fusionado automáticamente",
    "review.board.title": "Tablero",
    "review.list.title": "Cola",
    "review.col.pair": "Par",
    "review.col.score": "Puntuación",
    "review.col.quality": "Calidad",
    "review.col.method": "Método de detección",
    "review.col.status": "Estado",
    "review.col.actions": "Acciones",
    "review.compare.open": "Comparar",
    "review.compare.title": "Comparar el par",
    "review.compare.close": "Cerrar",
    "review.compare.loading": "Cargando ambos registros…",
    "review.compare.field": "Campo",
    "review.compare.a": "Registro A",
    "review.compare.b": "Registro B",
    "review.compare.none": "No registrado",
    "review.compare.partial":
      "No se pudo cargar un registro — puede haber sido fusionado o eliminado.",
    "review.field.score": "Puntuación de coincidencia",
    "review.field.quality": "Calidad de coincidencia",
    "review.field.method": "Método de detección",
    "review.field.status": "Estado",
    "review.breakdown.title": "Desglose de puntuación",
    "review.breakdown.none":
      "No se registró un desglose de puntuación para este par.",
    "review.breakdown.component": "Componente",
    "review.breakdown.weight": "Peso",
    "review.breakdown.score": "Puntuación",
    "review.decide.confirm": "Confirmar duplicado",
    "review.decide.reject": "Rechazar",
    "review.decide.deciding": "Guardando…",
    "review.decide.locked":
      "Ya decidido — solo los elementos pendientes se pueden decidir.",
    "review.merge.title": "Fusionar este par",
    "review.merge.note":
      "Confirmar solo registra el veredicto; no fusiona. Elija qué registro sobrevive.",
    "review.merge.keepA": "Conservar A, fusionar B en él",
    "review.merge.keepB": "Conservar B, fusionar A en él",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Panel",
    "nav.things": "Cosas",
    "nav.newThing": "Nueva cosa",
    "nav.matchCheck": "Comprobar coincidencias",
    "nav.merge": "Fusionar",
    "nav.toggle": "Alternar navegación",
    "chrome.theme": "Tema",
    "chrome.language": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "dashboard.title": "Panel",
    "dashboard.service": "Servicio:",
    "dashboard.recentActivity": "Actividad reciente",
    "dashboard.noRecent": "No hay entradas de auditoría recientes.",
    "things.title": "Cosas",
    "things.new": "Nueva cosa",
    "things.searchPlaceholder": "Buscar por nombre, identificador…",
    "things.fuzzy": "Difusa",
    "things.phonetic": "Fonética (Soundex)",
    "things.maskSensitive": "Ocultar sensible",
    "things.previousPage": "Anterior",
    "things.nextPage": "Siguiente",
    "things.pageRange": "{from}–{to} de {total}",
    "things.loading": "Cargando…",
    "things.recordCount": "{count} registro",
    "things.recordCountPlural": "{count} registros",
    "search.action": "Buscar",
    "grid.id": "ID",
    "grid.name": "Nombre",
    "grid.type": "Tipo (schema.org)",
    "grid.primaryId": "Identificador principal",
    "grid.url": "URL",
    "detail.loading": "Cargando…",
    "detail.edit": "Editar",
    "detail.audit": "Auditoría",
    "detail.delete": "Eliminar",
    "detail.exportGdpr": "Exportar datos (RGPD)",
    "detail.exportingGdpr": "Exportando…",
    "detail.showMasked": "Mostrar enmascarado",
    "detail.showFull": "Mostrar completo",
    "detail.maskedNotice":
      "Mostrando la vista enmascarada: algunos campos están ocultos.",
    "detail.confirmDelete":
      "¿Eliminar de forma reversible esta cosa? No se puede deshacer desde la interfaz.",
    "detail.identity": "Identidad",
    "detail.id": "ID",
    "detail.additionalType": "Tipo adicional",
    "detail.description": "Descripción",
    "detail.disambiguating": "Desambiguación",
    "detail.url": "URL",
    "detail.owner": "Propietario",
    "detail.mainEntityOfPage": "Entidad principal de la página",
    "detail.identifiers": "Identificadores",
    "detail.alternateNames": "Nombres alternativos",
    "detail.sameAs": "Igual que (URL autorizadas)",
    "detail.images": "Imágenes",
    "detail.customPrefix": "Personalizado: ",
    "audit.title": "Registro de auditoría",
    "audit.backToThing": "Volver a la cosa",
    "audit.loading": "Cargando…",
    "audit.none": "No hay entradas de auditoría.",
    "audit.by": "por",
    "audit.payload": "Carga útil",
    "edit.title": "Editar cosa",
    "edit.cancel": "Cancelar",
    "edit.loading": "Cargando…",
    "edit.submitLabel": "Guardar cambios",
    "new.title": "Nueva cosa",
    "new.submitLabel": "Crear",
    "new.possibleDuplicates": "Posibles duplicados",
    "new.duplicatesDetected":
      "Se detectaron duplicados ({count}) — revise abajo antes de reenviar.",
    "match.title": "Comprobar coincidencias",
    "match.name": "Nombre",
    "match.threshold": "Umbral",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "Descripción",
    "match.url": "URL",
    "match.sameAs": "URL «igual que»",
    "match.sameAsHint": "Una por línea",
    "match.identifiers": "Identificadores",
    "match.matching": "Coincidiendo…",
    "match.findMatches": "Buscar coincidencias",
    "merge.title": "Fusionar cosas",
    "merge.mainId": "ID de la cosa principal",
    "merge.mainIdHint": "El registro que sobrevive",
    "merge.dupId": "ID de la cosa duplicada",
    "merge.dupIdHint": "Se eliminará de forma reversible",
    "merge.reason": "Motivo",
    "merge.reasonHint": "Registrado en el rastro de auditoría de la fusión",
    "merge.reasonPlaceholder": "Duplicado confirmado",
    "merge.loadPreview": "Cargar vista previa",
    "merge.merging": "Fusionando…",
    "merge.merge": "Fusionar",
    "merge.preview": "Vista previa",
    "merge.main": "Principal",
    "merge.duplicate": "Duplicado",
    "merge.completed": "Fusión completada",
    "merge.recordCreated": "Registro de fusión {id} creado el {at}.",
    "merge.viewMain": "Ver la cosa principal fusionada",
    "merge.confirm":
      "¿Fusionar {dup}… en {main}…?\nEsto elimina de forma reversible el duplicado.",
    "results.title": "Resultados de coincidencia",
    "results.noCandidates": "Sin candidatos.",
    "results.scoreBreakdown": "Desglose de puntuación",
    "results.nameScore": "nombre",
    "results.identifierScore": "identificador",
    "results.descriptionScore": "descripción",
    "results.urlScore": "URL",
    "results.sameAsScore": "igual que",
    "results.phoneticMatch": "coincidencia fonética",
    "results.deterministicMatch": "determinista (DOI/ISBN/…)",
    "form.name": "Nombre",
    "form.additionalType": "Tipo adicional",
    "form.additionalTypeHint": "URL de subtipo de schema.org",
    "form.description": "Descripción",
    "form.disambiguating": "Descripción de desambiguación",
    "form.disambiguatingHint": "Breve detalle distintivo",
    "form.url": "URL",
    "form.owner": "Propietario",
    "form.alternateNames": "Nombres alternativos",
    "form.alternateNamesHint": "Uno por línea",
    "form.sameAs": "URL «igual que»",
    "form.sameAsHint": "Wikidata, Wikipedia, etc. — una por línea",
    "form.identifiers": "Identificadores",
    "form.saving": "Guardando…",
    "form.reset": "Restablecer",
    "identifier.type": "Tipo",
    "identifier.customOption": "Personalizado…",
    "identifier.customLabel": "Etiqueta personalizada",
    "identifier.value": "Valor",
    "identifier.url": "URL",
    "identifier.remove": "Eliminar",
    "identifier.add": "+ Añadir identificador",
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
    "splash.hero.title": "Un registro fiable para cada cosa",
    "splash.hero.subtitle":
      "Registra productos, activos y obras una sola vez, encuéntralos al instante y evita los duplicados antes de que aparezcan, con auditoría completa integrada.",
    "splash.benefits.1.title": "Menos duplicados",
    "splash.benefits.1.body":
      "La coincidencia en tiempo real señala un posible duplicado en el momento de crear la cosa.",
    "splash.benefits.2.title": "Encuentra cosas, rápido",
    "splash.benefits.2.body":
      "La búsqueda difusa y fonética encuentra cosas pese a errores tipográficos y variantes de ortografía.",
    "splash.benefits.3.title": "Un registro, muchos ID",
    "splash.benefits.3.body":
      "DOI, ISBN, GTIN y otros identificadores conviven en un único registro.",
    "splash.benefits.4.title": "Fusiones limpias",
    "splash.benefits.4.body":
      "Integra un duplicado confirmado en un único registro superviviente sin perder lo que ya se sabía.",
    "splash.benefits.5.title": "Decisiones revisadas",
    "splash.benefits.5.body":
      "Los pares de posibles duplicados esperan en una cola a que una persona los confirme o rechace.",
    "splash.benefits.6.title": "Un historial completo",
    "splash.benefits.6.body":
      "Cada cambio queda registrado en un registro de auditoría que puedes consultar para cualquier cosa.",
    "splash.features.1.title": "Buscar y explorar",
    "splash.features.1.body":
      "Una cuadrícula ordenable y filtrable lista cada cosa, con búsqueda de texto completo.",
    "splash.features.2.title": "Registros completos",
    "splash.features.2.body":
      "Guarda identificadores, nombres alternativos, enlaces «igual que» e imágenes en cada cosa.",
    "splash.features.3.title": "Comprobar coincidencias",
    "splash.features.3.body":
      "Puntúa un registro hipotético frente al índice antes de crearlo.",
    "splash.features.4.title": "Fusión lado a lado",
    "splash.features.4.body":
      "Elige un registro principal y un duplicado, y fusiónalos en un solo paso.",
    "splash.features.5.title": "Tablero de revisión",
    "splash.features.5.body":
      "Arrastra los pares pendientes para confirmarlos o rechazarlos, o abre uno para comparar ambos registros.",
    "splash.features.6.title": "Privado y accesible",
    "splash.features.6.body":
      "Oculta campos sensibles, exporta datos y cambia el idioma, el tema y el tamaño del texto.",
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
      "Un recorrido guiado por el registro de cosas: qué hace cada pantalla y los pasos para usarla, desde registrar un producto o activo hasta fusionar duplicados.",
    "tour.s1.title": "Registrar una cosa",
    "tour.s1.summary":
      "Crea un registro para un producto, activo u obra con sus identificadores y enlaces. El formulario lo comprueba y avisa de posibles duplicados antes de crear nada.",
    "tour.s1.step.1": "Abre «Nueva cosa» en el menú.",
    "tour.s1.step.2":
      "Escribe el Nombre, el único campo obligatorio, y añade la descripción, la URL, el propietario y los nombres alternativos que conozcas.",
    "tour.s1.step.3":
      "Añade identificadores como DOI, ISBN o GTIN con + Añadir identificador, y escribe las URL «igual que» una por línea.",
    "tour.s1.step.4":
      "Elige Crear. Si aparecen Posibles duplicados, revísalos antes de reenviar; si no, llegas a la página de la nueva cosa.",
    "tour.s2.title": "Encontrar una cosa",
    "tour.s2.summary":
      "Busca en todo el índice por nombre o identificador y abre cualquier resultado desde la cuadrícula de datos.",
    "tour.s2.step.1": "Abre «Cosas» en el menú.",
    "tour.s2.step.2":
      "Escribe un nombre o identificador en el cuadro de búsqueda y elige Buscar.",
    "tour.s2.step.3":
      "Activa Difusa o Fonética (Soundex) para captar errores y variantes de ortografía, u Ocultar sensible para ocultar detalles protegidos.",
    "tour.s2.step.4":
      "Usa Anterior y Siguiente para recorrer los resultados y selecciona una fila para abrir esa cosa.",
    "tour.s3.title": "Comprobar coincidencias",
    "tour.s3.summary":
      "Puntúa un registro hipotético frente al índice antes de crearlo y consulta por qué coincidió cada candidato.",
    "tour.s3.step.1": "Abre «Comprobar coincidencias» en el menú.",
    "tour.s3.step.2":
      "Rellena el Nombre y, si quieres, la descripción, la URL, las URL «igual que» y los identificadores.",
    "tour.s3.step.3":
      "Fija el Umbral entre 0.0 y 1.0 y elige Buscar coincidencias.",
    "tour.s3.step.4":
      "Lee los Resultados de coincidencia: cada candidato tiene un Desglose de puntuación por nombre, identificador, descripción, URL y «igual que», y se marcan las coincidencias fonéticas o deterministas.",
    "tour.s4.title": "Trabajar la cola de revisión",
    "tour.s4.summary":
      "Los pares de posibles duplicados del análisis por lotes esperan aquí a que una persona los confirme o rechace.",
    "tour.s4.step.1":
      "Abre «Revisión» en el menú y elige Ejecutar análisis para buscar pares candidatos.",
    "tour.s4.step.2":
      "Filtra por Estado y fija el tamaño de página; los pares aparecen como tarjetas en el Tablero y como filas en la Cola.",
    "tour.s4.step.3":
      "Elige Comparar en un par para ver el Registro A y el Registro B lado a lado, con el Desglose de puntuación.",
    "tour.s4.step.4":
      "Elige Confirmar duplicado o Rechazar. Confirmar solo registra el veredicto; para fusionar, elige Conservar A o Conservar B.",
    "tour.s5.title": "Fusionar duplicados",
    "tour.s5.summary":
      "Integra un duplicado confirmado en un único registro superviviente. El duplicado se elimina de forma reversible y la fusión queda registrada.",
    "tour.s5.step.1": "Abre «Fusionar» en el menú.",
    "tour.s5.step.2":
      "Introduce el ID de la cosa principal (el registro superviviente), el ID de la cosa duplicada y un Motivo, que se registra en el rastro de auditoría de la fusión.",
    "tour.s5.step.3":
      "Elige Cargar vista previa para comparar los registros Principal y Duplicado antes de que cambie nada.",
    "tour.s5.step.4":
      "Elige Fusionar y confirma. Aparece el mensaje Fusión completada, con un enlace a Ver la cosa principal fusionada.",
    "tour.s6.title": "Inspeccionar, proteger y auditar un registro",
    "tour.s6.summary":
      "Cada cosa tiene una página de detalle para leerla, editarla, exportarla y auditarla.",
    "tour.s6.step.1":
      "En «Cosas», busca y selecciona una fila para abrir la página de detalle de esa cosa.",
    "tour.s6.step.2":
      "Usa Mostrar enmascarado para la vista censurada, o Mostrar completo para ver todos los campos.",
    "tour.s6.step.3":
      "Elige Editar para cambiarla, Auditoría para leer el registro de cada cambio y quién lo hizo, o Exportar datos (RGPD) para descargar sus datos.",
    "tour.s6.step.4":
      "Elige Eliminar para eliminar la cosa de forma reversible tras confirmar; no se puede deshacer desde la pantalla.",
    "signin.sso": "Iniciar sesión con SSO",
  },
  "fr-001": {
    "auth.sessionExpired":
      "Votre session a expiré. Redirection vers la connexion…",
    "auth.accessDenied": "Vous n'avez pas la permission de faire cela.",
    "nav.review": "Révision",
    "review.run": "Lancer l'analyse",
    "review.intro":
      "Paires en double candidates issues de l'analyse par lot. Faites glisser une carte en attente, ou ouvrez une paire pour comparer les deux enregistrements côte à côte avant de décider.",
    "review.gap.provenance":
      "Ce service n'enregistre pas de provenance distincte pour chaque paire, seulement la méthode de détection.",
    "review.loading": "Chargement de la file d'attente de révision…",
    "review.empty": "Aucun élément à réviser pour ce filtre.",
    "review.filter.status": "Statut",
    "review.filter.statusAll": "Tous",
    "review.filter.limit": "Taille de page",
    "review.filter.limitHint":
      "Le service renvoie au plus 500 éléments et n'offre pas de pagination au-delà.",
    "review.status.pending": "En attente",
    "review.status.confirmed": "Confirmé",
    "review.status.rejected": "Rejeté",
    "review.status.automerged": "Fusionné automatiquement",
    "review.board.title": "Tableau",
    "review.list.title": "File d'attente",
    "review.col.pair": "Paire",
    "review.col.score": "Score",
    "review.col.quality": "Qualité",
    "review.col.method": "Méthode de détection",
    "review.col.status": "Statut",
    "review.col.actions": "Actions",
    "review.compare.open": "Comparer",
    "review.compare.title": "Comparer la paire",
    "review.compare.close": "Fermer",
    "review.compare.loading": "Chargement des deux enregistrements…",
    "review.compare.field": "Champ",
    "review.compare.a": "Enregistrement A",
    "review.compare.b": "Enregistrement B",
    "review.compare.none": "Non renseigné",
    "review.compare.partial":
      "Un enregistrement n'a pas pu être chargé — il a peut-être été fusionné ou supprimé.",
    "review.field.score": "Score de correspondance",
    "review.field.quality": "Qualité de correspondance",
    "review.field.method": "Méthode de détection",
    "review.field.status": "Statut",
    "review.breakdown.title": "Détail du score",
    "review.breakdown.none":
      "Aucun détail de score n'a été enregistré pour cette paire.",
    "review.breakdown.component": "Composant",
    "review.breakdown.weight": "Poids",
    "review.breakdown.score": "Score",
    "review.decide.confirm": "Confirmer le doublon",
    "review.decide.reject": "Rejeter",
    "review.decide.deciding": "Enregistrement…",
    "review.decide.locked":
      "Déjà décidé — seuls les éléments en attente peuvent être décidés.",
    "review.merge.title": "Fusionner cette paire",
    "review.merge.note":
      "Confirmer n'enregistre que le verdict ; cela ne fusionne pas. Choisissez quel enregistrement survit.",
    "review.merge.keepA": "Conserver A, y fusionner B",
    "review.merge.keepB": "Conserver B, y fusionner A",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Tableau de bord",
    "nav.things": "Choses",
    "nav.newThing": "Nouvelle chose",
    "nav.matchCheck": "Vérifier les correspondances",
    "nav.merge": "Fusionner",
    "nav.toggle": "Basculer la navigation",
    "chrome.theme": "Thème",
    "chrome.language": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "dashboard.title": "Tableau de bord",
    "dashboard.service": "Service :",
    "dashboard.recentActivity": "Activité récente",
    "dashboard.noRecent": "Aucune entrée d'audit récente.",
    "things.title": "Choses",
    "things.new": "Nouvelle chose",
    "things.searchPlaceholder": "Rechercher par nom, identifiant…",
    "things.fuzzy": "Floue",
    "things.phonetic": "Phonétique (Soundex)",
    "things.maskSensitive": "Masquer sensible",
    "things.previousPage": "Précédent",
    "things.nextPage": "Suivant",
    "things.pageRange": "{from}–{to} sur {total}",
    "things.loading": "Chargement…",
    "things.recordCount": "{count} enregistrement",
    "things.recordCountPlural": "{count} enregistrements",
    "search.action": "Rechercher",
    "grid.id": "ID",
    "grid.name": "Nom",
    "grid.type": "Type (schema.org)",
    "grid.primaryId": "Identifiant principal",
    "grid.url": "URL",
    "detail.loading": "Chargement…",
    "detail.edit": "Modifier",
    "detail.audit": "Audit",
    "detail.delete": "Supprimer",
    "detail.exportGdpr": "Exporter les données (RGPD)",
    "detail.exportingGdpr": "Exportation…",
    "detail.showMasked": "Afficher masqué",
    "detail.showFull": "Afficher complet",
    "detail.maskedNotice":
      "Affichage de la vue masquée — certains champs sont masqués.",
    "detail.confirmDelete":
      "Supprimer logiquement cette chose ? Action irréversible via l'interface.",
    "detail.identity": "Identité",
    "detail.id": "ID",
    "detail.additionalType": "Type supplémentaire",
    "detail.description": "Description",
    "detail.disambiguating": "Désambiguïsation",
    "detail.url": "URL",
    "detail.owner": "Propriétaire",
    "detail.mainEntityOfPage": "Entité principale de la page",
    "detail.identifiers": "Identifiants",
    "detail.alternateNames": "Noms alternatifs",
    "detail.sameAs": "Identique à (URL faisant autorité)",
    "detail.images": "Images",
    "detail.customPrefix": "Personnalisé : ",
    "audit.title": "Journal d'audit",
    "audit.backToThing": "Retour à la chose",
    "audit.loading": "Chargement…",
    "audit.none": "Aucune entrée d'audit.",
    "audit.by": "par",
    "audit.payload": "Charge utile",
    "edit.title": "Modifier la chose",
    "edit.cancel": "Annuler",
    "edit.loading": "Chargement…",
    "edit.submitLabel": "Enregistrer les modifications",
    "new.title": "Nouvelle chose",
    "new.submitLabel": "Créer",
    "new.possibleDuplicates": "Doublons possibles",
    "new.duplicatesDetected":
      "Doublons détectés ({count}) — vérifiez ci-dessous avant de renvoyer.",
    "match.title": "Vérifier les correspondances",
    "match.name": "Nom",
    "match.threshold": "Seuil",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "Description",
    "match.url": "URL",
    "match.sameAs": "URL « identique à »",
    "match.sameAsHint": "Une par ligne",
    "match.identifiers": "Identifiants",
    "match.matching": "Correspondance…",
    "match.findMatches": "Trouver des correspondances",
    "merge.title": "Fusionner des choses",
    "merge.mainId": "ID de la chose principale",
    "merge.mainIdHint": "L'enregistrement conservé",
    "merge.dupId": "ID de la chose en double",
    "merge.dupIdHint": "Sera supprimé logiquement",
    "merge.reason": "Motif",
    "merge.reasonHint": "Consigné dans le journal d'audit de la fusion",
    "merge.reasonPlaceholder": "Doublon confirmé",
    "merge.loadPreview": "Charger l'aperçu",
    "merge.merging": "Fusion…",
    "merge.merge": "Fusionner",
    "merge.preview": "Aperçu",
    "merge.main": "Principal",
    "merge.duplicate": "Doublon",
    "merge.completed": "Fusion terminée",
    "merge.recordCreated": "Enregistrement de fusion {id} créé le {at}.",
    "merge.viewMain": "Voir la chose principale fusionnée",
    "merge.confirm":
      "Fusionner {dup}… dans {main}… ?\nCela supprime logiquement le doublon.",
    "results.title": "Résultats de correspondance",
    "results.noCandidates": "Aucun candidat.",
    "results.scoreBreakdown": "Détail du score",
    "results.nameScore": "nom",
    "results.identifierScore": "identifiant",
    "results.descriptionScore": "description",
    "results.urlScore": "URL",
    "results.sameAsScore": "identique à",
    "results.phoneticMatch": "correspondance phonétique",
    "results.deterministicMatch": "déterministe (DOI/ISBN/…)",
    "form.name": "Nom",
    "form.additionalType": "Type supplémentaire",
    "form.additionalTypeHint": "URL de sous-type schema.org",
    "form.description": "Description",
    "form.disambiguating": "Description de désambiguïsation",
    "form.disambiguatingHint": "Bref détail distinctif",
    "form.url": "URL",
    "form.owner": "Propriétaire",
    "form.alternateNames": "Noms alternatifs",
    "form.alternateNamesHint": "Un par ligne",
    "form.sameAs": "URL « identique à »",
    "form.sameAsHint": "Wikidata, Wikipedia, etc. — une par ligne",
    "form.identifiers": "Identifiants",
    "form.saving": "Enregistrement…",
    "form.reset": "Réinitialiser",
    "identifier.type": "Type",
    "identifier.customOption": "Personnalisé…",
    "identifier.customLabel": "Étiquette personnalisée",
    "identifier.value": "Valeur",
    "identifier.url": "URL",
    "identifier.remove": "Supprimer",
    "identifier.add": "+ Ajouter un identifiant",
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
    "splash.hero.title": "Un dossier fiable pour chaque chose",
    "splash.hero.subtitle":
      "Enregistrez produits, actifs et œuvres une seule fois, retrouvez-les instantanément et évitez les doublons avant qu'ils n'apparaissent, avec piste d'audit intégrée.",
    "splash.benefits.1.title": "Moins de doublons",
    "splash.benefits.1.body":
      "Le rapprochement en temps réel signale un doublon probable dès la création de la chose.",
    "splash.benefits.2.title": "Retrouvez vite les choses",
    "splash.benefits.2.body":
      "La recherche floue et phonétique retrouve les choses malgré les fautes de frappe et les variantes d'orthographe.",
    "splash.benefits.3.title": "Une fiche, plusieurs identifiants",
    "splash.benefits.3.body":
      "DOI, ISBN, GTIN et autres identifiants sont réunis sur une seule fiche.",
    "splash.benefits.4.title": "Fusions propres",
    "splash.benefits.4.body":
      "Fusionnez un doublon confirmé dans une seule fiche conservée sans perdre ce qui était connu.",
    "splash.benefits.5.title": "Décisions vérifiées",
    "splash.benefits.5.body":
      "Les paires de doublons candidats attendent dans une file qu'une personne les confirme ou les rejette.",
    "splash.benefits.6.title": "Un historique complet",
    "splash.benefits.6.body":
      "Chaque modification est consignée dans un journal d'audit consultable pour n'importe quelle chose.",
    "splash.features.1.title": "Rechercher et parcourir",
    "splash.features.1.body":
      "Une grille triable et filtrable liste chaque chose, avec recherche en texte intégral.",
    "splash.features.2.title": "Fiches détaillées",
    "splash.features.2.body":
      "Conservez identifiants, autres noms, liens « identique à » et images sur chaque chose.",
    "splash.features.3.title": "Vérifier les correspondances",
    "splash.features.3.body":
      "Évaluez une fiche hypothétique par rapport à l'index avant de la créer.",
    "splash.features.4.title": "Fusion côte à côte",
    "splash.features.4.body":
      "Choisissez une fiche principale et un doublon, puis fusionnez-les en une seule étape.",
    "splash.features.5.title": "Tableau de révision",
    "splash.features.5.body":
      "Faites glisser les paires en attente pour les confirmer ou les rejeter, ou ouvrez-en une pour comparer les deux fiches.",
    "splash.features.6.title": "Privé et accessible",
    "splash.features.6.body":
      "Masquez les champs sensibles, exportez les données et changez de langue, de thème et de taille de texte.",
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
      "Une visite guidée du registre des choses : le rôle de chaque écran et les étapes pour l'utiliser, de l'enregistrement d'un produit ou d'un actif à la fusion des doublons.",
    "tour.s1.title": "Enregistrer une chose",
    "tour.s1.summary":
      "Créez une fiche pour un produit, un actif ou une œuvre avec ses identifiants et ses liens. Le formulaire la vérifie et signale les doublons probables avant toute création.",
    "tour.s1.step.1": "Ouvrez « Nouvelle chose » dans le menu.",
    "tour.s1.step.2":
      "Saisissez le Nom, seul champ obligatoire, puis ajoutez la description, l'URL, le propriétaire et les noms alternatifs que vous connaissez.",
    "tour.s1.step.3":
      "Ajoutez des identifiants comme DOI, ISBN ou GTIN avec + Ajouter un identifiant, et listez les URL « same-as » une par ligne.",
    "tour.s1.step.4":
      "Choisissez Créer. Si des Doublons possibles apparaissent, examinez-les avant de renvoyer ; sinon vous arrivez sur la page de la nouvelle chose.",
    "tour.s2.title": "Trouver une chose",
    "tour.s2.summary":
      "Recherchez dans tout l'index par nom ou identifiant, puis ouvrez n'importe quel résultat depuis la grille de données.",
    "tour.s2.step.1": "Ouvrez « Choses » dans le menu.",
    "tour.s2.step.2":
      "Saisissez un nom ou un identifiant dans la zone de recherche et choisissez Rechercher.",
    "tour.s2.step.3":
      "Activez Floue ou Phonétique (Soundex) pour rattraper les fautes et variantes d'orthographe, ou Masquer sensible pour cacher les détails protégés.",
    "tour.s2.step.4":
      "Utilisez Précédent et Suivant pour parcourir les résultats, et sélectionnez une ligne pour ouvrir cette chose.",
    "tour.s3.title": "Vérifier les correspondances",
    "tour.s3.summary":
      "Évaluez une fiche hypothétique par rapport à l'index avant de la créer, et voyez pourquoi chaque candidat correspond.",
    "tour.s3.step.1": "Ouvrez « Vérifier les correspondances » dans le menu.",
    "tour.s3.step.2":
      "Renseignez le Nom, et si vous le souhaitez la description, l'URL, les URL « same-as » et les identifiants.",
    "tour.s3.step.3":
      "Réglez le Seuil entre 0.0 et 1.0, puis choisissez Trouver des correspondances.",
    "tour.s3.step.4":
      "Lisez les Résultats de correspondance : chaque candidat a un Détail du score par nom, identifiant, description, URL et « same-as », et les correspondances phonétiques ou déterministes sont signalées.",
    "tour.s4.title": "Traiter la file de révision",
    "tour.s4.summary":
      "Les paires de doublons candidats issues de l'analyse par lots attendent ici qu'une personne les confirme ou les rejette.",
    "tour.s4.step.1":
      "Ouvrez « Révision » dans le menu et choisissez Lancer l'analyse pour chercher des paires candidates.",
    "tour.s4.step.2":
      "Filtrez par Statut et réglez la taille de page ; les paires apparaissent en cartes sur le Tableau et en lignes dans la File d'attente.",
    "tour.s4.step.3":
      "Choisissez Comparer sur une paire pour voir l'Enregistrement A et l'Enregistrement B côte à côte, avec le Détail du score.",
    "tour.s4.step.4":
      "Choisissez Confirmer le doublon ou Rejeter. Confirmer n'enregistre que le verdict ; pour fusionner, choisissez Conserver A ou Conserver B.",
    "tour.s5.title": "Fusionner des doublons",
    "tour.s5.summary":
      "Fondez un doublon confirmé dans un seul enregistrement conservé. Le doublon est supprimé logiquement et la fusion est consignée.",
    "tour.s5.step.1": "Ouvrez « Fusionner » dans le menu.",
    "tour.s5.step.2":
      "Saisissez l'ID de la chose principale (l'enregistrement conservé), l'ID de la chose en double et un Motif, consigné dans le journal d'audit de la fusion.",
    "tour.s5.step.3":
      "Choisissez Charger l'aperçu pour comparer les enregistrements Principal et En double avant que rien ne change.",
    "tour.s5.step.4":
      "Choisissez Fusionner et confirmez. Le message Fusion terminée s'affiche, avec un lien Voir la chose principale fusionnée.",
    "tour.s6.title": "Examiner, protéger et auditer une fiche",
    "tour.s6.summary":
      "Chaque chose a une page de détail pour la lire, la modifier, l'exporter et l'auditer.",
    "tour.s6.step.1":
      "Dans « Choses », recherchez et sélectionnez une ligne pour ouvrir la page de détail de cette chose.",
    "tour.s6.step.2":
      "Utilisez Afficher masqué pour la vue expurgée, ou Afficher complet pour voir tous les champs.",
    "tour.s6.step.3":
      "Choisissez Modifier pour la changer, Audit pour lire le journal de chaque modification et de son auteur, ou Exporter les données (RGPD) pour télécharger ses données.",
    "tour.s6.step.4":
      "Choisissez Supprimer pour supprimer logiquement la chose après confirmation ; on ne peut pas l'annuler depuis l'écran.",
    "signin.sso": "Se connecter avec SSO",
  },
  "hi-001": {
    "auth.sessionExpired":
      "आपका सत्र समाप्त हो गया है। साइन इन पर रीडायरेक्ट किया जा रहा है…",
    "auth.accessDenied": "आपको ऐसा करने की अनुमति नहीं है।",
    "nav.review": "समीक्षा",
    "review.run": "स्कैन चलाएं",
    "review.intro":
      "बैच स्कैन से संभावित डुप्लिकेट जोड़े। लंबित कार्ड को खींचें, या निर्णय लेने से पहले दोनों रिकॉर्ड की साथ-साथ तुलना करने के लिए एक जोड़ा खोलें।",
    "review.gap.provenance":
      "यह सेवा प्रत्येक जोड़े के लिए अलग उद्गम दर्ज नहीं करती — केवल पता लगाने की विधि दर्ज करती है।",
    "review.loading": "समीक्षा कतार लोड हो रही है…",
    "review.empty": "इस फ़िल्टर के लिए कोई समीक्षा आइटम नहीं है।",
    "review.filter.status": "स्थिति",
    "review.filter.statusAll": "सभी",
    "review.filter.limit": "पृष्ठ आकार",
    "review.filter.limitHint":
      "सेवा अधिकतम 500 आइटम लौटाती है और इससे आगे कोई पेजिंग प्रदान नहीं करती।",
    "review.status.pending": "लंबित",
    "review.status.confirmed": "पुष्टि की गई",
    "review.status.rejected": "अस्वीकृत",
    "review.status.automerged": "स्वचालित रूप से मर्ज किया गया",
    "review.board.title": "बोर्ड",
    "review.list.title": "कतार",
    "review.col.pair": "जोड़ा",
    "review.col.score": "स्कोर",
    "review.col.quality": "गुणवत्ता",
    "review.col.method": "पहचान विधि",
    "review.col.status": "स्थिति",
    "review.col.actions": "कार्रवाइयाँ",
    "review.compare.open": "तुलना करें",
    "review.compare.title": "जोड़े की तुलना करें",
    "review.compare.close": "बंद करें",
    "review.compare.loading": "दोनों रिकॉर्ड लोड हो रहे हैं…",
    "review.compare.field": "फ़ील्ड",
    "review.compare.a": "रिकॉर्ड A",
    "review.compare.b": "रिकॉर्ड B",
    "review.compare.none": "दर्ज नहीं",
    "review.compare.partial":
      "एक रिकॉर्ड लोड नहीं हो सका — इसे मर्ज या हटाया जा चुका हो सकता है।",
    "review.field.score": "मिलान स्कोर",
    "review.field.quality": "मिलान गुणवत्ता",
    "review.field.method": "पहचान विधि",
    "review.field.status": "स्थिति",
    "review.breakdown.title": "स्कोर विश्लेषण",
    "review.breakdown.none":
      "इस जोड़े के लिए कोई स्कोर विश्लेषण दर्ज नहीं किया गया।",
    "review.breakdown.component": "घटक",
    "review.breakdown.weight": "भार",
    "review.breakdown.score": "स्कोर",
    "review.decide.confirm": "डुप्लिकेट की पुष्टि करें",
    "review.decide.reject": "अस्वीकार करें",
    "review.decide.deciding": "सहेजा जा रहा है…",
    "review.decide.locked":
      "पहले से ही तय — केवल लंबित आइटम पर ही निर्णय लिया जा सकता है।",
    "review.merge.title": "इस जोड़े को मर्ज करें",
    "review.merge.note":
      "पुष्टि करने से केवल निर्णय दर्ज होता है; यह मर्ज नहीं करता। चुनें कि कौन सा रिकॉर्ड बना रहेगा।",
    "review.merge.keepA": "A रखें, B को इसमें मर्ज करें",
    "review.merge.keepB": "B रखें, A को इसमें मर्ज करें",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "डैशबोर्ड",
    "nav.things": "वस्तुएँ",
    "nav.newThing": "नई वस्तु",
    "nav.matchCheck": "मिलान जाँच",
    "nav.merge": "मर्ज करें",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "chrome.theme": "थीम",
    "chrome.language": "भाषा",
    "nav.share": "साझा करें",
    "nav.text_size": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "dashboard.title": "डैशबोर्ड",
    "dashboard.service": "सेवा:",
    "dashboard.recentActivity": "हाल की गतिविधि",
    "dashboard.noRecent": "कोई हाल की ऑडिट प्रविष्टियाँ नहीं।",
    "things.title": "वस्तुएँ",
    "things.new": "नई वस्तु",
    "things.searchPlaceholder": "नाम, पहचानकर्ता से खोजें…",
    "things.fuzzy": "अस्पष्ट",
    "things.phonetic": "ध्वन्यात्मक (Soundex)",
    "things.maskSensitive": "संवेदनशील छुपाएँ",
    "things.previousPage": "पिछला",
    "things.nextPage": "अगला",
    "things.pageRange": "{total} में से {from}–{to}",
    "things.loading": "लोड हो रहा है…",
    "things.recordCount": "{count} रिकॉर्ड",
    "things.recordCountPlural": "{count} रिकॉर्ड",
    "search.action": "खोज",
    "grid.id": "ID",
    "grid.name": "नाम",
    "grid.type": "प्रकार (schema.org)",
    "grid.primaryId": "प्राथमिक पहचानकर्ता",
    "grid.url": "URL",
    "detail.loading": "लोड हो रहा है…",
    "detail.edit": "संपादित करें",
    "detail.audit": "ऑडिट",
    "detail.delete": "हटाएँ",
    "detail.exportGdpr": "डेटा निर्यात करें (GDPR)",
    "detail.exportingGdpr": "निर्यात हो रहा है…",
    "detail.showMasked": "मास्क्ड दिखाएँ",
    "detail.showFull": "पूर्ण दिखाएँ",
    "detail.maskedNotice":
      "मास्क्ड दृश्य दिखाया जा रहा है — कुछ फ़ील्ड छिपे हुए हैं।",
    "detail.confirmDelete":
      "इस वस्तु को सॉफ़्ट-डिलीट करें? इसे इंटरफ़ेस से पूर्ववत नहीं किया जा सकता।",
    "detail.identity": "पहचान",
    "detail.id": "ID",
    "detail.additionalType": "अतिरिक्त प्रकार",
    "detail.description": "विवरण",
    "detail.disambiguating": "स्पष्टीकरण",
    "detail.url": "URL",
    "detail.owner": "स्वामी",
    "detail.mainEntityOfPage": "पृष्ठ की मुख्य इकाई",
    "detail.identifiers": "पहचानकर्ता",
    "detail.alternateNames": "वैकल्पिक नाम",
    "detail.sameAs": "के समान (आधिकारिक URL)",
    "detail.images": "छवियाँ",
    "detail.customPrefix": "कस्टम: ",
    "audit.title": "ऑडिट लॉग",
    "audit.backToThing": "वस्तु पर वापस जाएँ",
    "audit.loading": "लोड हो रहा है…",
    "audit.none": "कोई ऑडिट प्रविष्टियाँ नहीं।",
    "audit.by": "द्वारा",
    "audit.payload": "पेलोड",
    "edit.title": "वस्तु संपादित करें",
    "edit.cancel": "रद्द करें",
    "edit.loading": "लोड हो रहा है…",
    "edit.submitLabel": "परिवर्तन सहेजें",
    "new.title": "नई वस्तु",
    "new.submitLabel": "बनाएँ",
    "new.possibleDuplicates": "संभावित डुप्लिकेट",
    "new.duplicatesDetected":
      "डुप्लिकेट पाए गए ({count}) — पुनः सबमिट करने से पहले नीचे समीक्षा करें।",
    "match.title": "मिलान जाँच",
    "match.name": "नाम",
    "match.threshold": "सीमा",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "विवरण",
    "match.url": "URL",
    "match.sameAs": "«के समान» URL",
    "match.sameAsHint": "प्रति पंक्ति एक",
    "match.identifiers": "पहचानकर्ता",
    "match.matching": "मिलान हो रहा है…",
    "match.findMatches": "मिलान खोजें",
    "merge.title": "वस्तुएँ मर्ज करें",
    "merge.mainId": "मुख्य वस्तु ID",
    "merge.mainIdHint": "बची रहने वाली रिकॉर्ड",
    "merge.dupId": "डुप्लिकेट वस्तु ID",
    "merge.dupIdHint": "सॉफ़्ट-डिलीट किया जाएगा",
    "merge.reason": "कारण",
    "merge.reasonHint": "मर्ज ऑडिट ट्रेल में दर्ज",
    "merge.reasonPlaceholder": "पुष्ट डुप्लिकेट",
    "merge.loadPreview": "पूर्वावलोकन लोड करें",
    "merge.merging": "मर्ज हो रहा है…",
    "merge.merge": "मर्ज करें",
    "merge.preview": "पूर्वावलोकन",
    "merge.main": "मुख्य",
    "merge.duplicate": "डुप्लिकेट",
    "merge.completed": "मर्ज पूर्ण हुआ",
    "merge.recordCreated": "मर्ज रिकॉर्ड {id} {at} पर बनाया गया।",
    "merge.viewMain": "मर्ज की गई मुख्य वस्तु देखें",
    "merge.confirm":
      "{dup}… को {main}… में मर्ज करें?\nयह डुप्लिकेट को सॉफ़्ट-डिलीट कर देता है।",
    "results.title": "मिलान परिणाम",
    "results.noCandidates": "कोई उम्मीदवार नहीं।",
    "results.scoreBreakdown": "स्कोर विश्लेषण",
    "results.nameScore": "नाम",
    "results.identifierScore": "पहचानकर्ता",
    "results.descriptionScore": "विवरण",
    "results.urlScore": "URL",
    "results.sameAsScore": "के समान",
    "results.phoneticMatch": "ध्वन्यात्मक मिलान",
    "results.deterministicMatch": "निर्धारक (DOI/ISBN/…)",
    "form.name": "नाम",
    "form.additionalType": "अतिरिक्त प्रकार",
    "form.additionalTypeHint": "schema.org उपप्रकार URL",
    "form.description": "विवरण",
    "form.disambiguating": "स्पष्टीकरण विवरण",
    "form.disambiguatingHint": "संक्षिप्त विशिष्ट विवरण",
    "form.url": "URL",
    "form.owner": "स्वामी",
    "form.alternateNames": "वैकल्पिक नाम",
    "form.alternateNamesHint": "प्रति पंक्ति एक",
    "form.sameAs": "«के समान» URL",
    "form.sameAsHint": "Wikidata, Wikipedia, आदि — प्रति पंक्ति एक",
    "form.identifiers": "पहचानकर्ता",
    "form.saving": "सहेजा जा रहा है…",
    "form.reset": "रीसेट करें",
    "identifier.type": "प्रकार",
    "identifier.customOption": "कस्टम…",
    "identifier.customLabel": "कस्टम लेबल",
    "identifier.value": "मान",
    "identifier.url": "URL",
    "identifier.remove": "हटाएँ",
    "identifier.add": "+ पहचानकर्ता जोड़ें",
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
    "splash.hero.title": "हर वस्तु के लिए एक भरोसेमंद रिकॉर्ड",
    "splash.hero.subtitle":
      "उत्पादों, संपत्तियों और कृतियों को एक बार दर्ज करें, उन्हें तुरंत खोजें, और डुप्लिकेट बनने से पहले ही रोकें, पूरे ऑडिट ट्रेल के साथ।",
    "splash.benefits.1.title": "कम डुप्लिकेट",
    "splash.benefits.1.body":
      "वस्तु बनते ही रियल-टाइम मैचिंग संभावित डुप्लिकेट को चिह्नित कर देती है।",
    "splash.benefits.2.title": "वस्तुएँ जल्दी खोजें",
    "splash.benefits.2.body":
      "फ़ज़ी और ध्वन्यात्मक खोज टाइपिंग की गलतियों और वर्तनी-भेदों के बावजूद वस्तुएँ ढूँढ लेती है।",
    "splash.benefits.3.title": "एक रिकॉर्ड, कई पहचानकर्ता",
    "splash.benefits.3.body":
      "DOI, ISBN, GTIN और अन्य पहचानकर्ता सब एक ही रिकॉर्ड में रहते हैं।",
    "splash.benefits.4.title": "साफ़ मर्ज",
    "splash.benefits.4.body":
      "पुष्ट डुप्लिकेट को एक बचे हुए रिकॉर्ड में मिलाएँ, बिना ज्ञात जानकारी खोए।",
    "splash.benefits.5.title": "समीक्षित निर्णय",
    "splash.benefits.5.body":
      "संभावित डुप्लिकेट जोड़े कतार में प्रतीक्षा करते हैं, ताकि कोई व्यक्ति उन्हें पुष्ट या अस्वीकार करे।",
    "splash.benefits.6.title": "पूरा इतिहास",
    "splash.benefits.6.body":
      "हर बदलाव ऑडिट लॉग में दर्ज होता है, जिसे आप किसी भी वस्तु के लिए पढ़ सकते हैं।",
    "splash.features.1.title": "खोजें और ब्राउज़ करें",
    "splash.features.1.body":
      "क्रमबद्ध और फ़िल्टर करने योग्य डेटा ग्रिड हर वस्तु दिखाता है, साथ में पूर्ण-पाठ खोज।",
    "splash.features.2.title": "समृद्ध रिकॉर्ड",
    "splash.features.2.body":
      "हर वस्तु पर पहचानकर्ता, वैकल्पिक नाम, समान-लिंक और चित्र रखें।",
    "splash.features.3.title": "मिलान जाँच",
    "splash.features.3.body":
      "रिकॉर्ड बनाने से पहले एक काल्पनिक रिकॉर्ड को इंडेक्स के विरुद्ध अंकित करें।",
    "splash.features.4.title": "आमने-सामने मर्ज",
    "splash.features.4.body":
      "एक मुख्य रिकॉर्ड और एक डुप्लिकेट चुनें, फिर उन्हें एक ही चरण में मर्ज करें।",
    "splash.features.5.title": "समीक्षा बोर्ड",
    "splash.features.5.body":
      "लंबित जोड़ों को पुष्ट या अस्वीकार करने के लिए खींचें, या दोनों रिकॉर्ड की तुलना के लिए किसी को खोलें।",
    "splash.features.6.title": "निजी और सुलभ",
    "splash.features.6.body":
      "संवेदनशील फ़ील्ड छिपाएँ, डेटा निर्यात करें, और भाषा, थीम व पाठ का आकार बदलें।",
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
      "वस्तु रजिस्ट्री का निर्देशित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, किसी उत्पाद या संपत्ति को दर्ज करने से लेकर डुप्लिकेट मर्ज करने तक।",
    "tour.s1.title": "किसी वस्तु को दर्ज करना",
    "tour.s1.summary":
      "किसी उत्पाद, संपत्ति या कृति का रिकॉर्ड उसके पहचानकर्ताओं और लिंक के साथ बनाएँ। कुछ भी बनने से पहले फ़ॉर्म उसे जाँचता है और संभावित डुप्लिकेट की चेतावनी देता है।",
    "tour.s1.step.1": "मेनू से «नई वस्तु» खोलें।",
    "tour.s1.step.2":
      "नाम दर्ज करें, जो एकमात्र अनिवार्य फ़ील्ड है, फिर विवरण, URL, स्वामी और वैकल्पिक नाम जितने आपको पता हों जोड़ें।",
    "tour.s1.step.3":
      "+ पहचानकर्ता जोड़ें से DOI, ISBN या GTIN जैसे पहचानकर्ता जोड़ें, और same-as URL प्रति पंक्ति एक लिखें।",
    "tour.s1.step.4":
      "बनाएँ चुनें। यदि संभावित डुप्लिकेट दिखें तो दोबारा भेजने से पहले उनकी समीक्षा करें; अन्यथा आप नई वस्तु के पृष्ठ पर पहुँचते हैं।",
    "tour.s2.title": "किसी वस्तु को खोजना",
    "tour.s2.summary":
      "पूरे इंडेक्स में नाम या पहचानकर्ता से खोजें, फिर डेटा ग्रिड से किसी भी परिणाम को खोलें।",
    "tour.s2.step.1": "मेनू से «वस्तुएँ» खोलें।",
    "tour.s2.step.2": "खोज बॉक्स में नाम या पहचानकर्ता लिखें और खोज चुनें।",
    "tour.s2.step.3":
      "टाइपो और वर्तनी-भेद पकड़ने के लिए अस्पष्ट या ध्वन्यात्मक (Soundex) चालू करें, या संरक्षित विवरण छिपाने के लिए संवेदनशील छुपाएँ।",
    "tour.s2.step.4":
      "परिणामों में आगे-पीछे जाने के लिए पिछला और अगला का उपयोग करें, और उस वस्तु को खोलने के लिए कोई पंक्ति चुनें।",
    "tour.s3.title": "मिलान जाँचना",
    "tour.s3.summary":
      "किसी काल्पनिक रिकॉर्ड को बनाने से पहले इंडेक्स के विरुद्ध स्कोर करें और देखें कि हर उम्मीदवार क्यों मेल खाया।",
    "tour.s3.step.1": "मेनू से «मिलान जाँच» खोलें।",
    "tour.s3.step.2":
      "नाम भरें, और चाहें तो विवरण, URL, same-as URL और पहचानकर्ता भी भरें।",
    "tour.s3.step.3":
      "सीमा को 0.0 और 1.0 के बीच सेट करें, फिर मिलान खोजें चुनें।",
    "tour.s3.step.4":
      "मिलान परिणाम पढ़ें: हर उम्मीदवार का नाम, पहचानकर्ता, विवरण, URL और same-as के अनुसार स्कोर विश्लेषण होता है, और ध्वन्यात्मक या निर्धारक मिलान चिह्नित होते हैं।",
    "tour.s4.title": "समीक्षा कतार निपटाना",
    "tour.s4.summary":
      "बैच स्कैन से मिले संभावित डुप्लिकेट जोड़े यहाँ किसी व्यक्ति द्वारा पुष्टि या अस्वीकार किए जाने की प्रतीक्षा करते हैं।",
    "tour.s4.step.1":
      "मेनू से «समीक्षा» खोलें और उम्मीदवार जोड़े खोजने के लिए स्कैन चलाएं चुनें।",
    "tour.s4.step.2":
      "स्थिति से फ़िल्टर करें और पृष्ठ का आकार तय करें; जोड़े बोर्ड पर कार्ड के रूप में और कतार में पंक्तियों के रूप में दिखते हैं।",
    "tour.s4.step.3":
      "किसी जोड़े पर तुलना करें चुनें ताकि रिकॉर्ड A और रिकॉर्ड B स्कोर विश्लेषण के साथ साथ-साथ दिखें।",
    "tour.s4.step.4":
      "डुप्लिकेट की पुष्टि करें या अस्वीकार करें चुनें। पुष्टि केवल निर्णय दर्ज करती है; मर्ज करने के लिए A रखें या B रखें चुनें।",
    "tour.s5.title": "डुप्लिकेट मर्ज करना",
    "tour.s5.summary":
      "पुष्टि किए गए डुप्लिकेट को एक बचे रिकॉर्ड में मिला दें। डुप्लिकेट सॉफ़्ट-डिलीट होता है और मर्ज दर्ज किया जाता है।",
    "tour.s5.step.1": "मेनू से «मर्ज करें» खोलें।",
    "tour.s5.step.2":
      "मुख्य वस्तु ID (बचा रहने वाला रिकॉर्ड), डुप्लिकेट वस्तु ID और कारण दर्ज करें, जो मर्ज ऑडिट ट्रेल में दर्ज होता है।",
    "tour.s5.step.3":
      "कुछ भी बदलने से पहले मुख्य और डुप्लिकेट रिकॉर्ड की तुलना के लिए पूर्वावलोकन लोड करें चुनें।",
    "tour.s5.step.4":
      "मर्ज करें चुनें और पुष्टि करें। इसके बाद «मर्ज पूर्ण हुआ» संदेश आता है, जिसमें मर्ज की गई मुख्य वस्तु देखें का लिंक होता है।",
    "tour.s6.title": "किसी रिकॉर्ड को देखना, सुरक्षित करना और ऑडिट करना",
    "tour.s6.summary":
      "हर वस्तु का एक विवरण पृष्ठ होता है जहाँ उसे पढ़ा, संपादित, निर्यात और ऑडिट किया जा सकता है।",
    "tour.s6.step.1":
      "वस्तुएँ में खोजें और उस वस्तु का विवरण पृष्ठ खोलने के लिए कोई पंक्ति चुनें।",
    "tour.s6.step.2":
      "संपादित दृश्य के लिए मास्क्ड दिखाएँ, या सभी फ़ील्ड देखने के लिए पूर्ण दिखाएँ का उपयोग करें।",
    "tour.s6.step.3":
      "बदलने के लिए संपादित करें, हर बदलाव और उसके करने वाले का लॉग पढ़ने के लिए ऑडिट, या उसका डेटा डाउनलोड करने के लिए डेटा निर्यात करें (GDPR) चुनें।",
    "tour.s6.step.4":
      "पुष्टि के बाद वस्तु को सॉफ़्ट-डिलीट करने के लिए हटाएँ चुनें; इसे स्क्रीन से पूर्ववत नहीं किया जा सकता।",
    "signin.sso": "SSO से साइन इन करें",
  },
  "zh-cn": {
    "auth.sessionExpired": "您的会话已过期。正在重定向到登录…",
    "auth.accessDenied": "您没有权限执行此操作。",
    "nav.review": "审核",
    "review.run": "运行扫描",
    "review.intro":
      "批量扫描发现的候选重复对。拖动待处理卡片，或打开一对以在决定前并排比较两条记录。",
    "review.gap.provenance": "此服务不会为每一对单独记录来源——只记录检测方法。",
    "review.loading": "正在加载审核队列…",
    "review.empty": "此筛选条件下没有审核项。",
    "review.filter.status": "状态",
    "review.filter.statusAll": "全部",
    "review.filter.limit": "每页数量",
    "review.filter.limitHint":
      "服务最多返回 500 条记录，超出此数量不提供分页。",
    "review.status.pending": "待处理",
    "review.status.confirmed": "已确认",
    "review.status.rejected": "已拒绝",
    "review.status.automerged": "已自动合并",
    "review.board.title": "看板",
    "review.list.title": "队列",
    "review.col.pair": "配对",
    "review.col.score": "分数",
    "review.col.quality": "质量",
    "review.col.method": "检测方法",
    "review.col.status": "状态",
    "review.col.actions": "操作",
    "review.compare.open": "比较",
    "review.compare.title": "比较该配对",
    "review.compare.close": "关闭",
    "review.compare.loading": "正在加载两条记录…",
    "review.compare.field": "字段",
    "review.compare.a": "记录 A",
    "review.compare.b": "记录 B",
    "review.compare.none": "未记录",
    "review.compare.partial": "有一条记录无法加载——可能已被合并或删除。",
    "review.field.score": "匹配分数",
    "review.field.quality": "匹配质量",
    "review.field.method": "检测方法",
    "review.field.status": "状态",
    "review.breakdown.title": "分数明细",
    "review.breakdown.none": "此配对未记录分数明细。",
    "review.breakdown.component": "组成部分",
    "review.breakdown.weight": "权重",
    "review.breakdown.score": "分数",
    "review.decide.confirm": "确认为重复项",
    "review.decide.reject": "拒绝",
    "review.decide.deciding": "保存中…",
    "review.decide.locked": "已作出决定——只有待处理项可以决定。",
    "review.merge.title": "合并此配对",
    "review.merge.note": "确认只会记录裁定，不会执行合并。请选择保留哪条记录。",
    "review.merge.keepA": "保留 A，将 B 合并入其中",
    "review.merge.keepB": "保留 B，将 A 合并入其中",
    "brand.name": "Thing",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "仪表板",
    "nav.things": "事物",
    "nav.newThing": "新建事物",
    "nav.matchCheck": "匹配检查",
    "nav.merge": "合并",
    "nav.toggle": "切换导航",
    "chrome.theme": "主题",
    "chrome.language": "语言",
    "nav.share": "分享",
    "nav.text_size": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "dashboard.title": "仪表板",
    "dashboard.service": "服务：",
    "dashboard.recentActivity": "近期活动",
    "dashboard.noRecent": "没有近期审计记录。",
    "things.title": "事物",
    "things.new": "新建事物",
    "things.searchPlaceholder": "按名称、标识符搜索…",
    "things.fuzzy": "模糊",
    "things.phonetic": "语音（Soundex）",
    "things.maskSensitive": "遮蔽敏感",
    "things.previousPage": "上一页",
    "things.nextPage": "下一页",
    "things.pageRange": "第 {from}–{to} 项，共 {total} 项",
    "things.loading": "加载中…",
    "things.recordCount": "{count} 条记录",
    "things.recordCountPlural": "{count} 条记录",
    "search.action": "搜索",
    "grid.id": "ID",
    "grid.name": "名称",
    "grid.type": "类型（schema.org）",
    "grid.primaryId": "主标识符",
    "grid.url": "URL",
    "detail.loading": "加载中…",
    "detail.edit": "编辑",
    "detail.audit": "审计",
    "detail.delete": "删除",
    "detail.exportGdpr": "导出数据（GDPR）",
    "detail.exportingGdpr": "导出中…",
    "detail.showMasked": "显示脱敏视图",
    "detail.showFull": "显示完整视图",
    "detail.maskedNotice": "正在显示脱敏视图——部分字段已隐藏。",
    "detail.confirmDelete": "软删除此事物？此操作无法通过界面撤销。",
    "detail.identity": "身份",
    "detail.id": "ID",
    "detail.additionalType": "附加类型",
    "detail.description": "描述",
    "detail.disambiguating": "消歧",
    "detail.url": "URL",
    "detail.owner": "所有者",
    "detail.mainEntityOfPage": "页面主实体",
    "detail.identifiers": "标识符",
    "detail.alternateNames": "别名",
    "detail.sameAs": "等同于（权威 URL）",
    "detail.images": "图像",
    "detail.customPrefix": "自定义：",
    "audit.title": "审计日志",
    "audit.backToThing": "返回事物",
    "audit.loading": "加载中…",
    "audit.none": "没有审计记录。",
    "audit.by": "由",
    "audit.payload": "负载",
    "edit.title": "编辑事物",
    "edit.cancel": "取消",
    "edit.loading": "加载中…",
    "edit.submitLabel": "保存更改",
    "new.title": "新建事物",
    "new.submitLabel": "创建",
    "new.possibleDuplicates": "可能的重复项",
    "new.duplicatesDetected":
      "检测到重复项（{count}）— 重新提交前请在下方查看。",
    "match.title": "匹配检查",
    "match.name": "名称",
    "match.threshold": "阈值",
    "match.thresholdHint": "0.0 – 1.0",
    "match.description": "描述",
    "match.url": "URL",
    "match.sameAs": "“等同于” URL",
    "match.sameAsHint": "每行一个",
    "match.identifiers": "标识符",
    "match.matching": "匹配中…",
    "match.findMatches": "查找匹配项",
    "merge.title": "合并事物",
    "merge.mainId": "主事物 ID",
    "merge.mainIdHint": "保留的记录",
    "merge.dupId": "重复事物 ID",
    "merge.dupIdHint": "将被软删除",
    "merge.reason": "原因",
    "merge.reasonHint": "记录在合并审计追踪中",
    "merge.reasonPlaceholder": "已确认的重复项",
    "merge.loadPreview": "加载预览",
    "merge.merging": "合并中…",
    "merge.merge": "合并",
    "merge.preview": "预览",
    "merge.main": "主",
    "merge.duplicate": "重复项",
    "merge.completed": "合并完成",
    "merge.recordCreated": "合并记录 {id} 创建于 {at}。",
    "merge.viewMain": "查看已合并的主事物",
    "merge.confirm": "将 {dup}… 合并到 {main}…？\n这将软删除重复项。",
    "results.title": "匹配结果",
    "results.noCandidates": "没有候选项。",
    "results.scoreBreakdown": "分数明细",
    "results.nameScore": "名称",
    "results.identifierScore": "标识符",
    "results.descriptionScore": "描述",
    "results.urlScore": "URL",
    "results.sameAsScore": "等同于",
    "results.phoneticMatch": "语音匹配",
    "results.deterministicMatch": "确定性（DOI/ISBN/…）",
    "form.name": "名称",
    "form.additionalType": "附加类型",
    "form.additionalTypeHint": "schema.org 子类型 URL",
    "form.description": "描述",
    "form.disambiguating": "消歧描述",
    "form.disambiguatingHint": "简短的区分性细节",
    "form.url": "URL",
    "form.owner": "所有者",
    "form.alternateNames": "别名",
    "form.alternateNamesHint": "每行一个",
    "form.sameAs": "“等同于” URL",
    "form.sameAsHint": "Wikidata、Wikipedia 等 — 每行一个",
    "form.identifiers": "标识符",
    "form.saving": "保存中…",
    "form.reset": "重置",
    "identifier.type": "类型",
    "identifier.customOption": "自定义…",
    "identifier.customLabel": "自定义标签",
    "identifier.value": "值",
    "identifier.url": "URL",
    "identifier.remove": "删除",
    "identifier.add": "+ 添加标识符",
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
    "splash.hero.title": "每件事物，一份可信记录",
    "splash.hero.subtitle":
      "一次登记产品、资产和作品，即刻查找，在重复出现之前就将其拦截，并内置完整审计记录。",
    "splash.benefits.1.title": "更少重复",
    "splash.benefits.1.body": "创建事物的瞬间，实时匹配就会标出疑似重复。",
    "splash.benefits.2.title": "快速找到事物",
    "splash.benefits.2.body":
      "模糊搜索和语音搜索即使有拼写错误或写法差异也能找到事物。",
    "splash.benefits.3.title": "一条记录，多个标识",
    "splash.benefits.3.body": "DOI、ISBN、GTIN 等各类标识集中在同一条记录上。",
    "splash.benefits.4.title": "干净合并",
    "splash.benefits.4.body":
      "将已确认的重复项并入一条保留的记录，且不丢失已有信息。",
    "splash.benefits.5.title": "经过审核的决定",
    "splash.benefits.5.body": "候选重复对在队列中等待，由人员确认或拒绝。",
    "splash.benefits.6.title": "完整的历史",
    "splash.benefits.6.body":
      "每次更改都会记入审计日志，任何事物的日志均可查看。",
    "splash.features.1.title": "搜索与浏览",
    "splash.features.1.body":
      "可排序、可筛选的数据表列出每件事物，并支持全文搜索。",
    "splash.features.2.title": "内容丰富的记录",
    "splash.features.2.body":
      "在每件事物上保存标识、别名、同一实体链接和图片。",
    "splash.features.3.title": "匹配检查",
    "splash.features.3.body": "在创建之前，先用假设记录对照索引进行评分。",
    "splash.features.4.title": "并排合并",
    "splash.features.4.body": "选择主记录和重复记录，一步完成合并。",
    "splash.features.5.title": "审核看板",
    "splash.features.5.body":
      "拖动待处理的记录对以确认或拒绝，或打开某一对比较两条记录。",
    "splash.features.6.title": "隐私与无障碍",
    "splash.features.6.body":
      "可隐藏敏感字段、导出数据，并切换语言、主题和字号。",
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
      "事物登记库的图文导览：每个页面的作用和使用步骤，从登记产品或资产到合并重复记录。",
    "tour.s1.title": "登记一个事物",
    "tour.s1.summary":
      "用标识符和链接为产品、资产或作品创建记录。创建之前，表单会先检查并提示可能的重复项。",
    "tour.s1.step.1": "从菜单打开“新建事物”。",
    "tour.s1.step.2":
      "填写名称（唯一必填项），再按已知信息补充描述、URL、所有者和别名。",
    "tour.s1.step.3":
      "用“+ 添加标识符”添加 DOI、ISBN 或 GTIN 等标识符，并每行填写一个 same-as URL。",
    "tour.s1.step.4":
      "点击“创建”。如果出现“可能的重复项”，请先审阅再重新提交；否则会进入新事物的页面。",
    "tour.s2.title": "查找事物",
    "tour.s2.summary":
      "按名称或标识符搜索整个索引，然后从数据表格中打开任意结果。",
    "tour.s2.step.1": "从菜单打开“事物”。",
    "tour.s2.step.2": "在搜索框中输入名称或标识符，然后点击“搜索”。",
    "tour.s2.step.3":
      "开启“模糊”或“语音（Soundex）”以容纳拼写错误和变体，或开启“遮蔽敏感”以隐藏受保护的细节。",
    "tour.s2.step.4":
      "用“上一页”和“下一页”翻看结果，选择某一行即可打开该事物。",
    "tour.s3.title": "检查匹配项",
    "tour.s3.summary":
      "在创建之前，用索引对假设的记录打分，并查看每个候选项为何匹配。",
    "tour.s3.step.1": "从菜单打开“匹配检查”。",
    "tour.s3.step.2": "填写名称，也可以选填描述、URL、same-as URL 和标识符。",
    "tour.s3.step.3": "将阈值设在 0.0 到 1.0 之间，然后点击“查找匹配项”。",
    "tour.s3.step.4":
      "查看“匹配结果”：每个候选项都按名称、标识符、描述、URL 和 same-as 给出分数明细，语音匹配和确定性匹配会被标出。",
    "tour.s4.title": "处理审核队列",
    "tour.s4.summary": "批量扫描得出的候选重复配对在此等待人工确认或拒绝。",
    "tour.s4.step.1": "从菜单打开“审核”，点击“运行扫描”查找候选配对。",
    "tour.s4.step.2":
      "按“状态”筛选并设置每页数量；配对在“看板”中显示为卡片，在“队列”中显示为行。",
    "tour.s4.step.3":
      "在某个配对上点击“比较”，并排查看记录 A 和记录 B 以及分数明细。",
    "tour.s4.step.4":
      "点击“确认为重复项”或“拒绝”。确认只记录结论；要合并，请选择“保留 A”或“保留 B”。",
    "tour.s5.title": "合并重复项",
    "tour.s5.summary":
      "把已确认的重复项并入一条保留的记录。重复项被软删除，合并过程会被记录。",
    "tour.s5.step.1": "从菜单打开“合并”。",
    "tour.s5.step.2":
      "输入主事物 ID（保留的记录）、重复事物 ID 和原因，原因会记录在合并审计追踪中。",
    "tour.s5.step.3": "点击“加载预览”，在任何改动之前比较主记录和重复记录。",
    "tour.s5.step.4":
      "点击“合并”并确认。随后会显示“合并完成”，并附有“查看已合并的主事物”的链接。",
    "tour.s6.title": "查看、保护和审计一条记录",
    "tour.s6.summary": "每个事物都有一个详情页，可用来查看、编辑、导出和审计。",
    "tour.s6.step.1": "在“事物”中搜索并选择一行，打开该事物的详情页。",
    "tour.s6.step.2":
      "用“显示脱敏视图”查看隐去敏感信息的版本，或用“显示完整视图”查看所有字段。",
    "tour.s6.step.3":
      "点击“编辑”进行修改，点击“审计”查看每次更改及操作者的日志，或点击“导出数据（GDPR）”下载其数据。",
    "tour.s6.step.4":
      "确认后点击“删除”可软删除该事物；此操作无法在界面中撤销。",
    "signin.sso": "使用 SSO 登录",
  },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)["en-001"];

/**
 * All translation keys (the English catalog's key set). Useful for tests
 * that assert every locale covers the full key set.
 */
export const STRING_KEYS = Object.keys(STRINGS["en-001"]) as StringKey[];

// Normalise raw input to a supported locale, or null if unsupported.
// Accepts a region subtag (es-MX → es-001) and is case-insensitive.
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
 * to unit-test without a Svelte component.
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
 * Reactive translation accessor for components: `t("things.title")`.
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
