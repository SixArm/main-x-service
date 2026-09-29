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
    "nav.tour": "جولة",
    "splash.hero.tour": "ابدأ الجولة",
    "tour.head": "ابدأ الجولة",
    "tour.toc": "في هذه الصفحة",
    "tour.open": "افتح هذه الشاشة",
    "tour.top": "العودة إلى الأعلى",
    "tour.start.title": "قبل أن تبدأ",
    "tour.start.summary": "تحتاج إلى حساب للعمل مع البيانات الفعلية. يستغرق تسجيل الدخول أقل من دقيقة ولا يتطلب كلمة مرور.",
    "tour.start.step.1": "اختر «تسجيل الدخول» في أعلى اليمين وأدخل بريدك الإلكتروني.",
    "tour.start.step.2": "افتح الرابط السحري الذي نرسله إلى بريدك. يعمل مرة واحدة وتنتهي صلاحيته سريعًا.",
    "tour.start.step.3": "تعود إلى التطبيق وقد سجّلت الدخول، دون شيء لتتذكره أو تعيده.",
    "tour.start.step.4": "استخدم الأزرار بجوار «تسجيل الدخول» لتغيير السمة واللغة وحجم النص أو لمشاركة الصفحة.",
    "tour.intro": "جولة إرشادية في تدفق المرضى: ما تفعله كل شاشة وخطوات استخدامها، من قراءة سبورة الجناح إلى إخراج مريض ومراجعة سجل التدقيق.",
    "tour.s1.title": "قراءة لوحة أسرّة الجناح الحية",
    "tour.s1.summary": "كل سرير في الجناح كبطاقة مجمّعة حسب العنبر، تعرض المريض وتاريخ الخروج المتوقع والتنبيهات، وتتحدث تلقائيًا.",
    "tour.s1.step.1": "اختر الأجنحة من القائمة لعرض قائمة الأجنحة القابلة للتصفية حسب الرمز والجناح والنوع والتخصص، ثم حدّد صف الجناح لفتح سبورته.",
    "tour.s1.step.2": "اقرأ كل بطاقة سرير: حالتها واسم المريض ورموزًا مثل EDD وتحقق CCD ومسار الخروج وDTOC وجاهز ويوم أحمر أو أخضر واحتياطات العدوى.",
    "tour.s1.step.3": "في السرير الفارغ الذي ينتظر التنظيف استخدم Start clean ثم Clean done (أو Deep clean done عند الحاجة إلى تنظيف عميق) لإعادته إلى الخدمة.",
    "tour.s1.step.4": "تحقق من وقت «as of» فوق اللوحة؛ ولشاشة الحائط افتح عرض الكشك للجناح وأضف ?masked=1 إلى العنوان لإخفاء أسماء المرضى.",
    "tour.s2.title": "عرض المستشفى في لمحة",
    "tour.s2.summary": "صفحة واحدة بأرقام السعة لكل جناح، ليرى مدير الأسرّة أين الأسرّة وأين يتزايد الضغط.",
    "tour.s2.step.1": "اختر «نظرة سريعة» من القائمة لفتح صفحة المستشفى في لمحة.",
    "tour.s2.step.2": "اقرأ البطاقات في الأعلى: الأسرّة المتاحة الآن والمتوقع بحلول منتصف الليل وDTOC وطلبات الأسرّة المفتوحة مقسمة إلى طارئة وعاجلة وروتينية.",
    "tour.s2.step.3": "افحص جدول الأجنحة لمعرفة Occ % وAvail وResv وClean وClosed وEDD today وEDD overdue وReady وDTOC والإقامات التي تتجاوز 7 و21 يومًا.",
    "tour.s2.step.4": "لاحظ وقت «as of» في الأسفل لمعرفة مدى حداثة الأرقام، ثم انتقل من الجناح الذي يعاني من ضغط إلى سبورته.",
    "tour.s3.title": "وضع طلب سرير في الطابور وتخصيصه",
    "tour.s3.summary": "سجّل كل مريض ينتظر سريرًا مرتبًا حسب الأولوية، وضع كلًا منهم في سرير مؤهل.",
    "tour.s3.step.1": "اختر «طلبات الأسرّة» وتحت New request أدخل المريض بصيغة person:<uuid> ثم اختر المصدر (ed أو elective أو ward_transfer أو external أو virtual_step_up) والأولوية (emergency أو urgent أو routine).",
    "tour.s3.step.2": "اختر اختياريًا جناحًا مستهدفًا أو شرط الجنس أو العزل، ثم اختر Queue لإضافة الطلب إلى القائمة.",
    "tour.s3.step.3": "في الجدول تعرض الطلبات الأولوية والمصدر وعدد الأسرّة المؤهلة؛ ويُشار إلى الطلب الذي لا يوجد له سرير بعبارة “none — escalate”. اختر Show beds لعرض الأسرّة المؤهلة.",
    "tour.s3.step.4": "اختر سريرًا مثل «W7 · 12» (مع الإشارة إلى غرفة جانبية أو خارج الجناح عند الاقتضاء) لتخصيصه، أو Cancel لسحب الطلب.",
    "tour.s4.title": "تخطيط حالات الخروج على تقويم EDD",
    "tour.s4.summary": "تقويم شهري لتاريخ الخروج المتوقع لكل سرير مشغول، لتظهر حالات الخروج لليوم والأسبوع قبل حدوثها.",
    "tour.s4.step.1": "اختر «الخروج المتوقع» من القائمة لفتح صفحة الخروج المتوقع؛ وتعرض الصفحة وقت جمع البيانات.",
    "tour.s4.step.2": "كل إدخال هو مريض واحد موضوع في تاريخ EDD الخاص به، مع اسم الجناح ورقم السرير واسم المريض، لجميع الأجنحة معًا.",
    "tour.s4.step.3": "التقويم للقراءة فقط: لتصحيح تاريخ، اختر إدخالًا لفتح صفحة إقامة ذلك المريض.",
    "tour.s4.step.4": "في صفحة الإقامة، تحت SAFER، أدخل EDD وحدّد CCD met ثم اختر Save؛ وتُظهر بطاقة السرير التي بلا تاريخ «EDD missing» إلى أن تفعل ذلك.",
    "tour.s5.title": "إدارة الإقامة من SAFER إلى الخروج",
    "tour.s5.summary": "صفحة واحدة لكل إقامة تضم كل ما يحتاجه فريق الجناح: روتين SAFER اليومي وعلامات العدوى والنقل والخروج.",
    "tour.s5.step.1": "من السبورة اختر اسم المريض (أو استخدم «تحديد الموقع») لفتح إقامته؛ يعرض أعلى الصفحة الحالة ومدة الإقامة وEDD وDTOC وسجل أيام Red2Green.",
    "tour.s5.step.2": "تحت SAFER استخدم Mark senior review، وتحت Today's day سجّل اليوم أخضر أو أحمر مع اختيار السبب عند الأحمر؛ وأضف Infection flag مع الاحتياط والكائن الممرض.",
    "tour.s5.step.3": "لنقل المريض استخدم Transfer: أدخل السرير الوجهة وسببًا مثل clinical أو capacity أو isolation أو step_up؛ وتُدرج كل نقلة تحت Moves.",
    "tour.s5.step.4": "تحت Discharge اختر مسارًا (من P0 إلى P3) ثم Mark discharge-ready، بعدها اختر الوجهة واضغط Discharge لإنهاء الإقامة.",
    "tour.s6.title": "تحديد موقع مريض ومراجعة سجل التدقيق",
    "tour.s6.summary": "اعرف أين يوجد المريض الآن، مع العلم أن البحث نفسه يُسجَّل، ثم اقرأ سجل من فعل ماذا.",
    "tour.s6.step.1": "اختر «تحديد الموقع» من القائمة وأدخل المريض بصيغة person:<uuid> ثم اختر Locate.",
    "tour.s6.step.2": "تعرض النتيجة موقع المريض والجناح والعنبر والسرير، أو ملاحظة موقعه المنزلي إن لم يكن في سرير؛ وإن لم توجد إقامة يُخبرك بذلك. اختر Open stay للانتقال إلى صفحة الإقامة.",
    "tour.s6.step.3": "اختر «التدقيق» من القائمة؛ يعرض افتراضيًا الإدخالات الأخيرة مع When وEntity وAction وActor وDetail.",
    "tour.s6.step.4": "لتسليم المناوبة اختر جناحًا موسومًا «handover» وحدّد وقت البدء (افتراضيًا قبل 12 ساعة) ثم اختر Show.",
    "signin.sso": "تسجيل الدخول عبر SSO",
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
    "nav.tour": "Taith",
    "splash.hero.tour": "Cymerwch y daith",
    "tour.head": "Cymerwch y daith",
    "tour.toc": "Ar y dudalen hon",
    "tour.open": "Agor y sgrin hon",
    "tour.top": "Yn ôl i'r brig",
    "tour.start.title": "Cyn i chi ddechrau",
    "tour.start.summary": "Mae angen cyfrif arnoch i weithio gyda data go iawn. Mae mewngofnodi'n cymryd llai na munud ac nid oes angen cyfrinair.",
    "tour.start.step.1": "Dewiswch Mewngofnodi ar y brig ar y dde a rhowch eich cyfeiriad e-bost.",
    "tour.start.step.2": "Agorwch y ddolen hud a anfonwn atoch. Mae'n gweithio unwaith ac yn dod i ben yn gyflym.",
    "tour.start.step.3": "Byddwch yn dychwelyd i'r ap wedi mewngofnodi, heb ddim i'w gofio na'i ailosod.",
    "tour.start.step.4": "Defnyddiwch y botymau wrth ymyl Mewngofnodi i newid y thema, yr iaith a maint y testun, neu i rannu'r dudalen.",
    "tour.intro": "Taith dywys drwy Llif Cleifion: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o ddarllen bwrdd gwyn ward i ryddhau claf ac adolygu'r llwybr archwilio.",
    "tour.s1.title": "Darllen bwrdd gwelyau byw ward",
    "tour.s1.summary": "Pob gwely mewn ward fel cerdyn, wedi'i grwpio yn ôl bae, yn dangos y claf, dyddiad rhyddhau disgwyliedig a rhybuddion, ac yn adnewyddu'n awtomatig.",
    "tour.s1.step.1": "Dewiswch Wardiau yn y ddewislen i weld rhestr y wardiau, y gellir ei hidlo yn ôl cod, ward, math ac arbenigedd, yna dewiswch res ward i agor ei bwrdd gwyn.",
    "tour.s1.step.2": "Darllenwch bob cerdyn gwely: ei gyflwr, enw'r claf, a sgriniau fel EDD, CCD met, llwybr rhyddhau, DTOC, Ready, diwrnod Coch neu Wyrdd a rhagofalon haint.",
    "tour.s1.step.3": "Ar wely gwag sy'n aros am lanhau defnyddiwch Start clean, yna Clean done (neu Deep clean done pan fo angen glanhau dwfn) i'w ddychwelyd i'r gwasanaeth.",
    "tour.s1.step.4": "Gwiriwch yr amser «as of» uwchben y bwrdd; ar gyfer sgrin wal agorwch olwg kiosk y ward, ac ychwanegwch ?masked=1 at y cyfeiriad i guddio enwau cleifion.",
    "tour.s2.title": "Gweld yr ysbyty ar un olwg",
    "tour.s2.summary": "Un dudalen o rifau capasiti ar gyfer pob ward, fel y gall rheolwr gwelyau weld ble mae gwelyau a ble mae pwysau'n cynyddu.",
    "tour.s2.step.1": "Dewiswch Ar un olwg yn y ddewislen i agor Yr ysbyty ar un olwg.",
    "tour.s2.step.2": "Darllenwch y teils ar y brig: Beds available now, Predicted by midnight, DTOC, a'r ceisiadau am wely agored wedi'u rhannu'n argyfwng, brys a threfn.",
    "tour.s2.step.3": "Sganiwch dabl y wardiau am Occ %, Avail, Resv, Clean, Closed, EDD today, EDD overdue, Ready, DTOC, ac arosiadau dros 7 a 21 diwrnod.",
    "tour.s2.step.4": "Nodwch yr amser «as of» ar y gwaelod i wybod pa mor ffres yw'r ffigurau, a dilynwch ward dan bwysau i'w bwrdd gwyn.",
    "tour.s3.title": "Ciwio a dyrannu cais am wely",
    "tour.s3.summary": "Cofnodwch bob claf sy'n aros am wely, wedi'u trefnu yn ôl blaenoriaeth, a rhowch bob un mewn gwely addas.",
    "tour.s3.step.1": "Dewiswch Ceisiadau am wely, ac o dan New request rhowch y claf fel person:<uuid>, yna dewiswch y tarddiad (ed, elective, ward_transfer, external neu virtual_step_up) a'r flaenoriaeth (emergency, urgent neu routine).",
    "tour.s3.step.2": "Dewisol, dewiswch ward darged, gofyniad rhyw neu ynysu, yna dewiswch Queue i ychwanegu'r cais at y rhestr.",
    "tour.s3.step.3": "Yn y tabl mae ceisiadau'n dangos blaenoriaeth, tarddiad a faint o welyau sy'n addas; nodir cais heb yr un fel “none — escalate”. Dewiswch Show beds i restru'r gwelyau addas.",
    "tour.s3.step.4": "Dewiswch wely fel “W7 · 12” (wedi'i nodi'n ystafell ochr neu allanolyn lle bo'n berthnasol) i'w ddyrannu, neu Cancel i dynnu'r cais yn ôl.",
    "tour.s4.title": "Cynllunio rhyddhau ar galendr EDD",
    "tour.s4.summary": "Calendr mis o ddyddiad rhyddhau disgwyliedig pob gwely sy'n cael ei ddefnyddio, fel bod rhyddhau'r dydd a'r wythnos i'w gweld cyn iddynt ddigwydd.",
    "tour.s4.step.1": "Dewiswch EDD yn y ddewislen i agor Expected discharges; mae'r dudalen yn dangos yr amser y casglwyd y data.",
    "tour.s4.step.2": "Mae pob cofnod yn un claf wedi'i osod ar ei EDD, wedi'i labelu â'r ward, rhif y gwely ac enw'r claf, ar draws pob ward gyda'i gilydd.",
    "tour.s4.step.3": "Mae'r calendr yn ddarllen yn unig: i drwsio dyddiad, dewiswch gofnod i agor tudalen arhosiad y claf hwnnw.",
    "tour.s4.step.4": "Ar dudalen yr arhosiad, o dan SAFER, rhowch yr EDD a thicio CCD met, yna dewiswch Save; mae cerdyn gwely heb ddyddiad yn dangos “EDD missing” nes i chi wneud hynny.",
    "tour.s5.title": "Rheoli arhosiad o SAFER hyd at ryddhau",
    "tour.s5.summary": "Mae un dudalen ar gyfer pob arhosiad claf mewnol yn cynnwys pob gweithred sydd ei hangen ar dîm y ward: trefn SAFER ddyddiol, baneri haint, trosglwyddiadau a rhyddhau.",
    "tour.s5.step.1": "O fwrdd gwyn dewiswch enw'r claf (neu defnyddiwch Lleoli) i agor ei arhosiad; mae'r brig yn dangos statws, hyd arhosiad, EDD, DTOC a hanes diwrnodau Red2Green.",
    "tour.s5.step.2": "O dan SAFER defnyddiwch Mark senior review, ac o dan Today's day cofnodwch y diwrnod fel gwyrdd neu goch, gan ddewis rheswm pan fo'n goch; ychwanegwch Infection flag gyda rhagofal ac organeb.",
    "tour.s5.step.3": "I symud y claf defnyddiwch Transfer: rhowch y gwely cyrchfan a rheswm fel clinical, capacity, isolation neu step_up; rhestrir pob symudiad o dan Moves.",
    "tour.s5.step.4": "O dan Discharge dewiswch lwybr (P0 i P3) a dewiswch Mark discharge-ready, yna dewiswch y cyrchfan a dewiswch Discharge i orffen yr arhosiad.",
    "tour.s6.title": "Lleoli claf ac adolygu'r llwybr archwilio",
    "tour.s6.summary": "Darganfyddwch ble mae claf ar hyn o bryd, gan wybod bod y chwiliad ei hun yn cael ei gofnodi, yna darllenwch y llwybr o bwy wnaeth beth.",
    "tour.s6.step.1": "Dewiswch Lleoli yn y ddewislen, rhowch y claf fel person:<uuid>, a dewiswch Locate.",
    "tour.s6.step.2": "Mae'r canlyniad yn dangos safle, ward, bae a gwely'r claf, neu nodyn ei leoliad cartref os nad yw mewn gwely; dywedir wrthych os nad oes arhosiad. Dewiswch Open stay i fynd i dudalen yr arhosiad.",
    "tour.s6.step.3": "Dewiswch Archwiliadau yn y ddewislen; yn ddiofyn mae'n rhestru cofnodion diweddar gyda When, Entity, Action, Actor a Detail.",
    "tour.s6.step.4": "Ar gyfer trosglwyddo sifft, dewiswch ward wedi'i nodi “handover”, gosodwch yr amser cychwyn (12 awr yn ôl yn ddiofyn), a dewiswch Show.",
    "signin.sso": "Mewngofnodi gydag SSO",
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
    "nav.tour": "Tour",
    "splash.hero.tour": "Take the tour",
    "tour.head": "Take the tour",
    "tour.toc": "On this page",
    "tour.open": "Open this screen",
    "tour.top": "Back to top",
    "tour.start.title": "Before you begin",
    "tour.start.summary": "You need an account to work with real data. Signing in takes under a minute and needs no password.",
    "tour.start.step.1": "Choose Sign in at the top right and enter your email address.",
    "tour.start.step.2": "Open the magic link we email you. It works once and expires quickly.",
    "tour.start.step.3": "You return to the app signed in, with nothing to remember or reset.",
    "tour.start.step.4": "Use the buttons beside Sign in to change the theme, language and text size, or to share the page.",
    "tour.intro": "A guided walkthrough of Patient Flow: what each screen does and the steps to use it, from reading a ward whiteboard to discharging a patient and reviewing the audit trail.",
    "tour.s1.title": "Read a ward's live bed board",
    "tour.s1.summary": "Every bed in a ward as a card, grouped by bay, showing the patient, expected discharge date and alerts, and refreshed automatically.",
    "tour.s1.step.1": "Choose Wards in the menu to see the ward list, filterable by code, ward, kind and specialty, then select a ward's row to open its whiteboard.",
    "tour.s1.step.2": "Read each bed card: its state, the patient's name, and chips such as EDD, CCD met, discharge pathway, DTOC, Ready, Red or Green day, and infection precautions.",
    "tour.s1.step.3": "On an empty bed awaiting cleaning use Start clean, then Clean done (or Deep clean done when a deep clean is required) to return it to service.",
    "tour.s1.step.4": "Check the as-of time above the board; for a wall display open the ward's kiosk view, and add ?masked=1 to the address to hide patient names.",
    "tour.s2.title": "See the hospital at a glance",
    "tour.s2.summary": "One page of capacity numbers for every ward, so a bed manager can see where beds are and where pressure is building.",
    "tour.s2.step.1": "Choose At a glance in the menu to open Hospital at a glance.",
    "tour.s2.step.2": "Read the tiles at the top: Beds available now, Predicted by midnight, DTOC, and the open bed requests split into emergency, urgent and routine.",
    "tour.s2.step.3": "Scan the ward table for Occ %, Avail, Resv, Clean, Closed, EDD today, EDD overdue, Ready, DTOC, and stays over 7 and 21 days.",
    "tour.s2.step.4": "Note the as-of time at the bottom to know how fresh the figures are, and follow a ward with pressure to its whiteboard.",
    "tour.s3.title": "Queue and allocate a bed request",
    "tour.s3.summary": "Capture every patient waiting for a bed, ordered by priority, and place each one in an eligible bed.",
    "tour.s3.step.1": "Choose Bed requests, and under New request enter the patient as person:<uuid>, then pick the origin (ed, elective, ward_transfer, external or virtual_step_up) and priority (emergency, urgent or routine).",
    "tour.s3.step.2": "Optionally choose a target ward, a sex requirement, or isolation, then select Queue to add the request to the list.",
    "tour.s3.step.3": "In the table, requests show priority, origin and how many beds are eligible; a request with none is flagged “none — escalate”. Select Show beds to list the eligible beds.",
    "tour.s3.step.4": "Select a bed such as “W7 · 12” (marked side room or outlier where relevant) to allocate it, or Cancel to withdraw the request.",
    "tour.s4.title": "Plan discharges on the EDD calendar",
    "tour.s4.summary": "A month calendar of every occupied bed's expected discharge date, so the day's and week's discharges are visible before they happen.",
    "tour.s4.step.1": "Choose EDD in the menu to open Expected discharges; the page shows the time the data was gathered.",
    "tour.s4.step.2": "Each entry is one patient placed on their EDD, labelled with the ward, the bed number and the patient's name, across all wards together.",
    "tour.s4.step.3": "The calendar is read-only: to fix a date, select an entry to open that patient's stay page.",
    "tour.s4.step.4": "On the stay page, under SAFER, enter the EDD and tick CCD met, then select Save; a bed card without a date shows “EDD missing” until you do.",
    "tour.s5.title": "Run a stay from SAFER to discharge",
    "tour.s5.summary": "One page per inpatient stay holds every action the ward team needs: the daily SAFER routine, infection flags, transfers and discharge.",
    "tour.s5.step.1": "From a whiteboard select the patient's name (or use Locate) to open their stay; the top shows status, length of stay, EDD, DTOC, and the Red2Green day history.",
    "tour.s5.step.2": "Under SAFER use Mark senior review, and under Today's day record the day as green or red, choosing a reason when it is red; add an Infection flag with a precaution and organism.",
    "tour.s5.step.3": "To move the patient use Transfer: enter the destination bed and a reason such as clinical, capacity, isolation or step_up; each move is listed under Moves.",
    "tour.s5.step.4": "Under Discharge pick a pathway (P0 to P3) and select Mark discharge-ready, then choose the destination and select Discharge to finish the stay.",
    "tour.s6.title": "Locate a patient and review the audit trail",
    "tour.s6.summary": "Find where a patient is right now, knowing that the lookup is itself recorded, then read the trail of who did what.",
    "tour.s6.step.1": "Choose Locate in the menu, enter the patient as person:<uuid>, and select Locate.",
    "tour.s6.step.2": "The result shows the patient's site, ward, bay and bed, or their home location note if they are not in a bed; if no stay exists you are told so. Select Open stay to go to the stay page.",
    "tour.s6.step.3": "Choose Audits in the menu; by default it lists recent entries with When, Entity, Action, Actor and Detail.",
    "tour.s6.step.4": "For a shift handover, pick a ward marked “handover”, set the start time (it defaults to 12 hours ago), and select Show.",
    "signin.sso": "Sign in with SSO",
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
    "nav.tour": "Recorrido",
    "splash.hero.tour": "Haz el recorrido",
    "tour.head": "Haz el recorrido",
    "tour.toc": "En esta página",
    "tour.open": "Abrir esta pantalla",
    "tour.top": "Volver arriba",
    "tour.start.title": "Antes de empezar",
    "tour.start.summary": "Necesitas una cuenta para trabajar con datos reales. Iniciar sesión lleva menos de un minuto y no requiere contraseña.",
    "tour.start.step.1": "Elige Iniciar sesión arriba a la derecha e introduce tu correo electrónico.",
    "tour.start.step.2": "Abre el enlace mágico que te enviamos por correo. Funciona una sola vez y caduca pronto.",
    "tour.start.step.3": "Vuelves a la aplicación con la sesión iniciada, sin nada que recordar ni restablecer.",
    "tour.start.step.4": "Usa los botones junto a Iniciar sesión para cambiar el tema, el idioma y el tamaño del texto, o para compartir la página.",
    "tour.intro": "Un recorrido guiado por Flujo de pacientes: qué hace cada pantalla y los pasos para usarla, desde leer la pizarra de una sala hasta dar el alta a un paciente y revisar la auditoría.",
    "tour.s1.title": "Leer el tablero de camas en vivo de una sala",
    "tour.s1.summary": "Cada cama de una sala como una tarjeta, agrupada por bahía, con el paciente, la fecha de alta prevista y las alertas, y con actualización automática.",
    "tour.s1.step.1": "Elige Salas en el menú para ver la lista de salas, filtrable por código, sala, tipo y especialidad, y selecciona la fila de una sala para abrir su pizarra.",
    "tour.s1.step.2": "Lee cada tarjeta de cama: su estado, el nombre del paciente y etiquetas como EDD, CCD met, vía de alta, DTOC, Ready, día Rojo o Verde y precauciones de infección.",
    "tour.s1.step.3": "En una cama vacía pendiente de limpieza usa Start clean y luego Clean done (o Deep clean done si hace falta una limpieza a fondo) para devolverla al servicio.",
    "tour.s1.step.4": "Comprueba la hora «as of» sobre el tablero; para una pantalla de pared abre la vista de quiosco de la sala y añade ?masked=1 a la dirección para ocultar los nombres.",
    "tour.s2.title": "Ver el hospital de un vistazo",
    "tour.s2.summary": "Una página con las cifras de capacidad de cada sala, para que el gestor de camas vea dónde hay camas y dónde crece la presión.",
    "tour.s2.step.1": "Elige «De un vistazo» en el menú para abrir la página del hospital de un vistazo.",
    "tour.s2.step.2": "Lee los mosaicos superiores: Beds available now, Predicted by midnight, DTOC y las solicitudes de cama abiertas divididas en emergencia, urgente y rutina.",
    "tour.s2.step.3": "Recorre la tabla de salas: Occ %, Avail, Resv, Clean, Closed, EDD today, EDD overdue, Ready, DTOC y estancias de más de 7 y 21 días.",
    "tour.s2.step.4": "Fíjate en la hora «as of» al pie para saber cuán recientes son las cifras y pasa de una sala con presión a su pizarra.",
    "tour.s3.title": "Poner en cola y asignar una solicitud de cama",
    "tour.s3.summary": "Registra a cada paciente que espera una cama, ordenado por prioridad, y colócalo en una cama apta.",
    "tour.s3.step.1": "Elige Solicitudes de cama y, en New request, introduce al paciente como person:<uuid>; luego elige el origen (ed, elective, ward_transfer, external o virtual_step_up) y la prioridad (emergency, urgent o routine).",
    "tour.s3.step.2": "Si quieres, elige una sala destino, un requisito de sexo o aislamiento y pulsa Queue para añadir la solicitud a la lista.",
    "tour.s3.step.3": "En la tabla, las solicitudes muestran prioridad, origen y cuántas camas son aptas; una sin ninguna se marca “none — escalate”. Pulsa Show beds para listar las camas aptas.",
    "tour.s3.step.4": "Selecciona una cama como «W7 · 12» (marcada como side room u outlier si procede) para asignarla, o Cancel para retirar la solicitud.",
    "tour.s4.title": "Planificar altas en el calendario EDD",
    "tour.s4.summary": "Un calendario mensual con la fecha de alta prevista de cada cama ocupada, para ver las altas del día y de la semana antes de que ocurran.",
    "tour.s4.step.1": "Elige «FPA» en el menú para abrir Expected discharges; la página muestra la hora en que se recopilaron los datos.",
    "tour.s4.step.2": "Cada entrada es un paciente situado en su EDD, con la sala, el número de cama y el nombre del paciente, para todas las salas a la vez.",
    "tour.s4.step.3": "El calendario es de solo lectura: para corregir una fecha, selecciona una entrada para abrir la página de estancia de ese paciente.",
    "tour.s4.step.4": "En la página de estancia, en SAFER, introduce el EDD y marca CCD met, luego pulsa Save; una tarjeta de cama sin fecha muestra «EDD missing» hasta que lo hagas.",
    "tour.s5.title": "Gestionar una estancia desde SAFER hasta el alta",
    "tour.s5.summary": "Una página por estancia reúne todas las acciones del equipo de la sala: la rutina diaria SAFER, marcas de infección, traslados y alta.",
    "tour.s5.step.1": "Desde una pizarra selecciona el nombre del paciente (o usa Localizar) para abrir su estancia; arriba se ven estado, duración, EDD, DTOC y el historial de días Red2Green.",
    "tour.s5.step.2": "En SAFER usa Mark senior review y, en Today's day, registra el día como verde o rojo eligiendo un motivo si es rojo; añade una Infection flag con precaución y organismo.",
    "tour.s5.step.3": "Para trasladar al paciente usa Transfer: indica la cama de destino y un motivo como clinical, capacity, isolation o step_up; cada traslado aparece en Moves.",
    "tour.s5.step.4": "En Discharge elige una vía (P0 a P3) y pulsa Mark discharge-ready; después elige el destino y pulsa Discharge para terminar la estancia.",
    "tour.s6.title": "Localizar a un paciente y revisar la auditoría",
    "tour.s6.summary": "Averigua dónde está un paciente ahora mismo, sabiendo que la consulta queda registrada, y luego lee el rastro de quién hizo qué.",
    "tour.s6.step.1": "Elige «Localizar» en el menú, introduce al paciente como person:<uuid> y pulsa Locate.",
    "tour.s6.step.2": "El resultado muestra el centro, sala, bahía y cama del paciente, o su nota de ubicación en el domicilio si no está en una cama; si no hay estancia, se te indica. Pulsa Open stay para ir a la página de estancia.",
    "tour.s6.step.3": "Elige «Auditorías» en el menú; por defecto lista las entradas recientes con When, Entity, Action, Actor y Detail.",
    "tour.s6.step.4": "Para el relevo de turno, elige una sala marcada «handover», fija la hora de inicio (por defecto, hace 12 horas) y pulsa Show.",
    "signin.sso": "Iniciar sesión con SSO",
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
    "nav.tour": "Visite guidée",
    "splash.hero.tour": "Faire la visite guidée",
    "tour.head": "Faire la visite guidée",
    "tour.toc": "Sur cette page",
    "tour.open": "Ouvrir cet écran",
    "tour.top": "Retour en haut",
    "tour.start.title": "Avant de commencer",
    "tour.start.summary": "Un compte est nécessaire pour travailler avec des données réelles. La connexion prend moins d'une minute et n'exige aucun mot de passe.",
    "tour.start.step.1": "Choisissez Se connecter en haut à droite et saisissez votre adresse e-mail.",
    "tour.start.step.2": "Ouvrez le lien magique reçu par e-mail. Il ne fonctionne qu'une fois et expire vite.",
    "tour.start.step.3": "Vous revenez dans l'application connecté, sans rien à retenir ni à réinitialiser.",
    "tour.start.step.4": "Utilisez les boutons à côté de Se connecter pour changer le thème, la langue et la taille du texte, ou pour partager la page.",
    "tour.intro": "Une visite guidée de Flux des patients : le rôle de chaque écran et les étapes pour l'utiliser, de la lecture du tableau d'un service à la sortie d'un patient et à la consultation de la piste d'audit.",
    "tour.s1.title": "Lire le tableau de lits en direct d'un service",
    "tour.s1.summary": "Chaque lit d'un service sous forme de carte, regroupé par box, avec le patient, la date de sortie prévue et les alertes, actualisé automatiquement.",
    "tour.s1.step.1": "Choisissez Services dans le menu pour voir la liste des services, filtrable par code, service, type et spécialité, puis sélectionnez la ligne d'un service pour ouvrir son tableau.",
    "tour.s1.step.2": "Lisez chaque carte de lit : son état, le nom du patient et des pastilles telles que EDD, CCD met, parcours de sortie, DTOC, Ready, jour Rouge ou Vert et précautions d'infection.",
    "tour.s1.step.3": "Sur un lit vide en attente de nettoyage, utilisez Start clean puis Clean done (ou Deep clean done si un nettoyage approfondi est requis) pour le remettre en service.",
    "tour.s1.step.4": "Vérifiez l'heure « as of » au-dessus du tableau ; pour un écran mural, ouvrez la vue kiosque du service et ajoutez ?masked=1 à l'adresse pour masquer les noms des patients.",
    "tour.s2.title": "Voir l'hôpital d'un coup d'œil",
    "tour.s2.summary": "Une page de chiffres de capacité pour chaque service, afin que le gestionnaire de lits voie où sont les lits et où la pression monte.",
    "tour.s2.step.1": "Choisissez « En un coup d'œil » dans le menu pour ouvrir la page de l'hôpital d'un coup d'œil.",
    "tour.s2.step.2": "Lisez les tuiles en haut : Beds available now, Predicted by midnight, DTOC et les demandes de lit ouvertes réparties en urgence vitale, urgent et courant.",
    "tour.s2.step.3": "Parcourez le tableau des services : Occ %, Avail, Resv, Clean, Closed, EDD today, EDD overdue, Ready, DTOC et séjours de plus de 7 et 21 jours.",
    "tour.s2.step.4": "Notez l'heure « as of » en bas pour savoir à quel point les chiffres sont récents, puis passez d'un service sous tension à son tableau.",
    "tour.s3.title": "Mettre en file et attribuer une demande de lit",
    "tour.s3.summary": "Enregistrez chaque patient en attente d'un lit, classé par priorité, et placez-le dans un lit éligible.",
    "tour.s3.step.1": "Choisissez Demandes de lit et, sous New request, saisissez le patient sous la forme person:<uuid>, puis choisissez l'origine (ed, elective, ward_transfer, external ou virtual_step_up) et la priorité (emergency, urgent ou routine).",
    "tour.s3.step.2": "Si besoin, choisissez un service cible, une exigence de sexe ou l'isolement, puis sélectionnez Queue pour ajouter la demande à la liste.",
    "tour.s3.step.3": "Dans le tableau, les demandes affichent priorité, origine et nombre de lits éligibles ; une demande sans lit est signalée « none — escalate ». Sélectionnez Show beds pour lister les lits éligibles.",
    "tour.s3.step.4": "Sélectionnez un lit tel que « W7 · 12 » (marqué side room ou outlier le cas échéant) pour l'attribuer, ou Cancel pour retirer la demande.",
    "tour.s4.title": "Planifier les sorties sur le calendrier EDD",
    "tour.s4.summary": "Un calendrier mensuel de la date de sortie prévue de chaque lit occupé, pour voir les sorties du jour et de la semaine avant qu'elles n'aient lieu.",
    "tour.s4.step.1": "Choisissez « DPS » dans le menu pour ouvrir Expected discharges ; la page indique l'heure de collecte des données.",
    "tour.s4.step.2": "Chaque entrée est un patient placé à sa date EDD, avec le service, le numéro de lit et le nom du patient, pour tous les services réunis.",
    "tour.s4.step.3": "Le calendrier est en lecture seule : pour corriger une date, sélectionnez une entrée pour ouvrir la page du séjour de ce patient.",
    "tour.s4.step.4": "Sur la page du séjour, sous SAFER, saisissez l'EDD et cochez CCD met, puis sélectionnez Save ; une carte de lit sans date affiche « EDD missing » tant que ce n'est pas fait.",
    "tour.s5.title": "Conduire un séjour de SAFER à la sortie",
    "tour.s5.summary": "Une page par séjour regroupe toutes les actions de l'équipe du service : routine SAFER quotidienne, alertes infection, transferts et sortie.",
    "tour.s5.step.1": "Depuis un tableau, sélectionnez le nom du patient (ou utilisez Localiser) pour ouvrir son séjour ; le haut affiche statut, durée de séjour, EDD, DTOC et l'historique des jours Red2Green.",
    "tour.s5.step.2": "Sous SAFER utilisez Mark senior review et, sous Today's day, enregistrez le jour en vert ou rouge en choisissant un motif s'il est rouge ; ajoutez une Infection flag avec précaution et organisme.",
    "tour.s5.step.3": "Pour déplacer le patient, utilisez Transfer : saisissez le lit de destination et un motif tel que clinical, capacity, isolation ou step_up ; chaque déplacement figure sous Moves.",
    "tour.s5.step.4": "Sous Discharge choisissez un parcours (P0 à P3) et sélectionnez Mark discharge-ready, puis choisissez la destination et sélectionnez Discharge pour terminer le séjour.",
    "tour.s6.title": "Localiser un patient et consulter la piste d'audit",
    "tour.s6.summary": "Trouvez où se trouve un patient en ce moment, sachant que la recherche est elle-même enregistrée, puis lisez la trace de qui a fait quoi.",
    "tour.s6.step.1": "Choisissez « Localiser » dans le menu, saisissez le patient sous la forme person:<uuid> et sélectionnez Locate.",
    "tour.s6.step.2": "Le résultat indique le site, le service, le box et le lit du patient, ou sa note de localisation à domicile s'il n'est pas dans un lit ; s'il n'y a aucun séjour, vous en êtes informé. Sélectionnez Open stay pour aller à la page du séjour.",
    "tour.s6.step.3": "Choisissez « Audit » dans le menu ; par défaut, la page liste les entrées récentes avec When, Entity, Action, Actor et Detail.",
    "tour.s6.step.4": "Pour une relève d'équipe, choisissez un service marqué « handover », réglez l'heure de début (par défaut, il y a 12 heures) et sélectionnez Show.",
    "signin.sso": "Se connecter avec SSO",
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
    "nav.tour": "टूर",
    "splash.hero.tour": "टूर देखें",
    "tour.head": "टूर देखें",
    "tour.toc": "इस पृष्ठ पर",
    "tour.open": "यह स्क्रीन खोलें",
    "tour.top": "ऊपर लौटें",
    "tour.start.title": "शुरू करने से पहले",
    "tour.start.summary": "वास्तविक डेटा के साथ काम करने के लिए खाता चाहिए। साइन इन में एक मिनट से कम लगता है और पासवर्ड की ज़रूरत नहीं।",
    "tour.start.step.1": "ऊपर दाईं ओर साइन इन चुनें और अपना ईमेल पता दर्ज करें।",
    "tour.start.step.2": "हमारे भेजे मैजिक लिंक को खोलें। यह एक ही बार काम करता है और जल्दी समाप्त हो जाता है।",
    "tour.start.step.3": "आप साइन इन होकर ऐप में लौटते हैं, न कुछ याद रखना, न रीसेट करना।",
    "tour.start.step.4": "थीम, भाषा और टेक्स्ट का आकार बदलने या पृष्ठ साझा करने के लिए साइन इन के पास के बटन इस्तेमाल करें।",
    "tour.intro": "रोगी प्रवाह का निर्देशित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, वार्ड व्हाइटबोर्ड पढ़ने से लेकर मरीज़ को डिस्चार्ज करने और ऑडिट ट्रेल देखने तक।",
    "tour.s1.title": "वार्ड का लाइव बेड बोर्ड पढ़ें",
    "tour.s1.summary": "वार्ड का हर बिस्तर एक कार्ड के रूप में, बे के अनुसार समूहित, जिसमें मरीज़, अनुमानित डिस्चार्ज तिथि और अलर्ट दिखते हैं, और अपने आप ताज़ा होता है।",
    "tour.s1.step.1": "मेनू में वार्ड चुनें ताकि कोड, वार्ड, प्रकार और विशेषज्ञता से फ़िल्टर होने वाली वार्ड सूची दिखे, फिर किसी वार्ड की पंक्ति चुनकर उसका व्हाइटबोर्ड खोलें।",
    "tour.s1.step.2": "हर बेड कार्ड पढ़ें: उसकी स्थिति, मरीज़ का नाम, और EDD, CCD met, डिस्चार्ज पाथवे, DTOC, Ready, लाल या हरा दिन और संक्रमण सावधानियों जैसे चिप्स।",
    "tour.s1.step.3": "सफ़ाई की प्रतीक्षा वाले खाली बिस्तर पर Start clean और फिर Clean done (गहरी सफ़ाई ज़रूरी हो तो Deep clean done) चुनें ताकि वह फिर सेवा में आ जाए।",
    "tour.s1.step.4": "बोर्ड के ऊपर «as of» समय देखें; दीवार स्क्रीन के लिए वार्ड का किओस्क दृश्य खोलें और मरीज़ों के नाम छिपाने के लिए पते में ?masked=1 जोड़ें।",
    "tour.s2.title": "अस्पताल एक नज़र में देखें",
    "tour.s2.summary": "हर वार्ड के क्षमता आँकड़ों वाला एक पृष्ठ, ताकि बेड मैनेजर देख सके कि बिस्तर कहाँ हैं और दबाव कहाँ बढ़ रहा है।",
    "tour.s2.step.1": "मेनू में «एक नज़र में» चुनकर अस्पताल-एक-नज़र-में पृष्ठ खोलें।",
    "tour.s2.step.2": "ऊपर के टाइल पढ़ें: Beds available now, Predicted by midnight, DTOC, और खुले बेड अनुरोध जो आपातकालीन, तत्काल और सामान्य में बँटे हैं।",
    "tour.s2.step.3": "वार्ड तालिका में Occ %, Avail, Resv, Clean, Closed, EDD today, EDD overdue, Ready, DTOC और 7 व 21 दिन से अधिक के प्रवास देखें।",
    "tour.s2.step.4": "आँकड़े कितने ताज़ा हैं यह जानने के लिए नीचे «as of» समय देखें, और दबाव वाले वार्ड के व्हाइटबोर्ड पर जाएँ।",
    "tour.s3.title": "बेड अनुरोध कतार में डालें और आवंटित करें",
    "tour.s3.summary": "बिस्तर की प्रतीक्षा कर रहे हर मरीज़ को प्राथमिकता के क्रम में दर्ज करें और उसे योग्य बिस्तर में रखें।",
    "tour.s3.step.1": "बेड अनुरोध चुनें और New request में मरीज़ को person:<uuid> के रूप में डालें, फिर स्रोत (ed, elective, ward_transfer, external या virtual_step_up) और प्राथमिकता (emergency, urgent या routine) चुनें।",
    "tour.s3.step.2": "चाहें तो लक्ष्य वार्ड, लिंग की आवश्यकता या आइसोलेशन चुनें, फिर अनुरोध को सूची में जोड़ने के लिए Queue चुनें।",
    "tour.s3.step.3": "तालिका में अनुरोध प्राथमिकता, स्रोत और योग्य बिस्तरों की संख्या दिखाते हैं; जिसके लिए कोई नहीं है उस पर “none — escalate” का निशान होता है। योग्य बिस्तर देखने के लिए Show beds चुनें।",
    "tour.s3.step.4": "आवंटित करने के लिए «W7 · 12» जैसा बिस्तर चुनें (जहाँ लागू हो वहाँ side room या outlier चिह्नित), या अनुरोध वापस लेने के लिए Cancel चुनें।",
    "tour.s4.title": "EDD कैलेंडर पर डिस्चार्ज की योजना बनाएँ",
    "tour.s4.summary": "हर भरे बिस्तर की अनुमानित डिस्चार्ज तिथि का मासिक कैलेंडर, ताकि दिन और सप्ताह के डिस्चार्ज होने से पहले दिख जाएँ।",
    "tour.s4.step.1": "मेनू में «अनुमानित डिस्चार्ज» चुनकर Expected discharges खोलें; पृष्ठ डेटा जुटाने का समय दिखाता है।",
    "tour.s4.step.2": "हर प्रविष्टि एक मरीज़ है जो उसकी EDD तिथि पर रखा गया है, वार्ड, बिस्तर संख्या और नाम के लेबल के साथ, सभी वार्ड एक साथ।",
    "tour.s4.step.3": "कैलेंडर केवल पढ़ने के लिए है: तिथि सुधारने के लिए किसी प्रविष्टि को चुनकर उस मरीज़ का प्रवास पृष्ठ खोलें।",
    "tour.s4.step.4": "स्टे पृष्ठ पर SAFER के अंतर्गत EDD भरें और CCD met पर निशान लगाएँ, फिर Save चुनें; तिथि के बिना बेड कार्ड तब तक «EDD missing» दिखाता है।",
    "tour.s5.title": "SAFER से डिस्चार्ज तक प्रवास संभालें",
    "tour.s5.summary": "हर प्रवास का एक पृष्ठ वार्ड टीम की सभी कार्रवाइयाँ रखता है: दैनिक SAFER दिनचर्या, संक्रमण फ़्लैग, स्थानांतरण और डिस्चार्ज।",
    "tour.s5.step.1": "व्हाइटबोर्ड से मरीज़ का नाम चुनें (या «खोजें» उपयोग करें) ताकि उसका प्रवास खुले; ऊपर स्थिति, प्रवास अवधि, EDD, DTOC और Red2Green दिन इतिहास दिखता है।",
    "tour.s5.step.2": "SAFER में Mark senior review का उपयोग करें, और Today's day में दिन को हरा या लाल दर्ज करें (लाल होने पर कारण चुनें); सावधानी और जीव के साथ Infection flag जोड़ें।",
    "tour.s5.step.3": "मरीज़ को हटाने के लिए Transfer का उपयोग करें: गंतव्य बिस्तर और clinical, capacity, isolation या step_up जैसा कारण दें; हर स्थानांतरण Moves में सूचीबद्ध होता है।",
    "tour.s5.step.4": "Discharge में पाथवे (P0 से P3) चुनकर Mark discharge-ready चुनें, फिर गंतव्य चुनें और प्रवास पूरा करने के लिए Discharge चुनें।",
    "tour.s6.title": "मरीज़ खोजें और ऑडिट ट्रेल देखें",
    "tour.s6.summary": "पता करें कि मरीज़ अभी कहाँ है, यह जानते हुए कि खोज स्वयं दर्ज होती है, फिर देखें किसने क्या किया।",
    "tour.s6.step.1": "मेनू में «खोजें» चुनें, मरीज़ को person:<uuid> के रूप में दर्ज करें और Locate चुनें।",
    "tour.s6.step.2": "परिणाम में मरीज़ की साइट, वार्ड, बे और बिस्तर दिखते हैं, या बिस्तर में न होने पर घर की स्थिति का नोट; यदि कोई प्रवास न हो तो बताया जाता है। प्रवास पृष्ठ पर जाने के लिए Open stay चुनें।",
    "tour.s6.step.3": "मेनू में «ऑडिट» चुनें; डिफ़ॉल्ट रूप से यह हाल की प्रविष्टियाँ When, Entity, Action, Actor और Detail के साथ दिखाता है।",
    "tour.s6.step.4": "शिफ़्ट हैंडओवर के लिए «handover» चिह्नित वार्ड चुनें, प्रारंभ समय तय करें (डिफ़ॉल्ट 12 घंटे पहले) और Show चुनें।",
    "signin.sso": "SSO से साइन इन करें",
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
    "nav.tour": "导览",
    "splash.hero.tour": "开始导览",
    "tour.head": "开始导览",
    "tour.toc": "本页内容",
    "tour.open": "打开此页面",
    "tour.top": "返回顶部",
    "tour.start.title": "开始之前",
    "tour.start.summary": "处理真实数据需要账号。登录不到一分钟，也不需要密码。",
    "tour.start.step.1": "点击右上角的“登录”，输入你的邮箱地址。",
    "tour.start.step.2": "打开我们发到邮箱的魔法链接。它只能使用一次，且很快过期。",
    "tour.start.step.3": "你会以已登录状态回到应用，无需记忆或重置任何内容。",
    "tour.start.step.4": "使用“登录”旁边的按钮切换主题、语言和文字大小，或分享此页面。",
    "tour.intro": "患者流转导览：每个页面的作用和使用步骤，从查看病区白板到为患者办理出院并查阅审计记录。",
    "tour.s1.title": "查看病区实时床位板",
    "tour.s1.summary": "病区的每张床位显示为一张卡片，按病房间分组，展示患者、预计出院日期和提醒，并自动刷新。",
    "tour.s1.step.1": "在菜单中选择“病区”查看病区列表，可按代码、病区、类型和专科筛选，然后选中某个病区的行打开它的白板。",
    "tour.s1.step.2": "查看每张床位卡片：床位状态、患者姓名，以及 EDD、CCD met、出院路径、DTOC、Ready、红/绿日和感染防护等标签。",
    "tour.s1.step.3": "在等待清洁的空床上先点 Start clean，再点 Clean done（需要深度清洁时点 Deep clean done），使床位重新投入使用。",
    "tour.s1.step.4": "查看白板上方的“as of”时间；用于墙面屏时打开该病区的自助显示视图，并在地址后加 ?masked=1 以隐藏患者姓名。",
    "tour.s2.title": "一览全院情况",
    "tour.s2.summary": "一个页面汇总每个病区的容量数据，方便床位管理员了解床位在哪里、压力在哪里增加。",
    "tour.s2.step.1": "在菜单中选择“概览”，打开全院概览页面。",
    "tour.s2.step.2": "查看顶部的卡片：Beds available now、Predicted by midnight、DTOC，以及按紧急、加急和常规划分的未处理床位申请。",
    "tour.s2.step.3": "浏览病区表：Occ %、Avail、Resv、Clean、Closed、EDD today、EDD overdue、Ready、DTOC，以及住院超过 7 天和 21 天的人数。",
    "tour.s2.step.4": "查看底部的“as of”时间，了解数据有多新，并从压力较大的病区进入它的白板。",
    "tour.s3.title": "排队并分配床位申请",
    "tour.s3.summary": "登记每一位等待床位的患者，按优先级排序，并将其安排到符合条件的床位。",
    "tour.s3.step.1": "选择“床位申请”，在 New request 中以 person:<uuid> 形式输入患者，然后选择来源（ed、elective、ward_transfer、external 或 virtual_step_up）和优先级（emergency、urgent 或 routine）。",
    "tour.s3.step.2": "可选择目标病区、性别要求或隔离，然后点 Queue 将申请加入列表。",
    "tour.s3.step.3": "表格中每个申请显示优先级、来源和符合条件的床位数；没有可用床位的申请会标为“none — escalate”。点 Show beds 列出符合条件的床位。",
    "tour.s3.step.4": "选择如“W7 · 12”的床位进行分配（如适用会标注 side room 或 outlier），或点 Cancel 撤销申请。",
    "tour.s4.title": "在 EDD 日历上规划出院",
    "tour.s4.summary": "月历显示每张在用床位的预计出院日期，让当天和本周的出院在发生前就一目了然。",
    "tour.s4.step.1": "在菜单中选择“预计出院”打开 Expected discharges 页面；页面会显示数据采集时间。",
    "tour.s4.step.2": "每个条目是落在其 EDD 当天的一位患者，标注病区、床号和患者姓名，所有病区合并显示。",
    "tour.s4.step.3": "日历为只读：要修改日期，请选择条目，打开该患者的住院页面。",
    "tour.s4.step.4": "在住院页面的 SAFER 下填写 EDD 并勾选 CCD met，然后点 Save；没有日期的床位卡片会一直显示“EDD missing”。",
    "tour.s5.title": "从 SAFER 到出院管理一次住院",
    "tour.s5.summary": "每次住院一个页面，汇集病区团队所需的全部操作：每日 SAFER 流程、感染标记、转床和出院。",
    "tour.s5.step.1": "在白板上选择患者姓名（或使用“定位”）打开其住院页面；顶部显示状态、住院时长、EDD、DTOC 以及 Red2Green 每日历史。",
    "tour.s5.step.2": "在 SAFER 下使用 Mark senior review，在 Today's day 下把当天记为绿色或红色（红色时选择原因）；并添加带防护类型和病原体的 Infection flag。",
    "tour.s5.step.3": "转床用 Transfer：填写目标床位和原因（如 clinical、capacity、isolation 或 step_up）；每次转床都会列在 Moves 中。",
    "tour.s5.step.4": "在 Discharge 下选择路径（P0 到 P3）并点 Mark discharge-ready，再选择去向并点 Discharge 结束住院。",
    "tour.s6.title": "定位患者并查阅审计记录",
    "tour.s6.summary": "查询患者当前所在位置（查询本身也会被记录），然后查阅谁做了什么的记录。",
    "tour.s6.step.1": "在菜单中选择“定位”，以 person:<uuid> 形式输入患者，然后点 Locate。",
    "tour.s6.step.2": "结果会显示患者所在院区、病区、病房间和床位；若不在床位上则显示其居家位置备注；没有住院记录时会提示。点 Open stay 进入住院页面。",
    "tour.s6.step.3": "在菜单中选择“审计”；默认列出最近条目，包含 When、Entity、Action、Actor 和 Detail。",
    "tour.s6.step.4": "交接班时，选择标有“handover”的病区，设定起始时间（默认为 12 小时前），然后点 Show。",
    "signin.sso": "使用 SSO 登录",
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
