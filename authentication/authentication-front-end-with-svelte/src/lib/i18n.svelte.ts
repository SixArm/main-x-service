// Lightweight, dependency-free i18n for the auth SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is tiny and we keep the front-end dependency-light (drift
// across the family front-ends is accepted, see AGENTS.md).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. Shared chrome terms reuse the family's established
// translations for consistency. An unknown key/locale falls back to
// `en-001`, then to the key string itself. The chosen locale persists to
// localStorage, drives the UI strings, `<html lang>`, and `<html dir>`
// (right-to-left for `ar-001`), and is sent as the `locale` field on
// signup / magic-link requests so the email language matches the UI (the
// service reduces it to its primary language subtag).

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
      "العربية والصينية والإنجليزية والفرنسية والهندية والإسبانية والويلزية.",
    "splash.cta.title": "هل أنت مستعد للبدء؟",
    "splash.cta.body":
      "سجّل الدخول برابط سحري يصلك على بريدك الإلكتروني. لا حاجة لكلمة مرور.",
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
      "Arabeg, Tsieinëeg, Saesneg, Ffrangeg, Hindi, Sbaeneg a Chymraeg.",
    "splash.cta.title": "Barod i ddechrau?",
    "splash.cta.body":
      "Mewngofnodwch gyda dolen hud a anfonir i'ch e-bost. Dim angen cyfrinair.",
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
      "Arabic, Chinese, English, French, Hindi, Spanish and Welsh.",
    "splash.cta.title": "Ready to get started?",
    "splash.cta.body":
      "Sign in with a magic link sent to your email. No password needed.",
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
      "Árabe, chino, español, francés, galés, hindi e inglés.",
    "splash.cta.title": "¿Listo para empezar?",
    "splash.cta.body":
      "Inicia sesión con un enlace mágico enviado a tu correo. No necesitas contraseña.",
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
      "Anglais, arabe, chinois, espagnol, français, gallois et hindi.",
    "splash.cta.title": "Prêt à commencer ?",
    "splash.cta.body":
      "Connectez-vous avec un lien magique envoyé par e-mail. Aucun mot de passe requis.",
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
      "अरबी, चीनी, अंग्रेज़ी, फ़्रेंच, हिन्दी, स्पेनिश और वेल्श।",
    "splash.cta.title": "शुरू करने के लिए तैयार हैं?",
    "splash.cta.body":
      "अपने ईमेल पर भेजे गए मैजिक लिंक से साइन इन करें। पासवर्ड की ज़रूरत नहीं।",
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
      "阿拉伯语、中文、英语、法语、印地语、西班牙语和威尔士语。",
    "splash.cta.title": "准备好开始了吗？",
    "splash.cta.body": "通过发送到邮箱的魔法链接登录，无需密码。",
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
