// Lightweight, dependency-free i18n for the Patient Flow SPA. A per-locale
// strings map plus a reactive `$state` current-locale (Svelte 5 runes),
// exposed via a `t(key)` accessor. Deliberately no i18n library: the
// surface is small and we keep the front-end dependency-light (drift
// between front-ends is accepted family-wide).
//
// Supported locales (family-wide set, sorted by code): Arabic (`ar-001`,
// RTL), Welsh (`cy-001`, for the public-sector Welsh-language duty),
// English (`en-001`, the source of truth), Spanish (`es-001`), French
// (`fr-001`), Hindi (`hi-001`), and Simplified Chinese for China
// (`zh-cn`). `-001` is the UN M.49 code for "world": a language with no
// regional variant. An unknown key/locale falls back to `en-001`. The
// chosen locale persists to localStorage.
//
// Coverage: the chrome (brand, navigation, picker labels, sign in/out,
// share), the signed-out splash and the home page (ward list). The other
// pages (whiteboard, stays, bed requests, EDD, locate, audits, sign-in
// form) are still English-only and are translated as they are touched.

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
 * Right-to-left locales. The document `dir` is set to `rtl` for these and
 * `ltr` for everything else (see {@link isRtl}).
 */
export const RTL_LOCALES = ["ar-001"] as const;

/**
 * Whether `locale` is written right-to-left (Arabic).
 *
 * @param locale - A supported locale code.
 * @returns `true` for RTL locales, `false` otherwise.
 */
export function isRtl(locale: Locale): boolean {
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

/**
 * localStorage key under which the chosen UI locale is persisted. The
 * chrome's `PickerBar` locale picker calls {@link i18n}'s `set` on
 * change (its own `applyDir={false}`, since this store already reflects
 * `lang`/`dir` onto `<html>` — see `+layout.svelte`); this key is what
 * {@link readStoredLocale} reads on boot.
 */
export const LOCALE_KEY = "mxi.patient-flow.locale";

// Every translatable UI string, keyed by a stable dotted key. `en-001` is
// the source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
  "ar-001": {
    brand: "تدفق المرضى",
    "brand.tagline": "الأسرّة والأجنحة ورحلة المريض المنوَّم",
    "nav.toggle": "تبديل التنقل",
    "nav.wards": "الأجنحة",
    "nav.at_a_glance": "نظرة سريعة",
    "nav.bed_requests": "طلبات الأسرّة",
    "nav.edd": "الخروج المتوقع",
    "nav.locate": "تحديد الموقع",
    "nav.audits": "التدقيق",
    "nav.theme": "السمة",
    "nav.language": "اللغة",
    "nav.text_size": "حجم النص",
    "nav.share": "مشاركة",
    "share.copy_link": "نسخ الرابط",
    "share.copied": "تم نسخ الرابط",
    "share.copy_failed": "تعذّر النسخ — انسخه من شريط العنوان",
    "home.col.code": "الرمز",
    "home.col.ward": "الجناح",
    "home.col.kind": "النوع",
    "home.col.beds": "الأسرّة",
    "home.col.occupied": "مشغولة",
    "home.col.available": "متاحة",
    "home.col.ready": "جاهزة للخروج",
    "home.col.dtoc": "DTOC",
    "home.col.board": "اللوحة",
    "home.link.whiteboard": "السبورة",
    "home.link.kiosk": "الكشك",
    "home.chip.esc": "تصعيد",
    "home.chip.closed": "مغلق",
    "home.as_of": "حتى",
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
    "splash.hero.title": "كل سرير وكل جناح في صورة حيّة واحدة",
    "splash.hero.subtitle":
      "اطّلع على حالة الأسرّة وتدفق المرضى لحظة بلحظة، وخصّص الأسرّة مع فحص القواعد، وحافظ على سير حالات الخروج، مع سجل تدقيق كامل خلف كل حركة.",
    "splash.benefits.1.title": "أسرّة يمكنك الوثوق بها",
    "splash.benefits.1.body":
      "تحلّ حالة الأسرّة الحيّة محل المكالمات الهاتفية وجداول البيانات القديمة.",
    "splash.benefits.2.title": "قبول أسرع للمرضى",
    "splash.benefits.2.body":
      "تُطابَق طلبات الأسرّة مع أسرّة شاغرة مناسبة أثناء انتظار المريض.",
    "splash.benefits.3.title": "تأخير أقل في الخروج",
    "splash.benefits.3.body":
      "يظهر المرضى الجاهزون للمغادرة وما يعوقهم قبل أن يُحجَز سرير بلا داعٍ.",
    "splash.benefits.4.title": "صورة مشتركة واحدة",
    "splash.benefits.4.body":
      "تظهر سعة الجناح والمستشفى والموقع بالشكل نفسه للجميع.",
    "splash.benefits.5.title": "تخصيص مع فحص القواعد",
    "splash.benefits.5.body": "لا يُخصَّص السرير إلا بعد فحص قواعد الإيواء.",
    "splash.benefits.6.title": "تسليم يمكن مراجعته",
    "splash.benefits.6.body":
      "تُسجَّل كل حركة سرير وكل عملية تسليم ويسهل الرجوع إليها.",
    "splash.features.1.title": "سبورات الأجنحة",
    "splash.features.1.body":
      "تعرض بطاقات الأسرّة التفاعلية المريض والحالة والعلامات بنظرة واحدة.",
    "splash.features.2.title": "وضع الكشك",
    "splash.features.2.body":
      "عرض بأهداف لمس كبيرة لشاشات اللمس في الأجنحة، مع عرض مُخفى الهوية اختياري.",
    "splash.features.3.title": "المستشفى بنظرة واحدة",
    "splash.features.3.body": "السعة الحيّة لكل جناح وللموقع بأكمله.",
    "splash.features.4.title": "طلبات الأسرّة",
    "splash.features.4.body": "قائمة طلب يُخصَّص فيها لكل طلب سرير مناسب.",
    "splash.features.5.title": "تواريخ الخروج المتوقعة",
    "splash.features.5.body": "تقويم شهري للخروج المتوقع ومدى الجاهزية للخروج.",
    "splash.features.6.title": "تحديد الموقع والتدقيق",
    "splash.features.6.body": "اعرف مكان المريض الآن وراجع سجل تسليم كل جناح.",
  },
  "cy-001": {
    brand: "Llif Cleifion",
    "brand.tagline": "Gwelyau, wardiau a thaith y claf mewnol",
    "nav.toggle": "Toglo'r llywio",
    "nav.wards": "Wardiau",
    "nav.at_a_glance": "Ar un olwg",
    "nav.bed_requests": "Ceisiadau am wely",
    "nav.edd": "EDD",
    "nav.locate": "Lleoli",
    "nav.audits": "Archwiliadau",
    "nav.theme": "Thema",
    "nav.language": "Iaith",
    "nav.text_size": "Maint testun",
    "nav.share": "Rhannu",
    "share.copy_link": "Copïo'r Ddolen",
    "share.copied": "Dolen wedi'i chopïo",
    "share.copy_failed": "Methu copïo — copïwch o far y cyfeiriad",
    "home.col.code": "Cod",
    "home.col.ward": "Ward",
    "home.col.kind": "Math",
    "home.col.beds": "Gwelyau",
    "home.col.occupied": "Llawn",
    "home.col.available": "Ar gael",
    "home.col.ready": "Barod i'w rhyddhau",
    "home.col.dtoc": "DTOC",
    "home.col.board": "Bwrdd",
    "home.link.whiteboard": "bwrdd gwyn",
    "home.link.kiosk": "ciosg",
    "home.chip.esc": "uwchgyf.",
    "home.chip.closed": "ar gau",
    "home.as_of": "ar",
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
    "splash.hero.title": "Pob gwely, pob ward, un darlun byw",
    "splash.hero.subtitle":
      "Gwelwch gyflwr gwelyau a llif cleifion wrth iddo ddigwydd, dyrannwch welyau gyda'r rheolau wedi'u gwirio, a chadwch ryddhau'n symud, gyda llwybr archwilio llawn y tu ôl i bob symudiad.",
    "splash.benefits.1.title": "Gwelyau y gallwch ymddiried ynddynt",
    "splash.benefits.1.body":
      "Mae cyflwr gwelyau byw yn disodli galwadau ffôn a thaenlenni hen.",
    "splash.benefits.2.title": "Derbyniadau cyflymach",
    "splash.benefits.2.body":
      "Caiff ceisiadau am wely eu paru â gwelyau rhydd addas tra bo'r claf yn aros.",
    "splash.benefits.3.title": "Llai o ryddhau hwyr",
    "splash.benefits.3.body":
      "Mae cleifion sy'n barod i adael, a'r hyn sy'n eu dal yn ôl, i'w gweld cyn i wely gael ei rwystro.",
    "splash.benefits.4.title": "Un darlun a rennir",
    "splash.benefits.4.body":
      "Mae gallu'r ward, yr ysbyty a'r safle yr un fath i bawb.",
    "splash.benefits.5.title": "Dyrannu gyda gwiriad rheolau",
    "splash.benefits.5.body":
      "Dim ond ar ôl gwirio'r rheolau lleoli y caiff gwely ei ddyrannu.",
    "splash.benefits.6.title": "Trosglwyddo y gellir ei adolygu",
    "splash.benefits.6.body":
      "Caiff pob symudiad gwely a throsglwyddiad ei gofnodi ac mae'n hawdd edrych yn ôl arno.",
    "splash.features.1.title": "Byrddau gwyn wardiau",
    "splash.features.1.body":
      "Mae cardiau gwely rhyngweithiol yn dangos y claf, y cyflwr a'r baneri ar un olwg.",
    "splash.features.2.title": "Modd ciosg",
    "splash.features.2.body":
      "Arddangosfa â thargedau mawr ar gyfer sgriniau cyffwrdd wardiau, gyda golwg cuddiedig dewisol.",
    "splash.features.3.title": "Yr ysbyty ar un olwg",
    "splash.features.3.body":
      "Gallu byw ar gyfer pob ward ac ar draws y safle cyfan.",
    "splash.features.4.title": "Ceisiadau am wely",
    "splash.features.4.body":
      "Ciw galw lle caiff pob cais ei ddyrannu i wely addas.",
    "splash.features.5.title": "Dyddiadau rhyddhau disgwyliedig",
    "splash.features.5.body":
      "Calendr mis o ryddhau disgwyliedig a pharodrwydd i ryddhau.",
    "splash.features.6.title": "Lleoli ac archwilio",
    "splash.features.6.body":
      "Dewch o hyd i ble mae claf nawr, ac adolygwch lwybr trosglwyddo pob ward.",
  },
  "en-001": {
    brand: "Patient Flow",
    "brand.tagline": "Beds, wards and the inpatient journey",
    "nav.toggle": "Toggle navigation",
    "nav.wards": "Wards",
    "nav.at_a_glance": "At a glance",
    "nav.bed_requests": "Bed requests",
    "nav.edd": "EDD",
    "nav.locate": "Locate",
    "nav.audits": "Audits",
    "nav.theme": "Theme",
    "nav.language": "Language",
    "nav.text_size": "Text size",
    "nav.share": "Share",
    "share.copy_link": "Copy Link",
    "share.copied": "Link copied",
    "share.copy_failed": "Could not copy — copy it from the address bar",
    "home.col.code": "Code",
    "home.col.ward": "Ward",
    "home.col.kind": "Kind",
    "home.col.beds": "Beds",
    "home.col.occupied": "Occupied",
    "home.col.available": "Available",
    "home.col.ready": "Ready",
    "home.col.dtoc": "DTOC",
    "home.col.board": "Board",
    "home.link.whiteboard": "whiteboard",
    "home.link.kiosk": "kiosk",
    "home.chip.esc": "esc",
    "home.chip.closed": "closed",
    "home.as_of": "as of",
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
    "splash.hero.title": "Every bed, every ward, one live picture",
    "splash.hero.subtitle":
      "See bed state and patient flow as it happens, allocate beds with rules checked, and keep discharges moving, with a full audit trail behind every move.",
    "splash.benefits.1.title": "Beds you can trust",
    "splash.benefits.1.body":
      "Live bed state replaces phone calls and out-of-date spreadsheets.",
    "splash.benefits.2.title": "Faster admissions",
    "splash.benefits.2.body":
      "Bed requests are matched to suitable free beds while the patient waits.",
    "splash.benefits.3.title": "Fewer delayed discharges",
    "splash.benefits.3.body":
      "Patients ready to leave, and what is holding them, stand out before a bed is blocked.",
    "splash.benefits.4.title": "One shared picture",
    "splash.benefits.4.body":
      "Ward, hospital and site capacity read the same for everyone.",
    "splash.benefits.5.title": "Rule-checked allocation",
    "splash.benefits.5.body":
      "A bed is only allocated once the placement rules have been checked.",
    "splash.benefits.6.title": "Handover you can review",
    "splash.benefits.6.body":
      "Every bed move and handover is recorded and easy to look back on.",
    "splash.features.1.title": "Ward whiteboards",
    "splash.features.1.body":
      "Interactive bed cards show patient, state and flags at a glance.",
    "splash.features.2.title": "Kiosk mode",
    "splash.features.2.body":
      "A large-target display for ward touchscreens, with an optional masked view.",
    "splash.features.3.title": "Hospital at a glance",
    "splash.features.3.body":
      "Live capacity for each ward and across the whole site.",
    "splash.features.4.title": "Bed requests",
    "splash.features.4.body":
      "A demand queue where each request is allocated to a suitable bed.",
    "splash.features.5.title": "Expected discharge dates",
    "splash.features.5.body":
      "A month calendar of expected discharges and discharge readiness.",
    "splash.features.6.title": "Locate and audit",
    "splash.features.6.body":
      "Find where a patient is now, and review each ward's handover trail.",
  },
  "es-001": {
    brand: "Flujo de pacientes",
    "brand.tagline": "Camas, salas y el recorrido del paciente ingresado",
    "nav.toggle": "Alternar navegación",
    "nav.wards": "Salas",
    "nav.at_a_glance": "De un vistazo",
    "nav.bed_requests": "Solicitudes de cama",
    "nav.edd": "FPA",
    "nav.locate": "Localizar",
    "nav.audits": "Auditorías",
    "nav.theme": "Tema",
    "nav.language": "Idioma",
    "nav.text_size": "Tamaño del texto",
    "nav.share": "Compartir",
    "share.copy_link": "Copiar enlace",
    "share.copied": "Enlace copiado",
    "share.copy_failed":
      "No se pudo copiar: cópialo desde la barra de direcciones",
    "home.col.code": "Código",
    "home.col.ward": "Sala",
    "home.col.kind": "Tipo",
    "home.col.beds": "Camas",
    "home.col.occupied": "Ocupadas",
    "home.col.available": "Disponibles",
    "home.col.ready": "Listas para el alta",
    "home.col.dtoc": "DTOC",
    "home.col.board": "Tablero",
    "home.link.whiteboard": "pizarra",
    "home.link.kiosk": "quiosco",
    "home.chip.esc": "esc.",
    "home.chip.closed": "cerrada",
    "home.as_of": "a fecha de",
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
    "splash.hero.title": "Cada cama, cada sala, una imagen en vivo",
    "splash.hero.subtitle":
      "Consulta el estado de las camas y el flujo de pacientes al instante, asigna camas con las reglas comprobadas y mantén las altas en marcha, con auditoría completa detrás de cada movimiento.",
    "splash.benefits.1.title": "Camas en las que confiar",
    "splash.benefits.1.body":
      "El estado de las camas en vivo sustituye a las llamadas y a las hojas de cálculo desactualizadas.",
    "splash.benefits.2.title": "Ingresos más rápidos",
    "splash.benefits.2.body":
      "Las solicitudes de cama se emparejan con camas libres adecuadas mientras el paciente espera.",
    "splash.benefits.3.title": "Menos altas retrasadas",
    "splash.benefits.3.body":
      "Los pacientes listos para irse, y lo que los retiene, destacan antes de que se bloquee una cama.",
    "splash.benefits.4.title": "Una imagen compartida",
    "splash.benefits.4.body":
      "La capacidad de sala, hospital y centro se ve igual para todos.",
    "splash.benefits.5.title": "Asignación con reglas comprobadas",
    "splash.benefits.5.body":
      "Una cama solo se asigna una vez comprobadas las reglas de colocación.",
    "splash.benefits.6.title": "Traspasos revisables",
    "splash.benefits.6.body":
      "Cada movimiento de cama y cada traspaso queda registrado y es fácil de consultar.",
    "splash.features.1.title": "Pizarras de sala",
    "splash.features.1.body":
      "Las tarjetas de cama interactivas muestran paciente, estado e indicadores de un vistazo.",
    "splash.features.2.title": "Modo quiosco",
    "splash.features.2.body":
      "Una pantalla de botones grandes para pantallas táctiles de sala, con vista enmascarada opcional.",
    "splash.features.3.title": "El hospital de un vistazo",
    "splash.features.3.body":
      "Capacidad en vivo de cada sala y de todo el centro.",
    "splash.features.4.title": "Solicitudes de cama",
    "splash.features.4.body":
      "Una cola de demanda en la que cada solicitud se asigna a una cama adecuada.",
    "splash.features.5.title": "Fechas previstas de alta",
    "splash.features.5.body":
      "Un calendario mensual de altas previstas y de preparación para el alta.",
    "splash.features.6.title": "Localizar y auditar",
    "splash.features.6.body":
      "Encuentra dónde está un paciente ahora y revisa el historial de traspasos de cada sala.",
  },
  "fr-001": {
    brand: "Parcours patient",
    "brand.tagline": "Lits, services et parcours du patient hospitalisé",
    "nav.toggle": "Basculer la navigation",
    "nav.wards": "Services",
    "nav.at_a_glance": "En un coup d'œil",
    "nav.bed_requests": "Demandes de lit",
    "nav.edd": "DPS",
    "nav.locate": "Localiser",
    "nav.audits": "Audits",
    "nav.theme": "Thème",
    "nav.language": "Langue",
    "nav.text_size": "Taille du texte",
    "nav.share": "Partager",
    "share.copy_link": "Copier le lien",
    "share.copied": "Lien copié",
    "share.copy_failed":
      "Copie impossible : copiez-le depuis la barre d'adresse",
    "home.col.code": "Code",
    "home.col.ward": "Service",
    "home.col.kind": "Type",
    "home.col.beds": "Lits",
    "home.col.occupied": "Occupés",
    "home.col.available": "Disponibles",
    "home.col.ready": "Prêts à sortir",
    "home.col.dtoc": "DTOC",
    "home.col.board": "Tableau",
    "home.link.whiteboard": "tableau blanc",
    "home.link.kiosk": "borne",
    "home.chip.esc": "esc.",
    "home.chip.closed": "fermé",
    "home.as_of": "au",
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
    "splash.hero.title": "Chaque lit, chaque service, une seule vue en direct",
    "splash.hero.subtitle":
      "Consultez l'état des lits et le parcours des patients en temps réel, attribuez les lits avec les règles vérifiées et gardez les sorties en mouvement, avec une piste d'audit complète derrière chaque mouvement.",
    "splash.benefits.1.title": "Des lits fiables",
    "splash.benefits.1.body":
      "L'état des lits en direct remplace les appels téléphoniques et les tableurs périmés.",
    "splash.benefits.2.title": "Admissions plus rapides",
    "splash.benefits.2.body":
      "Les demandes de lit sont associées à des lits libres adaptés pendant que le patient attend.",
    "splash.benefits.3.title": "Moins de sorties retardées",
    "splash.benefits.3.body":
      "Les patients prêts à sortir, et ce qui les retient, ressortent avant qu'un lit ne soit bloqué.",
    "splash.benefits.4.title": "Une vue partagée",
    "splash.benefits.4.body":
      "La capacité du service, de l'hôpital et du site se lit de la même façon pour tous.",
    "splash.benefits.5.title": "Attribution contrôlée par des règles",
    "splash.benefits.5.body":
      "Un lit n'est attribué qu'une fois les règles de placement vérifiées.",
    "splash.benefits.6.title": "Des relèves consultables",
    "splash.benefits.6.body":
      "Chaque mouvement de lit et chaque relève est enregistré et facile à retrouver.",
    "splash.features.1.title": "Tableaux blancs de service",
    "splash.features.1.body":
      "Des cartes de lit interactives montrent patient, état et indicateurs en un coup d'œil.",
    "splash.features.2.title": "Mode borne",
    "splash.features.2.body":
      "Un affichage à grandes zones tactiles pour les écrans de service, avec vue masquée en option.",
    "splash.features.3.title": "L'hôpital en un coup d'œil",
    "splash.features.3.body":
      "Capacité en direct pour chaque service et pour tout le site.",
    "splash.features.4.title": "Demandes de lit",
    "splash.features.4.body":
      "Une file de demandes où chaque requête est attribuée à un lit adapté.",
    "splash.features.5.title": "Dates de sortie prévues",
    "splash.features.5.body":
      "Un calendrier mensuel des sorties prévues et de la préparation à la sortie.",
    "splash.features.6.title": "Localiser et auditer",
    "splash.features.6.body":
      "Retrouvez où se trouve un patient et consultez l'historique des relèves de chaque service.",
  },
  "hi-001": {
    brand: "रोगी प्रवाह",
    "brand.tagline": "बिस्तर, वार्ड और भर्ती मरीज़ की यात्रा",
    "nav.toggle": "नेविगेशन टॉगल करें",
    "nav.wards": "वार्ड",
    "nav.at_a_glance": "एक नज़र में",
    "nav.bed_requests": "बिस्तर अनुरोध",
    "nav.edd": "अनुमानित डिस्चार्ज",
    "nav.locate": "खोजें",
    "nav.audits": "ऑडिट",
    "nav.theme": "थीम",
    "nav.language": "भाषा",
    "nav.text_size": "टेक्स्ट आकार",
    "nav.share": "साझा करें",
    "share.copy_link": "लिंक कॉपी करें",
    "share.copied": "लिंक कॉपी हो गया",
    "share.copy_failed": "कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें",
    "home.col.code": "कोड",
    "home.col.ward": "वार्ड",
    "home.col.kind": "प्रकार",
    "home.col.beds": "बिस्तर",
    "home.col.occupied": "भरे हुए",
    "home.col.available": "उपलब्ध",
    "home.col.ready": "डिस्चार्ज के लिए तैयार",
    "home.col.dtoc": "DTOC",
    "home.col.board": "बोर्ड",
    "home.link.whiteboard": "व्हाइटबोर्ड",
    "home.link.kiosk": "किओस्क",
    "home.chip.esc": "एस्केलेशन",
    "home.chip.closed": "बंद",
    "home.as_of": "तक की स्थिति:",
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
    "splash.hero.title": "हर बिस्तर, हर वार्ड, एक जीवंत तस्वीर",
    "splash.hero.subtitle":
      "बिस्तरों की स्थिति और मरीज़ों का प्रवाह तुरंत देखें, नियमों की जाँच के साथ बिस्तर आवंटित करें, और डिस्चार्ज को गति में रखें, हर बदलाव के पीछे पूरे ऑडिट ट्रेल के साथ।",
    "splash.benefits.1.title": "भरोसेमंद बिस्तर स्थिति",
    "splash.benefits.1.body":
      "लाइव बिस्तर स्थिति फ़ोन कॉल और पुरानी स्प्रेडशीट की जगह लेती है।",
    "splash.benefits.2.title": "तेज़ भर्ती",
    "splash.benefits.2.body":
      "मरीज़ के इंतज़ार करते समय बिस्तर अनुरोधों को उपयुक्त खाली बिस्तरों से मिलाया जाता है।",
    "splash.benefits.3.title": "कम विलंबित डिस्चार्ज",
    "splash.benefits.3.body":
      "जाने के लिए तैयार मरीज़ और उन्हें रोकने वाली वजहें बिस्तर अटकने से पहले ही दिख जाती हैं।",
    "splash.benefits.4.title": "एक साझा तस्वीर",
    "splash.benefits.4.body":
      "वार्ड, अस्पताल और साइट की क्षमता सभी को एक जैसी दिखती है।",
    "splash.benefits.5.title": "नियम-जाँचा हुआ आवंटन",
    "splash.benefits.5.body":
      "बिस्तर तभी आवंटित होता है जब स्थान-निर्धारण नियमों की जाँच हो चुकी हो।",
    "splash.benefits.6.title": "समीक्षा योग्य हैंडओवर",
    "splash.benefits.6.body":
      "हर बिस्तर बदलाव और हैंडओवर दर्ज होता है और आसानी से देखा जा सकता है।",
    "splash.features.1.title": "वार्ड व्हाइटबोर्ड",
    "splash.features.1.body":
      "इंटरैक्टिव बिस्तर कार्ड मरीज़, स्थिति और संकेत एक नज़र में दिखाते हैं।",
    "splash.features.2.title": "किओस्क मोड",
    "splash.features.2.body":
      "वार्ड टचस्क्रीन के लिए बड़े टच-लक्ष्यों वाला डिस्प्ले, वैकल्पिक मास्क्ड दृश्य के साथ।",
    "splash.features.3.title": "अस्पताल एक नज़र में",
    "splash.features.3.body": "हर वार्ड और पूरी साइट की लाइव क्षमता।",
    "splash.features.4.title": "बिस्तर अनुरोध",
    "splash.features.4.body":
      "मांग की कतार जहाँ हर अनुरोध को उपयुक्त बिस्तर आवंटित किया जाता है।",
    "splash.features.5.title": "अनुमानित डिस्चार्ज तिथियाँ",
    "splash.features.5.body":
      "अनुमानित डिस्चार्ज और डिस्चार्ज की तैयारी का मासिक कैलेंडर।",
    "splash.features.6.title": "खोजें और ऑडिट करें",
    "splash.features.6.body":
      "देखें कि मरीज़ अभी कहाँ है, और हर वार्ड का हैंडओवर ट्रेल देखें।",
  },
  "zh-cn": {
    brand: "患者流转",
    "brand.tagline": "床位、病区与住院旅程",
    "nav.toggle": "切换导航",
    "nav.wards": "病区",
    "nav.at_a_glance": "概览",
    "nav.bed_requests": "床位申请",
    "nav.edd": "预计出院",
    "nav.locate": "定位",
    "nav.audits": "审计",
    "nav.theme": "主题",
    "nav.language": "语言",
    "nav.text_size": "文字大小",
    "nav.share": "分享",
    "share.copy_link": "复制链接",
    "share.copied": "链接已复制",
    "share.copy_failed": "无法复制——请从地址栏复制",
    "home.col.code": "代码",
    "home.col.ward": "病区",
    "home.col.kind": "类型",
    "home.col.beds": "床位",
    "home.col.occupied": "已占用",
    "home.col.available": "可用",
    "home.col.ready": "可出院",
    "home.col.dtoc": "DTOC",
    "home.col.board": "看板",
    "home.link.whiteboard": "白板",
    "home.link.kiosk": "自助屏",
    "home.chip.esc": "升级",
    "home.chip.closed": "已关闭",
    "home.as_of": "截至",
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
    "splash.hero.title": "每张床位、每个病区，一幅实时全景",
    "splash.hero.subtitle":
      "实时查看床位状态与患者流转，在规则校验后分配床位，让出院持续推进，每一次变动都有完整审计记录。",
    "splash.benefits.1.title": "可信赖的床位信息",
    "splash.benefits.1.body": "实时床位状态取代电话询问和过时的电子表格。",
    "splash.benefits.2.title": "更快入院",
    "splash.benefits.2.body": "患者等待期间，床位申请即与合适的空床匹配。",
    "splash.benefits.3.title": "更少延误出院",
    "splash.benefits.3.body":
      "可出院的患者及其阻碍因素，在床位被占用之前就一目了然。",
    "splash.benefits.4.title": "共享同一视图",
    "splash.benefits.4.body": "病区、医院和院区的容量，对每个人都一致。",
    "splash.benefits.5.title": "规则校验后的分配",
    "splash.benefits.5.body": "只有在放置规则校验通过后，才会分配床位。",
    "splash.benefits.6.title": "可回溯的交接",
    "splash.benefits.6.body": "每次床位变动和交接都有记录，便于回看。",
    "splash.features.1.title": "病区白板",
    "splash.features.1.body": "交互式床位卡片一眼呈现患者、状态和标记。",
    "splash.features.2.title": "自助屏模式",
    "splash.features.2.body": "面向病区触摸屏的大按键显示，可选隐去患者身份。",
    "splash.features.3.title": "医院全景一览",
    "splash.features.3.body": "每个病区及整个院区的实时容量。",
    "splash.features.4.title": "床位申请",
    "splash.features.4.body": "需求队列，每个申请都会分配到合适的床位。",
    "splash.features.5.title": "预计出院日期",
    "splash.features.5.body": "按月显示预计出院及出院准备情况的日历。",
    "splash.features.6.title": "定位与审计",
    "splash.features.6.body": "查找患者当前位置，并回看每个病区的交接记录。",
  },
} as const;

/** Every valid translation key (derived from the English source table). */
export type StringKey = keyof (typeof STRINGS)["en-001"];

// Resolve a raw locale string to a supported locale, or null.
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
  // Unknown locale → English table; unknown key → English → the key.
  const table = STRINGS[locale] ?? STRINGS[DEFAULT_LOCALE];
  return table[key] ?? STRINGS[DEFAULT_LOCALE][key] ?? key;
}

/**
 * Reactive translation accessor for components: `t("nav.wards")`.
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
