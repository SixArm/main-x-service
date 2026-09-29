// Lightweight, dependency-free i18n for the Case SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is small and we keep the front-end dependency-light (drift
// across the family front-ends is accepted, see AGENTS.md). Shared chrome
// / common terms reuse the family's established translations so this app
// stays consistent with its siblings.
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. An unknown key/locale falls back to `en-001`, then to
// the key string itself. The chosen locale persists to localStorage and
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
 * localStorage key under which the chosen UI locale is persisted. Exported
 * so a locale control can share it — the i18n store is the single source of
 * truth for the chosen locale, and the control persists to the same key.
 */
export const LOCALE_KEY = "mxi.case.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    "nav.board": "اللوحة",
    "brand.name": "Main X · القضايا",
    "nav.toggle": "تبديل التنقل",
    "nav.cases": "القضايا",
    "nav.newCase": "قضية جديدة",
    "chrome.language": "اللغة",
    "chrome.share": "مشاركة",
    "chrome.textSize": "حجم النص",
    "share.copyLink": "نسخ الرابط",
    "share.linkCopied": "تم نسخ الرابط",
    "share.copyFailed": "تعذر النسخ — انسخه من شريط العنوان",
    "chrome.theme": "السمة",
    "session.title": "الجلسة",
    "session.tokenAttached": "الرمز مرفق.",
    "session.clearToken": "مسح الرمز",
    "session.noToken": "لا يوجد رمز.",
    "session.signIn": "تسجيل الدخول",
    "session.pasteToken": "لصق رمز",
    "session.accessToken": "رمز الوصول",
    "session.pastePlaceholder": "الصق رمز الوصول",
    "session.useToken": "استخدام الرمز",
    "session.hint": "من authentication-service (تسجيل الدخول برابط سحري).",
    "list.title": "القضايا",
    "list.new": "قضية جديدة",
    "list.loading": "جارٍ التحميل…",
    "list.empty": "لا توجد قضايا بعد.",
    "list.createOne": "أنشئ واحدة",
    "list.loadFailed": "تعذّر تحميل القضايا",
    "detail.loading": "جارٍ التحميل…",
    "detail.notFound": "غير موجود",
    "detail.caseType": "نوع القضية:",
    "detail.status": "الحالة:",
    "detail.priority": "الأولوية:",
    "detail.agency": "الجهة:",
    "detail.caseNumber": "رقم القضية:",
    "detail.opened": "فُتحت:",
    "detail.subjects": "المواضيع:",
    "detail.identifiers": "المعرفات:",
    "detail.keywords": "الكلمات المفتاحية:",
    "detail.id": "المعرف:",
    "detail.edit": "تحرير",
    "detail.checkDuplicates": "فحص التكرارات",
    "detail.checking": "جارٍ الفحص…",
    "detail.checkFailed": "فشل الفحص",
    "detail.delete": "حذف",
    "detail.potentialDuplicates": "تكرارات محتملة",
    "detail.noneAboveThreshold": "لا شيء فوق حد التطابق.",
    "new.title": "قضية جديدة",
    "new.create": "إنشاء",
    "edit.title": "تحرير القضية",
    "edit.loading": "جارٍ التحميل…",
    "edit.notFound": "غير موجود",
    "edit.saveChanges": "حفظ التغييرات",
    "form.title": "العنوان",
    "form.caseType": "نوع القضية",
    "form.status": "الحالة",
    "form.priority": "الأولوية",
    "form.caseNumber": "رقم القضية",
    "form.openedDate": "تاريخ الفتح",
    "form.agencyId": "معرف الجهة",
    "form.agencyName": "اسم الجهة",
    "form.alternateTitles": "عناوين بديلة",
    "form.subjects": "المواضيع",
    "form.keywords": "الكلمات المفتاحية",
    "form.sameAs": "روابط مطابق لـ",
    "form.languages": "اللغات",
    "form.commaSeparated": "(مفصولة بفواصل)",
    "form.commaSeparatedIso": "(ISO 639-1 مفصولة بفواصل)",
    "form.identifiers": "المعرفات",
    "form.valuePlaceholder": "قيمة",
    "form.remove": "إزالة",
    "form.addIdentifier": "+ إضافة معرف",
    "form.empty": "—",
    "form.save": "حفظ",
    "form.saving": "جارٍ الحفظ…",
    "form.titleRequired": "العنوان مطلوب.",
    "form.customLabel": "تسمية مخصصة",
    "form.customLabelRequired": "التسمية المخصصة مطلوبة.",
    "form.saveFailed": "فشل الحفظ",
    "nav.merge": "دمج",
    "merge.title": "دمج القضايا",
    "merge.mainId": "معرف القضية الرئيسية",
    "merge.mainIdHint": "القضية الباقية — تحتفظ بمعرفها.",
    "merge.dupId": "معرف القضية المكررة",
    "merge.dupIdHint": "تُدمج في القضية الرئيسية ثم تُحذف حذفًا ناعمًا.",
    "merge.reason": "السبب",
    "merge.reasonHint": "اختياري؛ يُسجَّل في سجل الدمج.",
    "merge.reasonPlaceholder": "تكرار مؤكد لنفس القضية",
    "merge.loadPreview": "تحميل المعاينة",
    "merge.merging": "جارٍ الدمج…",
    "merge.merge": "دمج",
    "merge.bothIdsRequired": "معرّفا القضيتين مطلوبان.",
    "merge.mustDiffer": "يجب أن يختلف معرف القضية الرئيسية عن معرف المكررة.",
    "merge.preview": "معاينة",
    "merge.main": "الرئيسية",
    "merge.duplicate": "المكررة",
    "merge.completed": "اكتمل الدمج",
    "merge.viewMain": "عرض القضية الرئيسية",
    "merge.confirm":
      "دمج القضية {dup} في القضية {main}؟ ستُحذف المكررة حذفًا ناعمًا.",
    "merge.recent": "عمليات الدمج الأخيرة",
    "merge.recentEmpty": "لا توجد عمليات دمج مسجلة بعد.",
    "merge.recentFailed": "تعذّر تحميل عمليات الدمج الأخيرة",
    "merge.mergedAt": "تم الدمج في",
    "merge.actor": "المنفّذ",
    // Cross-service links (subject_of)
    "links.title": "الشخص موضوع هذه القضية",
    "links.note":
      "يسجّل الشخص الذي تتعلق به هذه القضية. هذا الإقرار حسّاس بقدر القضية نفسها: قراءته وكتابته تتطلبان التفويض نفسه، وكل تغيير يُدوَّن في سجل التدقيق.",
    "links.loading": "جارٍ التحميل…",
    "links.empty": "لم يُسجَّل أي شخص بعد.",
    "links.loadFailed": "تعذّر تحميل أشخاص هذه القضية",
    "links.person": "الشخص",
    "links.personHint": "مرجع الشخص بالصيغة person:<uuid>.",
    "links.confidence": "درجة الثقة",
    "links.confidenceHint": "اختياري؛ من 0 إلى 1. اتركه فارغًا للإقرار القاطع.",
    "links.provenance": "المصدر",
    "links.provenanceHint": "اختياري؛ القيمة الافتراضية «operator».",
    "links.validFrom": "ساري من",
    "links.validTo": "ساري حتى",
    "links.validity": "مدة السريان",
    "links.addTitle": "تسجيل شخص",
    "links.record": "تسجيل الشخص",
    "links.recording": "جارٍ التسجيل…",
    "links.withdraw": "سحب",
    "links.withdrawing": "جارٍ السحب…",
    "links.withdrawConfirm":
      "هل تسحب الإقرار بأن {ref} موضوع هذه القضية؟ يُسجَّل السحب ويخضع للتدقيق.",
    "links.recordFailed": "تعذّر تسجيل الشخص",
    "links.withdrawFailed": "تعذّر سحب الرابط",
    "links.invalidPersonRef": "أدخل مرجع شخص بالصيغة person:<uuid>.",
    "links.confidenceRange": "يجب أن تكون درجة الثقة بين 0 و1.",
    "search.placeholder": "بحث…",
    "search.submit": "بحث",
    "search.fuzzy": "تقريبي",
    "search.phonetic": "صوتي",
    "search.failed": "فشل البحث",
    "detail.viewAudit": "عرض سجل التدقيق",
    "nav.audit": "النشاط",
    "audit.title": "سجل التدقيق",
    "audit.backToCase": "العودة إلى القضية",
    "audit.loading": "جارٍ التحميل…",
    "audit.noEntries": "لا توجد إدخالات تدقيق بعد.",
    "audit.by": "بواسطة",
    "audit.payload": "التفاصيل",
    "audit.loadFailed": "تعذّر تحميل سجل التدقيق",
    "activity.title": "النشاط الأخير",
    "activity.recentAudit": "أحدث إدخالات التدقيق",
    "activity.recentEvents": "أحدث الأحداث",
    "activity.loading": "جارٍ التحميل…",
    "activity.loadFailed": "تعذّر تحميل النشاط الأخير",
    "activity.noAuditEntries": "لا توجد إدخالات تدقيق بعد.",
    "activity.noEvents": "لا توجد أحداث بعد.",
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
    "splash.hero.title": "سجل واضح واحد لكل قضية",
    "splash.hero.subtitle":
      "سجّل القضايا مرة واحدة، واعثر عليها فورًا، واكتشف التكرار قبل أن يتفاقم، مع تسجيل كل تغيير في سجل تدقيق.",
    "splash.benefits.1.title": "لكل قضية سجل واحد",
    "splash.benefits.1.body":
      "احتفظ بكل قضية في سجل واحد مشترك بدلًا من جداول البيانات والملفات المتفرقة.",
    "splash.benefits.2.title": "اعثر عليها بسرعة",
    "splash.benefits.2.body":
      "ابحث بالعنوان أو الموضوع أو الجهة أو المعرّف، حتى مع الأخطاء الإملائية أو الأسماء المتشابهة نطقًا.",
    "splash.benefits.3.title": "أوقف تكرار القضايا",
    "splash.benefits.3.body":
      "يتم تنبيهك إلى التكرارات المحتملة لمراجعتها حتى لا يظهر سجل ثانٍ دون أن يلاحظه أحد.",
    "splash.benefits.4.title": "حجم العمل في لمحة",
    "splash.benefits.4.body":
      "تجمع اللوحة كل القضايا حسب الحالة لترى أين يتراكم العمل.",
    "splash.benefits.5.title": "مساءلة واضحة",
    "splash.benefits.5.body":
      "يُسجَّل كل تغيير، فيمكنك دائمًا معرفة ما حدث لقضية ما ومتى.",
    "splash.benefits.6.title": "اربط القضايا بالأشخاص",
    "splash.benefits.6.body":
      "سجّل من تتعلق به القضية بربطها بسجل الشخص، دون نسخ بياناته.",
    "splash.features.1.title": "سجلات قضايا كاملة",
    "splash.features.1.body":
      "العنوان والجهة ورقم القضية والنوع والحالة والأولوية والموضوعات والمعرّفات في نموذج واحد.",
    "splash.features.2.title": "بحث نصي كامل",
    "splash.features.2.body":
      "تتيح الخيارات التقريبية والصوتية العثور على القضايا رغم الأخطاء الإملائية والأسماء البديلة.",
    "splash.features.3.title": "فحص التكرار",
    "splash.features.3.body":
      "قارن قضية بالقضايا المخزّنة وراجع التطابقات المُقيَّمة بدرجات قبل الحفظ.",
    "splash.features.4.title": "دمج القضايا",
    "splash.features.4.body":
      "ادمج تكرارًا مؤكدًا في السجل المتبقي وراجع عمليات الدمج الأخيرة.",
    "splash.features.5.title": "لوحة الحالات",
    "splash.features.5.body": "اسحب بطاقة إلى عمود آخر لتغيير حالة القضية.",
    "splash.features.6.title": "سجل التدقيق",
    "splash.features.6.body":
      "اطّلع على تاريخ قضية واحدة أو النشاط الأخير في كل القضايا، الأحدث أولًا.",
  },
  "cy-001": {
    "nav.board": "Bwrdd",
    "brand.name": "Main X · Achosion",
    "nav.toggle": "Toglo'r llywio",
    "nav.cases": "Achosion",
    "nav.newCase": "Achos newydd",
    "chrome.language": "Iaith",
    "chrome.share": "Rhannu",
    "chrome.textSize": "Maint testun",
    "share.copyLink": "Copïo dolen",
    "share.linkCopied": "Dolen wedi'i chopïo",
    "share.copyFailed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "chrome.theme": "Thema",
    "session.title": "Sesiwn",
    "session.tokenAttached": "Tocyn ynghlwm.",
    "session.clearToken": "Clirio'r tocyn",
    "session.noToken": "Dim tocyn.",
    "session.signIn": "Mewngofnodi",
    "session.pasteToken": "Gludo tocyn",
    "session.accessToken": "Tocyn mynediad",
    "session.pastePlaceholder": "Gludo tocyn mynediad",
    "session.useToken": "Defnyddio tocyn",
    "session.hint": "O'r authentication-service (mewngofnodi dolen-hud).",
    "list.title": "Achosion",
    "list.new": "Achos newydd",
    "list.loading": "Yn llwytho…",
    "list.empty": "Dim achosion eto.",
    "list.createOne": "Creu un",
    "list.loadFailed": "Methwyd llwytho achosion",
    "detail.loading": "Yn llwytho…",
    "detail.notFound": "Heb ei ganfod",
    "detail.caseType": "Math o achos:",
    "detail.status": "Statws:",
    "detail.priority": "Blaenoriaeth:",
    "detail.agency": "Asiantaeth:",
    "detail.caseNumber": "Rhif achos:",
    "detail.opened": "Agorwyd:",
    "detail.subjects": "Pynciau:",
    "detail.identifiers": "Dynodyddion:",
    "detail.keywords": "Allweddeiriau:",
    "detail.id": "ID:",
    "detail.edit": "Golygu",
    "detail.checkDuplicates": "Gwirio dyblygiadau",
    "detail.checking": "Yn gwirio…",
    "detail.checkFailed": "Methodd y gwiriad",
    "detail.delete": "Dileu",
    "detail.potentialDuplicates": "Dyblygiadau posibl",
    "detail.noneAboveThreshold": "Dim un uwchlaw'r trothwy cydweddu.",
    "new.title": "Achos newydd",
    "new.create": "Creu",
    "edit.title": "Golygu achos",
    "edit.loading": "Yn llwytho…",
    "edit.notFound": "Heb ei ganfod",
    "edit.saveChanges": "Cadw newidiadau",
    "form.title": "Teitl",
    "form.caseType": "Math o achos",
    "form.status": "Statws",
    "form.priority": "Blaenoriaeth",
    "form.caseNumber": "Rhif achos",
    "form.openedDate": "Dyddiad agor",
    "form.agencyId": "ID asiantaeth",
    "form.agencyName": "Enw asiantaeth",
    "form.alternateTitles": "Teitlau eraill",
    "form.subjects": "Pynciau",
    "form.keywords": "Allweddeiriau",
    "form.sameAs": "URLau yr un â",
    "form.languages": "Ieithoedd",
    "form.commaSeparated": "(wedi'u gwahanu gan goma)",
    "form.commaSeparatedIso": "(ISO 639-1 wedi'u gwahanu gan goma)",
    "form.identifiers": "Dynodyddion",
    "form.valuePlaceholder": "gwerth",
    "form.remove": "Tynnu",
    "form.addIdentifier": "+ Ychwanegu dynodydd",
    "form.empty": "—",
    "form.save": "Cadw",
    "form.saving": "Yn cadw…",
    "form.titleRequired": "Mae angen teitl.",
    "form.customLabel": "Label cyfaddas",
    "form.customLabelRequired": "Mae angen label cyfaddas.",
    "form.saveFailed": "Methodd y cadw",
    "nav.merge": "Uno",
    "merge.title": "Uno achosion",
    "merge.mainId": "ID y prif achos",
    "merge.mainIdHint": "Yr achos sy'n goroesi — mae'n cadw ei ID.",
    "merge.dupId": "ID yr achos dyblyg",
    "merge.dupIdHint": "Caiff ei blygu i'r prif achos, yna ei ddileu'n feddal.",
    "merge.reason": "Rheswm",
    "merge.reasonHint": "Dewisol; caiff ei gofnodi yn hanes yr uno.",
    "merge.reasonPlaceholder": "Dyblyg wedi'i gadarnhau o'r un achos",
    "merge.loadPreview": "Llwytho rhagolwg",
    "merge.merging": "Yn uno…",
    "merge.merge": "Uno",
    "merge.bothIdsRequired": "Mae angen y ddau ID achos.",
    "merge.mustDiffer": "Rhaid i ID y prif achos a'r dyblyg fod yn wahanol.",
    "merge.preview": "Rhagolwg",
    "merge.main": "Prif",
    "merge.duplicate": "Dyblyg",
    "merge.completed": "Uno wedi'i gwblhau",
    "merge.viewMain": "Gweld y prif achos",
    "merge.confirm":
      "Uno achos {dup} i achos {main}? Caiff y dyblyg ei ddileu'n feddal.",
    "merge.recent": "Unoiadau diweddar",
    "merge.recentEmpty": "Dim unoiadau wedi'u cofnodi eto.",
    "merge.recentFailed": "Methwyd llwytho'r unoiadau diweddar",
    "merge.mergedAt": "Unwyd am",
    "merge.actor": "Actor",
    // Cross-service links (subject_of)
    "links.title": "Testun yr achos hwn",
    "links.note":
      "Cofnodi'r person y mae'r achos hwn yn ymwneud ag ef. Mae'r honiad mor sensitif â'r achos ei hun: mae ei ddarllen a'i ysgrifennu yn gofyn am yr un awdurdod, ac archwilir pob newid.",
    "links.loading": "Yn llwytho…",
    "links.empty": "Dim testun wedi'i gofnodi eto.",
    "links.loadFailed": "Methwyd â llwytho testunau'r achos hwn",
    "links.person": "Person",
    "links.personHint": "Cyfeirnod y person, ar y ffurf person:<uuid>.",
    "links.confidence": "Hyder",
    "links.confidenceHint":
      "Dewisol; 0 i 1. Gadewch yn wag ar gyfer honiad pendant.",
    "links.provenance": "Tarddiad",
    "links.provenanceHint": "Dewisol; “operator” yn ddiofyn.",
    "links.validFrom": "Dilys o",
    "links.validTo": "Dilys hyd",
    "links.validity": "Dilysrwydd",
    "links.addTitle": "Cofnodi testun",
    "links.record": "Cofnodi'r testun",
    "links.recording": "Yn cofnodi…",
    "links.withdraw": "Tynnu'n ôl",
    "links.withdrawing": "Yn tynnu'n ôl…",
    "links.withdrawConfirm":
      "Tynnu'n ôl yr honiad mai {ref} yw testun yr achos hwn? Caiff y tynnu'n ôl ei gofnodi a'i archwilio.",
    "links.recordFailed": "Methwyd â chofnodi'r testun",
    "links.withdrawFailed": "Methwyd â thynnu'r cyswllt yn ôl",
    "links.invalidPersonRef":
      "Rhowch gyfeirnod person ar y ffurf person:<uuid>.",
    "links.confidenceRange": "Rhaid i'r hyder fod rhwng 0 ac 1.",
    "search.placeholder": "Chwilio…",
    "search.submit": "Chwilio",
    "search.fuzzy": "Aneglur",
    "search.phonetic": "Ffonetig",
    "search.failed": "Methodd y chwilio",
    "detail.viewAudit": "Gweld hanes archwilio",
    "nav.audit": "Gweithgarwch",
    "audit.title": "Hanes archwilio",
    "audit.backToCase": "Yn ôl i'r achos",
    "audit.loading": "Yn llwytho…",
    "audit.noEntries": "Dim cofnodion archwilio eto.",
    "audit.by": "gan",
    "audit.payload": "Manylion",
    "audit.loadFailed": "Methwyd llwytho'r hanes archwilio",
    "activity.title": "Gweithgarwch diweddar",
    "activity.recentAudit": "Cofnodion archwilio diweddar",
    "activity.recentEvents": "Digwyddiadau diweddar",
    "activity.loading": "Yn llwytho…",
    "activity.loadFailed": "Methwyd llwytho'r gweithgarwch diweddar",
    "activity.noAuditEntries": "Dim cofnodion archwilio eto.",
    "activity.noEvents": "Dim digwyddiadau eto.",
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
    "splash.hero.title": "Un cofnod clir ar gyfer pob achos",
    "splash.hero.subtitle":
      "Cofrestrwch achosion unwaith, dewch o hyd iddynt ar unwaith, a dalwch ddyblygiadau cyn iddynt luosogi, gyda phob newid wedi'i gofnodi mewn hanes archwilio.",
    "splash.benefits.1.title": "Pob achos, un cofnod",
    "splash.benefits.1.body":
      "Cadwch bob achos mewn un cofnod a rennir yn lle taenlenni a ffeiliau gwasgaredig.",
    "splash.benefits.2.title": "Dewch o hyd iddo'n gyflym",
    "splash.benefits.2.body":
      "Chwiliwch yn ôl teitl, pwnc, asiantaeth neu ddynodwr, hyd yn oed gyda theipos neu enwau sy'n swnio'n debyg.",
    "splash.benefits.3.title": "Atal achosion dyblyg",
    "splash.benefits.3.body":
      "Caiff dyblygiadau tebygol eu nodi i'w hadolygu fel na fydd ail gofnod yn ymddangos yn dawel bach.",
    "splash.benefits.4.title": "Llwyth gwaith ar un olwg",
    "splash.benefits.4.body":
      "Mae bwrdd yn grwpio pob achos yn ôl statws fel y gallwch weld ble mae gwaith yn pentyrru.",
    "splash.benefits.5.title": "Atebolrwydd clir",
    "splash.benefits.5.body":
      "Caiff pob newid ei gofnodi, felly gallwch bob amser weld beth ddigwyddodd i achos a phryd.",
    "splash.benefits.6.title": "Cysylltu achosion â phobl",
    "splash.benefits.6.body":
      "Cofnodwch pwy yw testun achos trwy gysylltu â'u cofnod person, heb gopïo eu manylion.",
    "splash.features.1.title": "Cofnodion achos cyflawn",
    "splash.features.1.body":
      "Teitl, asiantaeth, rhif achos, math, statws, blaenoriaeth, pynciau a dynodwyr mewn un ffurflen.",
    "splash.features.2.title": "Chwilio testun llawn",
    "splash.features.2.body":
      "Mae opsiynau annelwig a seinegol yn dod o hyd i achosion er gwaethaf camsillafu ac enwau amgen.",
    "splash.features.3.title": "Gwirio dyblygiadau",
    "splash.features.3.body":
      "Cymharwch achos â'r rhai sydd wedi'u storio ac adolygwch y cyfatebiaethau sgôr cyn cadw.",
    "splash.features.4.title": "Uno achosion",
    "splash.features.4.body":
      "Unwch ddyblyg wedi'i gadarnhau i'r cofnod sy'n goroesi ac adolygwch yr uno diweddar.",
    "splash.features.5.title": "Bwrdd statws",
    "splash.features.5.body":
      "Llusgwch gerdyn i golofn arall i newid statws achos.",
    "splash.features.6.title": "Hanes archwilio",
    "splash.features.6.body":
      "Gwelwch hanes un achos, neu weithgarwch diweddar ar draws pob achos, y diweddaraf yn gyntaf.",
  },
  "en-001": {
    "nav.board": "Board",
    // Layout / chrome
    "brand.name": "Main X · Cases",
    "nav.toggle": "Toggle navigation",
    "nav.cases": "Cases",
    "nav.newCase": "New case",
    "chrome.language": "Language",
    "chrome.share": "Share",
    "chrome.textSize": "Text size",
    "share.copyLink": "Copy Link",
    "share.linkCopied": "Link copied",
    "share.copyFailed": "Could not copy — copy it from the address bar",
    "chrome.theme": "Theme",
    // Session
    "session.title": "Session",
    "session.tokenAttached": "Token attached.",
    "session.clearToken": "Clear token",
    "session.noToken": "No token.",
    "session.signIn": "Sign in",
    "session.pasteToken": "Paste a token",
    "session.accessToken": "Access token",
    "session.pastePlaceholder": "Paste access token",
    "session.useToken": "Use token",
    "session.hint": "From the authentication-service (magic-link sign-in).",
    // Cases list
    "list.title": "Cases",
    "list.new": "New case",
    "list.loading": "Loading…",
    "list.empty": "No cases yet.",
    "list.createOne": "Create one",
    "list.loadFailed": "Failed to load cases",
    // Case detail
    "detail.loading": "Loading…",
    "detail.notFound": "Not found",
    "detail.caseType": "Case type:",
    "detail.status": "Status:",
    "detail.priority": "Priority:",
    "detail.agency": "Agency:",
    "detail.caseNumber": "Case number:",
    "detail.opened": "Opened:",
    "detail.subjects": "Subjects:",
    "detail.identifiers": "Identifiers:",
    "detail.keywords": "Keywords:",
    "detail.id": "ID:",
    "detail.edit": "Edit",
    "detail.checkDuplicates": "Check duplicates",
    "detail.checking": "Checking…",
    "detail.checkFailed": "Check failed",
    "detail.delete": "Delete",
    "detail.potentialDuplicates": "Potential duplicates",
    "detail.noneAboveThreshold": "None above the match threshold.",
    // New case
    "new.title": "New case",
    "new.create": "Create",
    // Edit case
    "edit.title": "Edit case",
    "edit.loading": "Loading…",
    "edit.notFound": "Not found",
    "edit.saveChanges": "Save changes",
    // Case form
    "form.title": "Title",
    "form.caseType": "Case type",
    "form.status": "Status",
    "form.priority": "Priority",
    "form.caseNumber": "Case number",
    "form.openedDate": "Opened date",
    "form.agencyId": "Agency id",
    "form.agencyName": "Agency name",
    "form.alternateTitles": "Alternate titles",
    "form.subjects": "Subjects",
    "form.keywords": "Keywords",
    "form.sameAs": "Same-as URLs",
    "form.languages": "Languages",
    "form.commaSeparated": "(comma-separated)",
    "form.commaSeparatedIso": "(comma-separated ISO 639-1)",
    "form.identifiers": "Identifiers",
    "form.valuePlaceholder": "value",
    "form.remove": "Remove",
    "form.addIdentifier": "+ Add identifier",
    "form.empty": "—",
    "form.save": "Save",
    "form.saving": "Saving…",
    "form.titleRequired": "Title is required.",
    "form.customLabel": "Custom label",
    "form.customLabelRequired": "A custom label is required.",
    "form.saveFailed": "Save failed",
    // Merge
    "nav.merge": "Merge",
    "merge.title": "Merge cases",
    "merge.mainId": "Main case id",
    "merge.mainIdHint": "The surviving case — it keeps its id.",
    "merge.dupId": "Duplicate case id",
    "merge.dupIdHint": "Folded into the main case, then soft-deleted.",
    "merge.reason": "Reason",
    "merge.reasonHint": "Optional; recorded in the merge history.",
    "merge.reasonPlaceholder": "Confirmed duplicate of the same case",
    "merge.loadPreview": "Load preview",
    "merge.merging": "Merging…",
    "merge.merge": "Merge",
    "merge.bothIdsRequired": "Both case ids are required.",
    "merge.mustDiffer": "The main and duplicate ids must differ.",
    "merge.preview": "Preview",
    "merge.main": "Main",
    "merge.duplicate": "Duplicate",
    "merge.completed": "Merge completed",
    "merge.viewMain": "View the main case",
    "merge.confirm":
      "Merge case {dup} into case {main}? The duplicate is soft-deleted.",
    "merge.recent": "Recent merges",
    "merge.recentEmpty": "No merges recorded yet.",
    "merge.recentFailed": "Failed to load recent merges",
    "merge.mergedAt": "Merged at",
    "merge.actor": "Actor",
    // Cross-service links (subject_of)
    "links.title": "Subject of this case",
    "links.note":
      "Records the person this case is about. The assertion is as sensitive as the case itself: reading and writing it takes the same authorisation, and every change is audited.",
    "links.loading": "Loading…",
    "links.empty": "No subject recorded yet.",
    "links.loadFailed": "Failed to load the subjects of this case",
    "links.person": "Person",
    "links.personHint": "The person's reference, in the form person:<uuid>.",
    "links.confidence": "Confidence",
    "links.confidenceHint":
      "Optional; 0 to 1. Leave blank for an outright assertion.",
    "links.provenance": "Provenance",
    "links.provenanceHint": "Optional; defaults to “operator”.",
    "links.validFrom": "Valid from",
    "links.validTo": "Valid to",
    "links.validity": "Validity",
    "links.addTitle": "Record a subject",
    "links.record": "Record subject",
    "links.recording": "Recording…",
    "links.withdraw": "Withdraw",
    "links.withdrawing": "Withdrawing…",
    "links.withdrawConfirm":
      "Withdraw the assertion that {ref} is the subject of this case? The withdrawal is recorded and audited.",
    "links.recordFailed": "Could not record the subject",
    "links.withdrawFailed": "Could not withdraw the link",
    "links.invalidPersonRef":
      "Enter a person reference of the form person:<uuid>.",
    "links.confidenceRange": "Confidence must be between 0 and 1.",
    // Search
    "search.placeholder": "Search…",
    "search.submit": "Search",
    "search.fuzzy": "Fuzzy",
    "search.phonetic": "Phonetic",
    "search.failed": "Search failed",
    "detail.viewAudit": "View audit trail",
    // Nav / audit / recent activity
    "nav.audit": "Activity",
    "audit.title": "Audit trail",
    "audit.backToCase": "Back to case",
    "audit.loading": "Loading…",
    "audit.noEntries": "No audit entries yet.",
    "audit.by": "by",
    "audit.payload": "Details",
    "audit.loadFailed": "Failed to load the audit trail",
    "activity.title": "Recent activity",
    "activity.recentAudit": "Recent audit entries",
    "activity.recentEvents": "Recent events",
    "activity.loading": "Loading…",
    "activity.loadFailed": "Failed to load recent activity",
    "activity.noAuditEntries": "No audit entries yet.",
    "activity.noEvents": "No events yet.",
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
    "splash.hero.title": "One clear record for every case",
    "splash.hero.subtitle":
      "Register cases once, find them instantly, and catch duplicates before they multiply, with every change recorded in an audit trail.",
    "splash.benefits.1.title": "Every case, one record",
    "splash.benefits.1.body":
      "Keep each case in a single shared record instead of scattered spreadsheets and files.",
    "splash.benefits.2.title": "Find it fast",
    "splash.benefits.2.body":
      "Search by title, subject, agency or identifier, even with typos or similar-sounding names.",
    "splash.benefits.3.title": "Stop duplicate cases",
    "splash.benefits.3.body":
      "Likely duplicates are flagged for review so a second record does not quietly appear.",
    "splash.benefits.4.title": "Workload at a glance",
    "splash.benefits.4.body":
      "A board groups every case by status so you can see where work is piling up.",
    "splash.benefits.5.title": "Clear accountability",
    "splash.benefits.5.body":
      "Every change is recorded, so you can always see what happened to a case and when.",
    "splash.benefits.6.title": "Link cases to people",
    "splash.benefits.6.body":
      "Record who a case is about by linking to their person record, without copying their details.",
    "splash.features.1.title": "Complete case records",
    "splash.features.1.body":
      "Title, agency, case number, type, status, priority, subjects and identifiers in one form.",
    "splash.features.2.title": "Full-text search",
    "splash.features.2.body":
      "Fuzzy and phonetic options find cases despite misspellings and alternate names.",
    "splash.features.3.title": "Duplicate check",
    "splash.features.3.body":
      "Compare a case with stored ones and review the scored matches before saving.",
    "splash.features.4.title": "Merge cases",
    "splash.features.4.body":
      "Fold a confirmed duplicate into the surviving record and review recent merges.",
    "splash.features.5.title": "Status board",
    "splash.features.5.body":
      "Drag a card to another column to change a case's status.",
    "splash.features.6.title": "Audit trail",
    "splash.features.6.body":
      "See one case's history, or recent activity across all cases, newest first.",
  },
  "es-001": {
    "nav.board": "Tablero",
    "brand.name": "Main X · Casos",
    "nav.toggle": "Alternar navegación",
    "nav.cases": "Casos",
    "nav.newCase": "Nuevo caso",
    "chrome.language": "Idioma",
    "chrome.share": "Compartir",
    "chrome.textSize": "Tamaño del texto",
    "share.copyLink": "Copiar enlace",
    "share.linkCopied": "Enlace copiado",
    "share.copyFailed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "chrome.theme": "Tema",
    "session.title": "Sesión",
    "session.tokenAttached": "Token adjunto.",
    "session.clearToken": "Borrar token",
    "session.noToken": "Sin token.",
    "session.signIn": "Iniciar sesión",
    "session.pasteToken": "Pegar un token",
    "session.accessToken": "Token de acceso",
    "session.pastePlaceholder": "Pegar token de acceso",
    "session.useToken": "Usar token",
    "session.hint": "Del authentication-service (inicio con enlace mágico).",
    "list.title": "Casos",
    "list.new": "Nuevo caso",
    "list.loading": "Cargando…",
    "list.empty": "Aún no hay casos.",
    "list.createOne": "Crear uno",
    "list.loadFailed": "No se pudieron cargar los casos",
    "detail.loading": "Cargando…",
    "detail.notFound": "No encontrado",
    "detail.caseType": "Tipo de caso:",
    "detail.status": "Estado:",
    "detail.priority": "Prioridad:",
    "detail.agency": "Agencia:",
    "detail.caseNumber": "Número de caso:",
    "detail.opened": "Abierto:",
    "detail.subjects": "Asuntos:",
    "detail.identifiers": "Identificadores:",
    "detail.keywords": "Palabras clave:",
    "detail.id": "ID:",
    "detail.edit": "Editar",
    "detail.checkDuplicates": "Comprobar duplicados",
    "detail.checking": "Comprobando…",
    "detail.checkFailed": "La comprobación falló",
    "detail.delete": "Eliminar",
    "detail.potentialDuplicates": "Posibles duplicados",
    "detail.noneAboveThreshold":
      "Ninguno por encima del umbral de coincidencia.",
    "new.title": "Nuevo caso",
    "new.create": "Crear",
    "edit.title": "Editar caso",
    "edit.loading": "Cargando…",
    "edit.notFound": "No encontrado",
    "edit.saveChanges": "Guardar cambios",
    "form.title": "Título",
    "form.caseType": "Tipo de caso",
    "form.status": "Estado",
    "form.priority": "Prioridad",
    "form.caseNumber": "Número de caso",
    "form.openedDate": "Fecha de apertura",
    "form.agencyId": "ID de agencia",
    "form.agencyName": "Nombre de agencia",
    "form.alternateTitles": "Títulos alternativos",
    "form.subjects": "Asuntos",
    "form.keywords": "Palabras clave",
    "form.sameAs": "URL de igual que",
    "form.languages": "Idiomas",
    "form.commaSeparated": "(separados por comas)",
    "form.commaSeparatedIso": "(ISO 639-1 separados por comas)",
    "form.identifiers": "Identificadores",
    "form.valuePlaceholder": "valor",
    "form.remove": "Eliminar",
    "form.addIdentifier": "+ Añadir identificador",
    "form.empty": "—",
    "form.save": "Guardar",
    "form.saving": "Guardando…",
    "form.titleRequired": "El título es obligatorio.",
    "form.customLabel": "Etiqueta personalizada",
    "form.customLabelRequired": "Se requiere una etiqueta personalizada.",
    "form.saveFailed": "Error al guardar",
    "nav.merge": "Fusionar",
    "merge.title": "Fusionar casos",
    "merge.mainId": "ID del caso principal",
    "merge.mainIdHint": "El caso que sobrevive: conserva su ID.",
    "merge.dupId": "ID del caso duplicado",
    "merge.dupIdHint":
      "Se integra en el caso principal y luego se elimina de forma lógica.",
    "merge.reason": "Motivo",
    "merge.reasonHint": "Opcional; se registra en el historial de fusiones.",
    "merge.reasonPlaceholder": "Duplicado confirmado del mismo caso",
    "merge.loadPreview": "Cargar vista previa",
    "merge.merging": "Fusionando…",
    "merge.merge": "Fusionar",
    "merge.bothIdsRequired": "Se requieren ambos ID de caso.",
    "merge.mustDiffer": "Los ID principal y duplicado deben ser distintos.",
    "merge.preview": "Vista previa",
    "merge.main": "Principal",
    "merge.duplicate": "Duplicado",
    "merge.completed": "Fusión completada",
    "merge.viewMain": "Ver el caso principal",
    "merge.confirm":
      "¿Fusionar el caso {dup} con el caso {main}? El duplicado se eliminará de forma lógica.",
    "merge.recent": "Fusiones recientes",
    "merge.recentEmpty": "Aún no hay fusiones registradas.",
    "merge.recentFailed": "No se pudieron cargar las fusiones recientes",
    "merge.mergedAt": "Fusionado el",
    "merge.actor": "Actor",
    // Cross-service links (subject_of)
    "links.title": "Persona objeto de este caso",
    "links.note":
      "Registra la persona a la que se refiere este caso. La afirmación es tan sensible como el propio caso: leerla y escribirla requiere la misma autorización, y cada cambio queda auditado.",
    "links.loading": "Cargando…",
    "links.empty": "Aún no se ha registrado ninguna persona.",
    "links.loadFailed":
      "No se pudieron cargar las personas objeto de este caso",
    "links.person": "Persona",
    "links.personHint":
      "Referencia de la persona, con el formato person:<uuid>.",
    "links.confidence": "Confianza",
    "links.confidenceHint":
      "Opcional; de 0 a 1. Déjelo en blanco para una afirmación rotunda.",
    "links.provenance": "Procedencia",
    "links.provenanceHint": "Opcional; «operator» por defecto.",
    "links.validFrom": "Válido desde",
    "links.validTo": "Válido hasta",
    "links.validity": "Vigencia",
    "links.addTitle": "Registrar una persona",
    "links.record": "Registrar persona",
    "links.recording": "Registrando…",
    "links.withdraw": "Retirar",
    "links.withdrawing": "Retirando…",
    "links.withdrawConfirm":
      "¿Retirar la afirmación de que {ref} es objeto de este caso? La retirada queda registrada y auditada.",
    "links.recordFailed": "No se pudo registrar la persona",
    "links.withdrawFailed": "No se pudo retirar el vínculo",
    "links.invalidPersonRef":
      "Introduzca una referencia de persona con el formato person:<uuid>.",
    "links.confidenceRange": "La confianza debe estar entre 0 y 1.",
    "search.placeholder": "Buscar…",
    "search.submit": "Buscar",
    "search.fuzzy": "Aproximada",
    "search.phonetic": "Fonética",
    "search.failed": "La búsqueda falló",
    "detail.viewAudit": "Ver historial de auditoría",
    "nav.audit": "Actividad",
    "audit.title": "Historial de auditoría",
    "audit.backToCase": "Volver al caso",
    "audit.loading": "Cargando…",
    "audit.noEntries": "Aún no hay entradas de auditoría.",
    "audit.by": "por",
    "audit.payload": "Detalles",
    "audit.loadFailed": "No se pudo cargar el historial de auditoría",
    "activity.title": "Actividad reciente",
    "activity.recentAudit": "Entradas de auditoría recientes",
    "activity.recentEvents": "Eventos recientes",
    "activity.loading": "Cargando…",
    "activity.loadFailed": "No se pudo cargar la actividad reciente",
    "activity.noAuditEntries": "Aún no hay entradas de auditoría.",
    "activity.noEvents": "Aún no hay eventos.",
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
    "splash.hero.title": "Un registro claro para cada caso",
    "splash.hero.subtitle":
      "Registra los casos una sola vez, encuéntralos al instante y detecta los duplicados antes de que se multipliquen, con cada cambio anotado en un historial de auditoría.",
    "splash.benefits.1.title": "Cada caso, un registro",
    "splash.benefits.1.body":
      "Mantén cada caso en un único registro compartido en lugar de hojas de cálculo y archivos dispersos.",
    "splash.benefits.2.title": "Encuéntralo rápido",
    "splash.benefits.2.body":
      "Busca por título, tema, organismo o identificador, incluso con errores de escritura o nombres que suenan parecido.",
    "splash.benefits.3.title": "Evita casos duplicados",
    "splash.benefits.3.body":
      "Los posibles duplicados se marcan para su revisión y así no aparece un segundo registro sin que nadie lo note.",
    "splash.benefits.4.title": "La carga de trabajo de un vistazo",
    "splash.benefits.4.body":
      "Un tablero agrupa cada caso por estado para que veas dónde se acumula el trabajo.",
    "splash.benefits.5.title": "Responsabilidad clara",
    "splash.benefits.5.body":
      "Cada cambio queda registrado, así que siempre puedes ver qué le pasó a un caso y cuándo.",
    "splash.benefits.6.title": "Vincula casos con personas",
    "splash.benefits.6.body":
      "Registra de quién trata un caso enlazando su ficha de persona, sin copiar sus datos.",
    "splash.features.1.title": "Fichas de caso completas",
    "splash.features.1.body":
      "Título, organismo, número de caso, tipo, estado, prioridad, temas e identificadores en un solo formulario.",
    "splash.features.2.title": "Búsqueda de texto completo",
    "splash.features.2.body":
      "Las opciones aproximada y fonética encuentran casos pese a errores ortográficos y nombres alternativos.",
    "splash.features.3.title": "Comprobación de duplicados",
    "splash.features.3.body":
      "Compara un caso con los almacenados y revisa las coincidencias puntuadas antes de guardar.",
    "splash.features.4.title": "Fusión de casos",
    "splash.features.4.body":
      "Incorpora un duplicado confirmado al registro superviviente y revisa las fusiones recientes.",
    "splash.features.5.title": "Tablero de estados",
    "splash.features.5.body":
      "Arrastra una tarjeta a otra columna para cambiar el estado de un caso.",
    "splash.features.6.title": "Historial de auditoría",
    "splash.features.6.body":
      "Consulta el historial de un caso o la actividad reciente de todos los casos, lo más nuevo primero.",
  },
  "fr-001": {
    "nav.board": "Tableau",
    "brand.name": "Main X · Affaires",
    "nav.toggle": "Basculer la navigation",
    "nav.cases": "Affaires",
    "nav.newCase": "Nouvelle affaire",
    "chrome.language": "Langue",
    "chrome.share": "Partager",
    "chrome.textSize": "Taille du texte",
    "share.copyLink": "Copier le lien",
    "share.linkCopied": "Lien copié",
    "share.copyFailed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "chrome.theme": "Thème",
    "session.title": "Session",
    "session.tokenAttached": "Jeton attaché.",
    "session.clearToken": "Effacer le jeton",
    "session.noToken": "Aucun jeton.",
    "session.signIn": "Se connecter",
    "session.pasteToken": "Coller un jeton",
    "session.accessToken": "Jeton d'accès",
    "session.pastePlaceholder": "Coller le jeton d'accès",
    "session.useToken": "Utiliser le jeton",
    "session.hint":
      "Depuis l'authentication-service (connexion par lien magique).",
    "list.title": "Affaires",
    "list.new": "Nouvelle affaire",
    "list.loading": "Chargement…",
    "list.empty": "Aucune affaire pour l'instant.",
    "list.createOne": "En créer une",
    "list.loadFailed": "Échec du chargement des affaires",
    "detail.loading": "Chargement…",
    "detail.notFound": "Introuvable",
    "detail.caseType": "Type d'affaire :",
    "detail.status": "Statut :",
    "detail.priority": "Priorité :",
    "detail.agency": "Agence :",
    "detail.caseNumber": "Numéro d'affaire :",
    "detail.opened": "Ouverte :",
    "detail.subjects": "Sujets :",
    "detail.identifiers": "Identifiants :",
    "detail.keywords": "Mots-clés :",
    "detail.id": "ID :",
    "detail.edit": "Modifier",
    "detail.checkDuplicates": "Vérifier les doublons",
    "detail.checking": "Vérification…",
    "detail.checkFailed": "Échec de la vérification",
    "detail.delete": "Supprimer",
    "detail.potentialDuplicates": "Doublons potentiels",
    "detail.noneAboveThreshold": "Aucun au-dessus du seuil de correspondance.",
    "new.title": "Nouvelle affaire",
    "new.create": "Créer",
    "edit.title": "Modifier l'affaire",
    "edit.loading": "Chargement…",
    "edit.notFound": "Introuvable",
    "edit.saveChanges": "Enregistrer les modifications",
    "form.title": "Titre",
    "form.caseType": "Type d'affaire",
    "form.status": "Statut",
    "form.priority": "Priorité",
    "form.caseNumber": "Numéro d'affaire",
    "form.openedDate": "Date d'ouverture",
    "form.agencyId": "ID de l'agence",
    "form.agencyName": "Nom de l'agence",
    "form.alternateTitles": "Titres alternatifs",
    "form.subjects": "Sujets",
    "form.keywords": "Mots-clés",
    "form.sameAs": "URL identiques à",
    "form.languages": "Langues",
    "form.commaSeparated": "(séparés par des virgules)",
    "form.commaSeparatedIso": "(ISO 639-1 séparés par des virgules)",
    "form.identifiers": "Identifiants",
    "form.valuePlaceholder": "valeur",
    "form.remove": "Supprimer",
    "form.addIdentifier": "+ Ajouter un identifiant",
    "form.empty": "—",
    "form.save": "Enregistrer",
    "form.saving": "Enregistrement…",
    "form.titleRequired": "Le titre est obligatoire.",
    "form.customLabel": "Étiquette personnalisée",
    "form.customLabelRequired": "Une étiquette personnalisée est requise.",
    "form.saveFailed": "Échec de l'enregistrement",
    "nav.merge": "Fusionner",
    "merge.title": "Fusionner des affaires",
    "merge.mainId": "ID de l'affaire principale",
    "merge.mainIdHint": "L'affaire conservée — elle garde son ID.",
    "merge.dupId": "ID de l'affaire en double",
    "merge.dupIdHint":
      "Intégrée à l'affaire principale, puis supprimée logiquement.",
    "merge.reason": "Motif",
    "merge.reasonHint":
      "Facultatif ; enregistré dans l'historique des fusions.",
    "merge.reasonPlaceholder": "Doublon confirmé de la même affaire",
    "merge.loadPreview": "Charger l'aperçu",
    "merge.merging": "Fusion…",
    "merge.merge": "Fusionner",
    "merge.bothIdsRequired": "Les deux ID d'affaire sont requis.",
    "merge.mustDiffer": "L'ID principal et l'ID en double doivent différer.",
    "merge.preview": "Aperçu",
    "merge.main": "Principale",
    "merge.duplicate": "Doublon",
    "merge.completed": "Fusion terminée",
    "merge.viewMain": "Voir l'affaire principale",
    "merge.confirm":
      "Fusionner l'affaire {dup} dans l'affaire {main} ? Le doublon sera supprimé logiquement.",
    "merge.recent": "Fusions récentes",
    "merge.recentEmpty": "Aucune fusion enregistrée pour l'instant.",
    "merge.recentFailed": "Échec du chargement des fusions récentes",
    "merge.mergedAt": "Fusionné le",
    "merge.actor": "Acteur",
    // Cross-service links (subject_of)
    "links.title": "Personne concernée par ce dossier",
    "links.note":
      "Enregistre la personne visée par ce dossier. L'affirmation est aussi sensible que le dossier lui-même : la lire et l'écrire exige la même autorisation, et chaque modification est auditée.",
    "links.loading": "Chargement…",
    "links.empty": "Aucune personne enregistrée pour l'instant.",
    "links.loadFailed":
      "Échec du chargement des personnes concernées par ce dossier",
    "links.person": "Personne",
    "links.personHint": "Référence de la personne, au format person:<uuid>.",
    "links.confidence": "Confiance",
    "links.confidenceHint":
      "Facultatif ; de 0 à 1. Laisser vide pour une affirmation ferme.",
    "links.provenance": "Provenance",
    "links.provenanceHint": "Facultatif ; « operator » par défaut.",
    "links.validFrom": "Valide à partir du",
    "links.validTo": "Valide jusqu'au",
    "links.validity": "Validité",
    "links.addTitle": "Enregistrer une personne",
    "links.record": "Enregistrer la personne",
    "links.recording": "Enregistrement…",
    "links.withdraw": "Retirer",
    "links.withdrawing": "Retrait…",
    "links.withdrawConfirm":
      "Retirer l'affirmation selon laquelle {ref} est concernée par ce dossier ? Le retrait est consigné et audité.",
    "links.recordFailed": "Impossible d'enregistrer la personne",
    "links.withdrawFailed": "Impossible de retirer le lien",
    "links.invalidPersonRef":
      "Saisissez une référence de personne au format person:<uuid>.",
    "links.confidenceRange": "La confiance doit être comprise entre 0 et 1.",
    "search.placeholder": "Rechercher…",
    "search.submit": "Rechercher",
    "search.fuzzy": "Approximative",
    "search.phonetic": "Phonétique",
    "search.failed": "Échec de la recherche",
    "detail.viewAudit": "Voir l'historique d'audit",
    "nav.audit": "Activité",
    "audit.title": "Historique d'audit",
    "audit.backToCase": "Retour à l'affaire",
    "audit.loading": "Chargement…",
    "audit.noEntries": "Aucune entrée d'audit pour l'instant.",
    "audit.by": "par",
    "audit.payload": "Détails",
    "audit.loadFailed": "Échec du chargement de l'historique d'audit",
    "activity.title": "Activité récente",
    "activity.recentAudit": "Entrées d'audit récentes",
    "activity.recentEvents": "Événements récents",
    "activity.loading": "Chargement…",
    "activity.loadFailed": "Échec du chargement de l'activité récente",
    "activity.noAuditEntries": "Aucune entrée d'audit pour l'instant.",
    "activity.noEvents": "Aucun événement pour l'instant.",
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
    "splash.hero.title": "Un dossier clair pour chaque affaire",
    "splash.hero.subtitle":
      "Enregistrez chaque affaire une seule fois, retrouvez-la instantanément et repérez les doublons avant qu'ils ne se multiplient, chaque modification étant consignée dans un historique d'audit.",
    "splash.benefits.1.title": "Chaque affaire, un dossier",
    "splash.benefits.1.body":
      "Gardez chaque affaire dans un seul dossier partagé plutôt que dans des tableurs et des fichiers éparpillés.",
    "splash.benefits.2.title": "Retrouvez-le vite",
    "splash.benefits.2.body":
      "Recherchez par titre, sujet, organisme ou identifiant, même avec des fautes de frappe ou des noms qui se ressemblent.",
    "splash.benefits.3.title": "Évitez les affaires en double",
    "splash.benefits.3.body":
      "Les doublons probables sont signalés pour examen afin qu'un second dossier n'apparaisse pas discrètement.",
    "splash.benefits.4.title": "La charge de travail en un coup d'œil",
    "splash.benefits.4.body":
      "Un tableau regroupe chaque affaire par statut pour voir où le travail s'accumule.",
    "splash.benefits.5.title": "Responsabilité claire",
    "splash.benefits.5.body":
      "Chaque modification est consignée : vous voyez toujours ce qui s'est passé pour une affaire et quand.",
    "splash.benefits.6.title": "Reliez les affaires aux personnes",
    "splash.benefits.6.body":
      "Indiquez qui est concerné par une affaire en la reliant à sa fiche personne, sans recopier ses données.",
    "splash.features.1.title": "Dossiers complets",
    "splash.features.1.body":
      "Titre, organisme, numéro d'affaire, type, statut, priorité, sujets et identifiants dans un seul formulaire.",
    "splash.features.2.title": "Recherche plein texte",
    "splash.features.2.body":
      "Les options approximative et phonétique retrouvent les affaires malgré les fautes et les noms alternatifs.",
    "splash.features.3.title": "Vérification des doublons",
    "splash.features.3.body":
      "Comparez une affaire aux affaires enregistrées et examinez les correspondances notées avant d'enregistrer.",
    "splash.features.4.title": "Fusion d'affaires",
    "splash.features.4.body":
      "Intégrez un doublon confirmé dans le dossier conservé et consultez les fusions récentes.",
    "splash.features.5.title": "Tableau des statuts",
    "splash.features.5.body":
      "Faites glisser une carte vers une autre colonne pour changer le statut d'une affaire.",
    "splash.features.6.title": "Historique d'audit",
    "splash.features.6.body":
      "Consultez l'historique d'une affaire ou l'activité récente de toutes les affaires, les plus récentes d'abord.",
  },
  "hi-001": {
    "nav.board": "बोर्ड",
    "brand.name": "Main X · मामले",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "nav.cases": "मामले",
    "nav.newCase": "नया मामला",
    "chrome.language": "भाषा",
    "chrome.share": "साझा करें",
    "chrome.textSize": "टेक्स्ट का आकार",
    "share.copyLink": "लिंक कॉपी करें",
    "share.linkCopied": "लिंक कॉपी हो गया",
    "share.copyFailed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "chrome.theme": "थीम",
    "session.title": "सत्र",
    "session.tokenAttached": "टोकन संलग्न।",
    "session.clearToken": "टोकन साफ़ करें",
    "session.noToken": "कोई टोकन नहीं।",
    "session.signIn": "साइन इन करें",
    "session.pasteToken": "टोकन चिपकाएँ",
    "session.accessToken": "एक्सेस टोकन",
    "session.pastePlaceholder": "एक्सेस टोकन चिपकाएँ",
    "session.useToken": "टोकन उपयोग करें",
    "session.hint": "authentication-service से (मैजिक-लिंक साइन-इन)।",
    "list.title": "मामले",
    "list.new": "नया मामला",
    "list.loading": "लोड हो रहा है…",
    "list.empty": "अभी तक कोई मामला नहीं।",
    "list.createOne": "एक बनाएँ",
    "list.loadFailed": "मामले लोड करने में विफल",
    "detail.loading": "लोड हो रहा है…",
    "detail.notFound": "नहीं मिला",
    "detail.caseType": "मामले का प्रकार:",
    "detail.status": "स्थिति:",
    "detail.priority": "प्राथमिकता:",
    "detail.agency": "एजेंसी:",
    "detail.caseNumber": "मामला संख्या:",
    "detail.opened": "खोला गया:",
    "detail.subjects": "विषय:",
    "detail.identifiers": "पहचानकर्ता:",
    "detail.keywords": "मुख्य शब्द:",
    "detail.id": "ID:",
    "detail.edit": "संपादित करें",
    "detail.checkDuplicates": "डुप्लिकेट जाँचें",
    "detail.checking": "जाँच हो रही है…",
    "detail.checkFailed": "जाँच विफल",
    "detail.delete": "हटाएँ",
    "detail.potentialDuplicates": "संभावित डुप्लिकेट",
    "detail.noneAboveThreshold": "मिलान सीमा से ऊपर कोई नहीं।",
    "new.title": "नया मामला",
    "new.create": "बनाएँ",
    "edit.title": "मामला संपादित करें",
    "edit.loading": "लोड हो रहा है…",
    "edit.notFound": "नहीं मिला",
    "edit.saveChanges": "परिवर्तन सहेजें",
    "form.title": "शीर्षक",
    "form.caseType": "मामले का प्रकार",
    "form.status": "स्थिति",
    "form.priority": "प्राथमिकता",
    "form.caseNumber": "मामला संख्या",
    "form.openedDate": "खोलने की तिथि",
    "form.agencyId": "एजेंसी ID",
    "form.agencyName": "एजेंसी का नाम",
    "form.alternateTitles": "वैकल्पिक शीर्षक",
    "form.subjects": "विषय",
    "form.keywords": "मुख्य शब्द",
    "form.sameAs": "समान-रूप URLs",
    "form.languages": "भाषाएँ",
    "form.commaSeparated": "(अल्पविराम से अलग)",
    "form.commaSeparatedIso": "(ISO 639-1 अल्पविराम से अलग)",
    "form.identifiers": "पहचानकर्ता",
    "form.valuePlaceholder": "मान",
    "form.remove": "हटाएँ",
    "form.addIdentifier": "+ पहचानकर्ता जोड़ें",
    "form.empty": "—",
    "form.save": "सहेजें",
    "form.saving": "सहेजा जा रहा है…",
    "form.titleRequired": "शीर्षक आवश्यक है।",
    "form.customLabel": "कस्टम लेबल",
    "form.customLabelRequired": "कस्टम लेबल आवश्यक है।",
    "form.saveFailed": "सहेजना विफल",
    "nav.merge": "विलय",
    "merge.title": "मामलों का विलय",
    "merge.mainId": "मुख्य मामले की ID",
    "merge.mainIdHint": "बचा रहने वाला मामला — इसकी ID वही रहती है।",
    "merge.dupId": "डुप्लिकेट मामले की ID",
    "merge.dupIdHint":
      "मुख्य मामले में मिला दिया जाता है, फिर सॉफ़्ट-डिलीट होता है।",
    "merge.reason": "कारण",
    "merge.reasonHint": "वैकल्पिक; विलय इतिहास में दर्ज होता है।",
    "merge.reasonPlaceholder": "उसी मामले की पुष्ट डुप्लिकेट",
    "merge.loadPreview": "पूर्वावलोकन लोड करें",
    "merge.merging": "विलय हो रहा है…",
    "merge.merge": "विलय करें",
    "merge.bothIdsRequired": "दोनों मामला ID आवश्यक हैं।",
    "merge.mustDiffer": "मुख्य और डुप्लिकेट ID अलग होने चाहिए।",
    "merge.preview": "पूर्वावलोकन",
    "merge.main": "मुख्य",
    "merge.duplicate": "डुप्लिकेट",
    "merge.completed": "विलय पूर्ण",
    "merge.viewMain": "मुख्य मामला देखें",
    "merge.confirm":
      "मामला {dup} को मामला {main} में विलय करें? डुप्लिकेट सॉफ़्ट-डिलीट हो जाएगा।",
    "merge.recent": "हाल के विलय",
    "merge.recentEmpty": "अभी तक कोई विलय दर्ज नहीं।",
    "merge.recentFailed": "हाल के विलय लोड करने में विफल",
    "merge.mergedAt": "विलय का समय",
    "merge.actor": "कर्ता",
    // Cross-service links (subject_of)
    "links.title": "इस मामले का विषय व्यक्ति",
    "links.note":
      "यह दर्ज करता है कि मामला किस व्यक्ति से संबंधित है। यह कथन स्वयं मामले जितना ही संवेदनशील है: इसे पढ़ने और लिखने के लिए वही अनुमति चाहिए, और हर बदलाव का ऑडिट होता है।",
    "links.loading": "लोड हो रहा है…",
    "links.empty": "अभी तक कोई व्यक्ति दर्ज नहीं।",
    "links.loadFailed": "इस मामले के विषय व्यक्ति लोड नहीं हो सके",
    "links.person": "व्यक्ति",
    "links.personHint": "व्यक्ति का संदर्भ, person:<uuid> प्रारूप में।",
    "links.confidence": "विश्वास",
    "links.confidenceHint":
      "वैकल्पिक; 0 से 1 तक। निश्चित कथन के लिए खाली छोड़ें।",
    "links.provenance": "स्रोत",
    "links.provenanceHint": "वैकल्पिक; डिफ़ॉल्ट “operator”।",
    "links.validFrom": "इस तिथि से मान्य",
    "links.validTo": "इस तिथि तक मान्य",
    "links.validity": "वैधता",
    "links.addTitle": "व्यक्ति दर्ज करें",
    "links.record": "व्यक्ति दर्ज करें",
    "links.recording": "दर्ज हो रहा है…",
    "links.withdraw": "वापस लें",
    "links.withdrawing": "वापस लिया जा रहा है…",
    "links.withdrawConfirm":
      "यह कथन वापस लें कि {ref} इस मामले का विषय है? वापसी दर्ज और ऑडिट की जाती है।",
    "links.recordFailed": "व्यक्ति दर्ज नहीं हो सका",
    "links.withdrawFailed": "कड़ी वापस नहीं ली जा सकी",
    "links.invalidPersonRef":
      "person:<uuid> प्रारूप में व्यक्ति का संदर्भ दर्ज करें।",
    "links.confidenceRange": "विश्वास 0 और 1 के बीच होना चाहिए।",
    "search.placeholder": "खोजें…",
    "search.submit": "खोजें",
    "search.fuzzy": "अस्पष्ट खोज",
    "search.phonetic": "ध्वन्यात्मक",
    "search.failed": "खोज विफल",
    "detail.viewAudit": "ऑडिट इतिहास देखें",
    "nav.audit": "गतिविधि",
    "audit.title": "ऑडिट इतिहास",
    "audit.backToCase": "मामले पर वापस जाएँ",
    "audit.loading": "लोड हो रहा है…",
    "audit.noEntries": "अभी तक कोई ऑडिट प्रविष्टि नहीं।",
    "audit.by": "द्वारा",
    "audit.payload": "विवरण",
    "audit.loadFailed": "ऑडिट इतिहास लोड करने में विफल",
    "activity.title": "हाल की गतिविधि",
    "activity.recentAudit": "हाल की ऑडिट प्रविष्टियाँ",
    "activity.recentEvents": "हाल की घटनाएँ",
    "activity.loading": "लोड हो रहा है…",
    "activity.loadFailed": "हाल की गतिविधि लोड करने में विफल",
    "activity.noAuditEntries": "अभी तक कोई ऑडिट प्रविष्टि नहीं।",
    "activity.noEvents": "अभी तक कोई घटना नहीं।",
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
    "splash.hero.title": "हर मामले के लिए एक स्पष्ट रिकॉर्ड",
    "splash.hero.subtitle":
      "मामलों को एक बार दर्ज करें, उन्हें तुरंत खोजें और डुप्लिकेट बढ़ने से पहले पकड़ें, हर बदलाव ऑडिट ट्रेल में दर्ज होता है।",
    "splash.benefits.1.title": "हर मामला, एक रिकॉर्ड",
    "splash.benefits.1.body":
      "बिखरी स्प्रेडशीट और फ़ाइलों की जगह हर मामले को एक साझा रिकॉर्ड में रखें।",
    "splash.benefits.2.title": "तुरंत खोजें",
    "splash.benefits.2.body":
      "शीर्षक, विषय, एजेंसी या पहचानकर्ता से खोजें, टाइपिंग की गलतियों या मिलते-जुलते नामों के साथ भी।",
    "splash.benefits.3.title": "डुप्लिकेट मामले रोकें",
    "splash.benefits.3.body":
      "संभावित डुप्लिकेट समीक्षा के लिए चिह्नित किए जाते हैं, ताकि दूसरा रिकॉर्ड चुपचाप न बन जाए।",
    "splash.benefits.4.title": "कार्यभार एक नज़र में",
    "splash.benefits.4.body":
      "बोर्ड हर मामले को स्थिति के अनुसार समूहित करता है, ताकि दिखे कि काम कहाँ जमा हो रहा है।",
    "splash.benefits.5.title": "स्पष्ट जवाबदेही",
    "splash.benefits.5.body":
      "हर बदलाव दर्ज होता है, इसलिए आप हमेशा देख सकते हैं कि किसी मामले में क्या हुआ और कब।",
    "splash.benefits.6.title": "मामलों को लोगों से जोड़ें",
    "splash.benefits.6.body":
      "किसी मामले का विषय-व्यक्ति उसके व्यक्ति-रिकॉर्ड से जोड़कर दर्ज करें, उसका विवरण दोबारा लिखे बिना।",
    "splash.features.1.title": "पूर्ण मामला रिकॉर्ड",
    "splash.features.1.body":
      "शीर्षक, एजेंसी, मामला संख्या, प्रकार, स्थिति, प्राथमिकता, विषय और पहचानकर्ता, एक ही फ़ॉर्म में।",
    "splash.features.2.title": "पूर्ण-पाठ खोज",
    "splash.features.2.body":
      "अनुमानित और ध्वन्यात्मक विकल्प वर्तनी की गलतियों और वैकल्पिक नामों के बावजूद मामले खोज लेते हैं।",
    "splash.features.3.title": "डुप्लिकेट जाँच",
    "splash.features.3.body":
      "किसी मामले की तुलना संग्रहीत मामलों से करें और सहेजने से पहले स्कोर वाले मिलान देखें।",
    "splash.features.4.title": "मामले मिलाएँ",
    "splash.features.4.body":
      "पुष्ट डुप्लिकेट को बचे हुए रिकॉर्ड में मिलाएँ और हाल के विलय देखें।",
    "splash.features.5.title": "स्थिति बोर्ड",
    "splash.features.5.body":
      "किसी मामले की स्थिति बदलने के लिए कार्ड को दूसरे कॉलम में खींचें।",
    "splash.features.6.title": "ऑडिट ट्रेल",
    "splash.features.6.body":
      "एक मामले का इतिहास या सभी मामलों की हाल की गतिविधि देखें, सबसे नई पहले।",
  },
  "zh-cn": {
    "nav.board": "看板",
    "brand.name": "Main X · 案件",
    "nav.toggle": "切换导航",
    "nav.cases": "案件",
    "nav.newCase": "新建案件",
    "chrome.language": "语言",
    "chrome.share": "分享",
    "chrome.textSize": "文字大小",
    "share.copyLink": "复制链接",
    "share.linkCopied": "链接已复制",
    "share.copyFailed": "无法复制 — 请从地址栏复制",
    "chrome.theme": "主题",
    "session.title": "会话",
    "session.tokenAttached": "已附加令牌。",
    "session.clearToken": "清除令牌",
    "session.noToken": "无令牌。",
    "session.signIn": "登录",
    "session.pasteToken": "粘贴令牌",
    "session.accessToken": "访问令牌",
    "session.pastePlaceholder": "粘贴访问令牌",
    "session.useToken": "使用令牌",
    "session.hint": "来自 authentication-service（魔法链接登录）。",
    "list.title": "案件",
    "list.new": "新建案件",
    "list.loading": "加载中…",
    "list.empty": "暂无案件。",
    "list.createOne": "创建一个",
    "list.loadFailed": "加载案件失败",
    "detail.loading": "加载中…",
    "detail.notFound": "未找到",
    "detail.caseType": "案件类型：",
    "detail.status": "状态：",
    "detail.priority": "优先级：",
    "detail.agency": "机构：",
    "detail.caseNumber": "案件编号：",
    "detail.opened": "开立：",
    "detail.subjects": "主题：",
    "detail.identifiers": "标识符：",
    "detail.keywords": "关键词：",
    "detail.id": "ID：",
    "detail.edit": "编辑",
    "detail.checkDuplicates": "检查重复",
    "detail.checking": "检查中…",
    "detail.checkFailed": "检查失败",
    "detail.delete": "删除",
    "detail.potentialDuplicates": "潜在重复",
    "detail.noneAboveThreshold": "没有超过匹配阈值的项。",
    "new.title": "新建案件",
    "new.create": "创建",
    "edit.title": "编辑案件",
    "edit.loading": "加载中…",
    "edit.notFound": "未找到",
    "edit.saveChanges": "保存更改",
    "form.title": "标题",
    "form.caseType": "案件类型",
    "form.status": "状态",
    "form.priority": "优先级",
    "form.caseNumber": "案件编号",
    "form.openedDate": "开立日期",
    "form.agencyId": "机构 ID",
    "form.agencyName": "机构名称",
    "form.alternateTitles": "备用标题",
    "form.subjects": "主题",
    "form.keywords": "关键词",
    "form.sameAs": "相同 URL",
    "form.languages": "语言",
    "form.commaSeparated": "（用逗号分隔）",
    "form.commaSeparatedIso": "（ISO 639-1，用逗号分隔）",
    "form.identifiers": "标识符",
    "form.valuePlaceholder": "值",
    "form.remove": "移除",
    "form.addIdentifier": "+ 添加标识符",
    "form.empty": "—",
    "form.save": "保存",
    "form.saving": "保存中…",
    "form.titleRequired": "标题为必填项。",
    "form.customLabel": "自定义标签",
    "form.customLabelRequired": "需要自定义标签。",
    "form.saveFailed": "保存失败",
    "nav.merge": "合并",
    "merge.title": "合并案件",
    "merge.mainId": "主案件 ID",
    "merge.mainIdHint": "保留的案件——其 ID 保持不变。",
    "merge.dupId": "重复案件 ID",
    "merge.dupIdHint": "并入主案件后被软删除。",
    "merge.reason": "原因",
    "merge.reasonHint": "可选；记入合并历史。",
    "merge.reasonPlaceholder": "已确认的同一案件重复项",
    "merge.loadPreview": "加载预览",
    "merge.merging": "合并中…",
    "merge.merge": "合并",
    "merge.bothIdsRequired": "两个案件 ID 均为必填。",
    "merge.mustDiffer": "主案件与重复案件的 ID 必须不同。",
    "merge.preview": "预览",
    "merge.main": "主案件",
    "merge.duplicate": "重复案件",
    "merge.completed": "合并完成",
    "merge.viewMain": "查看主案件",
    "merge.confirm": "将案件 {dup} 合并到案件 {main}？重复案件将被软删除。",
    "merge.recent": "最近的合并",
    "merge.recentEmpty": "尚无合并记录。",
    "merge.recentFailed": "加载最近的合并失败",
    "merge.mergedAt": "合并时间",
    "merge.actor": "操作者",
    // Cross-service links (subject_of)
    "links.title": "本案件涉及的当事人",
    "links.note":
      "记录本案件所涉及的当事人。该断言与案件本身同等敏感：读取和写入需要相同的授权，且每次变更都会记入审计日志。",
    "links.loading": "加载中…",
    "links.empty": "尚未记录当事人。",
    "links.loadFailed": "无法加载本案件的当事人",
    "links.person": "当事人",
    "links.personHint": "当事人引用，格式为 person:<uuid>。",
    "links.confidence": "置信度",
    "links.confidenceHint": "可选；0 至 1。若为明确断言可留空。",
    "links.provenance": "来源",
    "links.provenanceHint": "可选；默认为“operator”。",
    "links.validFrom": "生效日期",
    "links.validTo": "失效日期",
    "links.validity": "有效期",
    "links.addTitle": "记录当事人",
    "links.record": "记录当事人",
    "links.recording": "记录中…",
    "links.withdraw": "撤回",
    "links.withdrawing": "撤回中…",
    "links.withdrawConfirm":
      "确定撤回“{ref} 为本案件当事人”的断言吗？撤回将被记录并接受审计。",
    "links.recordFailed": "无法记录当事人",
    "links.withdrawFailed": "无法撤回该关联",
    "links.invalidPersonRef": "请输入格式为 person:<uuid> 的当事人引用。",
    "links.confidenceRange": "置信度必须介于 0 与 1 之间。",
    "search.placeholder": "搜索…",
    "search.submit": "搜索",
    "search.fuzzy": "模糊",
    "search.phonetic": "语音",
    "search.failed": "搜索失败",
    "detail.viewAudit": "查看审计记录",
    "nav.audit": "活动",
    "audit.title": "审计记录",
    "audit.backToCase": "返回案件",
    "audit.loading": "加载中…",
    "audit.noEntries": "暂无审计记录。",
    "audit.by": "操作者：",
    "audit.payload": "详情",
    "audit.loadFailed": "加载审计记录失败",
    "activity.title": "最近活动",
    "activity.recentAudit": "最近的审计记录",
    "activity.recentEvents": "最近的事件",
    "activity.loading": "加载中…",
    "activity.loadFailed": "加载最近活动失败",
    "activity.noAuditEntries": "暂无审计记录。",
    "activity.noEvents": "暂无事件。",
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
    "splash.hero.title": "每个案件，一份清晰记录",
    "splash.hero.subtitle":
      "案件只需登记一次，即时查找，在重复出现之前发现它们，每一次更改都记录在审计轨迹中。",
    "splash.benefits.1.title": "一案一记录",
    "splash.benefits.1.body":
      "把每个案件放在同一份共享记录中，而不是分散的表格和文件里。",
    "splash.benefits.2.title": "快速查找",
    "splash.benefits.2.body":
      "按标题、主题、机构或标识符搜索，即使有拼写错误或读音相近的名称也能找到。",
    "splash.benefits.3.title": "杜绝重复案件",
    "splash.benefits.3.body":
      "可能重复的案件会被标记以供审核，避免悄悄出现第二份记录。",
    "splash.benefits.4.title": "工作量一目了然",
    "splash.benefits.4.body":
      "看板按状态对所有案件分组，让你看到工作在哪里积压。",
    "splash.benefits.5.title": "责任清晰",
    "splash.benefits.5.body":
      "每一次更改都有记录，你随时可以查看案件发生了什么、何时发生。",
    "splash.benefits.6.title": "把案件关联到人员",
    "splash.benefits.6.body":
      "通过链接人员记录来标明案件涉及的人，无需复制其详细信息。",
    "splash.features.1.title": "完整的案件记录",
    "splash.features.1.body":
      "标题、机构、案号、类型、状态、优先级、主题和标识符，都在一个表单中。",
    "splash.features.2.title": "全文搜索",
    "splash.features.2.body":
      "模糊和语音选项可在拼写错误或别名情况下找到案件。",
    "splash.features.3.title": "重复检查",
    "splash.features.3.body":
      "将案件与已存案件比对，保存前查看带评分的匹配结果。",
    "splash.features.4.title": "合并案件",
    "splash.features.4.body":
      "把已确认的重复案件并入保留的记录，并查看最近的合并。",
    "splash.features.5.title": "状态看板",
    "splash.features.5.body": "把卡片拖到另一列即可更改案件状态。",
    "splash.features.6.title": "审计轨迹",
    "splash.features.6.body":
      "查看单个案件的历史，或所有案件的近期活动，最新的在前。",
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
