// Lightweight, dependency-free i18n for the auth SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is tiny and we keep the front-end dependency-light (drift
// across the family front-ends is accepted, see AGENTS.md).
//
// Supported locales: English (`en`, the source of truth) plus Welsh
// (`cy`, for the public-sector Welsh-language duty), Spanish (`es`),
// French (`fr`), German (`de`), Arabic (`ar`, RTL), Russian (`ru`),
// Hindi (`hi`), Mandarin Chinese (`zh`), Bengali (`bn`), Portuguese
// (`pt`), Indonesian (`id`), and Urdu (`ur`, RTL). Shared chrome terms
// reuse the family's established translations (see the course front-end)
// for consistency. An unknown key/locale falls back to `en`, then to the
// key string itself. The chosen locale persists to localStorage, drives
// the UI strings, `<html lang>`, and `<html dir>` (right-to-left for
// `ar` / `ur`), and is sent as the `locale` field on signup / magic-link
// requests so the email language matches the UI.

import { browser } from "$app/environment";

/**
 * Locales for which the UI is translated. To add one, extend this tuple
 * AND add a matching entry to {@link LOCALE_LABELS} and `STRINGS`.
 */
export const LOCALES = [
  "en",
  "cy",
  "es",
  "fr",
  "de",
  "ar",
  "ru",
  "hi",
  "zh",
  "bn",
  "pt",
  "id",
  "ur",
] as const;

/** A supported locale code (one of {@link LOCALES}). */
export type Locale = (typeof LOCALES)[number];

/** Fallback locale for an unknown key, locale, or missing translation. */
export const DEFAULT_LOCALE: Locale = "en";

/** Human-readable name for the locale switcher, written in that locale. */
export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  cy: "Cymraeg",
  es: "Español",
  fr: "Français",
  de: "Deutsch",
  ar: "العربية",
  ru: "Русский",
  hi: "हिन्दी",
  zh: "中文",
  bn: "বাংলা",
  pt: "Português",
  id: "Bahasa Indonesia",
  ur: "اردو",
};

/**
 * Right-to-left locales. The layout mirrors `<html dir>` to `rtl` for
 * these (and `ltr` for everyone else) via {@link isRtl}.
 */
export const RTL_LOCALES = ["ar", "ur"] as const satisfies readonly Locale[];

/**
 * Whether `locale` is written right-to-left. Used by the layout to set
 * `<html dir>`; tolerant of region subtags via {@link normaliseLocale}.
 *
 * @param locale - A locale code (region subtag like `ar-EG` accepted).
 * @returns `true` for `ar` / `ur`, otherwise `false`.
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
  en: {
    brand: "Main X Auth",
    "nav.home": "Home",
    "nav.signin": "Sign in",
    "nav.signup": "Sign up",
    "nav.locale": "Language",
    "nav.share": "Share",
    "nav.text_size": "Text size",
    "share.copy_link": "Copy link",
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
    "verify.error.requestNew": "Request a new link",
  },
  cy: {
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
    "signin.title": "Mewngofnodi",
    "signin.email": "E-bost",
    "signin.submit": "E-bostiwch ddolen hud ataf",
    "signin.submitting": "Yn anfon…",
    "signin.sent":
      "Os oes cyfrif gan yr e-bost hwnnw, mae dolen hud ar ei ffordd. Wrth ddatblygu, argreffir y ddolen i gonsol y gwasanaeth dilysu — agorwch hi i fewngofnodi.",
    "signin.noAccount": "Dim cyfrif eto?",
    "signin.create": "Crëwch un",
    "signin.failed": "Methodd y cais",
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
    "verify.error.requestNew": "Gofyn am ddolen newydd",
  },
  es: {
    brand: "Main X Auth",
    "nav.home": "Inicio",
    "nav.signin": "Iniciar sesión",
    "nav.signup": "Registrarse",
    "nav.locale": "Idioma",
    "nav.share": "Compartir",
    "nav.text_size": "Tamaño del texto",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed": "No se pudo copiar — cópielo desde la barra de direcciones",
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
    "signin.title": "Iniciar sesión",
    "signin.email": "Correo electrónico",
    "signin.submit": "Envíame un enlace mágico",
    "signin.submitting": "Enviando…",
    "signin.sent":
      "Si ese correo tiene una cuenta, un enlace mágico está en camino. En desarrollo, el enlace se imprime en la consola del servicio de autenticación — ábrelo para iniciar sesión.",
    "signin.noAccount": "¿Aún no tienes cuenta?",
    "signin.create": "Crea una",
    "signin.failed": "La solicitud falló",
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
    "verify.error.requestNew": "Solicitar un nuevo enlace",
  },
  fr: {
    brand: "Main X Auth",
    "nav.home": "Accueil",
    "nav.signin": "Se connecter",
    "nav.signup": "S'inscrire",
    "nav.locale": "Langue",
    "nav.share": "Partager",
    "nav.text_size": "Taille du texte",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed": "Impossible de copier — copiez-le depuis la barre d'adresse",
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
    "signin.title": "Se connecter",
    "signin.email": "E-mail",
    "signin.submit": "Envoyez-moi un lien magique",
    "signin.submitting": "Envoi…",
    "signin.sent":
      "Si cet e-mail correspond à un compte, un lien magique est en route. En développement, le lien est imprimé dans la console du service d'authentification — ouvrez-le pour vous connecter.",
    "signin.noAccount": "Pas encore de compte ?",
    "signin.create": "Créez-en un",
    "signin.failed": "La requête a échoué",
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
    "verify.error.requestNew": "Demander un nouveau lien",
  },
  de: {
    brand: "Main X Auth",
    "nav.home": "Startseite",
    "nav.signin": "Anmelden",
    "nav.signup": "Registrieren",
    "nav.locale": "Sprache",
    "nav.share": "Teilen",
    "nav.text_size": "Textgröße",
    "share.copy_link": "Link kopieren",
    "share.copied": "Link kopiert",
    "share.copy_failed": "Kopieren fehlgeschlagen — bitte aus der Adressleiste kopieren",
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
    "signin.title": "Anmelden",
    "signin.email": "E-Mail",
    "signin.submit": "Schick mir einen Magic Link",
    "signin.submitting": "Wird gesendet…",
    "signin.sent":
      "Wenn diese E-Mail ein Konto hat, ist ein Magic Link unterwegs. In der Entwicklung wird der Link in der Konsole des Authentifizierungsdienstes ausgegeben — öffnen Sie ihn zum Anmelden.",
    "signin.noAccount": "Noch kein Konto?",
    "signin.create": "Eines erstellen",
    "signin.failed": "Anfrage fehlgeschlagen",
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
    "verify.error.requestNew": "Neuen Link anfordern",
  },
  ar: {
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
    "signin.title": "تسجيل الدخول",
    "signin.email": "البريد الإلكتروني",
    "signin.submit": "أرسل لي رابطًا سحريًا",
    "signin.submitting": "جارٍ الإرسال…",
    "signin.sent":
      "إذا كان لهذا البريد الإلكتروني حساب، فإن رابطًا سحريًا في طريقه إليك. أثناء التطوير، يُطبع الرابط في وحدة تحكم خدمة المصادقة — افتحه لتسجيل الدخول.",
    "signin.noAccount": "ليس لديك حساب بعد؟",
    "signin.create": "أنشئ واحدًا",
    "signin.failed": "فشل الطلب",
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
    "verify.error.requestNew": "طلب رابط جديد",
  },
  ru: {
    brand: "Main X Auth",
    "nav.home": "Главная",
    "nav.signin": "Войти",
    "nav.signup": "Зарегистрироваться",
    "nav.locale": "Язык",
    "nav.share": "Поделиться",
    "nav.text_size": "Размер текста",
    "share.copy_link": "Копировать ссылку",
    "share.copied": "Ссылка скопирована",
    "share.copy_failed": "Не удалось скопировать — скопируйте из адресной строки",
    "nav.theme": "Тема",
    "nav.toggle": "Переключить навигацию",
    "session.signedInAs": "Вы вошли как",
    "account.title": "Учётная запись",
    "account.loading": "Загрузка…",
    "account.name": "Имя:",
    "account.email": "Эл. почта:",
    "account.id": "ID:",
    "account.signout": "Выйти",
    "account.notSignedIn": "Вы не вошли в систему.",
    "account.signinPrompt.signin": "Войти",
    "account.signinPrompt.or": "или",
    "account.signinPrompt.create": "создать учётную запись",
    "account.loadFailed": "Не удалось загрузить профиль",
    "signin.title": "Войти",
    "signin.email": "Эл. почта",
    "signin.submit": "Отправьте мне волшебную ссылку",
    "signin.submitting": "Отправка…",
    "signin.sent":
      "Если у этой почты есть учётная запись, волшебная ссылка уже в пути. В режиме разработки ссылка печатается в консоль службы аутентификации — откройте её, чтобы войти.",
    "signin.noAccount": "Ещё нет учётной записи?",
    "signin.create": "Создайте её",
    "signin.failed": "Запрос не выполнен",
    "signup.title": "Создать учётную запись",
    "signup.email": "Эл. почта",
    "signup.name": "Имя",
    "signup.nameOptional": "(необязательно)",
    "signup.submit": "Отправить волшебную ссылку",
    "signup.submitting": "Отправка…",
    "signup.sent":
      "Если эта почта действительна, волшебная ссылка уже в пути. В режиме разработки ссылка печатается в консоль службы аутентификации — откройте её, чтобы завершить вход.",
    "signup.backToSignin": "Назад ко входу",
    "signup.haveAccount": "Уже есть учётная запись?",
    "signup.signin": "Войти",
    "signup.failed": "Регистрация не удалась",
    "verify.working.title": "Выполняется вход…",
    "verify.working.body": "Проверка вашей волшебной ссылки.",
    "verify.error.title": "Не удалось выполнить вход",
    "verify.error.missingToken": "В этой ссылке отсутствует токен.",
    "verify.error.invalid": "Эта ссылка недействительна или просрочена.",
    "verify.error.requestNew": "Запросить новую ссылку",
  },
  hi: {
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
    "signin.title": "साइन इन करें",
    "signin.email": "ईमेल",
    "signin.submit": "मुझे एक मैजिक लिंक ईमेल करें",
    "signin.submitting": "भेजा जा रहा है…",
    "signin.sent":
      "यदि उस ईमेल का कोई खाता है, तो एक मैजिक लिंक रास्ते में है। डेवलपमेंट में लिंक प्रमाणीकरण सेवा कंसोल पर प्रिंट होता है — साइन इन करने के लिए उसे खोलें।",
    "signin.noAccount": "अभी तक कोई खाता नहीं?",
    "signin.create": "एक बनाएँ",
    "signin.failed": "अनुरोध विफल रहा",
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
    "verify.error.requestNew": "एक नया लिंक अनुरोध करें",
  },
  zh: {
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
    "signin.title": "登录",
    "signin.email": "电子邮箱",
    "signin.submit": "给我发送一个魔法链接",
    "signin.submitting": "发送中…",
    "signin.sent":
      "如果该邮箱已有账户，魔法链接正在发送途中。在开发环境中，链接会打印到认证服务控制台——打开它即可登录。",
    "signin.noAccount": "还没有账户？",
    "signin.create": "创建一个",
    "signin.failed": "请求失败",
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
    "verify.error.requestNew": "请求新链接",
  },
  bn: {
    brand: "Main X Auth",
    "nav.home": "হোম",
    "nav.signin": "সাইন ইন",
    "nav.signup": "সাইন আপ",
    "nav.locale": "ভাষা",
    "nav.share": "শেয়ার করুন",
    "nav.text_size": "টেক্সটের আকার",
    "share.copy_link": "লিঙ্ক কপি করুন",
    "share.copied": "লিঙ্ক কপি হয়েছে",
    "share.copy_failed": "কপি করা যায়নি — ঠিকানা বার থেকে কপি করুন",
    "nav.theme": "থিম",
    "nav.toggle": "নেভিগেশন টগল করুন",
    "session.signedInAs": "সাইন ইন করেছেন",
    "account.title": "অ্যাকাউন্ট",
    "account.loading": "লোড হচ্ছে…",
    "account.name": "নাম:",
    "account.email": "ইমেল:",
    "account.id": "ID:",
    "account.signout": "সাইন আউট",
    "account.notSignedIn": "আপনি সাইন ইন করেননি।",
    "account.signinPrompt.signin": "সাইন ইন",
    "account.signinPrompt.or": "অথবা",
    "account.signinPrompt.create": "একটি অ্যাকাউন্ট তৈরি করুন",
    "account.loadFailed": "প্রোফাইল লোড করতে ব্যর্থ",
    "signin.title": "সাইন ইন",
    "signin.email": "ইমেল",
    "signin.submit": "আমাকে একটি ম্যাজিক লিঙ্ক ইমেল করুন",
    "signin.submitting": "পাঠানো হচ্ছে…",
    "signin.sent":
      "যদি সেই ইমেলের একটি অ্যাকাউন্ট থাকে, একটি ম্যাজিক লিঙ্ক পথে রয়েছে। ডেভেলপমেন্টে লিঙ্কটি প্রমাণীকরণ পরিষেবার কনসোলে প্রিন্ট হয় — সাইন ইন করতে এটি খুলুন।",
    "signin.noAccount": "এখনও কোনো অ্যাকাউন্ট নেই?",
    "signin.create": "একটি তৈরি করুন",
    "signin.failed": "অনুরোধ ব্যর্থ হয়েছে",
    "signup.title": "অ্যাকাউন্ট তৈরি করুন",
    "signup.email": "ইমেল",
    "signup.name": "নাম",
    "signup.nameOptional": "(ঐচ্ছিক)",
    "signup.submit": "ম্যাজিক লিঙ্ক পাঠান",
    "signup.submitting": "পাঠানো হচ্ছে…",
    "signup.sent":
      "যদি সেই ইমেলটি বৈধ হয়, একটি ম্যাজিক লিঙ্ক পথে রয়েছে। ডেভেলপমেন্টে লিঙ্কটি প্রমাণীকরণ পরিষেবার কনসোলে প্রিন্ট হয় — সাইন ইন সম্পূর্ণ করতে এটি খুলুন।",
    "signup.backToSignin": "সাইন ইনে ফিরে যান",
    "signup.haveAccount": "ইতিমধ্যে একটি অ্যাকাউন্ট আছে?",
    "signup.signin": "সাইন ইন",
    "signup.failed": "সাইন আপ ব্যর্থ হয়েছে",
    "verify.working.title": "আপনাকে সাইন ইন করা হচ্ছে…",
    "verify.working.body": "আপনার ম্যাজিক লিঙ্ক যাচাই করা হচ্ছে।",
    "verify.error.title": "আপনাকে সাইন ইন করা যায়নি",
    "verify.error.missingToken": "এই লিঙ্কে এর টোকেন অনুপস্থিত।",
    "verify.error.invalid": "এই লিঙ্কটি অবৈধ বা মেয়াদোত্তীর্ণ।",
    "verify.error.requestNew": "একটি নতুন লিঙ্ক অনুরোধ করুন",
  },
  pt: {
    brand: "Main X Auth",
    "nav.home": "Início",
    "nav.signin": "Entrar",
    "nav.signup": "Registar",
    "nav.locale": "Idioma",
    "nav.share": "Partilhar",
    "nav.text_size": "Tamanho do texto",
    "share.copy_link": "Copiar link",
    "share.copied": "Link copiado",
    "share.copy_failed": "Não foi possível copiar — copie a partir da barra de endereço",
    "nav.theme": "Tema",
    "nav.toggle": "Alternar navegação",
    "session.signedInAs": "Sessão iniciada como",
    "account.title": "Conta",
    "account.loading": "Carregando…",
    "account.name": "Nome:",
    "account.email": "E-mail:",
    "account.id": "ID:",
    "account.signout": "Terminar sessão",
    "account.notSignedIn": "Não tem sessão iniciada.",
    "account.signinPrompt.signin": "Entrar",
    "account.signinPrompt.or": "ou",
    "account.signinPrompt.create": "criar uma conta",
    "account.loadFailed": "Falha ao carregar o perfil",
    "signin.title": "Entrar",
    "signin.email": "E-mail",
    "signin.submit": "Envie-me um link mágico",
    "signin.submitting": "Enviando…",
    "signin.sent":
      "Se esse e-mail tiver uma conta, um link mágico está a caminho. Em desenvolvimento, o link é impresso na consola do serviço de autenticação — abra-o para entrar.",
    "signin.noAccount": "Ainda não tem conta?",
    "signin.create": "Crie uma",
    "signin.failed": "O pedido falhou",
    "signup.title": "Criar conta",
    "signup.email": "E-mail",
    "signup.name": "Nome",
    "signup.nameOptional": "(opcional)",
    "signup.submit": "Enviar link mágico",
    "signup.submitting": "Enviando…",
    "signup.sent":
      "Se esse e-mail for válido, um link mágico está a caminho. Em desenvolvimento, o link é impresso na consola do serviço de autenticação — abra-o para concluir a entrada.",
    "signup.backToSignin": "Voltar a entrar",
    "signup.haveAccount": "Já tem uma conta?",
    "signup.signin": "Entrar",
    "signup.failed": "O registo falhou",
    "verify.working.title": "A iniciar a sua sessão…",
    "verify.working.body": "A verificar o seu link mágico.",
    "verify.error.title": "Não foi possível iniciar a sua sessão",
    "verify.error.missingToken": "Falta o token a este link.",
    "verify.error.invalid": "Este link é inválido ou expirou.",
    "verify.error.requestNew": "Solicitar um novo link",
  },
  id: {
    brand: "Main X Auth",
    "nav.home": "Beranda",
    "nav.signin": "Masuk",
    "nav.signup": "Daftar",
    "nav.locale": "Bahasa",
    "nav.share": "Bagikan",
    "nav.text_size": "Ukuran teks",
    "share.copy_link": "Salin tautan",
    "share.copied": "Tautan disalin",
    "share.copy_failed": "Tidak dapat menyalin — salin dari bilah alamat",
    "nav.theme": "Tema",
    "nav.toggle": "Alihkan navigasi",
    "session.signedInAs": "Masuk sebagai",
    "account.title": "Akun",
    "account.loading": "Memuat…",
    "account.name": "Nama:",
    "account.email": "Email:",
    "account.id": "ID:",
    "account.signout": "Keluar",
    "account.notSignedIn": "Anda belum masuk.",
    "account.signinPrompt.signin": "Masuk",
    "account.signinPrompt.or": "atau",
    "account.signinPrompt.create": "buat akun",
    "account.loadFailed": "Gagal memuat profil",
    "signin.title": "Masuk",
    "signin.email": "Email",
    "signin.submit": "Kirimi saya tautan ajaib",
    "signin.submitting": "Mengirim…",
    "signin.sent":
      "Jika email itu memiliki akun, tautan ajaib sedang dalam perjalanan. Dalam pengembangan, tautan dicetak ke konsol layanan autentikasi — buka untuk masuk.",
    "signin.noAccount": "Belum punya akun?",
    "signin.create": "Buat satu",
    "signin.failed": "Permintaan gagal",
    "signup.title": "Buat akun",
    "signup.email": "Email",
    "signup.name": "Nama",
    "signup.nameOptional": "(opsional)",
    "signup.submit": "Kirim tautan ajaib",
    "signup.submitting": "Mengirim…",
    "signup.sent":
      "Jika email itu valid, tautan ajaib sedang dalam perjalanan. Dalam pengembangan, tautan dicetak ke konsol layanan autentikasi — buka untuk menyelesaikan proses masuk.",
    "signup.backToSignin": "Kembali ke masuk",
    "signup.haveAccount": "Sudah punya akun?",
    "signup.signin": "Masuk",
    "signup.failed": "Pendaftaran gagal",
    "verify.working.title": "Memproses masuk Anda…",
    "verify.working.body": "Memverifikasi tautan ajaib Anda.",
    "verify.error.title": "Tidak dapat memproses masuk Anda",
    "verify.error.missingToken": "Tautan ini kehilangan tokennya.",
    "verify.error.invalid": "Tautan ini tidak valid atau kedaluwarsa.",
    "verify.error.requestNew": "Minta tautan baru",
  },
  ur: {
    brand: "Main X Auth",
    "nav.home": "ہوم",
    "nav.signin": "سائن ان کریں",
    "nav.signup": "سائن اپ کریں",
    "nav.locale": "زبان",
    "nav.share": "شیئر کریں",
    "nav.text_size": "متن کا سائز",
    "share.copy_link": "لنک کاپی کریں",
    "share.copied": "لنک کاپی ہو گیا",
    "share.copy_failed": "کاپی نہیں ہو سکا — اسے ایڈریس بار سے کاپی کریں",
    "nav.theme": "تھیم",
    "nav.toggle": "نیویگیشن ٹوگل کریں",
    "session.signedInAs": "بطور سائن ان",
    "account.title": "اکاؤنٹ",
    "account.loading": "لوڈ ہو رہا ہے…",
    "account.name": "نام:",
    "account.email": "ای میل:",
    "account.id": "ID:",
    "account.signout": "سائن آؤٹ کریں",
    "account.notSignedIn": "آپ سائن ان نہیں ہیں۔",
    "account.signinPrompt.signin": "سائن ان کریں",
    "account.signinPrompt.or": "یا",
    "account.signinPrompt.create": "ایک اکاؤنٹ بنائیں",
    "account.loadFailed": "پروفائل لوڈ کرنے میں ناکام",
    "signin.title": "سائن ان کریں",
    "signin.email": "ای میل",
    "signin.submit": "مجھے ایک جادوئی لنک ای میل کریں",
    "signin.submitting": "بھیجا جا رہا ہے…",
    "signin.sent":
      "اگر اس ای میل کا کوئی اکاؤنٹ ہے تو ایک جادوئی لنک راستے میں ہے۔ ڈیولپمنٹ میں لنک تصدیقی سروس کنسول پر پرنٹ ہوتا ہے — سائن ان کرنے کے لیے اسے کھولیں۔",
    "signin.noAccount": "ابھی تک کوئی اکاؤنٹ نہیں؟",
    "signin.create": "ایک بنائیں",
    "signin.failed": "درخواست ناکام ہوئی",
    "signup.title": "اکاؤنٹ بنائیں",
    "signup.email": "ای میل",
    "signup.name": "نام",
    "signup.nameOptional": "(اختیاری)",
    "signup.submit": "جادوئی لنک بھیجیں",
    "signup.submitting": "بھیجا جا رہا ہے…",
    "signup.sent":
      "اگر یہ ای میل درست ہے تو ایک جادوئی لنک راستے میں ہے۔ ڈیولپمنٹ میں لنک تصدیقی سروس کنسول پر پرنٹ ہوتا ہے — سائن ان مکمل کرنے کے لیے اسے کھولیں۔",
    "signup.backToSignin": "سائن ان پر واپس جائیں",
    "signup.haveAccount": "پہلے سے ایک اکاؤنٹ ہے؟",
    "signup.signin": "سائن ان کریں",
    "signup.failed": "سائن اپ ناکام ہوا",
    "verify.working.title": "آپ کو سائن ان کیا جا رہا ہے…",
    "verify.working.body": "آپ کے جادوئی لنک کی تصدیق ہو رہی ہے۔",
    "verify.error.title": "آپ کو سائن ان نہیں کیا جا سکا",
    "verify.error.missingToken": "اس لنک میں اس کا ٹوکن غائب ہے۔",
    "verify.error.invalid": "یہ لنک غلط یا میعاد ختم شدہ ہے۔",
    "verify.error.requestNew": "ایک نیا لنک طلب کریں",
  },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)["en"];

/**
 * Every translatable key (the English catalog's key set). Exported so a
 * coverage test can assert that each locale defines every key.
 */
export const STRING_KEYS = Object.keys(STRINGS.en) as StringKey[];

/** Raw per-locale strings table, exposed for coverage testing. */
export const STRINGS_BY_LOCALE: Record<
  Locale,
  Record<string, string>
> = STRINGS;

// Normalise raw input to a supported locale, or null if unsupported.
// Accepts a region subtag (cy-GB → cy) and is case-insensitive.
function normaliseLocale(raw: string | null | undefined): Locale | null {
  if (!raw) return null;
  // Take the primary subtag before any `-`/`_`, lowercased.
  const primary = raw.trim().split(/[-_]/)[0]?.toLowerCase() ?? "";
  return (LOCALES as readonly string[]).includes(primary)
    ? (primary as Locale)
    : null;
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
   * @param next - Desired locale (a region subtag like `cy-GB` is
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
