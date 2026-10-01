// Lightweight, dependency-free i18n for the auth SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is tiny and we keep the front-end dependency-light (drift
// across the family front-ends is accepted, see AGENTS.md).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// German (`de-de`), English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. Shared chrome terms reuse the family's established
// translations for consistency. An unknown key/locale falls back to
// `en-001`, then to the key string itself. The chosen locale persists to
// localStorage, drives the UI strings, `<html lang>`, and `<html dir>`
// (right-to-left for `ar-001`), and is sent as the `locale` field on
// signup / magic-link requests so the email language matches the UI (the
// service reduces it to its primary language subtag).

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
 * @returns `true` for `ar-001`, otherwise `false`.
 */
export function isRtl(locale: string): boolean {
  const primary = normaliseLocale(locale);

  return (
    primary !== null && (RTL_LOCALES as readonly string[]).includes(primary)
  );
}

// localStorage key under which the chosen UI locale is persisted.
const LOCALE_KEY = "mxi.auth.locale";

// Every translatable UI string, keyed by a stable dotted key. `en` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    brand: "Main X Auth",
    "nav.home": "الرئيسية",
    "nav.signin": "تسجيل الدخول",
    "nav.signup": "إنشاء حساب",
    "nav.locale": "اللغة",
    "nav.share": "مشاركة",
    "nav.text_size": "حجم النص",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذر النسخ — انسخه من شريط العنوان",
    "nav.theme": "السمة",
    "nav.toggle": "تبديل التنقل",
    "session.signedInAs": "تم تسجيل الدخول باسم",
    "account.title": "الحساب",
    "account.loading": "جارٍ التحميل…",
    "account.name": "الاسم:",
    "account.email": "البريد الإلكتروني:",
    "account.id": "المعرف:",
    "account.signout": "تسجيل الخروج",
    "account.notSignedIn": "لم تقم بتسجيل الدخول.",
    "account.signinPrompt.signin": "تسجيل الدخول",
    "account.signinPrompt.or": "أو",
    "account.signinPrompt.create": "إنشاء حساب",
    "account.loadFailed": "فشل تحميل الملف الشخصي",
    "account.rateLimited":
      "طلبات كثيرة جدًا. يرجى الانتظار بضع دقائق والمحاولة مرة أخرى.",
    "signin.title": "تسجيل الدخول",
    "signin.email": "البريد الإلكتروني",
    "signin.submit": "أرسل لي رابطًا سحريًا",
    "signin.submitting": "جارٍ الإرسال…",
    "signin.sent":
      "إذا كان لهذا البريد الإلكتروني حساب، فإن رابطًا سحريًا في طريقه إليك. أثناء التطوير، يُطبع الرابط في وحدة تحكم خدمة المصادقة — افتحه لتسجيل الدخول.",
    "signin.noAccount": "ليس لديك حساب بعد؟",
    "signin.create": "أنشئ واحدًا",
    "signin.failed": "فشل الطلب",
    "signin.sso": "تسجيل الدخول عبر SSO",
    "signup.title": "إنشاء حساب",
    "signup.email": "البريد الإلكتروني",
    "signup.name": "الاسم",
    "signup.nameOptional": "(اختياري)",
    "signup.submit": "إرسال رابط سحري",
    "signup.submitting": "جارٍ الإرسال…",
    "signup.sent":
      "إذا كان هذا البريد الإلكتروني صالحًا، فإن رابطًا سحريًا في طريقه إليك. أثناء التطوير، يُطبع الرابط في وحدة تحكم خدمة المصادقة — افتحه لإكمال تسجيل الدخول.",
    "signup.backToSignin": "العودة إلى تسجيل الدخول",
    "signup.haveAccount": "هل لديك حساب بالفعل؟",
    "signup.signin": "تسجيل الدخول",
    "signup.failed": "فشل إنشاء الحساب",
    "verify.working.title": "جارٍ تسجيل دخولك…",
    "verify.working.body": "جارٍ التحقق من رابطك السحري.",
    "verify.error.title": "تعذر تسجيل دخولك",
    "verify.error.missingToken": "هذا الرابط يفتقد إلى رمزه المميز.",
    "verify.error.invalid": "هذا الرابط غير صالح أو منتهي الصلاحية.",
    "verify.error.serviceUnavailable":
      "تعذر الوصول إلى خدمة تسجيل الدخول. يرجى المحاولة مرة أخرى بعد لحظات.",
    "verify.error.requestNew": "طلب رابط جديد",
    "brand.tagline": "تسجيل دخول موحّد لفهرس Main X",
    "splash.hero.title": "تسجيل دخول واحد للفهرس كله",
    "splash.hero.subtitle":
      "سجّل الدخول عبر رابط يصلك على بريدك الإلكتروني دون كلمة مرور تتذكرها، واستخدم جلسة آمنة واحدة في كل تطبيقات فهرس Main X.",
    "splash.benefits.1.title": "لا كلمات مرور لإدارتها",
    "splash.benefits.1.body":
      "لا شيء لتتذكره أو يتسرّب أو يُعاد ضبطه: رابط لمرة واحدة يكفي.",
    "splash.benefits.2.title": "حساب واحد في كل مكان",
    "splash.benefits.2.body":
      "الحساب نفسه يسجّل دخولك إلى كل تطبيقات فهرس Main X.",
    "splash.benefits.3.title": "تسجيل في ثوانٍ",
    "splash.benefits.3.body": "أنشئ حسابًا ببريدك الإلكتروني فقط.",
    "splash.benefits.4.title": "استخدم تسجيل دخول مؤسستك",
    "splash.benefits.4.body":
      "حيثما يكون مفعّلًا، سجّل الدخول عبر مزوّد الهوية الخاص بك.",
    "splash.benefits.5.title": "تسجيل الخروج فوري المفعول",
    "splash.benefits.5.body": "تسجيل الخروج يُلغي جلستك على الخادم على الفور.",
    "splash.benefits.6.title": "الخصوصية من التصميم",
    "splash.benefits.6.body":
      "طلب تسجيل الدخول لا يكشف أبدًا ما إذا كان للبريد حساب.",
    "splash.features.1.title": "الدخول برابط البريد",
    "splash.features.1.body": "اطلب رابطًا سحريًا لمرة واحدة لحسابك الحالي.",
    "splash.features.2.title": "إنشاء حساب",
    "splash.features.2.body": "سجّل ببريدك الإلكتروني واسم اختياري.",
    "splash.features.3.title": "تسجيل الدخول الموحّد",
    "splash.features.3.body": "تابع عبر مزوّد الهوية في مؤسستك عند تفعيله.",
    "splash.features.4.title": "عرض حسابك",
    "splash.features.4.body": "اطّلع على اسمك وبريدك ومعرّفك بعد تسجيل الدخول.",
    "splash.features.5.title": "إدارة سمات المستخدمين",
    "splash.features.5.body": "يعيّن المسؤولون السمات التي تحدد الصلاحيات.",
    "splash.features.6.title": "تسجيل خروج آمن",
    "splash.features.6.body":
      "أنهِ جلستك وامسح ملف تعريف الارتباط الخاص بها بخطوة واحدة.",
    "splash.trust.3.title": "تسجيلات دخول مدقّقة",
    "splash.trust.3.body": "تُسجَّل عمليات الدخول والخروج وتغييرات الصلاحيات.",
    "splash.trust.4.title": "جلسات محمية",
    "splash.trust.4.body":
      "تعيش جلستك في ملف تعريف ارتباط آمن لا تستطيع نصوص المتصفح قراءته.",
    "splash.trust.5.title": "معايير مفتوحة",
    "splash.trust.5.body":
      "رموز PASETO قصيرة العمر وتسجيل دخول موحّد عبر OpenID Connect.",
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
    "splash.trust.6.title": "يتحدث لغتك",
    "splash.trust.6.body":
      "العربية والصينية والألمانية والإنجليزية والفرنسية والهندية والإسبانية والويلزية.",
    "splash.cta.title": "هل أنت مستعد للبدء؟",
    "splash.cta.body":
      "سجّل الدخول برابط سحري يصلك على بريدك الإلكتروني. لا حاجة لكلمة مرور.",
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
      "جولة إرشادية في Main X Auth، تسجيل الدخول الموحّد لكل تطبيقات فهرس Main X: إنشاء حساب، والدخول برابط يصلك بالبريد، وإدارة جلستك.",
    "tour.s1.title": "إنشاء حساب",
    "tour.s1.summary": "سجّل ببريدك الإلكتروني فقط. لا تُنشأ أي كلمة مرور.",
    "tour.s1.step.1": "اختر إنشاء حساب من القائمة.",
    "tour.s1.step.2": "أدخل بريدك الإلكتروني، واسمك إن شئت (اختياري).",
    "tour.s1.step.3": "اضغط إرسال رابط سحري.",
    "tour.s1.step.4":
      "تخبرك الصفحة أن رابطًا في طريقه إليك إن كان العنوان صالحًا؛ افتحه من بريدك لإكمال تسجيل الدخول.",
    "tour.s2.title": "تسجيل الدخول برابط سحري",
    "tour.s2.summary": "اطلب رابطًا لمرة واحدة بدل كتابة كلمة مرور.",
    "tour.s2.step.1": "اختر تسجيل الدخول وأدخل بريدك الإلكتروني.",
    "tour.s2.step.2":
      "اضغط أرسل لي رابطًا سحريًا؛ الرد واحد سواء كان للعنوان حساب أم لا.",
    "tour.s2.step.3":
      "افتح الرابط في بريدك: يُتحقَّق منه على الخادم ويعمل مرة واحدة فقط.",
    "tour.s2.step.4":
      "تصل إلى الصفحة الرئيسية وقد سجّلت الدخول؛ وإن كان الرابط غير صالح أو منتهيًا فاختر طلب رابط جديد.",
    "tour.s3.title": "الدخول عبر مؤسستك",
    "tour.s3.summary":
      "حيثما يفعّل النشر ذلك، استخدم مزوّد الهوية الخاص بمؤسستك.",
    "tour.s3.step.1":
      "في صفحة تسجيل الدخول، ابحث عن تسجيل الدخول عبر SSO؛ يظهر فقط إذا فعّله النشر لديك.",
    "tour.s3.step.2": "اخترْه فينتقل متصفحك إلى مزوّد هوية مؤسستك.",
    "tour.s3.step.3":
      "سجّل الدخول هناك؛ تتحقق خدمة المصادقة من النتيجة وتنشئ جلستك.",
    "tour.s3.step.4":
      "تعود وقد سجّلت الدخول تمامًا كما بعد الرابط السحري؛ يُرفض بريد ليس له حساب إلا إذا سمح النشر بإنشاء الحسابات تلقائيًا.",
    "tour.s4.title": "عرض حسابك",
    "tour.s4.summary": "اعرف بأي حساب سجّلت الدخول، في كل تطبيقات فهرس Main X.",
    "tour.s4.step.1": "بعد تسجيل الدخول، تعرض الصفحة الرئيسية الحساب.",
    "tour.s4.step.2": "وتسرد الاسم والبريد الإلكتروني والمعرّف.",
    "tour.s4.step.3":
      "يعرض الشريط العلوي أيضًا شارة تسجيل الدخول لتعرف حالتك بنظرة.",
    "tour.s4.step.4":
      "تعيش جلستك في ملف تعريف ارتباط آمن لا تقرؤه نصوص الصفحة؛ وإن ظهرت رسالة تجاوز الحد فانتظر بضع دقائق وأعد المحاولة.",
    "tour.s5.title": "تسجيل الخروج",
    "tour.s5.summary": "أنهِ جلستك فورًا على الخادم.",
    "tour.s5.step.1": "اختر تسجيل الخروج في الشريط العلوي أو في صفحة الحساب.",
    "tour.s5.step.2": "تُلغي الخدمة جلستك فورًا، لا في هذا المتصفح فحسب.",
    "tour.s5.step.3": "يُمسح ملف تعريف ارتباط الجلسة.",
    "tour.s5.step.4":
      "تعرض الرئيسية صفحة الترحيب من جديد؛ وللعودة سجّل الدخول برابط سحري جديد.",
    "tour.s6.title": "إدارة سمات المستخدمين (للمسؤولين)",
    "tour.s6.summary": "يعيّن المسؤولون السمات التي تحدد ما يجوز لكل شخص فعله.",
    "tour.s6.step.1":
      "وأنت مسجّل الدخول كمسؤول، اختر Manage user attributes (admin) في صفحة الحساب.",
    "tour.s6.step.2":
      "الصق معرّف المستخدم (pid) واضغط Load لعرض سماته الحالية.",
    "tour.s6.step.3":
      'عدّل خريطة JSON، مثل {"access": ["write"]}، واضغط Save؛ أرسل {} لمسح كل شيء.',
    "tour.s6.step.4":
      "يتطلب ذلك جلسة مسؤول: إن رفضت الخدمة يظهر خطؤها، ويُوجَّه الزائر المجهول إلى تسجيل الدخول.",
  },
  "cy-001": {
    brand: "Main X Auth",
    "nav.home": "Hafan",
    "nav.signin": "Mewngofnodi",
    "nav.signup": "Cofrestru",
    "nav.locale": "Iaith",
    "nav.share": "Rhannu",
    "nav.text_size": "Maint testun",
    "share.copy_link": "Copïo dolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o'r bar cyfeiriad",
    "nav.theme": "Thema",
    "nav.toggle": "Toglo'r llywio",
    "session.signedInAs": "Wedi mewngofnodi fel",
    "account.title": "Cyfrif",
    "account.loading": "Yn llwytho…",
    "account.name": "Enw:",
    "account.email": "E-bost:",
    "account.id": "ID:",
    "account.signout": "Allgofnodi",
    "account.notSignedIn": "Nid ydych wedi mewngofnodi.",
    "account.signinPrompt.signin": "Mewngofnodi",
    "account.signinPrompt.or": "neu",
    "account.signinPrompt.create": "creu cyfrif",
    "account.loadFailed": "Methwyd â llwytho'r proffil",
    "account.rateLimited":
      "Gormod o geisiadau. Arhoswch ychydig funudau a rhowch gynnig arall arni.",
    "signin.title": "Mewngofnodi",
    "signin.email": "E-bost",
    "signin.submit": "E-bostiwch ddolen hud ataf",
    "signin.submitting": "Yn anfon…",
    "signin.sent":
      "Os oes cyfrif gan yr e-bost hwnnw, mae dolen hud ar ei ffordd. Wrth ddatblygu, argreffir y ddolen i gonsol y gwasanaeth dilysu — agorwch hi i fewngofnodi.",
    "signin.noAccount": "Dim cyfrif eto?",
    "signin.create": "Crëwch un",
    "signin.failed": "Methodd y cais",
    "signin.sso": "Mewngofnodi gydag SSO",
    "signup.title": "Creu cyfrif",
    "signup.email": "E-bost",
    "signup.name": "Enw",
    "signup.nameOptional": "(dewisol)",
    "signup.submit": "Anfon dolen hud",
    "signup.submitting": "Yn anfon…",
    "signup.sent":
      "Os yw'r e-bost hwnnw'n ddilys, mae dolen hud ar ei ffordd. Wrth ddatblygu, argreffir y ddolen i gonsol y gwasanaeth dilysu — agorwch hi i orffen mewngofnodi.",
    "signup.backToSignin": "Yn ôl i fewngofnodi",
    "signup.haveAccount": "Mae gennych gyfrif eisoes?",
    "signup.signin": "Mewngofnodi",
    "signup.failed": "Methodd y cofrestru",
    "verify.working.title": "Yn eich mewngofnodi…",
    "verify.working.body": "Yn gwirio eich dolen hud.",
    "verify.error.title": "Methwyd â'ch mewngofnodi",
    "verify.error.missingToken": "Mae'r ddolen hon yn colli ei thocyn.",
    "verify.error.invalid": "Mae'r ddolen hon yn annilys neu wedi dod i ben.",
    "verify.error.serviceUnavailable":
      "Ni allem gyrraedd y gwasanaeth mewngofnodi. Rhowch gynnig eto mewn munud.",
    "verify.error.requestNew": "Gofyn am ddolen newydd",
    "brand.tagline": "Mewngofnodi unigol ar gyfer Mynegai Main X",
    "splash.hero.title": "Un mewngofnodi ar gyfer y mynegai cyfan",
    "splash.hero.subtitle":
      "Mewngofnodwch gyda dolen a anfonir at eich e-bost, heb gyfrinair i'w gofio, a defnyddiwch un sesiwn ddiogel ar draws pob ap Mynegai Main X.",
    "splash.benefits.1.title": "Dim cyfrineiriau i'w rheoli",
    "splash.benefits.1.body":
      "Dim byd i'w gofio, i'w ollwng na'i ailosod: mae dolen untro yn gwneud y gwaith.",
    "splash.benefits.2.title": "Un cyfrif ym mhobman",
    "splash.benefits.2.body":
      "Mae'r un cyfrif yn eich mewngofnodi i bob ap Mynegai Main X.",
    "splash.benefits.3.title": "Cofrestru mewn eiliadau",
    "splash.benefits.3.body": "Crëwch gyfrif gyda'ch cyfeiriad e-bost yn unig.",
    "splash.benefits.4.title": "Defnyddiwch fewngofnodi eich sefydliad",
    "splash.benefits.4.body":
      "Lle bo wedi'i alluogi, mewngofnodwch drwy eich darparwr hunaniaeth eich hun.",
    "splash.benefits.5.title": "Mae allgofnodi yn effeithio ar unwaith",
    "splash.benefits.5.body":
      "Mae allgofnodi yn diddymu eich sesiwn ar y gweinydd ar unwaith.",
    "splash.benefits.6.title": "Preifatrwydd wrth ddylunio",
    "splash.benefits.6.body":
      "Nid yw cais mewngofnodi byth yn datgelu a oes cyfrif gan e-bost.",
    "splash.features.1.title": "Mewngofnodi drwy ddolen e-bost",
    "splash.features.1.body":
      "Gofynnwch am ddolen hud untro ar gyfer eich cyfrif presennol.",
    "splash.features.2.title": "Creu cyfrif",
    "splash.features.2.body": "Cofrestrwch gyda'ch e-bost ac enw dewisol.",
    "splash.features.3.title": "Mewngofnodi unigol",
    "splash.features.3.body":
      "Parhewch gyda darparwr hunaniaeth eich sefydliad pan fydd wedi'i alluogi.",
    "splash.features.4.title": "Gweld eich cyfrif",
    "splash.features.4.body":
      "Gwelwch eich enw, e-bost a'ch ID unwaith y byddwch wedi mewngofnodi.",
    "splash.features.5.title": "Rheoli priodoleddau defnyddwyr",
    "splash.features.5.body":
      "Mae gweinyddwyr yn neilltuo'r priodoleddau sy'n llywio caniatâd.",
    "splash.features.6.title": "Allgofnodi'n ddiogel",
    "splash.features.6.body":
      "Diweddwch eich sesiwn a chlirio ei chwci mewn un cam.",
    "splash.trust.3.title": "Mewngofnodi wedi'i archwilio",
    "splash.trust.3.body":
      "Cofnodir mewngofnodi, allgofnodi a newidiadau i ganiatâd.",
    "splash.trust.4.title": "Sesiynau diogel",
    "splash.trust.4.body":
      "Mae eich sesiwn mewn cwci diogel na all sgriptiau yn y porwr ei ddarllen.",
    "splash.trust.5.title": "Safonau agored",
    "splash.trust.5.body":
      "Tocynnau PASETO byrhoedlog a mewngofnodi unigol OpenID Connect.",
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
    "splash.trust.6.title": "Yn siarad eich iaith",
    "splash.trust.6.body":
      "Arabeg, Tsieinëeg, Almaeneg, Saesneg, Ffrangeg, Hindi, Sbaeneg a Chymraeg.",
    "splash.cta.title": "Barod i ddechrau?",
    "splash.cta.body":
      "Mewngofnodwch gyda dolen hud a anfonir i'ch e-bost. Dim angen cyfrinair.",
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
      "Taith dywys drwy Main X Auth, y mewngofnodi unigol ar gyfer pob ap Mynegai Main X: creu cyfrif, mewngofnodi gyda dolen a anfonir trwy e-bost, a rheoli eich sesiwn.",
    "tour.s1.title": "Creu cyfrif",
    "tour.s1.summary":
      "Cofrestrwch gyda chyfeiriad e-bost yn unig. Ni chaiff cyfrinair byth ei greu.",
    "tour.s1.step.1": "Dewiswch Cofrestru yn y ddewislen.",
    "tour.s1.step.2":
      "Rhowch eich cyfeiriad e-bost, a'ch enw os dymunwch (dewisol).",
    "tour.s1.step.3": "Pwyswch Anfon dolen hud.",
    "tour.s1.step.4":
      "Mae'r dudalen yn dweud bod dolen ar ei ffordd os yw'r cyfeiriad yn ddilys; agorwch hi o'ch e-bost i orffen mewngofnodi.",
    "tour.s2.title": "Mewngofnodi gyda dolen hud",
    "tour.s2.summary": "Gofynnwch am ddolen untro yn lle teipio cyfrinair.",
    "tour.s2.step.1": "Dewiswch Mewngofnodi a rhowch eich cyfeiriad e-bost.",
    "tour.s2.step.2":
      "Pwyswch E-bostiwch ddolen hud ataf; mae'r ateb yr un fath p'un a oes cyfrif gan y cyfeiriad ai peidio.",
    "tour.s2.step.3":
      "Agorwch y ddolen yn eich e-bost: caiff ei gwirio ar y gweinydd a dim ond unwaith y mae'n gweithio.",
    "tour.s2.step.4":
      "Cewch eich hun ar y hafan wedi mewngofnodi; os yw'r ddolen yn annilys neu wedi dod i ben, dewiswch Gofyn am ddolen newydd.",
    "tour.s3.title": "Mewngofnodi gyda'ch sefydliad",
    "tour.s3.summary":
      "Lle mae defnyddiad yn ei alluogi, defnyddiwch ddarparwr hunaniaeth eich sefydliad.",
    "tour.s3.step.1":
      "Ar y dudalen Mewngofnodi, chwiliwch am Mewngofnodi gydag SSO; dim ond os yw eich defnyddiad wedi'i droi ymlaen y mae'n ymddangos.",
    "tour.s3.step.2":
      "Dewiswch ef ac aiff eich porwr i ddarparwr hunaniaeth eich sefydliad.",
    "tour.s3.step.3":
      "Mewngofnodwch yno; mae'r gwasanaeth dilysu yn gwirio'r canlyniad ac yn sefydlu eich sesiwn.",
    "tour.s3.step.4":
      "Dychwelwch wedi mewngofnodi, yn union fel ar ôl dolen hud; gwrthodir e-bost heb gyfrif oni bai bod y defnyddiad yn caniatáu creu cyfrifon yn awtomatig.",
    "tour.s4.title": "Gweld eich cyfrif",
    "tour.s4.summary":
      "Gweld pwy sydd wedi mewngofnodi, ym mhob ap Mynegai Main X.",
    "tour.s4.step.1": "Ar ôl mewngofnodi, mae'r hafan yn dangos Cyfrif.",
    "tour.s4.step.2": "Mae'n rhestru eich Enw, E-bost ac ID.",
    "tour.s4.step.3":
      "Mae'r pennyn hefyd yn dangos bathodyn mewngofnodi, fel y gwelwch ar unwaith.",
    "tour.s4.step.4":
      "Mae eich sesiwn mewn cwci diogel na all sgriptiau'r dudalen ei ddarllen; os gwelwch neges cyfyngu cyfradd, arhoswch ychydig funudau a rhowch gynnig arall arni.",
    "tour.s5.title": "Allgofnodi",
    "tour.s5.summary": "Terfynwch eich sesiwn ar unwaith, ar y gweinydd.",
    "tour.s5.step.1":
      "Dewiswch Allgofnodi yn y pennyn, neu ar y dudalen Cyfrif.",
    "tour.s5.step.2":
      "Mae'r gwasanaeth yn dirymu eich sesiwn ar unwaith, nid yn y porwr hwn yn unig.",
    "tour.s5.step.3": "Caiff y cwci sesiwn ei glirio.",
    "tour.s5.step.4":
      "Mae'r hafan yn dangos y dudalen groeso eto; i ddod yn ôl, mewngofnodwch gyda dolen hud newydd.",
    "tour.s6.title": "Rheoli priodoleddau defnyddwyr (gweinyddwr)",
    "tour.s6.summary":
      "Mae gweinyddwyr yn neilltuo'r priodoleddau sy'n penderfynu beth gaiff pob person ei wneud.",
    "tour.s6.step.1":
      "Wedi mewngofnodi fel gweinyddwr, dewiswch Manage user attributes (admin) ar y dudalen Cyfrif.",
    "tour.s6.step.2":
      "Gludwch ID defnyddiwr (pid) a phwyso Llwytho i weld ei briodoleddau presennol.",
    "tour.s6.step.3":
      'Golygwch y map JSON, e.e. {"access": ["write"]}, a phwyso Cadw; anfonwch {} i glirio popeth.',
    "tour.s6.step.4":
      "Mae hyn angen sesiwn gweinyddwr: os yw'r gwasanaeth yn gwrthod, dangosir ei wall, ac anfonir ymwelwyr dienw i Mewngofnodi.",
  },
  "de-de": {
    brand: "Main X Auth",
    "nav.home": "Startseite",
    "nav.signin": "Anmelden",
    "nav.signup": "Registrieren",
    "nav.locale": "Sprache",
    "nav.share": "Teilen",
    "nav.text_size": "Textgröße",
    "share.copy_link": "Link kopieren",
    "share.copied": "Link kopiert",
    "share.copy_failed":
      "Kopieren fehlgeschlagen — bitte aus der Adressleiste kopieren",
    "nav.theme": "Thema",
    "nav.toggle": "Navigation umschalten",
    "session.signedInAs": "Angemeldet als",
    "account.title": "Konto",
    "account.loading": "Wird geladen…",
    "account.name": "Name:",
    "account.email": "E-Mail:",
    "account.id": "ID:",
    "account.signout": "Abmelden",
    "account.notSignedIn": "Sie sind nicht angemeldet.",
    "account.signinPrompt.signin": "Anmelden",
    "account.signinPrompt.or": "oder",
    "account.signinPrompt.create": "ein Konto erstellen",
    "account.loadFailed": "Profil konnte nicht geladen werden",
    "account.rateLimited":
      "Zu viele Anfragen. Bitte warten Sie einige Minuten und versuchen Sie es erneut.",
    "signin.title": "Anmelden",
    "signin.email": "E-Mail",
    "signin.submit": "Schick mir einen Magic Link",
    "signin.submitting": "Wird gesendet…",
    "signin.sent":
      "Wenn diese E-Mail ein Konto hat, ist ein Magic Link unterwegs. In der Entwicklung wird der Link in der Konsole des Authentifizierungsdienstes ausgegeben — öffnen Sie ihn zum Anmelden.",
    "signin.noAccount": "Noch kein Konto?",
    "signin.create": "Eines erstellen",
    "signin.failed": "Anfrage fehlgeschlagen",
    "signin.sso": "Mit SSO anmelden",
    "signup.title": "Konto erstellen",
    "signup.email": "E-Mail",
    "signup.name": "Name",
    "signup.nameOptional": "(optional)",
    "signup.submit": "Magic Link senden",
    "signup.submitting": "Wird gesendet…",
    "signup.sent":
      "Wenn diese E-Mail gültig ist, ist ein Magic Link unterwegs. In der Entwicklung wird der Link in der Konsole des Authentifizierungsdienstes ausgegeben — öffnen Sie ihn, um die Anmeldung abzuschließen.",
    "signup.backToSignin": "Zurück zur Anmeldung",
    "signup.haveAccount": "Haben Sie bereits ein Konto?",
    "signup.signin": "Anmelden",
    "signup.failed": "Registrierung fehlgeschlagen",
    "verify.working.title": "Sie werden angemeldet…",
    "verify.working.body": "Ihr Magic Link wird überprüft.",
    "verify.error.title": "Anmeldung nicht möglich",
    "verify.error.missingToken": "Diesem Link fehlt sein Token.",
    "verify.error.invalid": "Dieser Link ist ungültig oder abgelaufen.",
    "verify.error.serviceUnavailable":
      "Der Anmeldedienst konnte nicht erreicht werden. Bitte versuchen Sie es in Kürze erneut.",
    "verify.error.requestNew": "Neuen Link anfordern",
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
    "splash.trust.3.title": "Protokollierte Anmeldungen",
    "splash.trust.3.body":
      "Anmeldungen, Abmeldungen und Berechtigungsänderungen werden festgehalten.",
    "splash.trust.4.title": "Sicher aufbewahrte Sitzungen",
    "splash.trust.4.body":
      "Ihre Sitzung liegt in einem sicheren Cookie, das Skripte im Browser nicht lesen können.",
    "splash.trust.5.title": "Offene Standards",
    "splash.trust.5.body":
      "Kurzlebige PASETO-Token und OpenID-Connect-Single-Sign-on.",
    "splash.trust.6.title": "Spricht Ihre Sprache",
    "splash.trust.6.body":
      "Arabisch, Chinesisch, Deutsch, Englisch, Französisch, Hindi, Spanisch und Walisisch.",
    "splash.cta.title": "Bereit für den Einstieg?",
    "splash.cta.body":
      "Melden Sie sich mit einem Magic-Link an, der an Ihre E-Mail-Adresse gesendet wird. Kein Passwort nötig.",
    "splash.benefits.1.title": "Keine Passwörter zu verwalten",
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
    "brand.tagline": "Single Sign-on für den Main X Index",
    "splash.hero.title": "Eine Anmeldung für den gesamten Index",
    "splash.hero.subtitle":
      "Melden Sie sich mit einem an Ihre E-Mail-Adresse gesendeten Link an, ohne ein Passwort zu merken, und nutzen Sie eine sichere Sitzung in allen Apps des Main X Index.",
    "splash.benefits.1.body":
      "Nichts, was Sie sich merken, was verloren gehen oder zurückgesetzt werden müsste: Ein Einmal-Link erledigt es.",
    "splash.benefits.2.title": "Ein Konto für alles",
    "splash.benefits.2.body":
      "Dasselbe Konto meldet Sie in jeder App des Main X Index an.",
    "splash.benefits.3.title": "In Sekunden registriert",
    "splash.benefits.3.body":
      "Legen Sie ein Konto allein mit Ihrer E-Mail-Adresse an.",
    "splash.benefits.4.title": "Das Login Ihrer Organisation nutzen",
    "splash.benefits.4.body":
      "Wo aktiviert, melden Sie sich über Ihren eigenen Identitätsanbieter an.",
    "splash.benefits.5.title": "Abmelden wirkt sofort",
    "splash.benefits.5.body":
      "Beim Abmelden wird Ihre Sitzung sofort auf dem Server widerrufen.",
    "splash.benefits.6.title": "Datenschutz von Grund auf",
    "splash.benefits.6.body":
      "Eine Anmeldeanfrage verrät nie, ob zu einer E-Mail-Adresse ein Konto existiert.",
    "splash.features.1.title": "Anmelden per E-Mail-Link",
    "splash.features.1.body":
      "Fordern Sie für Ihr bestehendes Konto einen Einmal-Magic-Link an.",
    "splash.features.2.title": "Ein Konto erstellen",
    "splash.features.2.body":
      "Registrieren Sie sich mit Ihrer E-Mail-Adresse und einem optionalen Namen.",
    "splash.features.3.title": "Single Sign-on",
    "splash.features.3.body":
      "Fahren Sie mit dem Identitätsanbieter Ihrer Organisation fort, sofern aktiviert.",
    "splash.features.4.title": "Ihr Konto ansehen",
    "splash.features.4.body":
      "Sehen Sie nach der Anmeldung Ihren Namen, Ihre E-Mail-Adresse und Ihre ID.",
    "splash.features.5.title": "Benutzerattribute verwalten",
    "splash.features.5.body":
      "Administratoren weisen die Attribute zu, die die Berechtigungen steuern.",
    "splash.features.6.title": "Sicher abmelden",
    "splash.features.6.body":
      "Beenden Sie Ihre Sitzung und löschen Sie ihr Cookie in einem Schritt.",
    "tour.intro":
      "Ein geführter Rundgang durch Main X Auth, das Single Sign-on für jede App des Main X Index: ein Konto erstellen, sich mit einem per E-Mail gesendeten Link anmelden und Ihre Sitzung verwalten.",
    "tour.s1.title": "Ein Konto erstellen",
    "tour.s1.summary":
      "Registrieren Sie sich allein mit einer E-Mail-Adresse. Es wird nie ein Passwort angelegt.",
    "tour.s1.step.1": "Wählen Sie im Menü „Registrieren“.",
    "tour.s1.step.2":
      "Geben Sie Ihre E-Mail-Adresse und, wenn Sie möchten, Ihren Namen ein (optional).",
    "tour.s1.step.3": "Drücken Sie „Magic Link senden“.",
    "tour.s1.step.4":
      "Die Seite meldet, dass ein Link unterwegs ist, wenn die Adresse gültig ist; öffnen Sie ihn in Ihrer E-Mail, um die Anmeldung abzuschließen.",
    "tour.s2.title": "Mit einem Magic Link anmelden",
    "tour.s2.summary":
      "Fordern Sie einen Einmal-Link an, statt ein Passwort zu tippen.",
    "tour.s2.step.1":
      "Wählen Sie „Anmelden“ und geben Sie Ihre E-Mail-Adresse ein.",
    "tour.s2.step.2":
      "Drücken Sie „Schick mir einen Magic Link“; die Antwort ist dieselbe, ob die Adresse ein Konto hat oder nicht.",
    "tour.s2.step.3":
      "Öffnen Sie den Link in Ihrer E-Mail: Er wird auf dem Server geprüft und funktioniert nur einmal.",
    "tour.s2.step.4":
      "Sie landen angemeldet auf der Startseite; ist der Link ungültig oder abgelaufen, wählen Sie „Neuen Link anfordern“.",
    "tour.s3.title": "Mit Ihrer Organisation anmelden",
    "tour.s3.summary":
      "Wo eine Bereitstellung es aktiviert, nutzen Sie den Identitätsanbieter Ihrer eigenen Organisation.",
    "tour.s3.step.1":
      "Suchen Sie auf der Seite „Anmelden“ nach „Mit SSO anmelden“; es erscheint nur, wenn Ihre Bereitstellung es aktiviert hat.",
    "tour.s3.step.2":
      "Wählen Sie es, und Ihr Browser wechselt zum Identitätsanbieter Ihrer Organisation.",
    "tour.s3.step.3":
      "Melden Sie sich dort an; der Authentifizierungsdienst prüft das Ergebnis und richtet Ihre Sitzung ein.",
    "tour.s3.step.4":
      "Sie kehren angemeldet zurück, genau wie nach einem Magic Link; eine E-Mail ohne bestehendes Konto wird abgelehnt, sofern die Bereitstellung nicht die automatische Kontoerstellung erlaubt.",
    "tour.s4.title": "Ihr Konto ansehen",
    "tour.s4.summary":
      "Sehen Sie, als wer Sie angemeldet sind, in jeder App des Main X Index.",
    "tour.s4.step.1": "Nach der Anmeldung zeigt die Startseite „Konto“.",
    "tour.s4.step.2":
      "Es listet Ihren Namen, Ihre E-Mail-Adresse und Ihre ID auf.",
    "tour.s4.step.3":
      "Die Kopfzeile zeigt zusätzlich ein Abzeichen „Angemeldet“, sodass Sie es auf einen Blick erkennen.",
    "tour.s4.step.4":
      "Ihre Sitzung liegt in einem sicheren Cookie, das Seitenskripte nicht lesen können; erscheint eine Meldung zur Anfragebegrenzung, warten Sie ein paar Minuten und versuchen Sie es erneut.",
    "tour.s5.title": "Abmelden",
    "tour.s5.summary": "Beenden Sie Ihre Sitzung sofort, auf dem Server.",
    "tour.s5.step.1":
      "Wählen Sie in der Kopfzeile oder auf der Kontoseite „Abmelden“.",
    "tour.s5.step.2":
      "Der Dienst widerruft Ihre Sitzung sofort, nicht nur in diesem Browser.",
    "tour.s5.step.3": "Das Sitzungs-Cookie wird gelöscht.",
    "tour.s5.step.4":
      "Die Startseite zeigt wieder die Willkommensseite; um zurückzukehren, melden Sie sich mit einem neuen Magic Link an.",
    "tour.s6.title": "Benutzerattribute verwalten (Admin)",
    "tour.s6.summary":
      "Administratoren weisen die Attribute zu, die bestimmen, was jede Person tun darf.",
    "tour.s6.step.1":
      "Wählen Sie als Administrator auf der Kontoseite „Manage user attributes (admin)“ (diese Ansicht ist nur auf Englisch).",
    "tour.s6.step.2":
      "Fügen Sie die ID (pid) einer Person ein und drücken Sie „Load“, um ihre aktuellen Attribute zu sehen.",
    "tour.s6.step.3":
      'Bearbeiten Sie die JSON-Zuordnung, zum Beispiel {"access": ["write"]}, und drücken Sie „Save“; senden Sie {}, um alles zu löschen.',
    "tour.s6.step.4":
      "Dafür ist eine Admin-Sitzung nötig: Verweigert der Dienst den Zugriff, wird sein Fehler angezeigt, und anonyme Besucher werden zur Anmeldung geleitet.",
  },
  "en-001": {
    brand: "Main X Auth",
    "nav.home": "Home",
    "nav.signin": "Sign in",
    "nav.signup": "Sign up",
    "nav.locale": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    "nav.theme": "Theme",
    "nav.toggle": "Toggle navigation",
    "session.signedInAs": "Signed in as",
    // Home / account
    "account.title": "Account",
    "account.loading": "Loading…",
    "account.name": "Name:",
    "account.email": "Email:",
    "account.id": "ID:",
    "account.signout": "Sign out",
    "account.notSignedIn": "You are not signed in.",
    "account.signinPrompt.signin": "Sign in",
    "account.signinPrompt.or": "or",
    "account.signinPrompt.create": "create an account",
    "account.loadFailed": "Failed to load profile",
    "account.rateLimited":
      "Too many requests. Please wait a few minutes and try again.",
    // Sign in
    "signin.title": "Sign in",
    "signin.email": "Email",
    "signin.submit": "Email me a magic link",
    "signin.submitting": "Sending…",
    "signin.sent":
      "If that email has an account, a magic link is on its way. In development the link is printed to the auth service console — open it to sign in.",
    "signin.noAccount": "No account yet?",
    "signin.create": "Create one",
    "signin.failed": "Request failed",
    "signin.sso": "Sign in with SSO",
    // Sign up
    "signup.title": "Create account",
    "signup.email": "Email",
    "signup.name": "Name",
    "signup.nameOptional": "(optional)",
    "signup.submit": "Send magic link",
    "signup.submitting": "Sending…",
    "signup.sent":
      "If that email is valid, a magic link is on its way. In development the link is printed to the auth service console — open it to finish signing in.",
    "signup.backToSignin": "Back to sign in",
    "signup.haveAccount": "Already have an account?",
    "signup.signin": "Sign in",
    "signup.failed": "Sign up failed",
    // Verify
    "verify.working.title": "Signing you in…",
    "verify.working.body": "Verifying your magic link.",
    "verify.error.title": "Could not sign you in",
    "verify.error.missingToken": "This link is missing its token.",
    "verify.error.invalid": "This link is invalid or expired.",
    "verify.error.serviceUnavailable":
      "We could not reach the sign-in service. Please try again in a moment.",
    "verify.error.requestNew": "Request a new link",
    "brand.tagline": "Single sign-on for the Main X Index",
    "splash.hero.title": "One sign-in for the whole index",
    "splash.hero.subtitle":
      "Sign in with a link sent to your email, with no password to remember, and carry one secure session across every Main X Index app.",
    "splash.benefits.1.title": "No passwords to manage",
    "splash.benefits.1.body":
      "Nothing to remember, leak or reset: a one-time link does the job.",
    "splash.benefits.2.title": "One account everywhere",
    "splash.benefits.2.body":
      "The same account signs you in to every Main X Index app.",
    "splash.benefits.3.title": "Sign up in seconds",
    "splash.benefits.3.body": "Create an account with just your email address.",
    "splash.benefits.4.title": "Use your organisation's login",
    "splash.benefits.4.body":
      "Where enabled, sign in through your own identity provider.",
    "splash.benefits.5.title": "Sign out takes effect at once",
    "splash.benefits.5.body":
      "Signing out revokes your session on the server straight away.",
    "splash.benefits.6.title": "Private by design",
    "splash.benefits.6.body":
      "A sign-in request never reveals whether an email has an account.",
    "splash.features.1.title": "Sign in by email link",
    "splash.features.1.body":
      "Request a one-time magic link for your existing account.",
    "splash.features.2.title": "Create an account",
    "splash.features.2.body": "Sign up with your email and an optional name.",
    "splash.features.3.title": "Single sign-on",
    "splash.features.3.body":
      "Continue with your organisation's identity provider when it is enabled.",
    "splash.features.4.title": "View your account",
    "splash.features.4.body":
      "See your name, email and ID once you are signed in.",
    "splash.features.5.title": "Manage user attributes",
    "splash.features.5.body":
      "Administrators assign the attributes that drive permissions.",
    "splash.features.6.title": "Sign out securely",
    "splash.features.6.body":
      "End your session and clear its cookie in one step.",
    "splash.trust.3.title": "Audited sign-ins",
    "splash.trust.3.body":
      "Sign-ins, sign-outs and permission changes are recorded.",
    "splash.trust.4.title": "Sessions kept safe",
    "splash.trust.4.body":
      "Your session lives in a secure cookie that scripts in the browser cannot read.",
    "splash.trust.5.title": "Open standards",
    "splash.trust.5.body":
      "Short-lived PASETO tokens and OpenID Connect single sign-on.",
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
    "splash.trust.6.title": "Speaks your language",
    "splash.trust.6.body":
      "Arabic, Chinese, English, French, German, Hindi, Spanish and Welsh.",
    "splash.cta.title": "Ready to get started?",
    "splash.cta.body":
      "Sign in with a magic link sent to your email. No password needed.",
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
      "A guided walkthrough of Main X Auth, the single sign-on for every Main X Index app: creating an account, signing in with an emailed link, and managing your session.",
    "tour.s1.title": "Create an account",
    "tour.s1.summary":
      "Register with just an email address. No password is ever created.",
    "tour.s1.step.1": "Choose Sign up in the menu.",
    "tour.s1.step.2":
      "Enter your email address, and your name if you like (it is optional).",
    "tour.s1.step.3": "Press Send magic link.",
    "tour.s1.step.4":
      "The page says a link is on its way if the address is valid; open it from your email to finish signing in.",
    "tour.s2.title": "Sign in with a magic link",
    "tour.s2.summary": "Ask for a one-time link instead of typing a password.",
    "tour.s2.step.1": "Choose Sign in and enter your email address.",
    "tour.s2.step.2":
      "Press Email me a magic link; the reply is the same whether or not the address has an account.",
    "tour.s2.step.3":
      "Open the link in your email: it is checked on the server and works only once.",
    "tour.s2.step.4":
      "You land on the home page signed in; if the link is invalid or expired, choose Request a new link.",
    "tour.s3.title": "Sign in with your organization",
    "tour.s3.summary":
      "Where a deployment enables it, use your organization's own identity provider.",
    "tour.s3.step.1":
      "On the Sign in page, look for Sign in with SSO; it appears only when your deployment has turned it on.",
    "tour.s3.step.2":
      "Choose it and your browser goes to your organization's identity provider.",
    "tour.s3.step.3":
      "Sign in there; the authentication service checks the result and sets up your session.",
    "tour.s3.step.4":
      "You return signed in, exactly as after a magic link; an email with no existing account is refused unless the deployment allows automatic account creation.",
    "tour.s4.title": "View your account",
    "tour.s4.summary":
      "See who you are signed in as, on every Main X Index app.",
    "tour.s4.step.1": "After signing in, the home page shows Account.",
    "tour.s4.step.2": "It lists your Name, Email and ID.",
    "tour.s4.step.3":
      "The header also shows a signed-in badge, so you can tell at a glance.",
    "tour.s4.step.4":
      "Your session lives in a secure cookie that page scripts cannot read; if you see a rate-limit message, wait a few minutes and try again.",
    "tour.s5.title": "Sign out",
    "tour.s5.summary": "End your session immediately, on the server.",
    "tour.s5.step.1": "Choose Sign out in the header, or on the Account page.",
    "tour.s5.step.2":
      "The service revokes your session at once, not just in this browser.",
    "tour.s5.step.3": "The session cookie is cleared.",
    "tour.s5.step.4":
      "Home shows the welcome page again; to come back, sign in with a new magic link.",
    "tour.s6.title": "Manage user attributes (admin)",
    "tour.s6.summary":
      "Administrators assign the attributes that decide what each person may do.",
    "tour.s6.step.1":
      "Signed in as an administrator, choose Manage user attributes (admin) on the Account page.",
    "tour.s6.step.2":
      "Paste a user's ID (pid) and press Load to see their current attributes.",
    "tour.s6.step.3":
      'Edit the JSON map, for example {"access": ["write"]}, and press Save; send {} to clear everything.',
    "tour.s6.step.4":
      "This needs an admin session: if the service refuses, its error is shown, and anonymous visitors are sent to Sign in.",
  },
  "es-001": {
    brand: "Main X Auth",
    "nav.home": "Inicio",
    "nav.signin": "Iniciar sesión",
    "nav.signup": "Registrarse",
    "nav.locale": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar — cópielo desde la barra de direcciones",
    "nav.theme": "Tema",
    "nav.toggle": "Alternar navegación",
    "session.signedInAs": "Sesión iniciada como",
    "account.title": "Cuenta",
    "account.loading": "Cargando…",
    "account.name": "Nombre:",
    "account.email": "Correo electrónico:",
    "account.id": "ID:",
    "account.signout": "Cerrar sesión",
    "account.notSignedIn": "No has iniciado sesión.",
    "account.signinPrompt.signin": "Iniciar sesión",
    "account.signinPrompt.or": "o",
    "account.signinPrompt.create": "crear una cuenta",
    "account.loadFailed": "No se pudo cargar el perfil",
    "account.rateLimited":
      "Demasiadas solicitudes. Espere unos minutos e inténtelo de nuevo.",
    "signin.title": "Iniciar sesión",
    "signin.email": "Correo electrónico",
    "signin.submit": "Envíame un enlace mágico",
    "signin.submitting": "Enviando…",
    "signin.sent":
      "Si ese correo tiene una cuenta, un enlace mágico está en camino. En desarrollo, el enlace se imprime en la consola del servicio de autenticación — ábrelo para iniciar sesión.",
    "signin.noAccount": "¿Aún no tienes cuenta?",
    "signin.create": "Crea una",
    "signin.failed": "La solicitud falló",
    "signin.sso": "Iniciar sesión con SSO",
    "signup.title": "Crear cuenta",
    "signup.email": "Correo electrónico",
    "signup.name": "Nombre",
    "signup.nameOptional": "(opcional)",
    "signup.submit": "Enviar enlace mágico",
    "signup.submitting": "Enviando…",
    "signup.sent":
      "Si ese correo es válido, un enlace mágico está en camino. En desarrollo, el enlace se imprime en la consola del servicio de autenticación — ábrelo para terminar de iniciar sesión.",
    "signup.backToSignin": "Volver a iniciar sesión",
    "signup.haveAccount": "¿Ya tienes una cuenta?",
    "signup.signin": "Iniciar sesión",
    "signup.failed": "El registro falló",
    "verify.working.title": "Iniciando tu sesión…",
    "verify.working.body": "Verificando tu enlace mágico.",
    "verify.error.title": "No se pudo iniciar tu sesión",
    "verify.error.missingToken": "A este enlace le falta su token.",
    "verify.error.invalid": "Este enlace no es válido o ha caducado.",
    "verify.error.serviceUnavailable":
      "No pudimos comunicarnos con el servicio de inicio de sesión. Inténtalo de nuevo en un momento.",
    "verify.error.requestNew": "Solicitar un nuevo enlace",
    "brand.tagline": "Inicio de sesión único para el índice Main X",
    "splash.hero.title": "Un solo inicio de sesión para todo el índice",
    "splash.hero.subtitle":
      "Inicia sesión con un enlace enviado a tu correo, sin contraseña que recordar, y usa una sola sesión segura en todas las aplicaciones del índice Main X.",
    "splash.benefits.1.title": "Sin contraseñas que gestionar",
    "splash.benefits.1.body":
      "Nada que recordar, filtrar ni restablecer: basta un enlace de un solo uso.",
    "splash.benefits.2.title": "Una cuenta para todo",
    "splash.benefits.2.body":
      "La misma cuenta te da acceso a todas las aplicaciones del índice Main X.",
    "splash.benefits.3.title": "Regístrate en segundos",
    "splash.benefits.3.body":
      "Crea una cuenta solo con tu dirección de correo.",
    "splash.benefits.4.title": "Usa el acceso de tu organización",
    "splash.benefits.4.body":
      "Cuando esté activado, inicia sesión con tu propio proveedor de identidad.",
    "splash.benefits.5.title": "Cerrar sesión surte efecto al instante",
    "splash.benefits.5.body":
      "Al cerrar sesión, tu sesión se revoca de inmediato en el servidor.",
    "splash.benefits.6.title": "Privado por diseño",
    "splash.benefits.6.body":
      "Una solicitud de inicio de sesión nunca revela si un correo tiene cuenta.",
    "splash.features.1.title": "Acceso por enlace de correo",
    "splash.features.1.body":
      "Solicita un enlace mágico de un solo uso para tu cuenta existente.",
    "splash.features.2.title": "Crear una cuenta",
    "splash.features.2.body": "Regístrate con tu correo y un nombre opcional.",
    "splash.features.3.title": "Inicio de sesión único",
    "splash.features.3.body":
      "Continúa con el proveedor de identidad de tu organización cuando esté activado.",
    "splash.features.4.title": "Ver tu cuenta",
    "splash.features.4.body":
      "Consulta tu nombre, correo e ID una vez que hayas iniciado sesión.",
    "splash.features.5.title": "Gestionar atributos de usuario",
    "splash.features.5.body":
      "Los administradores asignan los atributos que determinan los permisos.",
    "splash.features.6.title": "Cerrar sesión con seguridad",
    "splash.features.6.body":
      "Termina tu sesión y borra su cookie en un solo paso.",
    "splash.trust.3.title": "Accesos auditados",
    "splash.trust.3.body":
      "Se registran los inicios y cierres de sesión y los cambios de permisos.",
    "splash.trust.4.title": "Sesiones protegidas",
    "splash.trust.4.body":
      "Tu sesión vive en una cookie segura que los scripts del navegador no pueden leer.",
    "splash.trust.5.title": "Estándares abiertos",
    "splash.trust.5.body":
      "Tokens PASETO de corta duración e inicio de sesión único con OpenID Connect.",
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
    "splash.trust.6.title": "Habla tu idioma",
    "splash.trust.6.body":
      "Alemán, árabe, chino, español, francés, galés, hindi e inglés.",
    "splash.cta.title": "¿Listo para empezar?",
    "splash.cta.body":
      "Inicia sesión con un enlace mágico enviado a tu correo. No necesitas contraseña.",
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
      "Un recorrido guiado por Main X Auth, el inicio de sesión único de todas las aplicaciones del índice Main X: crear una cuenta, iniciar sesión con un enlace por correo y gestionar tu sesión.",
    "tour.s1.title": "Crear una cuenta",
    "tour.s1.summary":
      "Regístrate solo con un correo electrónico. Nunca se crea una contraseña.",
    "tour.s1.step.1": "Elige Registrarse en el menú.",
    "tour.s1.step.2":
      "Escribe tu correo electrónico y, si quieres, tu nombre (es opcional).",
    "tour.s1.step.3": "Pulsa Enviar enlace mágico.",
    "tour.s1.step.4":
      "La página indica que un enlace va de camino si la dirección es válida; ábrelo desde tu correo para terminar de iniciar sesión.",
    "tour.s2.title": "Iniciar sesión con un enlace mágico",
    "tour.s2.summary":
      "Pide un enlace de un solo uso en lugar de escribir una contraseña.",
    "tour.s2.step.1": "Elige Iniciar sesión y escribe tu correo electrónico.",
    "tour.s2.step.2":
      "Pulsa Envíame un enlace mágico; la respuesta es la misma tenga o no cuenta esa dirección.",
    "tour.s2.step.3":
      "Abre el enlace en tu correo: se comprueba en el servidor y funciona una sola vez.",
    "tour.s2.step.4":
      "Llegas a la página de inicio con la sesión iniciada; si el enlace no es válido o caducó, elige Solicitar un nuevo enlace.",
    "tour.s3.title": "Iniciar sesión con tu organización",
    "tour.s3.summary":
      "Donde el despliegue lo active, usa el proveedor de identidad de tu organización.",
    "tour.s3.step.1":
      "En la página Iniciar sesión, busca Iniciar sesión con SSO; solo aparece si tu despliegue lo ha activado.",
    "tour.s3.step.2":
      "Elígelo y tu navegador va al proveedor de identidad de tu organización.",
    "tour.s3.step.3":
      "Inicia sesión allí; el servicio de autenticación comprueba el resultado y crea tu sesión.",
    "tour.s3.step.4":
      "Vuelves con la sesión iniciada, igual que tras un enlace mágico; un correo sin cuenta se rechaza salvo que el despliegue permita crear cuentas automáticamente.",
    "tour.s4.title": "Ver tu cuenta",
    "tour.s4.summary":
      "Consulta con qué cuenta has iniciado sesión, en cualquier aplicación del índice Main X.",
    "tour.s4.step.1":
      "Tras iniciar sesión, la página de inicio muestra Cuenta.",
    "tour.s4.step.2": "Enumera tu Nombre, Correo electrónico e ID.",
    "tour.s4.step.3":
      "El encabezado también muestra una insignia de sesión iniciada, para saberlo de un vistazo.",
    "tour.s4.step.4":
      "Tu sesión vive en una cookie segura que los scripts de la página no pueden leer; si ves un aviso de límite de peticiones, espera unos minutos e inténtalo de nuevo.",
    "tour.s5.title": "Cerrar sesión",
    "tour.s5.summary": "Termina tu sesión al instante, en el servidor.",
    "tour.s5.step.1":
      "Elige Cerrar sesión en el encabezado o en la página Cuenta.",
    "tour.s5.step.2":
      "El servicio revoca tu sesión de inmediato, no solo en este navegador.",
    "tour.s5.step.3": "Se borra la cookie de sesión.",
    "tour.s5.step.4":
      "El inicio vuelve a mostrar la página de bienvenida; para volver, inicia sesión con un nuevo enlace mágico.",
    "tour.s6.title": "Gestionar atributos de usuarios (admin)",
    "tour.s6.summary":
      "Los administradores asignan los atributos que deciden lo que puede hacer cada persona.",
    "tour.s6.step.1":
      "Con la sesión de administrador iniciada, elige Manage user attributes (admin) en la página Cuenta.",
    "tour.s6.step.2":
      "Pega el ID del usuario (pid) y pulsa Load para ver sus atributos actuales.",
    "tour.s6.step.3":
      'Edita el mapa JSON, por ejemplo {"access": ["write"]}, y pulsa Save; envía {} para borrarlo todo.',
    "tour.s6.step.4":
      "Requiere una sesión de administrador: si el servicio lo rechaza, se muestra su error, y los visitantes anónimos van a Iniciar sesión.",
  },
  "fr-001": {
    brand: "Main X Auth",
    "nav.home": "Accueil",
    "nav.signin": "Se connecter",
    "nav.signup": "S'inscrire",
    "nav.locale": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Impossible de copier — copiez-le depuis la barre d'adresse",
    "nav.theme": "Thème",
    "nav.toggle": "Basculer la navigation",
    "session.signedInAs": "Connecté en tant que",
    "account.title": "Compte",
    "account.loading": "Chargement…",
    "account.name": "Nom :",
    "account.email": "E-mail :",
    "account.id": "ID :",
    "account.signout": "Se déconnecter",
    "account.notSignedIn": "Vous n'êtes pas connecté.",
    "account.signinPrompt.signin": "Se connecter",
    "account.signinPrompt.or": "ou",
    "account.signinPrompt.create": "créer un compte",
    "account.loadFailed": "Échec du chargement du profil",
    "account.rateLimited":
      "Trop de requêtes. Veuillez patienter quelques minutes et réessayer.",
    "signin.title": "Se connecter",
    "signin.email": "E-mail",
    "signin.submit": "Envoyez-moi un lien magique",
    "signin.submitting": "Envoi…",
    "signin.sent":
      "Si cet e-mail correspond à un compte, un lien magique est en route. En développement, le lien est imprimé dans la console du service d'authentification — ouvrez-le pour vous connecter.",
    "signin.noAccount": "Pas encore de compte ?",
    "signin.create": "Créez-en un",
    "signin.failed": "La requête a échoué",
    "signin.sso": "Se connecter avec SSO",
    "signup.title": "Créer un compte",
    "signup.email": "E-mail",
    "signup.name": "Nom",
    "signup.nameOptional": "(facultatif)",
    "signup.submit": "Envoyer le lien magique",
    "signup.submitting": "Envoi…",
    "signup.sent":
      "Si cet e-mail est valide, un lien magique est en route. En développement, le lien est imprimé dans la console du service d'authentification — ouvrez-le pour terminer la connexion.",
    "signup.backToSignin": "Retour à la connexion",
    "signup.haveAccount": "Vous avez déjà un compte ?",
    "signup.signin": "Se connecter",
    "signup.failed": "L'inscription a échoué",
    "verify.working.title": "Connexion en cours…",
    "verify.working.body": "Vérification de votre lien magique.",
    "verify.error.title": "Impossible de vous connecter",
    "verify.error.missingToken": "Ce lien n'a pas de jeton.",
    "verify.error.invalid": "Ce lien est invalide ou expiré.",
    "verify.error.serviceUnavailable":
      "Nous n'avons pas pu joindre le service de connexion. Veuillez réessayer dans un instant.",
    "verify.error.requestNew": "Demander un nouveau lien",
    "brand.tagline": "Authentification unique pour l'index Main X",
    "splash.hero.title": "Une seule connexion pour tout l'index",
    "splash.hero.subtitle":
      "Connectez-vous avec un lien envoyé par e-mail, sans mot de passe à retenir, et gardez une seule session sécurisée dans toutes les applications de l'index Main X.",
    "splash.benefits.1.title": "Aucun mot de passe à gérer",
    "splash.benefits.1.body":
      "Rien à retenir, à divulguer ou à réinitialiser : un lien à usage unique suffit.",
    "splash.benefits.2.title": "Un compte partout",
    "splash.benefits.2.body":
      "Le même compte vous connecte à toutes les applications de l'index Main X.",
    "splash.benefits.3.title": "Inscription en quelques secondes",
    "splash.benefits.3.body":
      "Créez un compte avec votre seule adresse e-mail.",
    "splash.benefits.4.title": "Utilisez la connexion de votre organisation",
    "splash.benefits.4.body":
      "Lorsqu'il est activé, connectez-vous via votre propre fournisseur d'identité.",
    "splash.benefits.5.title": "La déconnexion est immédiate",
    "splash.benefits.5.body":
      "Se déconnecter révoque aussitôt votre session côté serveur.",
    "splash.benefits.6.title": "Confidentialité dès la conception",
    "splash.benefits.6.body":
      "Une demande de connexion ne révèle jamais si une adresse e-mail a un compte.",
    "splash.features.1.title": "Connexion par lien e-mail",
    "splash.features.1.body":
      "Demandez un lien magique à usage unique pour votre compte existant.",
    "splash.features.2.title": "Créer un compte",
    "splash.features.2.body":
      "Inscrivez-vous avec votre e-mail et un nom facultatif.",
    "splash.features.3.title": "Authentification unique",
    "splash.features.3.body":
      "Poursuivez avec le fournisseur d'identité de votre organisation lorsqu'il est activé.",
    "splash.features.4.title": "Consulter votre compte",
    "splash.features.4.body":
      "Voyez votre nom, votre e-mail et votre identifiant une fois connecté.",
    "splash.features.5.title": "Gérer les attributs des utilisateurs",
    "splash.features.5.body":
      "Les administrateurs attribuent les attributs qui pilotent les autorisations.",
    "splash.features.6.title": "Se déconnecter en toute sécurité",
    "splash.features.6.body":
      "Mettez fin à votre session et effacez son cookie en une seule étape.",
    "splash.trust.3.title": "Connexions auditées",
    "splash.trust.3.body":
      "Les connexions, déconnexions et changements d'autorisations sont enregistrés.",
    "splash.trust.4.title": "Sessions protégées",
    "splash.trust.4.body":
      "Votre session est stockée dans un cookie sécurisé que les scripts du navigateur ne peuvent pas lire.",
    "splash.trust.5.title": "Standards ouverts",
    "splash.trust.5.body":
      "Jetons PASETO de courte durée et authentification unique OpenID Connect.",
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
    "splash.trust.6.title": "Parle votre langue",
    "splash.trust.6.body":
      "Allemand, anglais, arabe, chinois, espagnol, français, gallois et hindi.",
    "splash.cta.title": "Prêt à commencer ?",
    "splash.cta.body":
      "Connectez-vous avec un lien magique envoyé par e-mail. Aucun mot de passe requis.",
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
      "Une visite guidée de Main X Auth, la connexion unique de toutes les applications de l'index Main X : créer un compte, se connecter avec un lien reçu par e-mail et gérer sa session.",
    "tour.s1.title": "Créer un compte",
    "tour.s1.summary":
      "Inscrivez-vous avec une simple adresse e-mail. Aucun mot de passe n'est jamais créé.",
    "tour.s1.step.1": "Choisissez S'inscrire dans le menu.",
    "tour.s1.step.2":
      "Saisissez votre adresse e-mail et, si vous le souhaitez, votre nom (facultatif).",
    "tour.s1.step.3": "Appuyez sur Envoyer le lien magique.",
    "tour.s1.step.4":
      "La page indique qu'un lien est en route si l'adresse est valide ; ouvrez-le depuis votre messagerie pour terminer la connexion.",
    "tour.s2.title": "Se connecter avec un lien magique",
    "tour.s2.summary":
      "Demandez un lien à usage unique plutôt que de saisir un mot de passe.",
    "tour.s2.step.1":
      "Choisissez Se connecter et saisissez votre adresse e-mail.",
    "tour.s2.step.2":
      "Appuyez sur Envoyez-moi un lien magique ; la réponse est la même que l'adresse ait un compte ou non.",
    "tour.s2.step.3":
      "Ouvrez le lien dans votre e-mail : il est vérifié côté serveur et ne fonctionne qu'une fois.",
    "tour.s2.step.4":
      "Vous arrivez à l'accueil, connecté ; si le lien est invalide ou expiré, choisissez Demander un nouveau lien.",
    "tour.s3.title": "Se connecter avec son organisation",
    "tour.s3.summary":
      "Là où le déploiement l'active, utilisez le fournisseur d'identité de votre organisation.",
    "tour.s3.step.1":
      "Sur la page Se connecter, cherchez Se connecter avec SSO ; il n'apparaît que si votre déploiement l'a activé.",
    "tour.s3.step.2":
      "Choisissez-le et votre navigateur se rend chez le fournisseur d'identité de votre organisation.",
    "tour.s3.step.3":
      "Connectez-vous là-bas ; le service d'authentification vérifie le résultat et établit votre session.",
    "tour.s3.step.4":
      "Vous revenez connecté, exactement comme après un lien magique ; une adresse sans compte est refusée sauf si le déploiement autorise la création automatique de comptes.",
    "tour.s4.title": "Consulter son compte",
    "tour.s4.summary":
      "Voyez sous quel compte vous êtes connecté, dans toute application de l'index Main X.",
    "tour.s4.step.1": "Une fois connecté, l'accueil affiche Compte.",
    "tour.s4.step.2": "Il indique votre Nom, votre E-mail et votre ID.",
    "tour.s4.step.3":
      "L'en-tête affiche aussi un badge de connexion, pour le savoir d'un coup d'œil.",
    "tour.s4.step.4":
      "Votre session vit dans un cookie sécurisé que les scripts de la page ne peuvent pas lire ; si un message de limite de requêtes apparaît, attendez quelques minutes et réessayez.",
    "tour.s5.title": "Se déconnecter",
    "tour.s5.summary":
      "Mettez fin à votre session immédiatement, côté serveur.",
    "tour.s5.step.1":
      "Choisissez Se déconnecter dans l'en-tête ou sur la page Compte.",
    "tour.s5.step.2":
      "Le service révoque votre session aussitôt, pas seulement dans ce navigateur.",
    "tour.s5.step.3": "Le cookie de session est effacé.",
    "tour.s5.step.4":
      "L'accueil affiche de nouveau la page de bienvenue ; pour revenir, connectez-vous avec un nouveau lien magique.",
    "tour.s6.title": "Gérer les attributs des utilisateurs (admin)",
    "tour.s6.summary":
      "Les administrateurs attribuent les attributs qui déterminent ce que chacun peut faire.",
    "tour.s6.step.1":
      "Connecté en administrateur, choisissez Manage user attributes (admin) sur la page Compte.",
    "tour.s6.step.2":
      "Collez l'ID de l'utilisateur (pid) et appuyez sur Load pour voir ses attributs actuels.",
    "tour.s6.step.3":
      'Modifiez la table JSON, par exemple {"access": ["write"]}, et appuyez sur Save ; envoyez {} pour tout effacer.',
    "tour.s6.step.4":
      "Cela exige une session administrateur : si le service refuse, son erreur s'affiche, et les visiteurs anonymes sont envoyés vers Se connecter.",
  },
  "hi-001": {
    brand: "Main X Auth",
    "nav.home": "होम",
    "nav.signin": "साइन इन करें",
    "nav.signup": "साइन अप करें",
    "nav.locale": "भाषा",
    "nav.share": "साझा करें",
    "nav.text_size": "टेक्स्ट का आकार",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "nav.theme": "थीम",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "session.signedInAs": "इस रूप में साइन इन",
    "account.title": "खाता",
    "account.loading": "लोड हो रहा है…",
    "account.name": "नाम:",
    "account.email": "ईमेल:",
    "account.id": "ID:",
    "account.signout": "साइन आउट करें",
    "account.notSignedIn": "आप साइन इन नहीं हैं।",
    "account.signinPrompt.signin": "साइन इन करें",
    "account.signinPrompt.or": "या",
    "account.signinPrompt.create": "एक खाता बनाएँ",
    "account.loadFailed": "प्रोफ़ाइल लोड करने में विफल",
    "account.rateLimited":
      "बहुत अधिक अनुरोध। कृपया कुछ मिनट प्रतीक्षा करें और फिर से प्रयास करें।",
    "signin.title": "साइन इन करें",
    "signin.email": "ईमेल",
    "signin.submit": "मुझे एक मैजिक लिंक ईमेल करें",
    "signin.submitting": "भेजा जा रहा है…",
    "signin.sent":
      "यदि उस ईमेल का कोई खाता है, तो एक मैजिक लिंक रास्ते में है। डेवलपमेंट में लिंक प्रमाणीकरण सेवा कंसोल पर प्रिंट होता है — साइन इन करने के लिए उसे खोलें।",
    "signin.noAccount": "अभी तक कोई खाता नहीं?",
    "signin.create": "एक बनाएँ",
    "signin.failed": "अनुरोध विफल रहा",
    "signin.sso": "SSO से साइन इन करें",
    "signup.title": "खाता बनाएँ",
    "signup.email": "ईमेल",
    "signup.name": "नाम",
    "signup.nameOptional": "(वैकल्पिक)",
    "signup.submit": "मैजिक लिंक भेजें",
    "signup.submitting": "भेजा जा रहा है…",
    "signup.sent":
      "यदि वह ईमेल मान्य है, तो एक मैजिक लिंक रास्ते में है। डेवलपमेंट में लिंक प्रमाणीकरण सेवा कंसोल पर प्रिंट होता है — साइन इन पूरा करने के लिए उसे खोलें।",
    "signup.backToSignin": "साइन इन पर वापस जाएँ",
    "signup.haveAccount": "पहले से एक खाता है?",
    "signup.signin": "साइन इन करें",
    "signup.failed": "साइन अप विफल रहा",
    "verify.working.title": "आपको साइन इन किया जा रहा है…",
    "verify.working.body": "आपके मैजिक लिंक की पुष्टि की जा रही है।",
    "verify.error.title": "आपको साइन इन नहीं किया जा सका",
    "verify.error.missingToken": "इस लिंक में इसका टोकन गायब है।",
    "verify.error.invalid": "यह लिंक अमान्य या समाप्त हो चुका है।",
    "verify.error.serviceUnavailable":
      "हम साइन-इन सेवा तक नहीं पहुँच सके। कृपया कुछ देर बाद पुनः प्रयास करें।",
    "verify.error.requestNew": "एक नया लिंक अनुरोध करें",
    "brand.tagline": "Main X इंडेक्स के लिए एकल साइन-इन",
    "splash.hero.title": "पूरे इंडेक्स के लिए एक साइन-इन",
    "splash.hero.subtitle":
      "अपने ईमेल पर भेजे गए लिंक से साइन इन करें, कोई पासवर्ड याद रखने की ज़रूरत नहीं, और Main X इंडेक्स के हर ऐप में एक ही सुरक्षित सत्र इस्तेमाल करें।",
    "splash.benefits.1.title": "कोई पासवर्ड नहीं सँभालना",
    "splash.benefits.1.body":
      "न याद रखना, न लीक होना, न रीसेट करना: एक बार वाला लिंक ही काफ़ी है।",
    "splash.benefits.2.title": "हर जगह एक खाता",
    "splash.benefits.2.body":
      "एक ही खाता Main X इंडेक्स के हर ऐप में साइन इन कराता है।",
    "splash.benefits.3.title": "कुछ सेकंड में साइन अप",
    "splash.benefits.3.body": "सिर्फ़ अपने ईमेल पते से खाता बनाएँ।",
    "splash.benefits.4.title": "अपने संगठन के लॉगिन का उपयोग करें",
    "splash.benefits.4.body":
      "सक्षम होने पर, अपने पहचान प्रदाता के ज़रिए साइन इन करें।",
    "splash.benefits.5.title": "साइन आउट तुरंत लागू",
    "splash.benefits.5.body":
      "साइन आउट करते ही सर्वर पर आपका सत्र तुरंत रद्द हो जाता है।",
    "splash.benefits.6.title": "डिज़ाइन से ही निजी",
    "splash.benefits.6.body":
      "साइन-इन अनुरोध कभी नहीं बताता कि किसी ईमेल का खाता है या नहीं।",
    "splash.features.1.title": "ईमेल लिंक से साइन इन",
    "splash.features.1.body":
      "अपने मौजूदा खाते के लिए एक बार वाला मैजिक लिंक मँगवाएँ।",
    "splash.features.2.title": "खाता बनाएँ",
    "splash.features.2.body": "अपने ईमेल और वैकल्पिक नाम से साइन अप करें।",
    "splash.features.3.title": "एकल साइन-ऑन",
    "splash.features.3.body":
      "सक्षम होने पर अपने संगठन के पहचान प्रदाता के साथ आगे बढ़ें।",
    "splash.features.4.title": "अपना खाता देखें",
    "splash.features.4.body":
      "साइन इन करने के बाद अपना नाम, ईमेल और आईडी देखें।",
    "splash.features.5.title": "उपयोगकर्ता विशेषताएँ प्रबंधित करें",
    "splash.features.5.body":
      "व्यवस्थापक वे विशेषताएँ तय करते हैं जिनसे अनुमतियाँ बनती हैं।",
    "splash.features.6.title": "सुरक्षित रूप से साइन आउट",
    "splash.features.6.body":
      "एक ही चरण में अपना सत्र समाप्त करें और उसकी कुकी हटाएँ।",
    "splash.trust.3.title": "ऑडिट किए गए साइन-इन",
    "splash.trust.3.body":
      "साइन-इन, साइन-आउट और अनुमतियों में बदलाव दर्ज किए जाते हैं।",
    "splash.trust.4.title": "सुरक्षित सत्र",
    "splash.trust.4.body":
      "आपका सत्र एक सुरक्षित कुकी में रहता है जिसे ब्राउज़र की स्क्रिप्ट नहीं पढ़ सकतीं।",
    "splash.trust.5.title": "खुले मानक",
    "splash.trust.5.body":
      "कम अवधि वाले PASETO टोकन और OpenID Connect एकल साइन-ऑन।",
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
    "splash.trust.6.title": "आपकी भाषा में",
    "splash.trust.6.body":
      "अरबी, चीनी, जर्मन, अंग्रेज़ी, फ़्रेंच, हिन्दी, स्पेनिश और वेल्श।",
    "splash.cta.title": "शुरू करने के लिए तैयार हैं?",
    "splash.cta.body":
      "अपने ईमेल पर भेजे गए मैजिक लिंक से साइन इन करें। पासवर्ड की ज़रूरत नहीं।",
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
      "Main X Auth का निर्देशित परिचय, जो Main X Index के हर ऐप का एकल साइन-ऑन है: खाता बनाना, ईमेल से मिले लिंक से साइन इन करना और अपना सत्र प्रबंधित करना।",
    "tour.s1.title": "खाता बनाना",
    "tour.s1.summary":
      "सिर्फ़ ईमेल पते से पंजीकरण करें। कोई पासवर्ड कभी नहीं बनता।",
    "tour.s1.step.1": "मेनू में साइन अप करें चुनें।",
    "tour.s1.step.2":
      "अपना ईमेल पता दर्ज करें, और चाहें तो अपना नाम भी (वैकल्पिक)।",
    "tour.s1.step.3": "मैजिक लिंक भेजें दबाएँ।",
    "tour.s1.step.4":
      "पता मान्य हो तो पेज बताता है कि लिंक आ रहा है; साइन इन पूरा करने के लिए उसे अपने ईमेल से खोलें।",
    "tour.s2.title": "मैजिक लिंक से साइन इन करना",
    "tour.s2.summary":
      "पासवर्ड टाइप करने के बजाय एक बार चलने वाला लिंक माँगें।",
    "tour.s2.step.1": "साइन इन चुनें और अपना ईमेल पता दर्ज करें।",
    "tour.s2.step.2":
      "मुझे एक मैजिक लिंक ईमेल करें दबाएँ; पते का खाता हो या न हो, जवाब एक जैसा रहता है।",
    "tour.s2.step.3":
      "अपने ईमेल में लिंक खोलें: उसकी जाँच सर्वर पर होती है और वह सिर्फ़ एक बार चलता है।",
    "tour.s2.step.4":
      "आप साइन इन होकर होम पेज पर पहुँचते हैं; लिंक अमान्य या समाप्त हो तो एक नया लिंक अनुरोध करें चुनें।",
    "tour.s3.title": "अपने संगठन से साइन इन करना",
    "tour.s3.summary":
      "जहाँ परिनियोजन में सक्षम हो, वहाँ अपने संगठन के पहचान प्रदाता का उपयोग करें।",
    "tour.s3.step.1":
      "साइन इन पेज पर SSO से साइन इन करें खोजें; यह तभी दिखता है जब आपके परिनियोजन ने इसे चालू किया हो।",
    "tour.s3.step.2":
      "इसे चुनें और आपका ब्राउज़र आपके संगठन के पहचान प्रदाता पर जाता है।",
    "tour.s3.step.3":
      "वहाँ साइन इन करें; प्रमाणीकरण सेवा नतीजा जाँचकर आपका सत्र बनाती है।",
    "tour.s3.step.4":
      "आप वैसे ही साइन इन होकर लौटते हैं जैसे मैजिक लिंक के बाद; जिस ईमेल का खाता नहीं है उसे अस्वीकार किया जाता है, जब तक परिनियोजन खाते अपने-आप बनाने की अनुमति न दे।",
    "tour.s4.title": "अपना खाता देखना",
    "tour.s4.summary":
      "देखें कि Main X Index के किसी भी ऐप में आप किस खाते से साइन इन हैं।",
    "tour.s4.step.1": "साइन इन के बाद होम पेज पर खाता दिखता है।",
    "tour.s4.step.2": "उसमें आपका नाम, ईमेल और ID सूचीबद्ध होते हैं।",
    "tour.s4.step.3":
      "हेडर में साइन-इन बैज भी दिखता है, ताकि एक नज़र में पता चल जाए।",
    "tour.s4.step.4":
      "आपका सत्र एक सुरक्षित कुकी में रहता है जिसे पेज की स्क्रिप्ट नहीं पढ़ सकतीं; दर की सीमा का संदेश दिखे तो कुछ मिनट रुककर फिर कोशिश करें।",
    "tour.s5.title": "साइन आउट करना",
    "tour.s5.summary": "सर्वर पर अपना सत्र तुरंत समाप्त करें।",
    "tour.s5.step.1": "हेडर में या खाता पेज पर साइन आउट करें चुनें।",
    "tour.s5.step.2":
      "सेवा आपका सत्र तुरंत रद्द कर देती है, सिर्फ़ इस ब्राउज़र में नहीं।",
    "tour.s5.step.3": "सत्र की कुकी हटा दी जाती है।",
    "tour.s5.step.4":
      "होम फिर स्वागत पेज दिखाता है; वापस आने के लिए नए मैजिक लिंक से साइन इन करें।",
    "tour.s6.title": "उपयोगकर्ता विशेषताएँ प्रबंधित करना (व्यवस्थापक)",
    "tour.s6.summary":
      "व्यवस्थापक वे विशेषताएँ तय करते हैं जो बताती हैं कि हर व्यक्ति क्या कर सकता है।",
    "tour.s6.step.1":
      "व्यवस्थापक के रूप में साइन इन होकर खाता पेज पर Manage user attributes (admin) चुनें।",
    "tour.s6.step.2":
      "उपयोगकर्ता की ID (pid) चिपकाएँ और उसकी मौजूदा विशेषताएँ देखने के लिए Load दबाएँ।",
    "tour.s6.step.3":
      'JSON मैप संपादित करें, जैसे {"access": ["write"]}, और Save दबाएँ; सब कुछ साफ़ करने के लिए {} भेजें।',
    "tour.s6.step.4":
      "इसके लिए व्यवस्थापक सत्र चाहिए: सेवा मना करे तो उसकी त्रुटि दिखती है, और अनाम आगंतुक साइन इन पर भेजे जाते हैं।",
  },
  "zh-cn": {
    brand: "Main X Auth",
    "nav.home": "首页",
    "nav.signin": "登录",
    "nav.signup": "注册",
    "nav.locale": "语言",
    "nav.share": "分享",
    "nav.text_size": "文字大小",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制 — 请从地址栏复制",
    "nav.theme": "主题",
    "nav.toggle": "切换导航",
    "session.signedInAs": "已登录为",
    "account.title": "账户",
    "account.loading": "加载中…",
    "account.name": "姓名：",
    "account.email": "电子邮箱：",
    "account.id": "ID：",
    "account.signout": "退出登录",
    "account.notSignedIn": "您尚未登录。",
    "account.signinPrompt.signin": "登录",
    "account.signinPrompt.or": "或",
    "account.signinPrompt.create": "创建账户",
    "account.loadFailed": "加载个人资料失败",
    "account.rateLimited": "请求过多。请稍等几分钟后重试。",
    "signin.title": "登录",
    "signin.email": "电子邮箱",
    "signin.submit": "给我发送一个魔法链接",
    "signin.submitting": "发送中…",
    "signin.sent":
      "如果该邮箱已有账户，魔法链接正在发送途中。在开发环境中，链接会打印到认证服务控制台——打开它即可登录。",
    "signin.noAccount": "还没有账户？",
    "signin.create": "创建一个",
    "signin.failed": "请求失败",
    "signin.sso": "使用 SSO 登录",
    "signup.title": "创建账户",
    "signup.email": "电子邮箱",
    "signup.name": "姓名",
    "signup.nameOptional": "（可选）",
    "signup.submit": "发送魔法链接",
    "signup.submitting": "发送中…",
    "signup.sent":
      "如果该邮箱有效，魔法链接正在发送途中。在开发环境中，链接会打印到认证服务控制台——打开它即可完成登录。",
    "signup.backToSignin": "返回登录",
    "signup.haveAccount": "已经有账户了？",
    "signup.signin": "登录",
    "signup.failed": "注册失败",
    "verify.working.title": "正在为您登录…",
    "verify.working.body": "正在验证您的魔法链接。",
    "verify.error.title": "无法为您登录",
    "verify.error.missingToken": "此链接缺少其令牌。",
    "verify.error.invalid": "此链接无效或已过期。",
    "verify.error.serviceUnavailable": "无法连接到登录服务，请稍后重试。",
    "verify.error.requestNew": "请求新链接",
    "brand.tagline": "Main X 索引的统一登录",
    "splash.hero.title": "整个索引，一次登录",
    "splash.hero.subtitle":
      "通过发送到邮箱的链接登录，无需记密码，并在所有 Main X 索引应用中共用一个安全会话。",
    "splash.benefits.1.title": "无需管理密码",
    "splash.benefits.1.body":
      "无需记忆、不会泄露、无需重置：一次性链接即可完成。",
    "splash.benefits.2.title": "一个账号通行各处",
    "splash.benefits.2.body": "同一个账号即可登录所有 Main X 索引应用。",
    "splash.benefits.3.title": "几秒钟即可注册",
    "splash.benefits.3.body": "只需一个邮箱地址即可创建账号。",
    "splash.benefits.4.title": "使用所在组织的登录",
    "splash.benefits.4.body": "启用后，可通过您自己的身份提供商登录。",
    "splash.benefits.5.title": "退出立即生效",
    "splash.benefits.5.body": "退出登录会立刻在服务器上撤销您的会话。",
    "splash.benefits.6.title": "隐私优先的设计",
    "splash.benefits.6.body": "登录请求绝不会透露某个邮箱是否已有账号。",
    "splash.features.1.title": "邮件链接登录",
    "splash.features.1.body": "为已有账号申请一次性魔法链接。",
    "splash.features.2.title": "创建账号",
    "splash.features.2.body": "使用邮箱和可选的姓名注册。",
    "splash.features.3.title": "统一登录",
    "splash.features.3.body": "启用后，可继续使用所在组织的身份提供商。",
    "splash.features.4.title": "查看您的账号",
    "splash.features.4.body": "登录后即可查看姓名、邮箱和 ID。",
    "splash.features.5.title": "管理用户属性",
    "splash.features.5.body": "管理员分配决定权限的属性。",
    "splash.features.6.title": "安全退出",
    "splash.features.6.body": "一步结束会话并清除其 Cookie。",
    "splash.trust.3.title": "登录可审计",
    "splash.trust.3.body": "登录、退出和权限变更都会被记录。",
    "splash.trust.4.title": "会话安全无忧",
    "splash.trust.4.body": "您的会话保存在浏览器脚本无法读取的安全 Cookie 中。",
    "splash.trust.5.title": "开放标准",
    "splash.trust.5.body": "短时效 PASETO 令牌与 OpenID Connect 统一登录。",
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
    "splash.trust.6.title": "支持你的语言",
    "splash.trust.6.body":
      "阿拉伯语、中文、德语、英语、法语、印地语、西班牙语和威尔士语。",
    "splash.cta.title": "准备好开始了吗？",
    "splash.cta.body": "通过发送到邮箱的魔法链接登录，无需密码。",
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
      "Main X Auth 导览，它是所有 Main X 索引应用共用的单点登录：创建账户、通过邮件链接登录，以及管理你的会话。",
    "tour.s1.title": "创建账户",
    "tour.s1.summary": "只需一个电子邮箱即可注册，不会创建任何密码。",
    "tour.s1.step.1": "在菜单中选择“注册”。",
    "tour.s1.step.2": "输入你的电子邮箱，也可以填写姓名（可选）。",
    "tour.s1.step.3": "点击“发送魔法链接”。",
    "tour.s1.step.4":
      "若地址有效，页面会提示链接已在路上；从邮箱中打开它即可完成登录。",
    "tour.s2.title": "用魔法链接登录",
    "tour.s2.summary": "申请一次性链接，而不是输入密码。",
    "tour.s2.step.1": "选择“登录”并输入你的电子邮箱。",
    "tour.s2.step.2":
      "点击“给我发送一个魔法链接”；无论该地址是否有账户，回复都相同。",
    "tour.s2.step.3": "打开邮件里的链接：它在服务器端校验，且只能使用一次。",
    "tour.s2.step.4":
      "你会以已登录状态进入首页；若链接无效或已过期，请选择“请求新链接”。",
    "tour.s3.title": "用所属机构账号登录",
    "tour.s3.summary": "若部署已启用，可使用你所在机构的身份提供方。",
    "tour.s3.step.1": "在登录页面查找“使用 SSO 登录”；只有部署启用后才会显示。",
    "tour.s3.step.2": "选择它，浏览器会前往你所在机构的身份提供方。",
    "tour.s3.step.3": "在那里登录；认证服务校验结果并建立你的会话。",
    "tour.s3.step.4":
      "你会以已登录状态返回，与魔法链接完全一样；没有现有账户的邮箱会被拒绝，除非部署允许自动创建账户。",
    "tour.s4.title": "查看你的账户",
    "tour.s4.summary": "在任何 Main X 索引应用中，都能看到自己以哪个账户登录。",
    "tour.s4.step.1": "登录后，首页会显示“账户”。",
    "tour.s4.step.2": "其中列出你的姓名、电子邮箱和 ID。",
    "tour.s4.step.3": "页眉还会显示已登录标记，一眼即可确认。",
    "tour.s4.step.4":
      "你的会话保存在页面脚本无法读取的安全 Cookie 中；若看到请求过多的提示，请等几分钟再试。",
    "tour.s5.title": "退出登录",
    "tour.s5.summary": "在服务器上立即结束你的会话。",
    "tour.s5.step.1": "在页眉或“账户”页面选择“退出登录”。",
    "tour.s5.step.2": "服务立即撤销你的会话，而不只是在此浏览器中。",
    "tour.s5.step.3": "会话 Cookie 被清除。",
    "tour.s5.step.4": "首页重新显示欢迎页；要回来，请用新的魔法链接登录。",
    "tour.s6.title": "管理用户属性（管理员）",
    "tour.s6.summary": "管理员分配属性，以决定每个人可以做什么。",
    "tour.s6.step.1":
      "以管理员身份登录后，在“账户”页面选择“Manage user attributes (admin)”。",
    "tour.s6.step.2": "粘贴用户的 ID（pid），点击“Load”查看其当前属性。",
    "tour.s6.step.3":
      '编辑 JSON 映射，例如 {"access": ["write"]}，然后点击“Save”；发送 {} 可清除全部属性。',
    "tour.s6.step.4":
      "这需要管理员会话：若服务拒绝，会显示其错误信息，匿名访客会被送到登录页。",
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
 * The store is the single source of truth for the chosen locale: it persists
 * to localStorage and the layout mirrors it to `<html lang>` / `<html dir>`.
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
 * Reactive translation accessor for components: `t("signin.title")`.
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
