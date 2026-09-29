// Lightweight, dependency-free i18n for the Event front-end SPA. A
// per-locale strings map plus a reactive `$state` current-locale (Svelte 5
// runes), exposed via a `t(key)` accessor. Deliberately no i18n library:
// the surface is tiny and we keep the front-end dependency-light (drift is
// accepted family-wide — see the repo `feedback_front_end_drift` memory).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. An unknown key/locale falls back to `en-001`. The
// chosen locale persists to localStorage under `mxi.event.locale` and is
// reflected onto `<html lang>` / `<html dir>` by the layout.

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
 * Right-to-left locales. The layout uses {@link isRtl} (derived from this
 * set) to set `<html dir="rtl">` for these.
 */
export const RTL_LOCALES = ["ar-001"] as const;

/**
 * Is `locale` written right-to-left?
 *
 * @param locale - A locale code (supported or not).
 * @returns `true` for Arabic, `false` otherwise.
 */
export function isRtl(locale: string): boolean {
  return (RTL_LOCALES as readonly string[]).includes(locale);
}

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

// localStorage key under which the chosen UI locale is persisted.
const LOCALE_KEY = "mxi.event.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    "nav.calendar": "التقويم",
    brand: "حدث",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "لوحة المعلومات",
    "nav.events": "الأحداث",
    "nav.newEvent": "حدث جديد",
    "nav.matchCheck": "فحص التطابق",
    "nav.merge": "دمج",
    "nav.toggle": "تبديل التنقل",
    "chrome.theme": "السمة",
    "chrome.language": "اللغة",
    "chrome.share": "مشاركة",
    "chrome.textSize": "حجم النص",
    "share.copyLink": "نسخ الرابط",
    "share.linkCopied": "تم نسخ الرابط",
    "share.copyFailed": "تعذر النسخ — انسخه من شريط العنوان",
    "dashboard.title": "لوحة المعلومات",
    "dashboard.service": "الخدمة:",
    "dashboard.recentActivity": "النشاط الأخير",
    "dashboard.noRecent": "لا توجد إدخالات تدقيق حديثة.",
    "events.title": "الأحداث",
    "events.new": "حدث جديد",
    "events.searchPlaceholder": "البحث بالاسم أو المنظم أو المعرّف…",
    "events.filter.from": "من",
    "events.filter.to": "إلى",
    "events.filter.status": "الحالة",
    "events.filter.type": "النوع",
    "events.filter.any": "أي",
    "events.filter.fuzzy": "تقريبي",
    "events.filter.apply": "تطبيق عوامل التصفية",
    "events.loading": "جارٍ التحميل…",
    "events.count.one": "{n} حدث",
    "events.count.other": "{n} حدثًا",
    "grid.id": "المعرّف",
    "grid.name": "الاسم",
    "grid.start": "البداية",
    "grid.type": "النوع",
    "grid.status": "الحالة",
    "grid.mode": "الوضع",
    "new.title": "حدث جديد",
    "new.create": "إنشاء",
    "new.duplicatesTitle": "تكرارات محتملة",
    "new.duplicatesDetected":
      "تم اكتشاف تكرارات ({n}) — راجعها أدناه قبل إعادة الإرسال.",
    "match.title": "فحص التطابق",
    "match.name": "الاسم",
    "match.threshold": "العتبة",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "البداية",
    "match.end": "النهاية",
    "match.organizerName": "اسم المنظم",
    "match.find": "البحث عن التطابقات",
    "match.matching": "جارٍ المطابقة…",
    "merge.title": "دمج الأحداث",
    "merge.mainId": "معرّف الحدث الرئيسي",
    "merge.mainIdHint": "السجل الباقي",
    "merge.dupId": "معرّف الحدث المكرر",
    "merge.dupIdHint": "سيُحذف حذفًا ناعمًا",
    "merge.reason": "السبب",
    "merge.reasonHint": "يُسجَّل في سجل تدقيق الدمج",
    "merge.reasonPlaceholder": "تكرار مؤكد",
    "merge.loadPreview": "تحميل المعاينة",
    "merge.merge": "دمج",
    "merge.merging": "جارٍ الدمج…",
    "merge.bothIdsRequired": "كلا المعرّفين مطلوبان",
    "merge.idsMustDiffer": "يجب أن يختلف السجل الرئيسي عن المكرر",
    "merge.confirm":
      "دمج {dup}… في {main}…؟\nسيؤدي ذلك إلى حذف المكرر حذفًا ناعمًا.",
    "merge.previewTitle": "معاينة",
    "merge.preview.main": "رئيسي",
    "merge.preview.duplicate": "مكرر",
    "merge.preview.none": "—",
    "merge.preview.noDate": "بلا تاريخ",
    "merge.completedTitle": "اكتمل الدمج",
    "merge.completedBody": "تم إنشاء سجل الدمج {id} في {at}.",
    "merge.viewMerged": "عرض الحدث الرئيسي المدموج",
    "detail.loading": "جارٍ التحميل…",
    "detail.edit": "تحرير",
    "detail.audit": "تدقيق",
    "detail.delete": "حذف",
    "detail.exportGdpr": "تصدير البيانات (GDPR)",
    "detail.exportingGdpr": "جارٍ التصدير…",
    "detail.showMasked": "إظهار المُقنَّع",
    "detail.showFull": "إظهار الكامل",
    "detail.maskedNotice": "يتم عرض العرض المُقنَّع — بعض الحقول مخفية.",
    "detail.identity": "الهوية",
    "detail.id": "المعرّف",
    "detail.start": "البداية",
    "detail.end": "النهاية",
    "detail.status": "الحالة",
    "detail.type": "النوع",
    "detail.mode": "الوضع",
    "detail.timeZone": "المنطقة الزمنية",
    "detail.duration": "المدة",
    "detail.description": "الوصف",
    "detail.empty": "—",
    "detail.loc.place": "المكان",
    "detail.loc.address": "العنوان",
    "detail.loc.virtual": "افتراضي",
    "detail.loc.text": "نص",
    "detail.location": "الموقع",
    "detail.organizers": "المنظمون",
    "detail.performers": "المؤدّون",
    "detail.identifiers": "المعرفات",
    "detail.offers": "العروض",
    "detail.ticket": "تذكرة",
    "detail.confirmDelete":
      "هل تريد حذف هذا الحدث حذفًا ناعمًا؟ لا يمكن التراجع عن ذلك عبر الواجهة.",
    "edit.title": "تحرير الحدث",
    "edit.cancel": "إلغاء",
    "edit.loading": "جارٍ التحميل…",
    "edit.save": "حفظ التغييرات",
    "audit.title": "سجل التدقيق",
    "audit.back": "العودة إلى الحدث",
    "audit.loading": "جارٍ التحميل…",
    "audit.none": "لا توجد إدخالات تدقيق.",
    "audit.by": "بواسطة",
    "audit.payload": "الحمولة",
    "form.name": "الاسم",
    "form.required": "مطلوب",
    "form.eventType": "نوع الحدث",
    "form.start": "البداية",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "النهاية",
    "form.endAfterStart": "يجب أن تكون النهاية ≥ البداية",
    "form.doorTime": "وقت فتح الأبواب",
    "form.doorBeforeStart": "يجب أن يكون وقت فتح الأبواب ≤ البداية",
    "form.status": "الحالة",
    "form.attendanceMode": "وضع الحضور",
    "form.timeZone": "المنطقة الزمنية",
    "form.timeZoneHint": "IANA، مثل America/Los_Angeles",
    "form.allDay": "طوال اليوم",
    "form.no": "لا",
    "form.yes": "نعم",
    "form.description": "الوصف",
    "form.url": "الرابط",
    "form.duration": "المدة",
    "form.durationHint": "ISO 8601، مثل PT1H30M",
    "form.maxCapacityTotal": "السعة القصوى (الإجمالية)",
    "form.maxPhysical": "الحد الأقصى الحضوري",
    "form.maxVirtual": "الحد الأقصى الافتراضي",
    "form.keywords": "الكلمات المفتاحية",
    "form.keywordsHint": "مفصولة بفواصل",
    "form.languages": "اللغات",
    "form.languagesHint": "ISO 639-1، مثل en fr",
    "form.saving": "جارٍ الحفظ…",
    "form.save": "حفظ",
    "form.reset": "إعادة تعيين",
    "search.placeholder": "بحث…",
    "search.submit": "بحث",
    "results.title": "نتائج المطابقة",
    "results.none": "لا يوجد مرشحون.",
    "results.breakdown": "تفصيل النتيجة",
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
    "splash.hero.title": "سجل موثوق واحد لكل حدث",
    "splash.hero.subtitle":
      "سجّل الأحداث مرة واحدة، واعثر عليها بالاسم أو بالتاريخ، وامنع التكرار قبل أن يبدأ، مع سجل تدقيق كامل مدمج.",
    "splash.benefits.1.title": "تكرار أقل",
    "splash.benefits.1.body":
      "ينبّهك إنشاء حدث جديد إلى التكرارات المحتملة قبل حفظه.",
    "splash.benefits.2.title": "عثور سريع على الأحداث",
    "splash.benefits.2.body":
      "ابحث بالاسم مع مطابقة تقريبية، ثم صفِّ حسب التاريخ والحالة والنوع.",
    "splash.benefits.3.title": "سجل واحد نظيف",
    "splash.benefits.3.body":
      "ادمج التكرارات المؤكدة في حدث واحد مع الاحتفاظ بالسجل التاريخي.",
    "splash.benefits.4.title": "التخطيط بالتقويم",
    "splash.benefits.4.body":
      "اعرض الأحداث في تقويم واسحب حدثًا إلى موعد جديد لإعادة جدولته.",
    "splash.benefits.5.title": "اعرف ما الذي تغيّر",
    "splash.benefits.5.body":
      "لكل حدث سجل تدقيق خاص به يبيّن من غيّر ماذا ومتى.",
    "splash.benefits.6.title": "بلغتك أنت",
    "splash.benefits.6.body":
      "استخدم التطبيق بالعربية أو الويلزية أو الإنجليزية أو الإسبانية أو الفرنسية أو الهندية أو الصينية.",
    "splash.features.1.title": "سجلات أحداث غنية",
    "splash.features.1.body":
      "الفترة الزمنية والحالة والنوع ووضع الحضور والمنطقة الزمنية والوصف في مكان واحد.",
    "splash.features.2.title": "الأماكن والأشخاص",
    "splash.features.2.body":
      "سجّل المواقع والمنظمين والمؤدين والمعرّفات وعروض التذاكر.",
    "splash.features.3.title": "فحص التطابق",
    "splash.features.3.body": "قيّم حدثًا افتراضيًا مقابل الفهرس قبل إنشائه.",
    "splash.features.4.title": "دمج موجَّه",
    "splash.features.4.body":
      "قارن حدثين جنبًا إلى جنب ثم ادمجهما مع تسجيل السبب.",
    "splash.features.5.title": "عرض التقويم",
    "splash.features.5.body":
      "تقويم بالسحب والإفلات فوق الفترة الزمنية لكل حدث.",
    "splash.features.6.title": "السمات وحجم النص",
    "splash.features.6.body":
      "اختر سمة ألوان وحجم نص مريحًا، ويُحفظ اختيارك للزيارة القادمة.",
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
      "جولة إرشادية في سجل الأحداث: ما تفعله كل شاشة وخطوات استخدامها، من تسجيل حدث إلى دمج التكرارات وإعادة الجدولة على التقويم.",
    "tour.s1.title": "تسجيل حدث",
    "tour.s1.summary":
      "أنشئ حدثًا بنافذته الزمنية وحالته ونوعه ووضع الحضور. تُنبَّه إلى التكرارات المحتملة قبل حفظ الحدث.",
    "tour.s1.step.1":
      "اختر «حدث جديد» من القائمة، أو اضغط «حدث جديد» في قائمة الأحداث.",
    "tour.s1.step.2":
      "أدخل الاسم وبداية الحدث وهما مطلوبان، ثم أضف النهاية ووقت فتح الأبواب والحالة ووضع الحضور والمنطقة الزمنية. لا يمكن أن تسبق النهاية البداية، ولا أن يتأخر وقت فتح الأبواب عنها.",
    "tour.s1.step.3":
      "أضف اختياريًا وصفًا ورابطًا ومدة وحدود السعة وكلمات مفتاحية ولغات، ثم اضغط «إنشاء».",
    "tour.s1.step.4":
      "إذا عثر السجل على تكرارات محتملة، تعرضها لوحة «تكرارات محتملة» مع درجاتها. راجعها قبل إعادة الإرسال.",
    "tour.s2.title": "العثور على الأحداث",
    "tour.s2.summary":
      "ابحث في السجل بالاسم أو المنظِّم أو المعرّف، ثم ضيّق القائمة بالتاريخ والحالة والنوع.",
    "tour.s2.step.1":
      "افتح «الأحداث» من القائمة؛ تُحمَّل القائمة مع مربع بحث وعدد الأحداث المطابقة.",
    "tour.s2.step.2":
      "اكتب في مربع البحث، مثل اسم أو منظِّم أو معرّف، ثم اضغط «بحث».",
    "tour.s2.step.3":
      "فعّل «تقريبي» لتحمّل اختلافات الإملاء، وحدّد «من» و«إلى» والحالة والنوع، ثم اضغط «تطبيق عوامل التصفية».",
    "tour.s2.step.4": "اختر صفًا في الجدول لفتح صفحة تفاصيل ذلك الحدث.",
    "tour.s3.title": "التحقق من التطابقات",
    "tour.s3.summary":
      "قيّم تفاصيل حدث افتراضي مقابل السجل دون إنشاء أي شيء، لترى ما هو موجود بالفعل.",
    "tour.s3.step.1": "افتح «فحص التطابق» من القائمة.",
    "tour.s3.step.2":
      "أدخل الاسم (مطلوب)، وإن كنت تعرفها فأدخل البداية والنهاية واسم المنظِّم.",
    "tour.s3.step.3": "اضبط العتبة بين 0.0 و1.0، ثم اضغط «البحث عن التطابقات».",
    "tour.s3.step.4":
      "اقرأ نتائج التطابق: يعرض كل مرشح درجته، ويوضح تفصيل الدرجة كيف تم التوصل إليها.",
    "tour.s4.title": "دمج التكرارات",
    "tour.s4.summary":
      "ادمج التكرار المؤكد في الحدث الذي تحتفظ به، مع تسجيل السبب في سجل التدقيق.",
    "tour.s4.step.1":
      "افتح «دمج» من القائمة وأدخل معرّف الحدث الرئيسي (السجل الباقي) ومعرّف الحدث المكرر.",
    "tour.s4.step.2":
      "اضغط «تحميل المعاينة» لرؤية السجلين جنبًا إلى جنب. يجب إدخال المعرّفين وأن يختلفا.",
    "tour.s4.step.3":
      "اكتب سببًا يُسجَّل في سجل تدقيق الدمج، ثم اضغط «دمج» وأكّد الرسالة.",
    "tour.s4.step.4":
      "يُحذف التكرار حذفًا ناعمًا، وتعرض رسالة «اكتمل الدمج» سجل الدمج، ويفتح «عرض الحدث الرئيسي المدموج» السجل الباقي.",
    "tour.s5.title": "التخطيط على التقويم",
    "tour.s5.summary":
      "اطّلع على النوافذ الزمنية للأحداث في عروض الشهر والأسبوع واليوم، وأعد الجدولة بالسحب.",
    "tour.s5.step.1":
      "افتح «التقويم» من القائمة لرؤية الأحداث المسجلة موضوعةً بحسب أوقات بدايتها ونهايتها.",
    "tour.s5.step.2": "بدّل بين عروض الشهر والأسبوع واليوم.",
    "tour.s5.step.3":
      "اسحب حدثًا إلى فترة جديدة؛ يُحفظ التغيير في سجل الحدث عبر التحديث المعتاد، فالتقويم ليس نسخة منفصلة.",
    "tour.s5.step.4": "اختر حدثًا لفتح صفحة تفاصيله.",
    "tour.s6.title": "مراجعة الحدث وتعديله وتدقيقه",
    "tour.s6.summary":
      "افتح حدثًا واحدًا لقراءة كل ما سُجّل عنه وتصحيحه ومعرفة من غيّره وإخفاء بياناته أو تصديره.",
    "tour.s6.step.1":
      "في صفحة تفاصيل الحدث، اقرأ الهوية والموقع والمنظِّمين والمؤدّين والمعرّفات والعروض حيثما وُجدت.",
    "tour.s6.step.2":
      "اختر «تحرير» لتغيير السجل ثم «حفظ التغييرات»، أو «حذف» لحذفه حذفًا ناعمًا بعد تأكيد الرسالة.",
    "tour.s6.step.3":
      "اختر «التدقيق» لفتح سجل التدقيق: يعرض كل إدخال من أجرى التغيير وحمولته.",
    "tour.s6.step.4":
      "استخدم «إظهار المُقنَّع» لعرض النسخة المحجوبة، أو «تصدير البيانات (GDPR)» لتنزيل السجل.",
    "signin.sso": "تسجيل الدخول عبر SSO",
  },
  "cy-001": {
    "nav.calendar": "Calendr",
    brand: "Digwyddiad",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Dangosfwrdd",
    "nav.events": "Digwyddiadau",
    "nav.newEvent": "Digwyddiad newydd",
    "nav.matchCheck": "Gwiriad cydweddu",
    "nav.merge": "Uno",
    "nav.toggle": "Toglo'r llywio",
    "chrome.theme": "Thema",
    "chrome.language": "Iaith",
    "chrome.share": "Rhannu",
    "chrome.textSize": "Maint testun",
    "share.copyLink": "Copïo dolen",
    "share.linkCopied": "Dolen wedi'i chopïo",
    "share.copyFailed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "dashboard.title": "Dangosfwrdd",
    "dashboard.service": "Gwasanaeth:",
    "dashboard.recentActivity": "Gweithgaredd diweddar",
    "dashboard.noRecent": "Dim cofnodion archwilio diweddar.",
    "events.title": "Digwyddiadau",
    "events.new": "Digwyddiad newydd",
    "events.searchPlaceholder": "Chwilio yn ôl enw, trefnydd, dynodydd…",
    "events.filter.from": "O",
    "events.filter.to": "I",
    "events.filter.status": "Statws",
    "events.filter.type": "Math",
    "events.filter.any": "unrhyw",
    "events.filter.fuzzy": "Aneglur",
    "events.filter.apply": "Cymhwyso hidlwyr",
    "events.loading": "Yn llwytho…",
    "events.count.one": "{n} digwyddiad",
    "events.count.other": "{n} digwyddiad",
    "grid.id": "ID",
    "grid.name": "Enw",
    "grid.start": "Dechrau",
    "grid.type": "Math",
    "grid.status": "Statws",
    "grid.mode": "Modd",
    "new.title": "Digwyddiad newydd",
    "new.create": "Creu",
    "new.duplicatesTitle": "Dyblygiadau posibl",
    "new.duplicatesDetected":
      "Canfuwyd dyblygiadau ({n}) — adolygwch isod cyn ailgyflwyno.",
    "match.title": "Gwiriad cydweddu",
    "match.name": "Enw",
    "match.threshold": "Trothwy",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "Dechrau",
    "match.end": "Diwedd",
    "match.organizerName": "Enw'r trefnydd",
    "match.find": "Canfod cyfatebiaethau",
    "match.matching": "Yn cydweddu…",
    "merge.title": "Uno digwyddiadau",
    "merge.mainId": "ID y prif ddigwyddiad",
    "merge.mainIdHint": "Y cofnod sy'n goroesi",
    "merge.dupId": "ID y digwyddiad dyblyg",
    "merge.dupIdHint": "Caiff ei feddal-ddileu",
    "merge.reason": "Rheswm",
    "merge.reasonHint": "Cofnodir yn llwybr archwilio'r uno",
    "merge.reasonPlaceholder": "Dyblyg wedi'i gadarnhau",
    "merge.loadPreview": "Llwytho rhagolwg",
    "merge.merge": "Uno",
    "merge.merging": "Yn uno…",
    "merge.bothIdsRequired": "Mae angen y ddau ID",
    "merge.idsMustDiffer": "Rhaid i'r prif un a'r dyblyg fod yn wahanol",
    "merge.confirm":
      "Uno {dup}… i mewn i {main}…?\nMae hyn yn meddal-ddileu'r dyblyg.",
    "merge.previewTitle": "Rhagolwg",
    "merge.preview.main": "Prif",
    "merge.preview.duplicate": "Dyblyg",
    "merge.preview.none": "—",
    "merge.preview.noDate": "dim dyddiad",
    "merge.completedTitle": "Uno wedi'i gwblhau",
    "merge.completedBody": "Crëwyd cofnod uno {id} am {at}.",
    "merge.viewMerged": "Gweld y prif ddigwyddiad unedig",
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
    "detail.identity": "Hunaniaeth",
    "detail.id": "ID",
    "detail.start": "Dechrau",
    "detail.end": "Diwedd",
    "detail.status": "Statws",
    "detail.type": "Math",
    "detail.mode": "Modd",
    "detail.timeZone": "Cylch amser",
    "detail.duration": "Hyd",
    "detail.description": "Disgrifiad",
    "detail.empty": "—",
    "detail.loc.place": "Lle",
    "detail.loc.address": "Cyfeiriad",
    "detail.loc.virtual": "Rhithwir",
    "detail.loc.text": "Testun",
    "detail.location": "Lleoliad",
    "detail.organizers": "Trefnwyr",
    "detail.performers": "Perfformwyr",
    "detail.identifiers": "Dynodyddion",
    "detail.offers": "Cynigion",
    "detail.ticket": "Tocyn",
    "detail.confirmDelete":
      "Meddal-ddileu'r digwyddiad hwn? Ni ellir ei ddadwneud trwy'r rhyngwyneb.",
    "edit.title": "Golygu digwyddiad",
    "edit.cancel": "Canslo",
    "edit.loading": "Yn llwytho…",
    "edit.save": "Cadw newidiadau",
    "audit.title": "Cofnod archwilio",
    "audit.back": "Yn ôl i'r digwyddiad",
    "audit.loading": "Yn llwytho…",
    "audit.none": "Dim cofnodion archwilio.",
    "audit.by": "gan",
    "audit.payload": "Llwyth",
    "form.name": "Enw",
    "form.required": "Gofynnol",
    "form.eventType": "Math o ddigwyddiad",
    "form.start": "Dechrau",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "Diwedd",
    "form.endAfterStart": "Rhaid i'r diwedd fod yn ≥ y dechrau",
    "form.doorTime": "Amser agor y drysau",
    "form.doorBeforeStart": "Rhaid i amser y drysau fod yn ≤ y dechrau",
    "form.status": "Statws",
    "form.attendanceMode": "Modd presenoldeb",
    "form.timeZone": "Cylch amser",
    "form.timeZoneHint": "IANA, e.e. America/Los_Angeles",
    "form.allDay": "Trwy'r dydd",
    "form.no": "Na",
    "form.yes": "Ie",
    "form.description": "Disgrifiad",
    "form.url": "URL",
    "form.duration": "Hyd",
    "form.durationHint": "ISO 8601, e.e. PT1H30M",
    "form.maxCapacityTotal": "Uchafswm capasiti (cyfanswm)",
    "form.maxPhysical": "Uchafswm corfforol",
    "form.maxVirtual": "Uchafswm rhithwir",
    "form.keywords": "Allweddeiriau",
    "form.keywordsHint": "Wedi'u gwahanu â choma",
    "form.languages": "Ieithoedd",
    "form.languagesHint": "ISO 639-1, e.e. en fr",
    "form.saving": "Yn cadw…",
    "form.save": "Cadw",
    "form.reset": "Ailosod",
    "search.placeholder": "Chwilio…",
    "search.submit": "Chwilio",
    "results.title": "Canlyniadau cydweddu",
    "results.none": "Dim ymgeiswyr.",
    "results.breakdown": "Dadansoddiad sgôr",
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
    "splash.hero.title": "Un cofnod dibynadwy ar gyfer pob digwyddiad",
    "splash.hero.subtitle":
      "Cofrestrwch ddigwyddiadau unwaith, dewch o hyd iddynt yn ôl enw neu ddyddiad, a rhwystrwch ddyblygu cyn iddo ddechrau, gyda llwybr archwilio llawn wedi'i gynnwys.",
    "splash.benefits.1.title": "Llai o ddyblygu",
    "splash.benefits.1.body":
      "Wrth greu digwyddiad, cewch rybudd am ddyblygion tebygol cyn ei gadw.",
    "splash.benefits.2.title": "Dod o hyd i ddigwyddiadau'n gyflym",
    "splash.benefits.2.body":
      "Chwiliwch yn ôl enw gyda chydweddu bras, yna hidlwch yn ôl dyddiad, statws a math.",
    "splash.benefits.3.title": "Un cofnod glân",
    "splash.benefits.3.body":
      "Unwch ddyblygion cadarnhaol yn un digwyddiad a chadwch yr hanes.",
    "splash.benefits.4.title": "Cynllunio gyda chalendr",
    "splash.benefits.4.body":
      "Gwelwch ddigwyddiadau ar galendr a llusgwch un i slot newydd i'w aildrefnu.",
    "splash.benefits.5.title": "Gwybod beth a newidiodd",
    "splash.benefits.5.body":
      "Mae gan bob digwyddiad ei gofnod archwilio ei hun yn dangos pwy newidiodd beth a phryd.",
    "splash.benefits.6.title": "Yn eich iaith chi",
    "splash.benefits.6.body":
      "Defnyddiwch yr ap yn Gymraeg, Saesneg, Arabeg, Sbaeneg, Ffrangeg, Hindi neu Tsieinëeg.",
    "splash.features.1.title": "Cofnodion digwyddiad cyfoethog",
    "splash.features.1.body":
      "Ffenestr amser, statws, math, modd presenoldeb, cylch amser a disgrifiad mewn un lle.",
    "splash.features.2.title": "Lleoedd a phobl",
    "splash.features.2.body":
      "Cofnodwch leoliadau, trefnwyr, perfformwyr, dynodwyr a chynigion tocynnau.",
    "splash.features.3.title": "Gwiriad cydweddu",
    "splash.features.3.body":
      "Sgoriwch digwyddiad damcaniaethol yn erbyn y mynegai cyn ei greu.",
    "splash.features.4.title": "Uno dan arweiniad",
    "splash.features.4.body":
      "Cymharwch ddau ddigwyddiad ochr yn ochr, yna unwch gyda rheswm wedi'i gofnodi.",
    "splash.features.5.title": "Golwg calendr",
    "splash.features.5.body":
      "Calendr llusgo a gollwng dros ffenestr amser pob digwyddiad.",
    "splash.features.6.title": "Themâu a maint testun",
    "splash.features.6.body":
      "Dewiswch thema lliw a maint testun cyfforddus, a gofir ar yr ymweliad nesaf.",
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
      "Taith dywys drwy'r gofrestr Digwyddiadau: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o gofrestru digwyddiad i uno dyblygion ac aildrefnu ar y calendr.",
    "tour.s1.title": "Cofrestru digwyddiad",
    "tour.s1.summary":
      "Crëwch ddigwyddiad gyda'i ffenestr amser, ei statws, ei fath a'i ddull mynychu. Caiff dyblygion tebygol eu nodi cyn cadw'r digwyddiad.",
    "tour.s1.step.1":
      "Dewiswch Digwyddiad newydd yn y ddewislen, neu pwyswch Digwyddiad newydd ar y rhestr Digwyddiadau.",
    "tour.s1.step.2":
      "Llenwch Enw a Dechrau, sy'n ofynnol, yna ychwanegwch Diwedd, Amser agor y drysau, Statws, Modd presenoldeb a Chylchfa amser. Ni all Diwedd fod cyn Dechrau, ac ni all Amser agor y drysau fod ar ôl Dechrau.",
    "tour.s1.step.3":
      "Ychwanegwch Disgrifiad, URL, Hyd, terfynau capasiti, Geiriau allweddol ac Ieithoedd os dymunwch, yna pwyswch Creu.",
    "tour.s1.step.4":
      "Os yw'r gofrestr yn canfod dyblygion tebygol, mae panel Dyblygiadau posibl yn eu rhestru gyda'u sgorau. Adolygwch nhw cyn ailgyflwyno.",
    "tour.s2.title": "Dod o hyd i ddigwyddiadau",
    "tour.s2.summary":
      "Chwiliwch y gofrestr yn ôl enw, trefnydd neu ddynodwr, yna cyfyngwch y rhestr yn ôl dyddiad, statws a math.",
    "tour.s2.step.1":
      "Agorwch Digwyddiadau yn y ddewislen; mae'r rhestr yn llwytho gyda blwch chwilio a chyfrif o'r digwyddiadau sy'n cyfateb.",
    "tour.s2.step.2":
      "Teipiwch yn y blwch chwilio, er enghraifft enw, trefnydd neu ddynodwr, a phwyswch Chwilio.",
    "tour.s2.step.3":
      "Ticiwch Aneglur i oddef gwahaniaethau sillafu, a gosodwch O, I, Statws a Math, yna pwyswch Cymhwyso hidlwyr.",
    "tour.s2.step.4":
      "Dewiswch res yn y grid i agor tudalen manylion y digwyddiad hwnnw.",
    "tour.s3.title": "Gwirio am gydweddiadau",
    "tour.s3.summary":
      "Sgoriwch fanylion digwyddiad damcaniaethol yn erbyn y gofrestr heb greu dim, i weld beth sydd eisoes yn bodoli.",
    "tour.s3.step.1": "Agorwch Gwiriad cydweddu yn y ddewislen.",
    "tour.s3.step.2":
      "Rhowch Enw (gofynnol) ac, os ydych yn eu gwybod, Dechrau, Diwedd ac Enw'r trefnydd.",
    "tour.s3.step.3":
      "Gosodwch y Trothwy rhwng 0.0 ac 1.0, yna pwyswch Canfod cyfatebiaethau.",
    "tour.s3.step.4":
      "Darllenwch y Canlyniadau cydweddu: mae pob ymgeisydd yn dangos ei sgôr, ac mae'r Dadansoddiad sgôr yn dangos sut y cyrhaeddwyd ato.",
    "tour.s4.title": "Uno dyblygion",
    "tour.s4.summary":
      "Plygwch ddyblyg cadarnhaol i'r digwyddiad rydych yn ei gadw, gyda'r rheswm wedi'i gofnodi yn y llwybr archwilio.",
    "tour.s4.step.1":
      "Agorwch Uno yn y ddewislen a rhowch ID y prif ddigwyddiad (y cofnod sy'n goroesi) ac ID y digwyddiad dyblyg.",
    "tour.s4.step.2":
      "Pwyswch Llwytho rhagolwg i weld y ddau gofnod ochr yn ochr. Rhaid rhoi'r ddau ID a rhaid iddynt fod yn wahanol.",
    "tour.s4.step.3":
      "Rhowch Reswm, a gofnodir yn llwybr archwilio'r uno, yna pwyswch Uno a chadarnhewch yr anogwr.",
    "tour.s4.step.4":
      "Caiff y dyblyg ei ddileu'n feddal, mae neges Uno wedi'i gwblhau yn dangos cofnod yr uno, ac mae Gweld y prif ddigwyddiad unedig yn agor y cofnod sy'n goroesi.",
    "tour.s5.title": "Cynllunio ar y calendr",
    "tour.s5.summary":
      "Gwelwch ffenestri amser digwyddiadau mewn golygon mis, wythnos a diwrnod, ac aildrefnwch drwy lusgo.",
    "tour.s5.step.1":
      "Agorwch Calendr yn y ddewislen i weld digwyddiadau cofrestredig wedi'u gosod yn ôl eu hamserau dechrau a gorffen.",
    "tour.s5.step.2": "Newidiwch rhwng y golygon mis, wythnos a diwrnod.",
    "tour.s5.step.3":
      "Llusgwch ddigwyddiad i slot newydd; caiff y newid ei gadw yng nghofnod y digwyddiad drwy'r diweddariad arferol, felly nid copi ar wahân yw'r calendr.",
    "tour.s5.step.4": "Dewiswch ddigwyddiad i agor ei dudalen manylion.",
    "tour.s6.title": "Adolygu, golygu ac archwilio digwyddiad",
    "tour.s6.summary":
      "Agorwch un digwyddiad i ddarllen popeth a gofnodwyd amdano, ei gywiro, gweld pwy a'i newidiodd, ei fasgio neu ei allforio.",
    "tour.s6.step.1":
      "Ar dudalen manylion digwyddiad, darllenwch Hunaniaeth, Lleoliad, Trefnwyr, Perfformwyr, Dynodwyr a Chynigion lle maent wedi'u cofnodi.",
    "tour.s6.step.2":
      "Dewiswch Golygu i newid y cofnod a Chadw newidiadau, neu Dileu i'w ddileu'n feddal ar ôl cadarnhau'r anogwr.",
    "tour.s6.step.3":
      "Dewiswch Archwilio i agor y Cofnod archwilio: mae pob cofnod yn dangos pwy wnaeth y newid a'i lwyth.",
    "tour.s6.step.4":
      "Defnyddiwch Dangos wedi'i guddio i weld y fersiwn wedi'i golygu, neu Allforio data (GDPR) i lawrlwytho'r cofnod.",
    "signin.sso": "Mewngofnodi gydag SSO",
  },
  "en-001": {
    "nav.calendar": "Calendar",
    // Layout / chrome
    brand: "Event",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Dashboard",
    "nav.events": "Events",
    "nav.newEvent": "New event",
    "nav.matchCheck": "Match check",
    "nav.merge": "Merge",
    "nav.toggle": "Toggle navigation",
    "chrome.theme": "Theme",
    "chrome.language": "Language",
    "chrome.share": "Share",
    "chrome.textSize": "Text size",
    "share.copyLink": "Copy Link",
    "share.linkCopied": "Link copied",
    "share.copyFailed": "Could not copy — copy it from the address bar",
    // Dashboard
    "dashboard.title": "Dashboard",
    "dashboard.service": "Service:",
    "dashboard.recentActivity": "Recent activity",
    "dashboard.noRecent": "No recent audit entries.",
    // Events list
    "events.title": "Events",
    "events.new": "New event",
    "events.searchPlaceholder": "Search by name, organizer, identifier…",
    "events.filter.from": "From",
    "events.filter.to": "To",
    "events.filter.status": "Status",
    "events.filter.type": "Type",
    "events.filter.any": "any",
    "events.filter.fuzzy": "Fuzzy",
    "events.filter.apply": "Apply filters",
    "events.loading": "Loading…",
    "events.count.one": "{n} event",
    "events.count.other": "{n} events",
    // Grid headers
    "grid.id": "ID",
    "grid.name": "Name",
    "grid.start": "Start",
    "grid.type": "Type",
    "grid.status": "Status",
    "grid.mode": "Mode",
    // New event
    "new.title": "New event",
    "new.create": "Create",
    "new.duplicatesTitle": "Possible duplicates",
    "new.duplicatesDetected":
      "Duplicates detected ({n}) — review below before resubmitting.",
    // Match check
    "match.title": "Match check",
    "match.name": "Name",
    "match.threshold": "Threshold",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "Start",
    "match.end": "End",
    "match.organizerName": "Organizer name",
    "match.find": "Find matches",
    "match.matching": "Matching…",
    // Merge
    "merge.title": "Merge events",
    "merge.mainId": "Main event ID",
    "merge.mainIdHint": "The surviving record",
    "merge.dupId": "Duplicate event ID",
    "merge.dupIdHint": "Will be soft-deleted",
    "merge.reason": "Reason",
    "merge.reasonHint": "Recorded in the merge audit trail",
    "merge.reasonPlaceholder": "Confirmed duplicate",
    "merge.loadPreview": "Load preview",
    "merge.merge": "Merge",
    "merge.merging": "Merging…",
    "merge.bothIdsRequired": "Both IDs required",
    "merge.idsMustDiffer": "Main and duplicate must differ",
    "merge.confirm":
      "Merge {dup}… into {main}…?\nThis soft-deletes the duplicate.",
    "merge.previewTitle": "Preview",
    "merge.preview.main": "Main",
    "merge.preview.duplicate": "Duplicate",
    "merge.preview.none": "—",
    "merge.preview.noDate": "no date",
    "merge.completedTitle": "Merge completed",
    "merge.completedBody": "Merge record {id} created at {at}.",
    "merge.viewMerged": "View merged main event",
    // Event detail
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
    "detail.identity": "Identity",
    "detail.id": "ID",
    "detail.start": "Start",
    "detail.end": "End",
    "detail.status": "Status",
    "detail.type": "Type",
    "detail.mode": "Mode",
    "detail.timeZone": "Time zone",
    "detail.duration": "Duration",
    "detail.description": "Description",
    "detail.empty": "—",
    "detail.loc.place": "Place",
    "detail.loc.address": "Address",
    "detail.loc.virtual": "Virtual",
    "detail.loc.text": "Text",
    "detail.location": "Location",
    "detail.organizers": "Organizers",
    "detail.performers": "Performers",
    "detail.identifiers": "Identifiers",
    "detail.offers": "Offers",
    "detail.ticket": "Ticket",
    "detail.confirmDelete":
      "Soft-delete this event? This cannot be undone via the UI.",
    // Edit event
    "edit.title": "Edit event",
    "edit.cancel": "Cancel",
    "edit.loading": "Loading…",
    "edit.save": "Save changes",
    // Audit page
    "audit.title": "Audit log",
    "audit.back": "Back to event",
    "audit.loading": "Loading…",
    "audit.none": "No audit entries.",
    "audit.by": "by",
    "audit.payload": "Payload",
    // Event form
    "form.name": "Name",
    "form.required": "Required",
    "form.eventType": "Event type",
    "form.start": "Start",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "End",
    "form.endAfterStart": "End must be ≥ start",
    "form.doorTime": "Door time",
    "form.doorBeforeStart": "Door time must be ≤ start",
    "form.status": "Status",
    "form.attendanceMode": "Attendance mode",
    "form.timeZone": "Time zone",
    "form.timeZoneHint": "IANA, e.g. America/Los_Angeles",
    "form.allDay": "All day",
    "form.no": "No",
    "form.yes": "Yes",
    "form.description": "Description",
    "form.url": "URL",
    "form.duration": "Duration",
    "form.durationHint": "ISO 8601, e.g. PT1H30M",
    "form.maxCapacityTotal": "Max capacity (total)",
    "form.maxPhysical": "Max physical",
    "form.maxVirtual": "Max virtual",
    "form.keywords": "Keywords",
    "form.keywordsHint": "Comma-separated",
    "form.languages": "Languages",
    "form.languagesHint": "ISO 639-1, e.g. en fr",
    "form.saving": "Saving…",
    "form.save": "Save",
    "form.reset": "Reset",
    // Search box
    "search.placeholder": "Search…",
    "search.submit": "Search",
    // Match results list
    "results.title": "Match results",
    "results.none": "No candidates.",
    "results.breakdown": "Score breakdown",
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
    "splash.hero.title": "One trusted record for every event",
    "splash.hero.subtitle":
      "Register events once, find them by name or date, and stop duplicates before they start, with a full audit trail built in.",
    "splash.benefits.1.title": "Fewer duplicates",
    "splash.benefits.1.body":
      "Creating an event warns you about likely duplicates before it is saved.",
    "splash.benefits.2.title": "Find events fast",
    "splash.benefits.2.body":
      "Search by name with fuzzy matching, then filter by date, status and type.",
    "splash.benefits.3.title": "One clean record",
    "splash.benefits.3.body":
      "Merge confirmed duplicates into a single event and keep the history.",
    "splash.benefits.4.title": "Plan by calendar",
    "splash.benefits.4.body":
      "See events on a calendar and drag one to a new slot to reschedule it.",
    "splash.benefits.5.title": "Know what changed",
    "splash.benefits.5.body":
      "Every event has its own audit log showing who changed what and when.",
    "splash.benefits.6.title": "Works in your language",
    "splash.benefits.6.body":
      "Use the app in Arabic, Welsh, English, Spanish, French, Hindi or Chinese.",
    "splash.features.1.title": "Rich event records",
    "splash.features.1.body":
      "Time window, status, type, attendance mode, time zone and description in one place.",
    "splash.features.2.title": "Places and people",
    "splash.features.2.body":
      "Record locations, organizers, performers, identifiers and ticket offers.",
    "splash.features.3.title": "Match check",
    "splash.features.3.body":
      "Score a hypothetical event against the index before you create it.",
    "splash.features.4.title": "Guided merging",
    "splash.features.4.body":
      "Compare two events side by side, then merge with a recorded reason.",
    "splash.features.5.title": "Calendar view",
    "splash.features.5.body":
      "A drag-and-drop calendar over every event's time window.",
    "splash.features.6.title": "Themes and text size",
    "splash.features.6.body":
      "Choose a colour theme and a comfortable text size, remembered next visit.",
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
      "A guided walkthrough of the Event registry: what each screen does and the steps to use it, from registering an event to merging duplicates and rescheduling on the calendar.",
    "tour.s1.title": "Register an event",
    "tour.s1.summary":
      "Create an event with its time window, status, type and attendance mode. Likely duplicates are flagged before the event is saved.",
    "tour.s1.step.1":
      "Choose New event in the menu, or press New event on the Events list.",
    "tour.s1.step.2":
      "Fill in Name and Start, which are required, then add End, Door time, Status, Attendance mode and Time zone. End cannot be before Start, and Door time cannot be after it.",
    "tour.s1.step.3":
      "Optionally add a Description, URL, Duration, capacity limits, Keywords and Languages, then press Create.",
    "tour.s1.step.4":
      "If the registry finds likely duplicates, a Possible duplicates panel lists them with their scores. Review them before you resubmit.",
    "tour.s2.title": "Find events",
    "tour.s2.summary":
      "Search the registry by name, organizer or identifier, then narrow the list by date, status and type.",
    "tour.s2.step.1":
      "Open Events in the menu; the list loads with a search box and a count of matching events.",
    "tour.s2.step.2":
      "Type in the search box, for example a name, organizer or identifier, and press Search.",
    "tour.s2.step.3":
      "Tick Fuzzy to tolerate spelling differences, and set From, To, Status and Type, then press Apply filters.",
    "tour.s2.step.4":
      "Select a row in the grid to open that event's detail page.",
    "tour.s3.title": "Check for matches",
    "tour.s3.summary":
      "Score hypothetical event details against the registry without creating anything, to see what already exists.",
    "tour.s3.step.1": "Open Match check in the menu.",
    "tour.s3.step.2":
      "Enter a Name (required) and, if you know them, Start, End and Organizer name.",
    "tour.s3.step.3":
      "Set the Threshold between 0.0 and 1.0, then press Find matches.",
    "tour.s3.step.4":
      "Read the Match results: each candidate shows its score, and the Score breakdown shows how it was reached.",
    "tour.s4.title": "Merge duplicates",
    "tour.s4.summary":
      "Fold a confirmed duplicate into the event you are keeping, with the reason recorded in the audit trail.",
    "tour.s4.step.1":
      "Open Merge in the menu and enter the Main event ID (the surviving record) and the Duplicate event ID.",
    "tour.s4.step.2":
      "Press Load preview to see the two records side by side. The IDs must both be given and must differ.",
    "tour.s4.step.3":
      "Give a Reason, which is recorded in the merge audit trail, then press Merge and confirm the prompt.",
    "tour.s4.step.4":
      "The duplicate is soft-deleted, a Merge completed message shows the merge record, and View merged main event opens the survivor.",
    "tour.s5.title": "Plan on the calendar",
    "tour.s5.summary":
      "See event time windows in month, week and day views, and reschedule by dragging.",
    "tour.s5.step.1":
      "Open Calendar in the menu to see registered events placed by their start and end times.",
    "tour.s5.step.2": "Switch between the month, week and day views.",
    "tour.s5.step.3":
      "Drag an event to a new slot; the change is saved to the event record through the normal update, so the calendar is not a separate copy.",
    "tour.s5.step.4": "Select an event to open its detail page.",
    "tour.s6.title": "Review, edit and audit an event",
    "tour.s6.summary":
      "Open one event to read everything recorded about it, correct it, see who changed it, mask it or export it.",
    "tour.s6.step.1":
      "On an event's detail page, read Identity, Location, Organizers, Performers, Identifiers and Offers where they are recorded.",
    "tour.s6.step.2":
      "Choose Edit to change the record and Save changes, or Delete to soft-delete it after confirming the prompt.",
    "tour.s6.step.3":
      "Choose Audit to open the Audit log: each entry shows who made the change and its payload.",
    "tour.s6.step.4":
      "Use Show masked to view the redacted version, or Export data (GDPR) to download the record.",
    "signin.sso": "Sign in with SSO",
  },
  "es-001": {
    "nav.calendar": "Calendario",
    brand: "Evento",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Panel",
    "nav.events": "Eventos",
    "nav.newEvent": "Nuevo evento",
    "nav.matchCheck": "Comprobar coincidencias",
    "nav.merge": "Fusionar",
    "nav.toggle": "Alternar navegación",
    "chrome.theme": "Tema",
    "chrome.language": "Idioma",
    "chrome.share": "Compartir",
    "chrome.textSize": "Tamaño del texto",
    "share.copyLink": "Copiar enlace",
    "share.linkCopied": "Enlace copiado",
    "share.copyFailed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "dashboard.title": "Panel",
    "dashboard.service": "Servicio:",
    "dashboard.recentActivity": "Actividad reciente",
    "dashboard.noRecent": "No hay entradas de auditoría recientes.",
    "events.title": "Eventos",
    "events.new": "Nuevo evento",
    "events.searchPlaceholder":
      "Buscar por nombre, organizador, identificador…",
    "events.filter.from": "Desde",
    "events.filter.to": "Hasta",
    "events.filter.status": "Estado",
    "events.filter.type": "Tipo",
    "events.filter.any": "cualquiera",
    "events.filter.fuzzy": "Difusa",
    "events.filter.apply": "Aplicar filtros",
    "events.loading": "Cargando…",
    "events.count.one": "{n} evento",
    "events.count.other": "{n} eventos",
    "grid.id": "ID",
    "grid.name": "Nombre",
    "grid.start": "Inicio",
    "grid.type": "Tipo",
    "grid.status": "Estado",
    "grid.mode": "Modo",
    "new.title": "Nuevo evento",
    "new.create": "Crear",
    "new.duplicatesTitle": "Posibles duplicados",
    "new.duplicatesDetected":
      "Duplicados detectados ({n}) — revísalos abajo antes de reenviar.",
    "match.title": "Comprobar coincidencias",
    "match.name": "Nombre",
    "match.threshold": "Umbral",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "Inicio",
    "match.end": "Fin",
    "match.organizerName": "Nombre del organizador",
    "match.find": "Buscar coincidencias",
    "match.matching": "Buscando…",
    "merge.title": "Fusionar eventos",
    "merge.mainId": "ID del evento principal",
    "merge.mainIdHint": "El registro que sobrevive",
    "merge.dupId": "ID del evento duplicado",
    "merge.dupIdHint": "Se eliminará de forma reversible",
    "merge.reason": "Motivo",
    "merge.reasonHint": "Registrado en el historial de auditoría de la fusión",
    "merge.reasonPlaceholder": "Duplicado confirmado",
    "merge.loadPreview": "Cargar vista previa",
    "merge.merge": "Fusionar",
    "merge.merging": "Fusionando…",
    "merge.bothIdsRequired": "Se requieren ambos ID",
    "merge.idsMustDiffer": "El principal y el duplicado deben ser distintos",
    "merge.confirm":
      "¿Fusionar {dup}… en {main}…?\nEsto elimina el duplicado de forma reversible.",
    "merge.previewTitle": "Vista previa",
    "merge.preview.main": "Principal",
    "merge.preview.duplicate": "Duplicado",
    "merge.preview.none": "—",
    "merge.preview.noDate": "sin fecha",
    "merge.completedTitle": "Fusión completada",
    "merge.completedBody": "Registro de fusión {id} creado el {at}.",
    "merge.viewMerged": "Ver el evento principal fusionado",
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
    "detail.identity": "Identidad",
    "detail.id": "ID",
    "detail.start": "Inicio",
    "detail.end": "Fin",
    "detail.status": "Estado",
    "detail.type": "Tipo",
    "detail.mode": "Modo",
    "detail.timeZone": "Zona horaria",
    "detail.duration": "Duración",
    "detail.description": "Descripción",
    "detail.empty": "—",
    "detail.loc.place": "Lugar",
    "detail.loc.address": "Dirección",
    "detail.loc.virtual": "Virtual",
    "detail.loc.text": "Texto",
    "detail.location": "Ubicación",
    "detail.organizers": "Organizadores",
    "detail.performers": "Artistas",
    "detail.identifiers": "Identificadores",
    "detail.offers": "Ofertas",
    "detail.ticket": "Entrada",
    "detail.confirmDelete":
      "¿Eliminar este evento de forma reversible? No se puede deshacer desde la interfaz.",
    "edit.title": "Editar evento",
    "edit.cancel": "Cancelar",
    "edit.loading": "Cargando…",
    "edit.save": "Guardar cambios",
    "audit.title": "Registro de auditoría",
    "audit.back": "Volver al evento",
    "audit.loading": "Cargando…",
    "audit.none": "No hay entradas de auditoría.",
    "audit.by": "por",
    "audit.payload": "Carga útil",
    "form.name": "Nombre",
    "form.required": "Obligatorio",
    "form.eventType": "Tipo de evento",
    "form.start": "Inicio",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "Fin",
    "form.endAfterStart": "El fin debe ser ≥ el inicio",
    "form.doorTime": "Hora de apertura",
    "form.doorBeforeStart": "La hora de apertura debe ser ≤ el inicio",
    "form.status": "Estado",
    "form.attendanceMode": "Modo de asistencia",
    "form.timeZone": "Zona horaria",
    "form.timeZoneHint": "IANA, p. ej. America/Los_Angeles",
    "form.allDay": "Todo el día",
    "form.no": "No",
    "form.yes": "Sí",
    "form.description": "Descripción",
    "form.url": "URL",
    "form.duration": "Duración",
    "form.durationHint": "ISO 8601, p. ej. PT1H30M",
    "form.maxCapacityTotal": "Aforo máximo (total)",
    "form.maxPhysical": "Máximo presencial",
    "form.maxVirtual": "Máximo virtual",
    "form.keywords": "Palabras clave",
    "form.keywordsHint": "Separadas por comas",
    "form.languages": "Idiomas",
    "form.languagesHint": "ISO 639-1, p. ej. en fr",
    "form.saving": "Guardando…",
    "form.save": "Guardar",
    "form.reset": "Restablecer",
    "search.placeholder": "Buscar…",
    "search.submit": "Buscar",
    "results.title": "Resultados de coincidencia",
    "results.none": "Sin candidatos.",
    "results.breakdown": "Desglose de puntuación",
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
    "splash.hero.title": "Un registro fiable para cada evento",
    "splash.hero.subtitle":
      "Registra los eventos una sola vez, encuéntralos por nombre o fecha y evita los duplicados antes de que aparezcan, con auditoría completa integrada.",
    "splash.benefits.1.title": "Menos duplicados",
    "splash.benefits.1.body":
      "Al crear un evento se te avisa de posibles duplicados antes de guardarlo.",
    "splash.benefits.2.title": "Encuentra eventos rápido",
    "splash.benefits.2.body":
      "Busca por nombre con coincidencia aproximada y filtra por fecha, estado y tipo.",
    "splash.benefits.3.title": "Un registro limpio",
    "splash.benefits.3.body":
      "Fusiona los duplicados confirmados en un solo evento y conserva el historial.",
    "splash.benefits.4.title": "Planifica con el calendario",
    "splash.benefits.4.body":
      "Consulta los eventos en un calendario y arrastra uno a otro horario para reprogramarlo.",
    "splash.benefits.5.title": "Sabe qué cambió",
    "splash.benefits.5.body":
      "Cada evento tiene su propio registro de auditoría con quién cambió qué y cuándo.",
    "splash.benefits.6.title": "En tu idioma",
    "splash.benefits.6.body":
      "Usa la aplicación en árabe, galés, inglés, español, francés, hindi o chino.",
    "splash.features.1.title": "Registros de eventos completos",
    "splash.features.1.body":
      "Periodo, estado, tipo, modo de asistencia, zona horaria y descripción en un solo lugar.",
    "splash.features.2.title": "Lugares y personas",
    "splash.features.2.body":
      "Registra ubicaciones, organizadores, artistas, identificadores y ofertas de entradas.",
    "splash.features.3.title": "Comprobar coincidencias",
    "splash.features.3.body":
      "Puntúa un evento hipotético frente al índice antes de crearlo.",
    "splash.features.4.title": "Fusión guiada",
    "splash.features.4.body":
      "Compara dos eventos lado a lado y fusiónalos indicando el motivo.",
    "splash.features.5.title": "Vista de calendario",
    "splash.features.5.body":
      "Un calendario de arrastrar y soltar sobre el periodo de cada evento.",
    "splash.features.6.title": "Temas y tamaño del texto",
    "splash.features.6.body":
      "Elige un tema de color y un tamaño de texto cómodo; se recuerdan en la próxima visita.",
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
      "Un recorrido guiado por el registro de eventos: qué hace cada pantalla y los pasos para usarla, desde registrar un evento hasta fusionar duplicados y reprogramar en el calendario.",
    "tour.s1.title": "Registrar un evento",
    "tour.s1.summary":
      "Crea un evento con su ventana de tiempo, estado, tipo y modalidad de asistencia. Los posibles duplicados se señalan antes de guardar el evento.",
    "tour.s1.step.1":
      "Elige Nuevo evento en el menú, o pulsa Nuevo evento en la lista de eventos.",
    "tour.s1.step.2":
      "Rellena Nombre e Inicio, que son obligatorios, y luego añade Fin, Hora de apertura, Estado, Modo de asistencia y Zona horaria. El fin no puede ser anterior al inicio, ni la hora de apertura posterior a él.",
    "tour.s1.step.3":
      "Si quieres, añade Descripción, URL, Duración, límites de aforo, Palabras clave e Idiomas, y pulsa Crear.",
    "tour.s1.step.4":
      "Si el registro encuentra posibles duplicados, un panel de Posibles duplicados los lista con sus puntuaciones. Revísalos antes de volver a enviar.",
    "tour.s2.title": "Encontrar eventos",
    "tour.s2.summary":
      "Busca en el registro por nombre, organizador o identificador y acota la lista por fecha, estado y tipo.",
    "tour.s2.step.1":
      "Abre Eventos en el menú; la lista se carga con un cuadro de búsqueda y el número de eventos coincidentes.",
    "tour.s2.step.2":
      "Escribe en el cuadro de búsqueda, por ejemplo un nombre, organizador o identificador, y pulsa Buscar.",
    "tour.s2.step.3":
      "Marca Difusa para tolerar diferencias de ortografía, define Desde, Hasta, Estado y Tipo, y pulsa Aplicar filtros.",
    "tour.s2.step.4":
      "Selecciona una fila de la cuadrícula para abrir la página de detalle de ese evento.",
    "tour.s3.title": "Comprobar coincidencias",
    "tour.s3.summary":
      "Puntúa los datos de un evento hipotético frente al registro sin crear nada, para ver qué existe ya.",
    "tour.s3.step.1": "Abre Comprobar coincidencias en el menú.",
    "tour.s3.step.2":
      "Introduce un Nombre (obligatorio) y, si los conoces, Inicio, Fin y Nombre del organizador.",
    "tour.s3.step.3":
      "Fija el Umbral entre 0.0 y 1.0 y pulsa Buscar coincidencias.",
    "tour.s3.step.4":
      "Lee los Resultados de coincidencia: cada candidato muestra su puntuación y el Desglose de puntuación indica cómo se obtuvo.",
    "tour.s4.title": "Fusionar duplicados",
    "tour.s4.summary":
      "Incorpora un duplicado confirmado en el evento que conservas, con el motivo registrado en la auditoría.",
    "tour.s4.step.1":
      "Abre Fusionar en el menú e introduce el ID del evento principal (el registro que se conserva) y el ID del evento duplicado.",
    "tour.s4.step.2":
      "Pulsa Cargar vista previa para ver los dos registros lado a lado. Ambos ID son obligatorios y deben ser distintos.",
    "tour.s4.step.3":
      "Indica un Motivo, que queda registrado en la auditoría de la fusión, y pulsa Fusionar y confirma el aviso.",
    "tour.s4.step.4":
      "El duplicado se elimina de forma lógica, un mensaje de Fusión completada muestra el registro de la fusión y Ver evento principal fusionado abre el registro superviviente.",
    "tour.s5.title": "Planificar en el calendario",
    "tour.s5.summary":
      "Consulta las ventanas de tiempo de los eventos en vistas de mes, semana y día, y reprograma arrastrando.",
    "tour.s5.step.1":
      "Abre Calendario en el menú para ver los eventos registrados según sus horas de inicio y fin.",
    "tour.s5.step.2": "Cambia entre las vistas de mes, semana y día.",
    "tour.s5.step.3":
      "Arrastra un evento a un nuevo horario; el cambio se guarda en el registro del evento mediante la actualización normal, así que el calendario no es una copia aparte.",
    "tour.s5.step.4": "Selecciona un evento para abrir su página de detalle.",
    "tour.s6.title": "Revisar, editar y auditar un evento",
    "tour.s6.summary":
      "Abre un evento para leer todo lo registrado, corregirlo, ver quién lo cambió, enmascararlo o exportarlo.",
    "tour.s6.step.1":
      "En la página de detalle de un evento, lee Identidad, Ubicación, Organizadores, Intérpretes, Identificadores y Ofertas donde estén registrados.",
    "tour.s6.step.2":
      "Elige Editar para cambiar el registro y Guardar cambios, o Eliminar para borrarlo de forma lógica tras confirmar el aviso.",
    "tour.s6.step.3":
      "Elige Auditoría para abrir el Registro de auditoría: cada entrada muestra quién hizo el cambio y su contenido.",
    "tour.s6.step.4":
      "Usa Mostrar enmascarado para ver la versión censurada, o Exportar datos (RGPD) para descargar el registro.",
    "signin.sso": "Iniciar sesión con SSO",
  },
  "fr-001": {
    "nav.calendar": "Calendrier",
    brand: "Événement",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "Tableau de bord",
    "nav.events": "Événements",
    "nav.newEvent": "Nouvel événement",
    "nav.matchCheck": "Vérifier les correspondances",
    "nav.merge": "Fusionner",
    "nav.toggle": "Basculer la navigation",
    "chrome.theme": "Thème",
    "chrome.language": "Langue",
    "chrome.share": "Partager",
    "chrome.textSize": "Taille du texte",
    "share.copyLink": "Copier le lien",
    "share.linkCopied": "Lien copié",
    "share.copyFailed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "dashboard.title": "Tableau de bord",
    "dashboard.service": "Service :",
    "dashboard.recentActivity": "Activité récente",
    "dashboard.noRecent": "Aucune entrée d'audit récente.",
    "events.title": "Événements",
    "events.new": "Nouvel événement",
    "events.searchPlaceholder":
      "Rechercher par nom, organisateur, identifiant…",
    "events.filter.from": "Du",
    "events.filter.to": "Au",
    "events.filter.status": "Statut",
    "events.filter.type": "Type",
    "events.filter.any": "tout",
    "events.filter.fuzzy": "Approximatif",
    "events.filter.apply": "Appliquer les filtres",
    "events.loading": "Chargement…",
    "events.count.one": "{n} événement",
    "events.count.other": "{n} événements",
    "grid.id": "ID",
    "grid.name": "Nom",
    "grid.start": "Début",
    "grid.type": "Type",
    "grid.status": "Statut",
    "grid.mode": "Mode",
    "new.title": "Nouvel événement",
    "new.create": "Créer",
    "new.duplicatesTitle": "Doublons possibles",
    "new.duplicatesDetected":
      "Doublons détectés ({n}) — vérifiez ci-dessous avant de renvoyer.",
    "match.title": "Vérifier les correspondances",
    "match.name": "Nom",
    "match.threshold": "Seuil",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "Début",
    "match.end": "Fin",
    "match.organizerName": "Nom de l'organisateur",
    "match.find": "Trouver des correspondances",
    "match.matching": "Recherche…",
    "merge.title": "Fusionner des événements",
    "merge.mainId": "ID de l'événement principal",
    "merge.mainIdHint": "L'enregistrement conservé",
    "merge.dupId": "ID de l'événement en double",
    "merge.dupIdHint": "Sera supprimé de façon réversible",
    "merge.reason": "Motif",
    "merge.reasonHint": "Consigné dans le journal d'audit de la fusion",
    "merge.reasonPlaceholder": "Doublon confirmé",
    "merge.loadPreview": "Charger l'aperçu",
    "merge.merge": "Fusionner",
    "merge.merging": "Fusion…",
    "merge.bothIdsRequired": "Les deux ID sont requis",
    "merge.idsMustDiffer": "Le principal et le doublon doivent être différents",
    "merge.confirm":
      "Fusionner {dup}… dans {main}… ?\nCela supprime le doublon de façon réversible.",
    "merge.previewTitle": "Aperçu",
    "merge.preview.main": "Principal",
    "merge.preview.duplicate": "Doublon",
    "merge.preview.none": "—",
    "merge.preview.noDate": "aucune date",
    "merge.completedTitle": "Fusion terminée",
    "merge.completedBody": "Enregistrement de fusion {id} créé le {at}.",
    "merge.viewMerged": "Voir l'événement principal fusionné",
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
    "detail.identity": "Identité",
    "detail.id": "ID",
    "detail.start": "Début",
    "detail.end": "Fin",
    "detail.status": "Statut",
    "detail.type": "Type",
    "detail.mode": "Mode",
    "detail.timeZone": "Fuseau horaire",
    "detail.duration": "Durée",
    "detail.description": "Description",
    "detail.empty": "—",
    "detail.loc.place": "Lieu",
    "detail.loc.address": "Adresse",
    "detail.loc.virtual": "Virtuel",
    "detail.loc.text": "Texte",
    "detail.location": "Lieu",
    "detail.organizers": "Organisateurs",
    "detail.performers": "Intervenants",
    "detail.identifiers": "Identifiants",
    "detail.offers": "Offres",
    "detail.ticket": "Billet",
    "detail.confirmDelete":
      "Supprimer cet événement de façon réversible ? Cela ne peut pas être annulé via l'interface.",
    "edit.title": "Modifier l'événement",
    "edit.cancel": "Annuler",
    "edit.loading": "Chargement…",
    "edit.save": "Enregistrer les modifications",
    "audit.title": "Journal d'audit",
    "audit.back": "Retour à l'événement",
    "audit.loading": "Chargement…",
    "audit.none": "Aucune entrée d'audit.",
    "audit.by": "par",
    "audit.payload": "Charge utile",
    "form.name": "Nom",
    "form.required": "Requis",
    "form.eventType": "Type d'événement",
    "form.start": "Début",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "Fin",
    "form.endAfterStart": "La fin doit être ≥ le début",
    "form.doorTime": "Heure d'ouverture des portes",
    "form.doorBeforeStart": "L'heure d'ouverture doit être ≤ le début",
    "form.status": "Statut",
    "form.attendanceMode": "Mode de participation",
    "form.timeZone": "Fuseau horaire",
    "form.timeZoneHint": "IANA, p. ex. America/Los_Angeles",
    "form.allDay": "Toute la journée",
    "form.no": "Non",
    "form.yes": "Oui",
    "form.description": "Description",
    "form.url": "URL",
    "form.duration": "Durée",
    "form.durationHint": "ISO 8601, p. ex. PT1H30M",
    "form.maxCapacityTotal": "Capacité maximale (totale)",
    "form.maxPhysical": "Maximum sur place",
    "form.maxVirtual": "Maximum virtuel",
    "form.keywords": "Mots-clés",
    "form.keywordsHint": "Séparés par des virgules",
    "form.languages": "Langues",
    "form.languagesHint": "ISO 639-1, p. ex. en fr",
    "form.saving": "Enregistrement…",
    "form.save": "Enregistrer",
    "form.reset": "Réinitialiser",
    "search.placeholder": "Rechercher…",
    "search.submit": "Rechercher",
    "results.title": "Résultats de correspondance",
    "results.none": "Aucun candidat.",
    "results.breakdown": "Détail du score",
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
    "splash.hero.title": "Un dossier fiable pour chaque événement",
    "splash.hero.subtitle":
      "Enregistrez chaque événement une seule fois, retrouvez-le par nom ou par date et évitez les doublons avant qu'ils n'apparaissent, avec piste d'audit intégrée.",
    "splash.benefits.1.title": "Moins de doublons",
    "splash.benefits.1.body":
      "La création d'un événement signale les doublons probables avant l'enregistrement.",
    "splash.benefits.2.title": "Trouvez vite les événements",
    "splash.benefits.2.body":
      "Recherchez par nom avec correspondance approximative, puis filtrez par date, statut et type.",
    "splash.benefits.3.title": "Un dossier propre",
    "splash.benefits.3.body":
      "Fusionnez les doublons confirmés en un seul événement et conservez l'historique.",
    "splash.benefits.4.title": "Planifiez au calendrier",
    "splash.benefits.4.body":
      "Visualisez les événements dans un calendrier et faites-en glisser un vers un autre créneau pour le reprogrammer.",
    "splash.benefits.5.title": "Sachez ce qui a changé",
    "splash.benefits.5.body":
      "Chaque événement a son journal d'audit indiquant qui a modifié quoi et quand.",
    "splash.benefits.6.title": "Dans votre langue",
    "splash.benefits.6.body":
      "Utilisez l'application en arabe, gallois, anglais, espagnol, français, hindi ou chinois.",
    "splash.features.1.title": "Fiches d'événement complètes",
    "splash.features.1.body":
      "Période, statut, type, mode de participation, fuseau horaire et description au même endroit.",
    "splash.features.2.title": "Lieux et personnes",
    "splash.features.2.body":
      "Enregistrez lieux, organisateurs, artistes, identifiants et offres de billets.",
    "splash.features.3.title": "Vérifier les correspondances",
    "splash.features.3.body":
      "Évaluez un événement hypothétique par rapport à l'index avant de le créer.",
    "splash.features.4.title": "Fusion guidée",
    "splash.features.4.body":
      "Comparez deux événements côte à côte, puis fusionnez en consignant le motif.",
    "splash.features.5.title": "Vue calendrier",
    "splash.features.5.body":
      "Un calendrier en glisser-déposer sur la période de chaque événement.",
    "splash.features.6.title": "Thèmes et taille du texte",
    "splash.features.6.body":
      "Choisissez un thème de couleurs et une taille de texte confortable, mémorisés pour la prochaine visite.",
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
      "Une visite guidée du registre des événements : le rôle de chaque écran et les étapes pour l'utiliser, de l'enregistrement d'un événement à la fusion des doublons et à la replanification dans le calendrier.",
    "tour.s1.title": "Enregistrer un événement",
    "tour.s1.summary":
      "Créez un événement avec sa plage horaire, son statut, son type et son mode de participation. Les doublons probables sont signalés avant l'enregistrement.",
    "tour.s1.step.1":
      "Choisissez Nouvel événement dans le menu, ou appuyez sur Nouvel événement dans la liste des événements.",
    "tour.s1.step.2":
      "Renseignez Nom et Début, obligatoires, puis ajoutez Fin, Heure d'ouverture des portes, Statut, Mode de participation et Fuseau horaire. La fin ne peut précéder le début, ni l'heure d'ouverture le suivre.",
    "tour.s1.step.3":
      "Ajoutez si besoin une Description, une URL, une Durée, des limites de capacité, des Mots-clés et des Langues, puis appuyez sur Créer.",
    "tour.s1.step.4":
      "Si le registre trouve des doublons probables, un panneau Doublons possibles les liste avec leurs scores. Examinez-les avant de soumettre à nouveau.",
    "tour.s2.title": "Trouver des événements",
    "tour.s2.summary":
      "Recherchez dans le registre par nom, organisateur ou identifiant, puis affinez la liste par date, statut et type.",
    "tour.s2.step.1":
      "Ouvrez Événements dans le menu ; la liste se charge avec une zone de recherche et le nombre d'événements correspondants.",
    "tour.s2.step.2":
      "Saisissez dans la zone de recherche, par exemple un nom, un organisateur ou un identifiant, puis appuyez sur Rechercher.",
    "tour.s2.step.3":
      "Cochez Approximatif pour tolérer les écarts d'orthographe, réglez Du, Au, Statut et Type, puis appuyez sur Appliquer les filtres.",
    "tour.s2.step.4":
      "Sélectionnez une ligne de la grille pour ouvrir la page de détail de cet événement.",
    "tour.s3.title": "Vérifier les correspondances",
    "tour.s3.summary":
      "Évaluez les détails d'un événement hypothétique par rapport au registre sans rien créer, pour voir ce qui existe déjà.",
    "tour.s3.step.1": "Ouvrez Vérifier les correspondances dans le menu.",
    "tour.s3.step.2":
      "Saisissez un Nom (obligatoire) et, si vous les connaissez, Début, Fin et Nom de l'organisateur.",
    "tour.s3.step.3":
      "Réglez le Seuil entre 0.0 et 1.0, puis appuyez sur Trouver des correspondances.",
    "tour.s3.step.4":
      "Lisez les Résultats de correspondance : chaque candidat affiche son score et le Détail du score montre comment il a été obtenu.",
    "tour.s4.title": "Fusionner les doublons",
    "tour.s4.summary":
      "Fondez un doublon confirmé dans l'événement que vous conservez, avec le motif consigné dans la piste d'audit.",
    "tour.s4.step.1":
      "Ouvrez Fusionner dans le menu et saisissez l'ID de l'événement principal (la fiche conservée) et l'ID de l'événement en double.",
    "tour.s4.step.2":
      "Appuyez sur Charger l'aperçu pour voir les deux fiches côte à côte. Les deux ID sont requis et doivent être différents.",
    "tour.s4.step.3":
      "Indiquez un Motif, consigné dans la piste d'audit de la fusion, puis appuyez sur Fusionner et confirmez.",
    "tour.s4.step.4":
      "Le doublon est supprimé logiquement, un message Fusion terminée affiche la fiche de fusion et Voir l'événement principal fusionné ouvre la fiche conservée.",
    "tour.s5.title": "Planifier dans le calendrier",
    "tour.s5.summary":
      "Consultez les plages horaires des événements en vues mois, semaine et jour, et replanifiez par glisser-déposer.",
    "tour.s5.step.1":
      "Ouvrez Calendrier dans le menu pour voir les événements enregistrés placés selon leurs heures de début et de fin.",
    "tour.s5.step.2": "Basculez entre les vues mois, semaine et jour.",
    "tour.s5.step.3":
      "Faites glisser un événement vers un nouveau créneau ; la modification est enregistrée dans la fiche par la mise à jour habituelle, le calendrier n'est donc pas une copie distincte.",
    "tour.s5.step.4":
      "Sélectionnez un événement pour ouvrir sa page de détail.",
    "tour.s6.title": "Consulter, modifier et auditer un événement",
    "tour.s6.summary":
      "Ouvrez un événement pour lire tout ce qui y est consigné, le corriger, voir qui l'a modifié, le masquer ou l'exporter.",
    "tour.s6.step.1":
      "Sur la page de détail d'un événement, lisez Identité, Lieu, Organisateurs, Intervenants, Identifiants et Offres lorsqu'ils sont renseignés.",
    "tour.s6.step.2":
      "Choisissez Modifier pour changer la fiche puis Enregistrer les modifications, ou Supprimer pour la supprimer logiquement après confirmation.",
    "tour.s6.step.3":
      "Choisissez Audit pour ouvrir le Journal d'audit : chaque entrée indique qui a fait la modification et son contenu.",
    "tour.s6.step.4":
      "Utilisez Afficher masqué pour voir la version expurgée, ou Exporter les données (RGPD) pour télécharger la fiche.",
    "signin.sso": "Se connecter avec SSO",
  },
  "hi-001": {
    "nav.calendar": "कैलेंडर",
    brand: "इवेंट",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "डैशबोर्ड",
    "nav.events": "इवेंट",
    "nav.newEvent": "नया इवेंट",
    "nav.matchCheck": "मिलान जाँच",
    "nav.merge": "मर्ज करें",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "chrome.theme": "थीम",
    "chrome.language": "भाषा",
    "chrome.share": "साझा करें",
    "chrome.textSize": "टेक्स्ट का आकार",
    "share.copyLink": "लिंक कॉपी करें",
    "share.linkCopied": "लिंक कॉपी हो गया",
    "share.copyFailed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "dashboard.title": "डैशबोर्ड",
    "dashboard.service": "सेवा:",
    "dashboard.recentActivity": "हाल की गतिविधि",
    "dashboard.noRecent": "कोई हालिया ऑडिट प्रविष्टि नहीं।",
    "events.title": "इवेंट",
    "events.new": "नया इवेंट",
    "events.searchPlaceholder": "नाम, आयोजक, पहचानकर्ता से खोजें…",
    "events.filter.from": "से",
    "events.filter.to": "तक",
    "events.filter.status": "स्थिति",
    "events.filter.type": "प्रकार",
    "events.filter.any": "कोई भी",
    "events.filter.fuzzy": "अस्पष्ट",
    "events.filter.apply": "फ़िल्टर लागू करें",
    "events.loading": "लोड हो रहा है…",
    "events.count.one": "{n} इवेंट",
    "events.count.other": "{n} इवेंट",
    "grid.id": "ID",
    "grid.name": "नाम",
    "grid.start": "आरंभ",
    "grid.type": "प्रकार",
    "grid.status": "स्थिति",
    "grid.mode": "मोड",
    "new.title": "नया इवेंट",
    "new.create": "बनाएँ",
    "new.duplicatesTitle": "संभावित डुप्लिकेट",
    "new.duplicatesDetected":
      "डुप्लिकेट मिले ({n}) — पुनः सबमिट करने से पहले नीचे समीक्षा करें।",
    "match.title": "मिलान जाँच",
    "match.name": "नाम",
    "match.threshold": "सीमा",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "आरंभ",
    "match.end": "समाप्ति",
    "match.organizerName": "आयोजक का नाम",
    "match.find": "मिलान खोजें",
    "match.matching": "मिलान हो रहा है…",
    "merge.title": "इवेंट मर्ज करें",
    "merge.mainId": "मुख्य इवेंट ID",
    "merge.mainIdHint": "शेष रहने वाला रिकॉर्ड",
    "merge.dupId": "डुप्लिकेट इवेंट ID",
    "merge.dupIdHint": "सॉफ़्ट-डिलीट किया जाएगा",
    "merge.reason": "कारण",
    "merge.reasonHint": "मर्ज ऑडिट ट्रेल में दर्ज किया गया",
    "merge.reasonPlaceholder": "पुष्ट डुप्लिकेट",
    "merge.loadPreview": "पूर्वावलोकन लोड करें",
    "merge.merge": "मर्ज करें",
    "merge.merging": "मर्ज हो रहा है…",
    "merge.bothIdsRequired": "दोनों ID आवश्यक हैं",
    "merge.idsMustDiffer": "मुख्य और डुप्लिकेट भिन्न होने चाहिए",
    "merge.confirm":
      "{dup}… को {main}… में मर्ज करें?\nयह डुप्लिकेट को सॉफ़्ट-डिलीट कर देगा।",
    "merge.previewTitle": "पूर्वावलोकन",
    "merge.preview.main": "मुख्य",
    "merge.preview.duplicate": "डुप्लिकेट",
    "merge.preview.none": "—",
    "merge.preview.noDate": "कोई तिथि नहीं",
    "merge.completedTitle": "मर्ज पूर्ण",
    "merge.completedBody": "मर्ज रिकॉर्ड {id} {at} पर बनाया गया।",
    "merge.viewMerged": "मर्ज किया गया मुख्य इवेंट देखें",
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
    "detail.identity": "पहचान",
    "detail.id": "ID",
    "detail.start": "आरंभ",
    "detail.end": "समाप्ति",
    "detail.status": "स्थिति",
    "detail.type": "प्रकार",
    "detail.mode": "मोड",
    "detail.timeZone": "समय क्षेत्र",
    "detail.duration": "अवधि",
    "detail.description": "विवरण",
    "detail.empty": "—",
    "detail.loc.place": "स्थान",
    "detail.loc.address": "पता",
    "detail.loc.virtual": "वर्चुअल",
    "detail.loc.text": "पाठ",
    "detail.location": "स्थान",
    "detail.organizers": "आयोजक",
    "detail.performers": "कलाकार",
    "detail.identifiers": "पहचानकर्ता",
    "detail.offers": "ऑफ़र",
    "detail.ticket": "टिकट",
    "detail.confirmDelete":
      "इस इवेंट को सॉफ़्ट-डिलीट करें? इसे UI के माध्यम से पूर्ववत नहीं किया जा सकता।",
    "edit.title": "इवेंट संपादित करें",
    "edit.cancel": "रद्द करें",
    "edit.loading": "लोड हो रहा है…",
    "edit.save": "परिवर्तन सहेजें",
    "audit.title": "ऑडिट लॉग",
    "audit.back": "इवेंट पर वापस जाएँ",
    "audit.loading": "लोड हो रहा है…",
    "audit.none": "कोई ऑडिट प्रविष्टि नहीं।",
    "audit.by": "द्वारा",
    "audit.payload": "पेलोड",
    "form.name": "नाम",
    "form.required": "आवश्यक",
    "form.eventType": "इवेंट प्रकार",
    "form.start": "आरंभ",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "समाप्ति",
    "form.endAfterStart": "समाप्ति ≥ आरंभ होनी चाहिए",
    "form.doorTime": "द्वार खुलने का समय",
    "form.doorBeforeStart": "द्वार समय ≤ आरंभ होना चाहिए",
    "form.status": "स्थिति",
    "form.attendanceMode": "उपस्थिति मोड",
    "form.timeZone": "समय क्षेत्र",
    "form.timeZoneHint": "IANA, जैसे America/Los_Angeles",
    "form.allDay": "पूरे दिन",
    "form.no": "नहीं",
    "form.yes": "हाँ",
    "form.description": "विवरण",
    "form.url": "URL",
    "form.duration": "अवधि",
    "form.durationHint": "ISO 8601, जैसे PT1H30M",
    "form.maxCapacityTotal": "अधिकतम क्षमता (कुल)",
    "form.maxPhysical": "अधिकतम भौतिक",
    "form.maxVirtual": "अधिकतम वर्चुअल",
    "form.keywords": "कीवर्ड",
    "form.keywordsHint": "अल्पविराम से अलग",
    "form.languages": "भाषाएँ",
    "form.languagesHint": "ISO 639-1, जैसे en fr",
    "form.saving": "सहेजा जा रहा है…",
    "form.save": "सहेजें",
    "form.reset": "रीसेट करें",
    "search.placeholder": "खोजें…",
    "search.submit": "खोज",
    "results.title": "मिलान परिणाम",
    "results.none": "कोई उम्मीदवार नहीं।",
    "results.breakdown": "स्कोर विवरण",
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
    "splash.hero.title": "हर इवेंट के लिए एक भरोसेमंद रिकॉर्ड",
    "splash.hero.subtitle":
      "इवेंट को एक बार दर्ज करें, नाम या तारीख़ से खोजें, और डुप्लिकेट बनने से पहले ही रोकें, पूरे ऑडिट ट्रेल के साथ।",
    "splash.benefits.1.title": "कम डुप्लिकेट",
    "splash.benefits.1.body":
      "इवेंट बनाते समय सहेजने से पहले संभावित डुप्लिकेट की चेतावनी मिलती है।",
    "splash.benefits.2.title": "इवेंट जल्दी खोजें",
    "splash.benefits.2.body":
      "अनुमानित मिलान के साथ नाम से खोजें, फिर तारीख़, स्थिति और प्रकार से फ़िल्टर करें।",
    "splash.benefits.3.title": "एक साफ़ रिकॉर्ड",
    "splash.benefits.3.body":
      "पुष्ट डुप्लिकेट को एक इवेंट में मर्ज करें और इतिहास सुरक्षित रखें।",
    "splash.benefits.4.title": "कैलेंडर से योजना बनाएँ",
    "splash.benefits.4.body":
      "इवेंट को कैलेंडर पर देखें और दोबारा शेड्यूल करने के लिए उसे नए स्लॉट में खींचें।",
    "splash.benefits.5.title": "जानें क्या बदला",
    "splash.benefits.5.body":
      "हर इवेंट का अपना ऑडिट लॉग है जो दिखाता है कि किसने क्या और कब बदला।",
    "splash.benefits.6.title": "आपकी अपनी भाषा में",
    "splash.benefits.6.body":
      "ऐप का उपयोग अरबी, वेल्श, अंग्रेज़ी, स्पेनिश, फ़्रेंच, हिंदी या चीनी में करें।",
    "splash.features.1.title": "समृद्ध इवेंट रिकॉर्ड",
    "splash.features.1.body":
      "समय अवधि, स्थिति, प्रकार, उपस्थिति मोड, समय क्षेत्र और विवरण एक ही जगह।",
    "splash.features.2.title": "स्थान और लोग",
    "splash.features.2.body":
      "स्थान, आयोजक, कलाकार, पहचानकर्ता और टिकट ऑफ़र दर्ज करें।",
    "splash.features.3.title": "मिलान जाँच",
    "splash.features.3.body":
      "इवेंट बनाने से पहले किसी काल्पनिक इवेंट को इंडेक्स के विरुद्ध अंकित करें।",
    "splash.features.4.title": "निर्देशित मर्जिंग",
    "splash.features.4.body":
      "दो इवेंट की साथ-साथ तुलना करें, फिर कारण दर्ज करके मर्ज करें।",
    "splash.features.5.title": "कैलेंडर दृश्य",
    "splash.features.5.body":
      "हर इवेंट की समय अवधि पर ड्रैग-एंड-ड्रॉप कैलेंडर।",
    "splash.features.6.title": "थीम और टेक्स्ट आकार",
    "splash.features.6.body":
      "रंग थीम और आरामदायक टेक्स्ट आकार चुनें, अगली बार भी याद रहेगा।",
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
      "इवेंट रजिस्ट्री का निर्देशित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, इवेंट दर्ज करने से लेकर डुप्लिकेट मर्ज करने और कैलेंडर पर समय बदलने तक।",
    "tour.s1.title": "इवेंट दर्ज करना",
    "tour.s1.summary":
      "समय-सीमा, स्थिति, प्रकार और उपस्थिति मोड के साथ इवेंट बनाएँ। इवेंट सहेजने से पहले संभावित डुप्लिकेट चिह्नित किए जाते हैं।",
    "tour.s1.step.1":
      "मेनू में नया इवेंट चुनें, या इवेंट सूची में नया इवेंट दबाएँ।",
    "tour.s1.step.2":
      "नाम और आरंभ भरें, जो अनिवार्य हैं, फिर समाप्ति, द्वार समय, स्थिति, उपस्थिति मोड और समय क्षेत्र जोड़ें। समाप्ति आरंभ से पहले और द्वार समय आरंभ के बाद नहीं हो सकता।",
    "tour.s1.step.3":
      "चाहें तो विवरण, URL, अवधि, क्षमता सीमा, कीवर्ड और भाषाएँ जोड़ें, फिर बनाएँ दबाएँ।",
    "tour.s1.step.4":
      "यदि रजिस्ट्री को संभावित डुप्लिकेट मिलते हैं, तो संभावित डुप्लिकेट पैनल उन्हें स्कोर सहित सूचीबद्ध करता है। दोबारा जमा करने से पहले उनकी समीक्षा करें।",
    "tour.s2.title": "इवेंट खोजना",
    "tour.s2.summary":
      "नाम, आयोजक या पहचानकर्ता से रजिस्ट्री खोजें, फिर तिथि, स्थिति और प्रकार से सूची सीमित करें।",
    "tour.s2.step.1":
      "मेनू में इवेंट खोलें; सूची खोज बॉक्स और मेल खाने वाले इवेंट की गिनती के साथ लोड होती है।",
    "tour.s2.step.2":
      "खोज बॉक्स में टाइप करें, जैसे नाम, आयोजक या पहचानकर्ता, और खोजें दबाएँ।",
    "tour.s2.step.3":
      "वर्तनी के अंतर सहने के लिए फ़ज़ी चुनें, फिर से, तक, स्थिति और प्रकार तय करें और फ़िल्टर लागू करें दबाएँ।",
    "tour.s2.step.4":
      "उस इवेंट का विवरण पृष्ठ खोलने के लिए ग्रिड में कोई पंक्ति चुनें।",
    "tour.s3.title": "मिलान जाँचना",
    "tour.s3.summary":
      "कुछ बनाए बिना काल्पनिक इवेंट के विवरण को रजिस्ट्री के विरुद्ध स्कोर करें, ताकि देख सकें कि पहले से क्या मौजूद है।",
    "tour.s3.step.1": "मेनू में मिलान जाँच खोलें।",
    "tour.s3.step.2":
      "नाम (अनिवार्य) दर्ज करें और यदि पता हो तो आरंभ, समाप्ति और आयोजक का नाम भी दें।",
    "tour.s3.step.3": "सीमा 0.0 से 1.0 के बीच तय करें, फिर मिलान खोजें दबाएँ।",
    "tour.s3.step.4":
      "मिलान परिणाम पढ़ें: हर उम्मीदवार अपना स्कोर दिखाता है और स्कोर विवरण बताता है कि वह कैसे बना।",
    "tour.s4.title": "डुप्लिकेट मर्ज करना",
    "tour.s4.summary":
      "पुष्ट डुप्लिकेट को उस इवेंट में मिलाएँ जिसे आप रख रहे हैं, कारण ऑडिट ट्रेल में दर्ज होता है।",
    "tour.s4.step.1":
      "मेनू में मर्ज खोलें और मुख्य इवेंट ID (जो रिकॉर्ड बचेगा) और डुप्लिकेट इवेंट ID दर्ज करें।",
    "tour.s4.step.2":
      "दोनों रिकॉर्ड साथ-साथ देखने के लिए पूर्वावलोकन लोड करें दबाएँ। दोनों ID देना अनिवार्य है और वे अलग होने चाहिए।",
    "tour.s4.step.3":
      "कारण दें, जो मर्ज ऑडिट ट्रेल में दर्ज होता है, फिर मर्ज दबाएँ और पुष्टि करें।",
    "tour.s4.step.4":
      "डुप्लिकेट सॉफ़्ट-डिलीट हो जाता है, मर्ज पूर्ण संदेश मर्ज रिकॉर्ड दिखाता है और मर्ज किया गया मुख्य इवेंट देखें बचे हुए रिकॉर्ड को खोलता है।",
    "tour.s5.title": "कैलेंडर पर योजना बनाना",
    "tour.s5.summary":
      "माह, सप्ताह और दिन के दृश्यों में इवेंट की समय-सीमाएँ देखें और खींचकर समय बदलें।",
    "tour.s5.step.1":
      "मेनू में कैलेंडर खोलें, जहाँ दर्ज इवेंट उनके आरंभ और समाप्ति समय के अनुसार दिखते हैं।",
    "tour.s5.step.2": "माह, सप्ताह और दिन के दृश्यों के बीच बदलें।",
    "tour.s5.step.3":
      "किसी इवेंट को नए स्लॉट पर खींचें; बदलाव सामान्य अपडेट के ज़रिए इवेंट रिकॉर्ड में सहेजा जाता है, इसलिए कैलेंडर कोई अलग प्रति नहीं है।",
    "tour.s5.step.4": "किसी इवेंट का विवरण पृष्ठ खोलने के लिए उसे चुनें।",
    "tour.s6.title": "इवेंट की समीक्षा, संपादन और ऑडिट",
    "tour.s6.summary":
      "एक इवेंट खोलकर उसके बारे में दर्ज सब कुछ पढ़ें, सुधारें, देखें कि किसने बदला, उसे मास्क करें या निर्यात करें।",
    "tour.s6.step.1":
      "इवेंट के विवरण पृष्ठ पर पहचान, स्थान, आयोजक, कलाकार, पहचानकर्ता और ऑफ़र पढ़ें, जहाँ वे दर्ज हैं।",
    "tour.s6.step.2":
      "रिकॉर्ड बदलने के लिए संपादित करें चुनकर बदलाव सहेजें दबाएँ, या पुष्टि के बाद सॉफ़्ट-डिलीट के लिए हटाएँ चुनें।",
    "tour.s6.step.3":
      "ऑडिट लॉग खोलने के लिए ऑडिट चुनें: हर प्रविष्टि दिखाती है कि बदलाव किसने किया और उसका पेलोड क्या है।",
    "tour.s6.step.4":
      "संपादित संस्करण देखने के लिए मास्क किया हुआ दिखाएँ, या रिकॉर्ड डाउनलोड करने के लिए डेटा निर्यात (GDPR) उपयोग करें।",
    "signin.sso": "SSO से साइन इन करें",
  },
  "zh-cn": {
    "nav.calendar": "日历",
    brand: "活动",
    "brand.tagline": "Main X Index",
    "nav.dashboard": "仪表板",
    "nav.events": "活动",
    "nav.newEvent": "新建活动",
    "nav.matchCheck": "匹配检查",
    "nav.merge": "合并",
    "nav.toggle": "切换导航",
    "chrome.theme": "主题",
    "chrome.language": "语言",
    "chrome.share": "分享",
    "chrome.textSize": "文字大小",
    "share.copyLink": "复制链接",
    "share.linkCopied": "链接已复制",
    "share.copyFailed": "无法复制 — 请从地址栏复制",
    "dashboard.title": "仪表板",
    "dashboard.service": "服务：",
    "dashboard.recentActivity": "近期活动",
    "dashboard.noRecent": "没有近期审计记录。",
    "events.title": "活动",
    "events.new": "新建活动",
    "events.searchPlaceholder": "按名称、组织者、标识符搜索…",
    "events.filter.from": "从",
    "events.filter.to": "至",
    "events.filter.status": "状态",
    "events.filter.type": "类型",
    "events.filter.any": "任意",
    "events.filter.fuzzy": "模糊",
    "events.filter.apply": "应用筛选",
    "events.loading": "加载中…",
    "events.count.one": "{n} 个活动",
    "events.count.other": "{n} 个活动",
    "grid.id": "ID",
    "grid.name": "名称",
    "grid.start": "开始",
    "grid.type": "类型",
    "grid.status": "状态",
    "grid.mode": "模式",
    "new.title": "新建活动",
    "new.create": "创建",
    "new.duplicatesTitle": "可能的重复项",
    "new.duplicatesDetected": "检测到重复项（{n}）— 重新提交前请在下方查看。",
    "match.title": "匹配检查",
    "match.name": "名称",
    "match.threshold": "阈值",
    "match.thresholdHint": "0.0 – 1.0",
    "match.start": "开始",
    "match.end": "结束",
    "match.organizerName": "组织者名称",
    "match.find": "查找匹配",
    "match.matching": "匹配中…",
    "merge.title": "合并活动",
    "merge.mainId": "主活动 ID",
    "merge.mainIdHint": "保留的记录",
    "merge.dupId": "重复活动 ID",
    "merge.dupIdHint": "将被软删除",
    "merge.reason": "原因",
    "merge.reasonHint": "记录在合并审计跟踪中",
    "merge.reasonPlaceholder": "已确认的重复项",
    "merge.loadPreview": "加载预览",
    "merge.merge": "合并",
    "merge.merging": "合并中…",
    "merge.bothIdsRequired": "需要两个 ID",
    "merge.idsMustDiffer": "主记录与重复项必须不同",
    "merge.confirm": "将 {dup}… 合并到 {main}…？\n这将软删除重复项。",
    "merge.previewTitle": "预览",
    "merge.preview.main": "主",
    "merge.preview.duplicate": "重复",
    "merge.preview.none": "—",
    "merge.preview.noDate": "无日期",
    "merge.completedTitle": "合并完成",
    "merge.completedBody": "合并记录 {id} 已于 {at} 创建。",
    "merge.viewMerged": "查看合并后的主活动",
    "detail.loading": "加载中…",
    "detail.edit": "编辑",
    "detail.audit": "审计",
    "detail.delete": "删除",
    "detail.exportGdpr": "导出数据（GDPR）",
    "detail.exportingGdpr": "导出中…",
    "detail.showMasked": "显示脱敏视图",
    "detail.showFull": "显示完整视图",
    "detail.maskedNotice": "正在显示脱敏视图——部分字段已隐藏。",
    "detail.identity": "身份",
    "detail.id": "ID",
    "detail.start": "开始",
    "detail.end": "结束",
    "detail.status": "状态",
    "detail.type": "类型",
    "detail.mode": "模式",
    "detail.timeZone": "时区",
    "detail.duration": "时长",
    "detail.description": "描述",
    "detail.empty": "—",
    "detail.loc.place": "地点",
    "detail.loc.address": "地址",
    "detail.loc.virtual": "虚拟",
    "detail.loc.text": "文本",
    "detail.location": "位置",
    "detail.organizers": "组织者",
    "detail.performers": "表演者",
    "detail.identifiers": "标识符",
    "detail.offers": "优惠",
    "detail.ticket": "门票",
    "detail.confirmDelete": "软删除此活动？无法通过界面撤销。",
    "edit.title": "编辑活动",
    "edit.cancel": "取消",
    "edit.loading": "加载中…",
    "edit.save": "保存更改",
    "audit.title": "审计日志",
    "audit.back": "返回活动",
    "audit.loading": "加载中…",
    "audit.none": "没有审计记录。",
    "audit.by": "由",
    "audit.payload": "负载",
    "form.name": "名称",
    "form.required": "必填",
    "form.eventType": "活动类型",
    "form.start": "开始",
    "form.startHint": "ISO 8601 / RFC 3339",
    "form.end": "结束",
    "form.endAfterStart": "结束必须 ≥ 开始",
    "form.doorTime": "入场时间",
    "form.doorBeforeStart": "入场时间必须 ≤ 开始",
    "form.status": "状态",
    "form.attendanceMode": "参与模式",
    "form.timeZone": "时区",
    "form.timeZoneHint": "IANA，如 America/Los_Angeles",
    "form.allDay": "全天",
    "form.no": "否",
    "form.yes": "是",
    "form.description": "描述",
    "form.url": "URL",
    "form.duration": "时长",
    "form.durationHint": "ISO 8601，如 PT1H30M",
    "form.maxCapacityTotal": "最大容量（总计）",
    "form.maxPhysical": "最大现场",
    "form.maxVirtual": "最大虚拟",
    "form.keywords": "关键词",
    "form.keywordsHint": "以逗号分隔",
    "form.languages": "语言",
    "form.languagesHint": "ISO 639-1，如 en fr",
    "form.saving": "保存中…",
    "form.save": "保存",
    "form.reset": "重置",
    "search.placeholder": "搜索…",
    "search.submit": "搜索",
    "results.title": "匹配结果",
    "results.none": "没有候选项。",
    "results.breakdown": "分数明细",
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
    "splash.hero.title": "每个活动，一份可信记录",
    "splash.hero.subtitle":
      "一次登记活动，按名称或日期查找，在重复出现之前就将其拦截，并内置完整审计记录。",
    "splash.benefits.1.title": "更少重复",
    "splash.benefits.1.body": "创建活动时，保存前会提示可能的重复项。",
    "splash.benefits.2.title": "快速查找活动",
    "splash.benefits.2.body": "按名称模糊搜索，再按日期、状态和类型筛选。",
    "splash.benefits.3.title": "一份干净记录",
    "splash.benefits.3.body": "将确认的重复项合并为一个活动，并保留历史。",
    "splash.benefits.4.title": "按日历规划",
    "splash.benefits.4.body": "在日历上查看活动，拖到新的时段即可重新安排。",
    "splash.benefits.5.title": "了解变更内容",
    "splash.benefits.5.body":
      "每个活动都有独立的审计日志，记录谁在何时改了什么。",
    "splash.benefits.6.title": "支持您的语言",
    "splash.benefits.6.body":
      "可使用阿拉伯语、威尔士语、英语、西班牙语、法语、印地语或中文使用本应用。",
    "splash.features.1.title": "内容丰富的活动记录",
    "splash.features.1.body":
      "时间段、状态、类型、参与方式、时区和描述集中一处。",
    "splash.features.2.title": "地点与人员",
    "splash.features.2.body": "记录地点、组织者、表演者、标识符和票务优惠。",
    "splash.features.3.title": "匹配检查",
    "splash.features.3.body":
      "创建活动之前，先将假设的活动与索引进行评分比对。",
    "splash.features.4.title": "引导式合并",
    "splash.features.4.body": "并排比较两个活动，然后记录原因并合并。",
    "splash.features.5.title": "日历视图",
    "splash.features.5.body": "覆盖每个活动时间段的拖放式日历。",
    "splash.features.6.title": "主题与文字大小",
    "splash.features.6.body":
      "选择配色主题和舒适的文字大小，下次访问时仍会保留。",
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
      "活动登记库的图文导览：每个页面的作用和使用步骤，从登记活动到合并重复记录、在日历上重新安排时间。",
    "tour.s1.title": "登记活动",
    "tour.s1.summary":
      "创建包含时间范围、状态、类型和参与方式的活动。保存前会提示可能的重复项。",
    "tour.s1.step.1": "在菜单中选择“新建活动”，或在活动列表中点击“新建活动”。",
    "tour.s1.step.2":
      "填写必填的名称和开始时间，再补充结束时间、开门时间、状态、参与方式和时区。结束时间不能早于开始时间，开门时间不能晚于开始时间。",
    "tour.s1.step.3":
      "可选填写描述、网址、时长、容量上限、关键词和语言，然后点击“创建”。",
    "tour.s1.step.4":
      "如果登记库发现可能的重复项，“可能的重复项”面板会列出它们及得分。请先核对，再重新提交。",
    "tour.s2.title": "查找活动",
    "tour.s2.summary":
      "按名称、组织者或标识符搜索登记库，再按日期、状态和类型缩小列表范围。",
    "tour.s2.step.1": "在菜单中打开“活动”；列表会加载搜索框和匹配活动的数量。",
    "tour.s2.step.2": "在搜索框中输入名称、组织者或标识符等，然后点击“搜索”。",
    "tour.s2.step.3":
      "勾选“模糊”以容忍拼写差异，设置开始、结束、状态和类型，然后点击“应用筛选”。",
    "tour.s2.step.4": "在表格中选择一行，即可打开该活动的详情页。",
    "tour.s3.title": "检查匹配项",
    "tour.s3.summary":
      "在不创建任何记录的情况下，将假设的活动信息与登记库比对打分，看看已有哪些记录。",
    "tour.s3.step.1": "在菜单中打开“匹配检查”。",
    "tour.s3.step.2":
      "输入名称（必填），如果知道，也填写开始时间、结束时间和组织者名称。",
    "tour.s3.step.3": "将阈值设为 0.0 到 1.0 之间，然后点击“查找匹配”。",
    "tour.s3.step.4":
      "查看“匹配结果”：每个候选项都显示得分，“得分明细”说明得分的构成。",
    "tour.s4.title": "合并重复记录",
    "tour.s4.summary":
      "把已确认的重复记录并入要保留的活动，并将原因记入审计记录。",
    "tour.s4.step.1":
      "在菜单中打开“合并”，输入主活动 ID（保留的记录）和重复活动 ID。",
    "tour.s4.step.2":
      "点击“加载预览”并排查看两条记录。两个 ID 都必须填写且不能相同。",
    "tour.s4.step.3":
      "填写“原因”（会记入合并审计记录），然后点击“合并”并确认提示。",
    "tour.s4.step.4":
      "重复记录会被软删除，“合并完成”消息会显示合并记录，点击“查看合并后的主活动”可打开保留的记录。",
    "tour.s5.title": "在日历上安排",
    "tour.s5.summary":
      "在月、周、日视图中查看活动的时间范围，并通过拖动重新安排。",
    "tour.s5.step.1":
      "在菜单中打开“日历”，可看到已登记的活动按开始和结束时间排布。",
    "tour.s5.step.2": "在月、周、日视图之间切换。",
    "tour.s5.step.3":
      "把活动拖到新的时段；更改会通过常规更新保存到活动记录中，因此日历不是独立的副本。",
    "tour.s5.step.4": "选择某个活动即可打开其详情页。",
    "tour.s6.title": "查看、编辑和审计活动",
    "tour.s6.summary":
      "打开一个活动，查看其全部记录、修改它、查看谁改过它、进行脱敏或导出。",
    "tour.s6.step.1":
      "在活动详情页中，查看已记录的身份、地点、组织者、表演者、标识符和优惠。",
    "tour.s6.step.2":
      "选择“编辑”修改记录并点击“保存更改”，或选择“删除”并确认提示后将其软删除。",
    "tour.s6.step.3":
      "选择“审计”打开审计日志：每条记录显示谁做了更改及其载荷。",
    "tour.s6.step.4":
      "使用“显示脱敏”查看已遮蔽的版本，或使用“导出数据 (GDPR)”下载该记录。",
    "signin.sso": "使用 SSO 登录",
  },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)["en-001"];

// Normalise raw input to a supported locale, or null if unsupported.
function normaliseLocale(raw: string | null | undefined): Locale | null {
  if (!raw) return null;
  const trimmed = raw.trim();
  // Exact match first (case- and hyphen/underscore-insensitive).
  const normalized = trimmed.replace(/_/g, "-").toLowerCase();
  const exact = (LOCALES as readonly string[]).find(
    (l) => l.toLowerCase() === normalized,
  );
  if (exact) return exact as Locale;
  // Otherwise resolve by primary subtag: `en`, `en-US`, `es-MX` and
  // `zh-Hans-CN` map to the supported locale that starts with that
  // language (`en-001`, `es-001`, `zh-cn`).
  const primary = normalized.split("-")[0] ?? "";
  return (
    ((LOCALES as readonly string[]).find(
      (l) => l.toLowerCase().split("-")[0] === primary,
    ) as Locale | undefined) ?? null
  );
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
  // Unknown locale → English table; unknown key → English → the key.
  const table = STRINGS[locale] ?? STRINGS[DEFAULT_LOCALE];
  return table[key] ?? STRINGS[DEFAULT_LOCALE][key] ?? key;
}

/**
 * Reactive translation accessor for components: `t("events.title")`.
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
