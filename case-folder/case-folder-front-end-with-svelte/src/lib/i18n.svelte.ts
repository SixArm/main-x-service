// Lightweight, dependency-free i18n for the Case Tracking (case-folder)
// SPA. A per-locale strings map plus a reactive `$state` current-locale
// (Svelte 5 runes), exposed via a `t(key)` accessor. Deliberately no i18n
// library: the surface is small and we keep the front-end dependency-light
// (drift across the family front-ends is accepted, see AGENTS.md). This
// module mirrors the design of the family reference module at
// course/course-front-end-with-svelte/src/lib/i18n.svelte.ts.
//
// Supported locales: English (`en-001`, the source of truth), Welsh
// (`cy-001`, for the public-sector Welsh-language duty), Spanish
// (`es-001`), French (`fr-001`), Arabic (`ar-001`, RTL), Hindi (`hi-001`),
// and Mainland Chinese (`zh-cn`). An unknown key/locale falls back to
// `en-001`, then to the key string itself. The chosen locale persists to
// localStorage and drives the UI strings, `<html lang>`, and `<html dir>`
// (right-to-left for `ar-001`).
//
// The i18n store is the SINGLE SOURCE OF TRUTH for the locale: it owns
// persistence and is reflected onto `<html lang>` / `<html dir>` by the
// root layout. The Lily PickerBar's LocalePicker calls `i18n.set` on
// change (its own `applyDir={false}`, since this store already reflects
// `lang`/`dir` onto `<html>` — see `+layout.svelte`).

import { browser } from '$app/environment';

/**
 * Locales for which the UI is translated. To add one, extend this tuple
 * AND add a matching entry to {@link LOCALE_LABELS} and `STRINGS`.
 */
export const LOCALES = [
    'ar-001',
    'cy-001',
    'en-001',
    'es-001',
    'fr-001',
    'hi-001',
    'zh-cn',
] as const;

/** A supported locale code (one of {@link LOCALES}). */
export type Locale = (typeof LOCALES)[number];

/** Fallback locale for an unknown key, locale, or missing translation. */
export const DEFAULT_LOCALE: Locale = 'en-001';

/**
 * Human-readable name for the locale switcher, written in that locale.
 * A `-001` code names the language only (no region), so no parentheses.
 */
export const LOCALE_LABELS: Record<Locale, string> = {
    'ar-001': 'العربية',
    'cy-001': 'Cymraeg',
    'en-001': 'English',
    'es-001': 'Español',
    'fr-001': 'Français',
    'hi-001': 'हिन्दी',
    'zh-cn': '中文 - 中国',
};

/**
 * Right-to-left locales. The layout mirrors `<html dir>` to `rtl` for
 * these (and `ltr` for everyone else) via {@link isRtl}.
 */
export const RTL_LOCALES = ['ar-001'] as const satisfies readonly Locale[];

/**
 * Whether `locale` is written right-to-left. Used by the layout to set
 * `<html dir>`; tolerant of region subtags via {@link normaliseLocale}.
 *
 * @param locale - A locale code (`ar-001`, or a variant like `ar-EG`).
 * @returns `true` for `ar-001`, otherwise `false`.
 */
export function isRtl(locale: string): boolean {
    const resolved = normaliseLocale(locale);
    return (
        resolved !== null &&
        (RTL_LOCALES as readonly string[]).includes(resolved)
    );
}

/**
 * localStorage key under which the chosen UI locale is persisted. Exported
 * so a future locale switcher can share it — the i18n store is the single
 * source of truth for the chosen locale, and any such control would persist
 * to the same key.
 */
export const LOCALE_KEY = 'mxi.case-folder.locale';

// Every translatable UI string, keyed by a stable dotted key. `en-001` is the
// source of truth; every other locale must cover the same key set so a
// missing translation is a type error (the `StringKey` union below).
const STRINGS = {
    'ar-001': {
        'brand.name': 'تتبع الحالات',
        'brand.tagline': 'السجلات الورقية لهيئة الخدمات الصحية الوطنية',
        'chrome.language': 'اللغة',
        'chrome.theme': 'السمة',
        'nav.share': 'مشاركة',
        'nav.text_size': 'حجم النص',
        'share.copy_link': 'نسخ الرابط',
        'share.copied': 'تم نسخ الرابط',
        'share.copy_failed': 'تعذر النسخ — انسخه من شريط العنوان',
        'theme.default': 'افتراضي',
        'theme.highContrast': 'تباين عالٍ',
        'nav.toggle': 'تبديل التنقل',
        'auth.signedInAs': 'تم تسجيل الدخول باسم',
        'auth.signOut': 'تسجيل الخروج',
        'nav.dashboard': 'لوحة المعلومات',
        'nav.patients': 'المرضى',
        'nav.folders': 'المجلدات',
        'nav.volumes': 'المجلّدات',
        'nav.workers': 'العاملون',
        'nav.buildings': 'المباني',
        'nav.cabinets': 'الخزائن',
        'nav.move': 'نقل مجلد',
        'nav.scan': 'مسح',
        'nav.history': 'سجل النقل',
        'nav.alerts': 'التنبيهات',
        'nav.reports': 'التقارير',
        'layout.skipToContent': 'تخطٍ إلى المحتوى الرئيسي',
        'layout.siteHeader': 'رأس الموقع',
        'layout.siteFooter': 'تذييل الموقع',
        'layout.primaryNavigation': 'التنقل الأساسي',
        'footer.text':
            'تتبع الحالات — مبني باستخدام نظام تصميم Lily (سمة NHS) و SVAR Svelte. بيانات تجريبية فقط؛ ليس سجلاً طبياً منظماً.',
        'common.backToDashboard': 'العودة إلى لوحة المعلومات',
        'common.cancel': 'إلغاء',
        'common.move': 'نقل',
        'common.view': 'عرض',
        'common.remove': 'إزالة',
        'common.action': 'إجراء',
        'common.status': 'الحالة',
        'common.patient': 'المريض',
        'common.folder': 'المجلد',
        'common.cabinet': 'الخزانة',
        'common.title': 'العنوان',
        'common.name': 'الاسم',
        'common.role': 'الدور',
        'common.reason': 'السبب',
        'common.movedBy': 'نُقل بواسطة',
        'common.lastMoved': 'آخر نقل',
        'common.nhsNumber': 'رقم NHS',
        'common.dateOfBirth': 'تاريخ الميلاد',
        'common.description': 'الوصف',
        'common.notes': 'ملاحظات',
        'common.entered': 'دخل',
        'common.left': 'غادر',
        'common.when': 'متى',
        'common.from': 'من',
        'common.to': 'إلى',
        'common.source': 'المصدر',
        'common.volume': 'المجلّد',
        'common.stillHere': 'ما زال هنا',
        'common.noMovesYet': 'لم يتم تسجيل أي عمليات نقل بعد.',
        'common.inTransitPorter': 'قيد النقل (يحمله الحامل)',
        'common.selectCabinetOption': '— اختر خزانة —',
        'common.inTransitOption': '— قيد النقل —',
        'status.inCabinet': 'في-الخزانة',
        'status.inTransit': 'قيد-النقل',
        'badge.located': 'محدد الموقع',
        'badge.porterInMotion': 'الحامل في حركة',
        'dashboard.welcomePrefix': 'مرحباً.',
        'dashboard.inTransit.one':
            'لديك {n} مجلد قيد النقل حالياً. استخدم صفحة',
        'dashboard.inTransit.other':
            'لديك {n} مجلدات قيد النقل حالياً. استخدم صفحة',
        'dashboard.moveFolderPage': 'نقل مجلد',
        'dashboard.pageToRecord': 'لتسجيل موضع.',
        'dashboard.folderSummary': 'ملخص المجلدات',
        'dashboard.patients': 'المرضى',
        'dashboard.foldersTracked': '{n} مجلدات متتبعة',
        'dashboard.inCabinet': 'في الخزانة',
        'dashboard.inTransitCard': 'قيد النقل',
        'dashboard.buildings': 'المباني',
        'dashboard.roomsCabinets': '{rooms} غرف · {cabinets} خزائن',
        'dashboard.moves24h': 'عمليات النقل (24 ساعة)',
        'dashboard.auditedPlacements': 'مواضع المجلدات المدققة',
        'dashboard.folderRegister': 'سجل المجلدات',
        'dashboard.viewAll': 'عرض الكل',
        'dashboard.addFolder': 'إضافة مجلد',
        'dashboard.recentMoves': 'عمليات النقل الأخيرة',
        'dashboard.seeFullHistory': 'عرض سجل التدقيق الكامل ←',
        'dashboard.cabinetUtilisation': 'استخدام الخزائن',
        'error.backToDashboard': 'العودة إلى لوحة المعلومات',
        'error.heading': 'خطأ في واجهة برمجة تطبيقات تتبع الحالات',
        'error.unknown': 'خطأ غير معروف',
        'error.apiHint':
            'يتواصل التطبيق مع واجهة Loco JSON عبر /api (موجَّهة إلى خادم Loco في التطوير). تأكد من تشغيل تلك الواجهة. راجع README.md § "Quick start".',
        'login.title': 'تسجيل الدخول',
        'login.intro':
            'أدخل عنوان بريدك الإلكتروني الخاص بالعمل. إذا تم التعرف عليه، سنرسل لك رابط تسجيل دخول لمرة واحدة. لا حاجة لكلمة مرور.',
        'login.sendError': 'تعذر إرسال رابط تسجيل الدخول',
        'login.enterEmail': 'أدخل عنوان بريدك الإلكتروني.',
        'login.checkEmail': 'تحقق من بريدك الإلكتروني',
        'login.sentBody':
            'يطابق حساباً معروفاً، فإن رابط تسجيل الدخول في طريقه إليك. ينتهي الرابط خلال 10 دقائق.',
        'login.sentPrefix': 'إذا كان',
        'login.devShortcut': 'اختصار التطوير:',
        'login.openLink': 'افتح رابط تسجيل الدخول الخاص بك',
        'login.formLabel': 'تسجيل الدخول',
        'login.emailLabel': 'عنوان البريد الإلكتروني',
        'login.submit': 'أرسل لي رابط تسجيل الدخول',
        'callback.title': 'جارٍ تسجيل دخولك',
        'callback.error': 'تعذر استخدام رابط تسجيل الدخول',
        'callback.backToSignIn': 'العودة إلى تسجيل الدخول',
        'callback.completing': 'لحظة — جارٍ إكمال تسجيل دخولك…',
        'scan.heading': 'مسح مجلد',
        'scan.intro':
            'امسح رمزاً شريطياً أو اكتب رقم NHS (أو معرّف مجلد) للانتقال مباشرة إلى مجلد وتسجيل نقله — المسار السريع Scan4Safety. لا حاجة لماسح ضوئي؛ يكتب الماسح الموصول بلوحة المفاتيح في المربع أدناه.',
        'scan.failed': 'فشل المسح',
        'scan.formLabel': 'مسح',
        'scan.fieldLabel': 'مسح أو بحث',
        'scan.fieldDescription': 'رقم NHS (مثال 943 476 5919) أو معرّف مجلد.',
        'scan.placeholder': 'امسح أو اكتب…',
        'scan.matches': 'التطابقات ({n})',
        'scan.moveThisFolder': 'نقل هذا المجلد',
        'scan.noFolderFound': 'لم يتم العثور على مجلد لـ ‏“{term}”.',
        'move.heading': 'نقل مجلد',
        'move.intro':
            'أدخل رقم NHS الخاص بالمريض، واختر المجلد الذي تنقله، ثم اختر الخزانة الوجهة (أو ضع علامة عليه كقيد النقل).',
        'move.recorded': 'تم تسجيل النقل',
        'move.formLabel': 'نقل مجلد',
        'move.patientNhs': 'رقم NHS للمريض',
        'move.invalidNhs': 'أدخل رقم NHS صالحاً من 10 أرقام.',
        'move.folder': 'المجلد',
        'move.selectFolderError': 'اختر أي مجلد لنقله.',
        'move.pickFolderDescription': 'اختر أيّ مجلدات هذا المريض لنقلها.',
        'move.enterNhsDescription': 'أدخل رقم NHS لعرض المجلدات.',
        'move.selectFolderOption': '— اختر مجلداً —',
        'move.destination': 'الوجهة',
        'move.workerLabel': 'العامل (من خدمة العاملين الرئيسية)',
        'move.workerDescription':
            'اختر عاملاً مسجلاً، أو اتركه فارغاً لاستخدام حقل النص الحر أدناه.',
        'move.freeTextOnly': '— نص حر فقط —',
        'move.movedByLabel': 'نُقل بواسطة (نص حر)',
        'move.movedByDescription': 'يُستخدم عند عدم اختيار عامل.',
        'move.movedByPlaceholder': 'مثال أليس (حامل)',
        'move.reasonPlaceholder': 'مثال موعد العيادة الخارجية',
        'move.recordMove': 'تسجيل النقل',
        'move.patientFolders': 'مجلدات المريض',
        'move.enterValidNhs': 'أدخل رقم NHS صالحاً لعرض مجلدات هذا المريض.',
        'move.folderNotFound': 'لم يتم العثور على المجلد.',
        'move.recordedSummary':
            'تم تسجيل نقل {patient} — {folder} من {from} إلى {to}.',
        'folders.register': 'سجل المجلدات',
        'folders.searchPlaceholder':
            'البحث برقم NHS أو المريض أو عنوان المجلد أو الخزانة',
        'folders.searchLabel': 'البحث في المجلدات',
        'folders.addFolder': 'إضافة مجلد',
        'folders.tableLabel': 'المجلدات',
        'folders.tableCaption':
            'جميع مجلدات ملاحظات الحالات الورقية المتتبعة بواسطة النظام',
        'folders.colNhsNumber': 'رقم NHS',
        'folders.colPatient': 'المريض',
        'folders.colFolder': 'المجلد',
        'folders.colCabinet': 'الخزانة',
        'folders.colStatus': 'الحالة',
        'folders.colLastMoved': 'آخر نقل',
        'folders.colAction': 'إجراء',
        'folders.noMatch': 'لا توجد مجلدات تطابق',
        'folderNew.backToFolders': 'العودة إلى المجلدات',
        'folderNew.heading': 'إضافة مجلد جديد',
        'folderNew.intro':
            'ينتمي المجلد إلى مريض واحد. إذا لم يكن المريض مسجلاً بعد لدى خدمة المرضى الرئيسية، فسننشئه؛ وإلا فسيُرفق المجلد الجديد بسجل المريض الحالي.',
        'folderNew.cannotSave': 'تعذر حفظ المجلد',
        'folderNew.invalidNhs':
            'أدخل رقم NHS صالحاً من 10 أرقام (فشل فحص Modulus 11).',
        'folderNew.titleRequired': 'عنوان المجلد مطلوب.',
        'folderNew.formLabel': 'إضافة مجلد',
        'folderNew.nhsDescription': '10 أرقام، بالتنسيق XXX XXX XXXX.',
        'folderNew.titleLabel': 'عنوان المجلد',
        'folderNew.titleDescription': 'مثال المجلّد 1، أمراض القلب 2023',
        'folderNew.patientName': 'اسم المريض',
        'folderNew.patientNameDescription': 'مطلوب فقط لمريض جديد.',
        'folderNew.dobDescription': 'مطلوب فقط لمريض جديد.',
        'folderNew.initialCabinet': 'الخزانة الأولية',
        'folderNew.initialCabinetDescription':
            'اتركه فارغاً إذا كان المجلد قيد النقل.',
        'folderNew.saveFolder': 'حفظ المجلد',
        'folderDetail.backToFolders': 'العودة إلى المجلدات',
        'folderDetail.patientPrefix': 'المريض:',
        'folderDetail.detailsLabel': 'تفاصيل المجلد',
        'folderDetail.folderTitle': 'عنوان المجلد',
        'folderDetail.currentCabinet': 'الخزانة الحالية',
        'folderDetail.moveThisFolder': 'نقل هذا المجلد',
        'folderDetail.moveHistory': 'سجل النقل',
        'patients.heading': 'المرضى',
        'patients.searchPlaceholder': 'البحث برقم NHS أو الاسم',
        'patients.searchLabel': 'البحث عن المرضى',
        'patients.tableLabel': 'المرضى',
        'patients.tableCaption':
            'جميع المرضى الذين لديهم مجلد مسجل واحد أو أكثر',
        'patients.colFolders': 'المجلدات',
        'patients.noMatch': 'لا يوجد مرضى يطابقون',
        'patientDetail.backToPatients': 'العودة إلى المرضى',
        'patientDetail.notFoundHeading':
            'لم يتم العثور على المريض في خدمة المرضى الرئيسية',
        'patientDetail.notFoundBody':
            'لا يوجد سجل مريض لرقم NHS {nhs}. تتم إعادة بناء المجلدات أدناه من لقطات محلية كُتبت عند إنشاء كل مجلد.',
        'patientDetail.sourcePrefix': 'المصدر:',
        'patientDetail.recordActions': 'إجراءات سجل المريض',
        'patientDetail.nhsNumberHeading': 'رقم NHS {nhs}',
        'patientDetail.foldersForPatient': 'مجلدات هذا المريض ({n})',
        'patientDetail.patientFoldersTable': 'مجلدات المريض',
        'patientDetail.colVolume': 'المجلّد',
        'patientDetail.noFoldersYet': 'لا توجد مجلدات بعد.',
        'patientDetail.addFolderForPatient': 'إضافة مجلد لهذا المريض',
        'patientDetail.moveHistoryForPatient': 'سجل النقل لهذا المريض',
        'patientDetail.demoUnavailable':
            '‏“{action}” غير متاح في هذا العرض التجريبي.',
        'buildings.heading': 'المباني',
        'buildings.addBuilding': 'إضافة مبنى',
        'buildings.tableLabel': 'المباني',
        'buildings.tableCaption': 'المواقع المادية التي تضم غرف السجلات',
        'buildings.colRooms': 'الغرف',
        'buildings.noBuildings': 'لا توجد مبانٍ بعد.',
        'buildingNew.backToBuildings': 'العودة إلى المباني',
        'buildingNew.heading': 'إضافة مبنى',
        'buildingNew.intro':
            'يمكن أن يحتوي المبنى الواحد على غرف كثيرة؛ ويمكن لكل غرفة أن تضم خزائن كثيرة.',
        'buildingNew.cannotSave': 'تعذر حفظ المبنى',
        'buildingNew.nameRequired': 'اسم المبنى مطلوب.',
        'buildingNew.formLabel': 'إضافة مبنى',
        'buildingNew.nameLabel': 'اسم المبنى',
        'buildingNew.saveBuilding': 'حفظ المبنى',
        'buildingDetail.backToBuildings': 'العودة إلى المباني',
        'buildingDetail.rooms': 'الغرف ({n})',
        'buildingDetail.roomsTable': 'الغرف',
        'buildingDetail.colCabinets': 'الخزائن',
        'buildingDetail.noRooms': 'لا توجد غرف بعد.',
        'buildingDetail.addRoom': 'إضافة غرفة',
        'buildingDetail.addRoomLabel': 'إضافة غرفة',
        'buildingDetail.roomNameRequired': 'اسم الغرفة مطلوب.',
        'buildingDetail.roomName': 'اسم الغرفة',
        'buildingDetail.saveRoom': 'حفظ الغرفة',
        'buildingDetail.presenceHistory': 'سجل وجود المجلدات',
        'buildingDetail.presenceIntro':
            'المجلدات التي كانت في أي خزانة في هذا المبنى، الأحدث أولاً.',
        'buildingDetail.presenceTable': 'سجل وجود مجلدات المبنى',
        'buildingDetail.presenceCaption': 'مجمّع عبر خزائن هذا المبنى',
        'buildingDetail.noPresence':
            'لم يُسجَّل وجود أي مجلد في هذا المبنى بعد.',
        'cabinets.heading': 'خزائن الملفات',
        'cabinets.addCabinet': 'إضافة خزانة',
        'cabinets.tableLabel': 'الخزائن',
        'cabinets.tableCaption':
            'خزائن الملفات المادية، ومبناها/غرفتها، وإشغالها',
        'cabinets.colLabel': 'التسمية',
        'cabinets.colBuilding': 'المبنى',
        'cabinets.colRoom': 'الغرفة',
        'cabinets.colCapacity': 'السعة',
        'cabinets.colFolders': 'المجلدات',
        'cabinets.colUtilisation': 'الاستخدام',
        'cabinetNew.backToCabinets': 'العودة إلى الخزائن',
        'cabinetNew.heading': 'إضافة خزانة ملفات',
        'cabinetNew.intro': 'تقع الخزانة داخل غرفة (تقع داخل مبنى).',
        'cabinetNew.noRoomsHeading': 'لا توجد غرف بعد',
        'cabinetNew.noRoomsBody': 'أولاً، ثم أضف غرفة من صفحة المبنى.',
        'cabinetNew.noRoomsBuilding': 'مبنى',
        'cabinetNew.noRoomsCreate': 'أنشئ',
        'cabinetNew.cannotSave': 'تعذر حفظ الخزانة',
        'cabinetNew.labelRequired': 'تسمية الخزانة مطلوبة.',
        'cabinetNew.roomRequired': 'اختر غرفة.',
        'cabinetNew.formLabel': 'إضافة خزانة',
        'cabinetNew.labelLabel': 'تسمية الخزانة',
        'cabinetNew.labelPlaceholder': 'مثال الخزانة D3',
        'cabinetNew.roomLabel': 'الغرفة',
        'cabinetNew.selectRoomOption': '— اختر غرفة —',
        'cabinetNew.capacityLabel': 'السعة',
        'cabinetNew.capacityDescription':
            'العدد التقريبي للمجلدات التي تتسع لها الخزانة. اتركه فارغاً إذا كان غير معروف.',
        'cabinetNew.saveCabinet': 'حفظ الخزانة',
        'cabinetDetail.backToCabinets': 'العودة إلى الخزائن',
        'cabinetDetail.currentFolders':
            'المجلدات الموجودة حالياً في هذه الخزانة ({n})',
        'cabinetDetail.currentFoldersTable': 'المجلدات الحالية',
        'cabinetDetail.empty': 'هذه الخزانة فارغة حالياً.',
        'cabinetDetail.presenceHistory': 'سجل وجود المجلدات',
        'cabinetDetail.presenceIntro':
            'أي المجلدات كانت في هذه الخزانة، ومتى. الأحدث أولاً.',
        'cabinetDetail.presenceTable': 'سجل وجود المجلدات',
        'cabinetDetail.presenceCaption': 'مستمد من سجل تدقيق النقل',
        'cabinetDetail.colReasonLeft': 'سبب المغادرة',
        'cabinetDetail.noPresence': 'لم يُسجَّل أي مجلد في هذه الخزانة بعد.',
        'roomDetail.backToBuildings': 'العودة إلى المباني',
        'roomDetail.presenceHistory': 'سجل وجود المجلدات',
        'roomDetail.presenceIntro':
            'المجلدات التي كانت في أي خزانة في هذه الغرفة، الأحدث أولاً.',
        'roomDetail.presenceTable': 'سجل وجود مجلدات الغرفة',
        'roomDetail.presenceCaption': 'مجمّع عبر خزائن هذه الغرفة',
        'roomDetail.noPresence': 'لم يُسجَّل وجود أي مجلد في هذه الغرفة بعد.',
        'volumes.heading': 'المجلّدات',
        'volumes.printLabels': 'طباعة التسميات',
        'volumes.newVolume': 'مجلّد جديد',
        'volumes.intro':
            'المجلّد عبارة عن حزمة قابلة للنقل من مجلدات مريض واحد. انقل المجلّد وستنتقل معه كل المجلدات بداخله.',
        'volumes.tableLabel': 'المجلّدات',
        'volumes.tableCaption': 'حزم من المجلدات، كل منها ينتمي إلى مريض واحد',
        'volumes.colFolders': 'المجلدات',
        'volumes.colLocation': 'الموقع',
        'volumes.noVolumes':
            'لا توجد مجلّدات بعد. أنشئ واحداً لتجميع مجلدات مريض.',
        'volumes.noLabelsSelected': 'لم يتم اختيار أي تسميات.',
        'volumes.queued.one':
            'تم وضع {n} تسمية × {copies} {copy} في قائمة انتظار الطباعة.',
        'volumes.queued.other':
            'تم وضع {n} تسميات × {copies} {copy} في قائمة انتظار الطباعة.',
        'volumes.copy': 'نسخة',
        'volumes.copies': 'نسخ',
        'volumeNew.backToVolumes': 'العودة إلى المجلّدات',
        'volumeNew.heading': 'مجلّد جديد',
        'volumeNew.intro':
            'يجمّع المجلّد مجلدات مريض واحد. يجب أن يكون المريض مسجلاً مسبقاً (أنشئ له مجلداً أولاً). يمكنك إضافة مجلدات إلى المجلّد بمجرد وجوده.',
        'volumeNew.cannotCreate': 'تعذر إنشاء المجلّد',
        'volumeNew.invalidNhs':
            'أدخل رقم NHS صالحاً من 10 أرقام (فشل فحص Modulus 11).',
        'volumeNew.titleRequired': 'عنوان المجلّد مطلوب.',
        'volumeNew.formLabel': 'مجلّد جديد',
        'volumeNew.patientNhs': 'رقم NHS للمريض',
        'volumeNew.nhsDescription': '10 أرقام، بالتنسيق XXX XXX XXXX.',
        'volumeNew.titleLabel': 'عنوان المجلّد',
        'volumeNew.titleDescription': 'مثال أليس جونسون — المجلّد 1',
        'volumeNew.initialCabinet': 'الخزانة الأولية',
        'volumeNew.initialCabinetDescription':
            'حيث يوجد المجلّد. اتركه فارغاً إذا كان قيد النقل.',
        'volumeNew.createVolume': 'إنشاء مجلّد',
        'volumeDetail.backToVolumes': 'العودة إلى المجلّدات',
        'volumeDetail.somethingWrong': 'حدث خطأ ما',
        'volumeDetail.foldersIn': 'المجلدات في هذا المجلّد ({n})',
        'volumeDetail.foldersTable': 'مجلدات المجلّد',
        'volumeDetail.noFolders': 'لا توجد مجلدات في هذا المجلّد بعد.',
        'volumeDetail.addFolderLabel': 'إضافة مجلد',
        'volumeDetail.addFolderFor': 'إضافة مجلد لـ {patient}',
        'volumeDetail.chooseFolder': '— اختر مجلداً —',
        'volumeDetail.addToVolume': 'إضافة إلى المجلّد',
        'volumeDetail.renameVolume': 'إعادة تسمية المجلّد',
        'volumeDetail.rename': 'إعادة تسمية',
        'volumeDetail.moveVolume': 'نقل هذا المجلّد',
        'volumeDetail.moveIntro': 'ينقل كل المجلدات في المجلّد معاً.',
        'volumeDetail.moveFormLabel': 'نقل المجلّد',
        'volumeDetail.destinationCabinet': 'الخزانة الوجهة',
        'volumeDetail.moveReasonPlaceholder': 'مثال عيادة خارجية',
        'volumeDetail.moveVolumeButton': 'نقل المجلّد',
        'volumeDetail.moveHistory': 'سجل النقل',
        'workers.heading': 'العاملون',
        'workers.intro':
            'الموظفون الذين ينقلون المجلدات. افتح عاملاً لرؤية المجلدات التي نقلها وكل مجلد ينتمي إلى مرضاه.',
        'workers.tableLabel': 'العاملون',
        'workers.tableCaption': 'العاملون من خدمة العاملين الرئيسية',
        'workers.noWorkers': 'لم يتم العثور على عاملين.',
        'workerDetail.backToWorkers': 'العودة إلى العاملين',
        'workerDetail.foldersMoved': 'المجلدات التي نقلها هذا العامل ({n})',
        'workerDetail.foldersMovedTable': 'المجلدات التي نقلها هذا العامل',
        'workerDetail.noMovedFolders': 'لم ينقل هذا العامل أي مجلدات بعد.',
        'workerDetail.patientsFolders': 'جميع مجلدات مرضاه ({n})',
        'workerDetail.patientsFoldersIntro':
            'كل مجلد ينتمي إلى مريض تعامل معه هذا العامل.',
        'workerDetail.patientsFoldersTable': 'مجلدات مرضى هذا العامل',
        'workerDetail.noPatientFolders': 'لا توجد مجلدات مرضى لعرضها بعد.',
        'workerDetail.movesByWorker': 'عمليات النقل بواسطة هذا العامل',
        'history.backToDashboard': 'العودة إلى لوحة المعلومات',
        'history.heading': 'سجل النقل (سجل التدقيق)',
        'history.filterPlaceholder':
            'التصفية حسب المريض أو رقم NHS أو الخزانة أو الحامل',
        'history.filterLabel': 'تصفية سجل التدقيق',
        'history.tableLabel': 'سجل تدقيق النقل',
        'history.tableCaption': 'كل نقلة مسجلة لمجلد ورقي، الأحدث أولاً',
        'history.noMatch': 'لا توجد عمليات نقل تطابق التصفية.',
        'historyDetail.backToHistory': 'العودة إلى سجل النقل',
        'historyDetail.heading': 'حدث النقل',
        'historyDetail.folderInvolved': 'المجلد المعني',
        'historyDetail.detailsLabel': 'تفاصيل حدث النقل',
        'historyDetail.otherFolders': 'مجلدات أخرى لـ {patient}',
        'historyDetail.noOtherFolders': 'لا توجد مجلدات أخرى لهذا المريض.',
        'reports.backToDashboard': 'العودة إلى لوحة المعلومات',
        'reports.heading': 'التقارير',
        'reports.atAGlance': 'نظرة سريعة',
        'reports.kpiPatients': 'المرضى:',
        'reports.kpiFolders': 'المجلدات:',
        'reports.kpiFoldersDetail':
            '({inCabinet} في الخزانة، {inTransit} قيد النقل)',
        'reports.kpiVolumes': 'المجلّدات:',
        'reports.kpiCabinets': 'الخزائن:',
        'reports.kpiCabinetsDetail': 'في {n} مبانٍ',
        'reports.kpiMoves': 'عمليات النقل — آخر 24 ساعة:',
        'reports.kpiMoves7d': 'آخر 7 أيام:',
        'reports.cabinetUtilisation': 'استخدام الخزائن',
        'reports.colCabinet': 'الخزانة',
        'reports.colFolders': 'المجلدات',
        'reports.colCapacity': 'السعة',
        'reports.colUtilisation': 'الاستخدام',
        'reports.inTransit': 'قيد النقل ({n})',
        'reports.noInTransit': 'لا توجد مجلدات قيد النقل.',
        'reports.activityByWorker': 'النشاط حسب العامل',
        'reports.colWorker': 'العامل',
        'reports.colMoves': 'عمليات النقل',
        'reports.noMovesYet': 'لم يتم تسجيل أي عمليات نقل بعد.',
        'reports.derivedLive':
            'تُشتق التقارير مباشرة من الواجهة — لا يوجد مخزن تقارير منفصل.',
        'alerts.backToDashboard': 'العودة إلى لوحة المعلومات',
        'alerts.heading': 'تنبيهات السياج الجغرافي',
        'alerts.intro':
            'ملاحظات الحالات التي عبرت حدود مبنى. خرق السياج الجغرافي هو أي نقلة تكون فيها خزانتا المصدر والوجهة في مبنيين مختلفين.',
        'alerts.tableLabel': 'تنبيهات السياج الجغرافي',
        'alerts.tableCaption': 'عمليات النقل العابرة للحدود، الأحدث أولاً',
        'alerts.colCrossed': 'عبر',
        'alerts.none':
            'لا توجد خروقات للسياج الجغرافي — بقي كل مجلد داخل مبناه.',
        'grid.nhsNumber': 'رقم NHS',
        'grid.patient': 'المريض',
        'grid.folder': 'المجلد',
        'grid.cabinet': 'الخزانة',
        'grid.status': 'الحالة',
        'grid.lastMoved': 'آخر نقل',
        'addressograph.label': 'بطاقة المريض',
        'addressograph.nhsNo': 'رقم NHS',
        'addressograph.dob': 'ت. الميلاد',
        'addressograph.sex': 'الجنس',
        'addressograph.address': 'العنوان',
        'buttonBar.label': 'الإجراءات',
        'buttonBar.patient': 'المريض',
        'buttonBar.referrals': 'الإحالات',
        'buttonBar.activate': 'تفعيل',
        'buttonBar.caseNotes': 'ملاحظات الحالة',
        'buttonBar.pathways': 'المسارات',
        'buttonBar.legalStatus': 'الوضع القانوني',
        'buttonBar.documents': 'المستندات',
        'buttonBar.wrapper': 'الغلاف',
        'buttonBar.audit': 'تدقيق',
        'buttonBar.quickReports': 'تقارير سريعة',
        'labels.title': 'التسميات',
        'labels.searchLabel': 'البحث في التسميات',
        'labels.searchPlaceholder': 'أدخل نصاً للبحث...',
        'labels.find': 'بحث',
        'labels.clear': 'مسح',
        'labels.volumeTitles': 'عناوين المجلّدات',
        'labels.noMatching': 'لا توجد مجلّدات مطابقة.',
        'labels.numberOfCopies': 'عدد النسخ:',
        'labels.copiesLabel': 'عدد النسخ',
        'labels.print': 'طباعة',
        'labels.close': 'إغلاق',
        'auth.signin': 'تسجيل الدخول',
        'auth.signout': 'تسجيل الخروج',
        'share.email': 'إرسال الرابط بالبريد',
        'share.linkedin': 'المشاركة على LinkedIn',
        'share.reddit': 'المشاركة على Reddit',
        'share.bluesky': 'المشاركة على Bluesky',
        'share.mastodon': 'المشاركة على Mastodon',
        'splash.hero.secondary': 'اكتشف ما بالداخل',
        'splash.benefits.title': 'لماذا تختارها الفرق',
        'splash.features.title': 'ما يمكنك فعله',
        'splash.trust.title': 'مصمَّم للثقة',
        'splash.trust.1.title': 'تسجيل دخول بلا كلمة مرور',
        'splash.trust.1.body':
            'رابط سحري يصل إلى بريدك الإلكتروني: لا كلمة مرور لتُسرَّب أو يُعاد استخدامها.',
        'splash.trust.2.title': 'أذونات قائمة على السمات',
        'splash.trust.2.body':
            'قواعد دقيقة تحدد من يجوز له القراءة أو الكتابة أو الدمج أو الحذف.',
        'splash.trust.3.title': 'سجل تدقيق يكشف العبث',
        'splash.trust.3.body': 'سجل للإضافة فقط يوثّق كل تغيير ومن أجراه.',
        'splash.trust.4.title': 'ضوابط الخصوصية',
        'splash.trust.4.body':
            'تُخفى التفاصيل الحساسة ما لم يكن من حقك الاطلاع عليها.',
        'splash.trust.5.title': 'معايير مفتوحة',
        'splash.trust.5.body':
            'REST مع OpenAPI، وHL7 FHIR حيثما تحتاجه الأنظمة الصحية.',
        'splash.trust.6.title': 'يتحدث لغتك',
        'splash.trust.6.body':
            'العربية والصينية والإنجليزية والفرنسية والهندية والإسبانية والويلزية.',
        'splash.cta.title': 'هل أنت مستعد للبدء؟',
        'splash.cta.body':
            'سجّل الدخول برابط سحري يصلك على بريدك الإلكتروني. لا حاجة لكلمة مرور.',
        'splash.hero.title': 'اعرف مكان كل مجلد',
        'splash.hero.subtitle':
            'تتبّع مجلدات ملاحظات الحالات الورقية برقم هيئة الخدمات الصحية الوطنية عبر المباني والغرف والخزائن، مع سجل تدقيق كامل لكل عملية نقل.',
        'splash.benefits.1.title': 'اعثر على أي مجلد بسرعة',
        'splash.benefits.1.body':
            'ابحث عن مجلد ملاحظات ورقي برقم هيئة الخدمات الصحية الوطنية وشاهد مكانه الآن.',
        'splash.benefits.2.title': 'مجلدات مفقودة أقل',
        'splash.benefits.2.body':
            'يُسجَّل كل نقل، فلا يكون أي مجلد مجرد شيء ضائع في المبنى.',
        'splash.benefits.3.title': 'وقت أقل في البحث',
        'splash.benefits.3.body':
            'يقضي الحمّالون والأطباء وقتهم في الرعاية لا في التفتيش بين الخزائن.',
        'splash.benefits.4.title': 'سجل موثوق',
        'splash.benefits.4.body':
            'يُظهر سجل كامل ومؤرَّخ من نقل كل مجلد ومتى ولماذا.',
        'splash.benefits.5.title': 'هوية مريض آمنة',
        'splash.benefits.5.body':
            'يُتحقَّق من أرقام هيئة الخدمات الصحية الوطنية قبل حفظها، فلا تتحول أخطاء الكتابة إلى سجلات.',
        'splash.benefits.6.title': 'رؤية واضحة للمرافق',
        'splash.benefits.6.body':
            'اطّلع على مدى امتلاء كل خزانة وعدد المجلدات قيد النقل.',
        'splash.features.1.title': 'سجل المجلدات',
        'splash.features.1.body':
            'ابحث في كل المجلدات حسب المريض أو العنوان أو رقم هيئة الخدمات الصحية الوطنية.',
        'splash.features.2.title': 'النقل والمسح',
        'splash.features.2.body':
            'سجّل عملية النقل في ثوانٍ بمسح رقم هيئة الخدمات الصحية الوطنية أو إدخاله.',
        'splash.features.3.title': 'المجلّدات',
        'splash.features.3.body':
            'اجمع مجلدات المريض في مجلّد واحد وانقلها معًا.',
        'splash.features.4.title': 'المباني والغرف والخزائن',
        'splash.features.4.body':
            'صِف التسلسل المادي من كل مبنى نزولًا إلى خزائنه.',
        'splash.features.5.title': 'سجل النقل',
        'splash.features.5.body':
            'ابحث في سجل التدقيق الكامل لكل حركة نقل للمجلدات.',
        'splash.features.6.title': 'التنبيهات والتقارير',
        'splash.features.6.body':
            'راجع التنبيهات بين المباني واستخدام الخزائن والأرقام الرئيسية.',
        'nav.tour': 'جولة',
        'splash.hero.tour': 'ابدأ الجولة',
        'tour.head': 'ابدأ الجولة',
        'tour.toc': 'في هذه الصفحة',
        'tour.open': 'افتح هذه الشاشة',
        'tour.top': 'العودة إلى الأعلى',
        'tour.start.title': 'قبل أن تبدأ',
        'tour.start.summary':
            'تحتاج إلى حساب للعمل مع البيانات الفعلية. يستغرق تسجيل الدخول أقل من دقيقة ولا يتطلب كلمة مرور.',
        'tour.start.step.1':
            'اختر «تسجيل الدخول» في أعلى اليمين وأدخل بريدك الإلكتروني.',
        'tour.start.step.2':
            'افتح الرابط السحري الذي نرسله إلى بريدك. يعمل مرة واحدة وتنتهي صلاحيته سريعًا.',
        'tour.start.step.3':
            'تعود إلى التطبيق وقد سجّلت الدخول، دون شيء لتتذكره أو تعيده.',
        'tour.start.step.4':
            'استخدم الأزرار بجوار «تسجيل الدخول» لتغيير السمة واللغة وحجم النص أو لمشاركة الصفحة.',
        'tour.intro':
            'جولة إرشادية في تتبع الحالات: ما تفعله كل شاشة وخطوات استخدامها، من إضافة مجلد إلى مراجعة سجل تدقيق النقل.',
        'tour.s1.title': 'إضافة مجلد إلى السجل',
        'tour.s1.summary':
            'ينتمي كل مجلد ورقي لملاحظات الحالة إلى مريض واحد ويُتتبَّع منذ لحظة إضافته.',
        'tour.s1.step.1':
            'افتح المجلدات واختر إضافة مجلد، ثم أدخل رقم NHS المكوّن من 10 أرقام للمريض، ويجب أن يجتاز فحص المقياس 11.',
        'tour.s1.step.2':
            'اكتب عنوان المجلد، مثل «المجلّد 1» أو «أمراض القلب 2023».',
        'tour.s1.step.3':
            'إذا لم يكن المريض مسجلاً بعد، فأدخل أيضاً اسم المريض وتاريخ الميلاد؛ أما للمريض المسجل فلا حاجة إليهما.',
        'tour.s1.step.4':
            'اختر الخزانة الأولية، أو اتركها فارغة إن كان المجلد قيد النقل، ثم اختر حفظ المجلد.',
        'tour.s2.title': 'العثور على مجلد',
        'tour.s2.summary':
            'يجيب سجل المجلدات عن سؤال «أين هذا المجلد الآن؟» من مربع بحث واحد.',
        'tour.s2.step.1':
            'افتح المجلدات واكتب رقم NHS أو اسم المريض أو عنوان المجلد أو الخزانة في البحث في المجلدات.',
        'tour.s2.step.2':
            'اقرأ في كل صف الخزانة والحالة (في الخزانة أو قيد النقل) وآخر نقل.',
        'tour.s2.step.3':
            'افتح مجلداً لعرض تفاصيله وسجل النقل الخاص به، أو افتح المريض من المرضى لرؤية جميع مجلداته.',
        'tour.s2.step.4': 'اختر نقل هذا المجلد للانتقال مباشرة إلى تسجيل نقله.',
        'tour.s3.title': 'نقل مجلد',
        'tour.s3.summary': 'يُسجَّل كل وضع للمجلد، فيبقى موقعه محدّثاً دائماً.',
        'tour.s3.step.1':
            'افتح نقل مجلد، وأدخل رقم NHS للمريض، ثم اختر أي مجلدات هذا المريض تنقل.',
        'tour.s3.step.2':
            'اختر الخزانة الوجهة، أو قيد النقل (يحمله الحامل) إن كان في الطريق.',
        'tour.s3.step.3':
            'اختر عاملاً من القائمة، أو اكتب اسماً ضمن نُقل بواسطة، وأضف السبب.',
        'tour.s3.step.4':
            'اختر تسجيل النقل؛ تؤكد الصفحة تم تسجيل النقل وتُضاف العملية إلى سجل التدقيق.',
        'tour.s4.title': 'مسح مجلد',
        'tour.s4.summary':
            'المسار السريع لمكتب السجلات: لا حاجة إلى ماسح ضوئي مخصص، مع أن الماسح الذي يكتب عبر لوحة المفاتيح يعمل أيضاً.',
        'tour.s4.step.1': 'افتح مسح وانقر داخل مربع مسح أو بحث.',
        'tour.s4.step.2': 'امسح رمزاً شريطياً، أو اكتب رقم NHS أو معرّف مجلد.',
        'tour.s4.step.3': 'راجع قائمة التطابقات وافتح مجلداً لعرض تفاصيله.',
        'tour.s4.step.4':
            'اختر نقل هذا المجلد لتسجيل نقله مع تحديد المجلد مسبقاً؛ وإن لم يوجد تطابق، تخبرك الصفحة بأنه لم يُعثر على مجلد.',
        'tour.s5.title': 'تجميع المجلدات في مجلّدات',
        'tour.s5.summary':
            'المجلّد حزمة قابلة للنقل من مجلدات مريض واحد، فتنتقل معاً.',
        'tour.s5.step.1':
            'افتح المجلّدات واختر مجلّد جديد، ثم أدخل رقم NHS للمريض وعنوان المجلّد؛ ويجب أن يكون للمريض مجلد بالفعل.',
        'tour.s5.step.2':
            'افتح المجلّد واستخدم إضافة مجلد لضم مجلدات ذلك المريض إليه.',
        'tour.s5.step.3':
            'استخدم نقل هذا المجلّد لنقل كل مجلد بداخله إلى خزانة وجهة واحدة في خطوة واحدة.',
        'tour.s5.step.4':
            'عد إلى المجلّدات، واختر طباعة التسميات، وحدّد المجلّدات، واضبط عدد النسخ، ثم اختر طباعة لإضافتها إلى قائمة الطباعة.',
        'tour.s6.title': 'مراجعة السجل والتنبيهات والتقارير',
        'tour.s6.summary':
            'تُحفظ كل عملية نقل، فيمكنك معرفة من نقل ماذا ومتى ولماذا.',
        'tour.s6.step.1':
            'افتح سجل النقل واستخدم تصفية سجل التدقيق للتضييق بحسب المريض أو رقم NHS أو الخزانة أو الحامل؛ وتظهر أحدث العمليات أولاً.',
        'tour.s6.step.2':
            'افتح صفاً لعرض حدث النقل كاملاً والمجلد المعني ومجلدات المريض الأخرى.',
        'tour.s6.step.3':
            'افتح التنبيهات لعرض تنبيهات السياج الجغرافي: عمليات نقل تقع خزانتا مصدرها ووجهتها في مبنيين مختلفين.',
        'tour.s6.step.4':
            'افتح التقارير لعرض الأعداد بنظرة سريعة واستخدام الخزائن والمجلدات قيد النقل والنشاط بحسب العامل، وكلها مشتقة مباشرة.',
        'signin.sso': 'تسجيل الدخول عبر SSO',
    },
    'cy-001': {
        'brand.name': 'Olrhain Achosion',
        'brand.tagline': 'Cofnodion papur y GIG',
        'chrome.language': 'Iaith',
        'chrome.theme': 'Thema',
        'nav.share': 'Rhannu',
        'nav.text_size': 'Maint testun',
        'share.copy_link': 'Copïo dolen',
        'share.copied': "Dolen wedi'i chopïo",
        'share.copy_failed': "Methu copïo — copïwch o'r bar cyfeiriad",
        'theme.default': 'Rhagosodedig',
        'theme.highContrast': 'Cyferbyniad uchel',
        'nav.toggle': "Toglo'r llywio",
        'auth.signedInAs': 'Wedi mewngofnodi fel',
        'auth.signOut': 'Allgofnodi',
        'nav.dashboard': 'Dangosfwrdd',
        'nav.patients': 'Cleifion',
        'nav.folders': 'Ffolderi',
        'nav.volumes': 'Cyfrolau',
        'nav.workers': 'Gweithwyr',
        'nav.buildings': 'Adeiladau',
        'nav.cabinets': 'Cabinetau',
        'nav.move': 'Symud ffolder',
        'nav.scan': 'Sganio',
        'nav.history': 'Hanes symud',
        'nav.alerts': 'Rhybuddion',
        'nav.reports': 'Adroddiadau',
        'layout.skipToContent': "Neidio i'r prif gynnwys",
        'layout.siteHeader': 'Pennyn y wefan',
        'layout.siteFooter': 'Troedyn y wefan',
        'layout.primaryNavigation': 'Llywio sylfaenol',
        'footer.text':
            "Olrhain Achosion — wedi'i adeiladu gyda System Ddylunio Lily (thema GIG) a SVAR Svelte. Data demo yn unig; nid cofnod meddygol rheoledig.",
        'common.backToDashboard': "Yn ôl i'r dangosfwrdd",
        'common.cancel': 'Canslo',
        'common.move': 'Symud',
        'common.view': 'Gweld',
        'common.remove': 'Tynnu',
        'common.action': 'Gweithred',
        'common.status': 'Statws',
        'common.patient': 'Claf',
        'common.folder': 'Ffolder',
        'common.cabinet': 'Cabinet',
        'common.title': 'Teitl',
        'common.name': 'Enw',
        'common.role': 'Rôl',
        'common.reason': 'Rheswm',
        'common.movedBy': 'Symudwyd gan',
        'common.lastMoved': 'Symudwyd ddiwethaf',
        'common.nhsNumber': 'Rhif GIG',
        'common.dateOfBirth': 'Dyddiad geni',
        'common.description': 'Disgrifiad',
        'common.notes': 'Nodiadau',
        'common.entered': 'Mynediad',
        'common.left': 'Gadawodd',
        'common.when': 'Pryd',
        'common.from': 'O',
        'common.to': 'I',
        'common.source': 'Ffynhonnell',
        'common.volume': 'Cyfrol',
        'common.stillHere': 'Yma o hyd',
        'common.noMovesYet': "Dim symudiadau wedi'u cofnodi eto.",
        'common.inTransitPorter': 'Ar daith (porthor yn cario)',
        'common.selectCabinetOption': '— Dewis cabinet —',
        'common.inTransitOption': '— Ar daith —',
        'status.inCabinet': 'yn-y-cabinet',
        'status.inTransit': 'ar-daith',
        'badge.located': "Wedi'i leoli",
        'badge.porterInMotion': 'Porthor ar symud',
        'dashboard.welcomePrefix': 'Croeso.',
        'dashboard.inTransit.one':
            'Mae gennych {n} ffolder ar daith ar hyn o bryd. Defnyddiwch y dudalen',
        'dashboard.inTransit.other':
            'Mae gennych {n} ffolder ar daith ar hyn o bryd. Defnyddiwch y dudalen',
        'dashboard.moveFolderPage': 'Symud ffolder',
        'dashboard.pageToRecord': 'i gofnodi lleoliad.',
        'dashboard.folderSummary': 'Crynodeb ffolderi',
        'dashboard.patients': 'Cleifion',
        'dashboard.foldersTracked': '{n} ffolder yn cael eu holrhain',
        'dashboard.inCabinet': 'Yn y cabinet',
        'dashboard.inTransitCard': 'Ar daith',
        'dashboard.buildings': 'Adeiladau',
        'dashboard.roomsCabinets': '{rooms} ystafell · {cabinets} cabinet',
        'dashboard.moves24h': 'Symudiadau (24awr)',
        'dashboard.auditedPlacements': "Lleoliadau ffolderi wedi'u harchwilio",
        'dashboard.folderRegister': 'Cofrestr ffolderi',
        'dashboard.viewAll': 'Gweld pob un',
        'dashboard.addFolder': 'Ychwanegu ffolder',
        'dashboard.recentMoves': 'Symudiadau diweddar',
        'dashboard.seeFullHistory': 'Gweld hanes archwilio llawn →',
        'dashboard.cabinetUtilisation': 'Defnydd cabinetau',
        'error.backToDashboard': "Yn ôl i'r dangosfwrdd",
        'error.heading': 'Gwall API Olrhain Achosion',
        'error.unknown': 'Gwall anhysbys',
        'error.apiHint':
            "Mae'r ap yn siarad â'r API JSON Loco drwy /api (wedi'i ddirprwyo i'r gweinydd Loco wrth ddatblygu). Sicrhewch fod yr API hwnnw'n rhedeg. Gweler README.md § \"Quick start\".",
        'login.title': 'Mewngofnodi',
        'login.intro':
            'Rhowch eich cyfeiriad e-bost gwaith. Os caiff ei adnabod, byddwn yn anfon dolen fewngofnodi un-tro atoch. Dim cyfrinair yn ofynnol.',
        'login.sendError': 'Methu anfon dolen fewngofnodi',
        'login.enterEmail': 'Rhowch eich cyfeiriad e-bost.',
        'login.checkEmail': 'Gwiriwch eich e-bost',
        'login.sentBody':
            "yn cyfateb i gyfrif hysbys, mae dolen fewngofnodi ar ei ffordd. Mae'r ddolen yn dod i ben ymhen 10 munud.",
        'login.sentPrefix': 'Os yw',
        'login.devShortcut': 'Llwybr datblygu:',
        'login.openLink': 'agor eich dolen fewngofnodi',
        'login.formLabel': 'Mewngofnodi',
        'login.emailLabel': 'Cyfeiriad e-bost',
        'login.submit': 'E-bostiwch ddolen fewngofnodi ataf',
        'callback.title': 'Yn eich mewngofnodi',
        'callback.error': "Methwyd defnyddio'r ddolen fewngofnodi",
        'callback.backToSignIn': 'Yn ôl i fewngofnodi',
        'callback.completing': 'Eiliad — yn cwblhau eich mewngofnodi…',
        'scan.heading': 'Sganio ffolder',
        'scan.intro':
            "Sganiwch god bar neu deipiwch Rif GIG (neu id ffolder) i neidio'n syth at ffolder a chofnodi ei symudiad — llwybr cyflym Scan4Safety. Dim angen sganiwr caledwedd; mae sganiwr lletem-bysellfwrdd yn teipio i'r blwch isod.",
        'scan.failed': 'Methodd y sgan',
        'scan.formLabel': 'Sganio',
        'scan.fieldLabel': 'Sganio neu chwilio',
        'scan.fieldDescription': 'Rhif GIG (e.e. 943 476 5919) neu id ffolder.',
        'scan.placeholder': 'Sganio neu deipio…',
        'scan.matches': 'Cydweddiadau ({n})',
        'scan.moveThisFolder': 'Symud y ffolder hwn',
        'scan.noFolderFound': 'Ni chanfuwyd ffolder ar gyfer “{term}”.',
        'move.heading': 'Symud ffolder',
        'move.intro':
            'Rhowch Rif GIG claf, dewiswch y ffolder rydych yn ei symud, yna dewiswch y cabinet cyrchfan (neu nodwch ei fod ar daith).',
        'move.recorded': "Symudiad wedi'i gofnodi",
        'move.formLabel': 'Symud ffolder',
        'move.patientNhs': 'Rhif GIG y claf',
        'move.invalidNhs': 'Rhowch Rif GIG dilys 10 digid.',
        'move.folder': 'Ffolder',
        'move.selectFolderError': "Dewiswch pa ffolder i'w symud.",
        'move.pickFolderDescription':
            "Dewiswch pa rai o ffolderi'r claf hwn i'w symud.",
        'move.enterNhsDescription': 'Rhowch Rif GIG i weld ffolderi.',
        'move.selectFolderOption': '— Dewis ffolder —',
        'move.destination': 'Cyrchfan',
        'move.workerLabel': "Gweithiwr (o'r Prif Wasanaeth Gweithwyr)",
        'move.workerDescription':
            "Dewiswch weithiwr cofrestredig, neu gadewch yn wag i ddefnyddio'r maes testun rhydd isod.",
        'move.freeTextOnly': '— Testun rhydd yn unig —',
        'move.movedByLabel': 'Symudwyd gan (testun rhydd)',
        'move.movedByDescription': 'Defnyddir pan na ddewisir gweithiwr.',
        'move.movedByPlaceholder': 'e.e. Alice (porthor)',
        'move.reasonPlaceholder': 'e.e. Apwyntiad cleifion allanol',
        'move.recordMove': 'Cofnodi symudiad',
        'move.patientFolders': "Ffolderi'r claf",
        'move.enterValidNhs':
            "Rhowch Rif GIG dilys i weld ffolderi'r claf hwn.",
        'move.folderNotFound': 'Ffolder heb ei ganfod.',
        'move.recordedSummary':
            'Wedi cofnodi symudiad {patient} — {folder} o {from} i {to}.',
        'folders.register': 'Cofrestr ffolderi',
        'folders.searchPlaceholder':
            'Chwilio yn ôl Rhif GIG, claf, teitl ffolder, neu gabinet',
        'folders.searchLabel': 'Chwilio ffolderi',
        'folders.addFolder': 'Ychwanegu ffolder',
        'folders.tableLabel': 'Ffolderi',
        'folders.tableCaption':
            'Pob ffolder cofnodion achos papur a olrheinir gan y system',
        'folders.colNhsNumber': 'Rhif GIG',
        'folders.colPatient': 'Claf',
        'folders.colFolder': 'Ffolder',
        'folders.colCabinet': 'Cabinet',
        'folders.colStatus': 'Statws',
        'folders.colLastMoved': 'Symudwyd ddiwethaf',
        'folders.colAction': 'Gweithred',
        'folders.noMatch': "Dim ffolderi'n cyfateb i",
        'folderNew.backToFolders': 'Yn ôl i ffolderi',
        'folderNew.heading': 'Ychwanegu ffolder newydd',
        'folderNew.intro':
            "Mae ffolder yn perthyn i un claf. Os nad yw'r claf wedi'i gofrestru eto gyda'r Prif Wasanaeth Cleifion, byddwn yn ei greu; fel arall caiff y ffolder newydd ei atodi i'r cofnod claf presennol.",
        'folderNew.cannotSave': 'Methu cadw ffolder',
        'folderNew.invalidNhs':
            'Rhowch Rif GIG dilys 10 digid (methodd gwiriad Modulus 11).',
        'folderNew.titleRequired': 'Mae angen teitl ffolder.',
        'folderNew.formLabel': 'Ychwanegu ffolder',
        'folderNew.nhsDescription': '10 digid, ar ffurf XXX XXX XXXX.',
        'folderNew.titleLabel': 'Teitl ffolder',
        'folderNew.titleDescription': 'e.e. Cyfrol 1, Cardioleg 2023',
        'folderNew.patientName': "Enw'r claf",
        'folderNew.patientNameDescription': 'Dim ond ar gyfer claf newydd.',
        'folderNew.dobDescription': 'Dim ond ar gyfer claf newydd.',
        'folderNew.initialCabinet': 'Cabinet cychwynnol',
        'folderNew.initialCabinetDescription':
            "Gadewch yn wag os yw'r ffolder ar daith.",
        'folderNew.saveFolder': 'Cadw ffolder',
        'folderDetail.backToFolders': 'Yn ôl i ffolderi',
        'folderDetail.patientPrefix': 'Claf:',
        'folderDetail.detailsLabel': 'Manylion ffolder',
        'folderDetail.folderTitle': 'Teitl ffolder',
        'folderDetail.currentCabinet': 'Cabinet cyfredol',
        'folderDetail.moveThisFolder': 'Symud y ffolder hwn',
        'folderDetail.moveHistory': 'Hanes symud',
        'patients.heading': 'Cleifion',
        'patients.searchPlaceholder': 'Chwilio yn ôl Rhif GIG neu enw',
        'patients.searchLabel': 'Chwilio cleifion',
        'patients.tableLabel': 'Cleifion',
        'patients.tableCaption':
            'Pob claf ag un neu fwy o ffolderi cofrestredig',
        'patients.colFolders': 'Ffolderi',
        'patients.noMatch': 'Dim cleifion yn cyfateb i',
        'patientDetail.backToPatients': 'Yn ôl i gleifion',
        'patientDetail.notFoundHeading':
            'Claf heb ei ganfod yn y Prif Wasanaeth Cleifion',
        'patientDetail.notFoundBody':
            "Nid oes cofnod claf yn bodoli ar gyfer Rhif GIG {nhs}. Mae'r ffolderi isod wedi'u hailadeiladu o gipluniau lleol a ysgrifennwyd pan grëwyd pob ffolder.",
        'patientDetail.sourcePrefix': 'Ffynhonnell:',
        'patientDetail.recordActions': 'Gweithredoedd cofnod claf',
        'patientDetail.nhsNumberHeading': 'Rhif GIG {nhs}',
        'patientDetail.foldersForPatient': 'Ffolderi ar gyfer y claf hwn ({n})',
        'patientDetail.patientFoldersTable': "Ffolderi'r claf",
        'patientDetail.colVolume': 'Cyfrol',
        'patientDetail.noFoldersYet': 'Dim ffolderi eto.',
        'patientDetail.addFolderForPatient':
            'Ychwanegu ffolder ar gyfer y claf hwn',
        'patientDetail.moveHistoryForPatient':
            'Hanes symud ar gyfer y claf hwn',
        'patientDetail.demoUnavailable':
            'Nid yw “{action}” ar gael yn y demo hwn.',
        'buildings.heading': 'Adeiladau',
        'buildings.addBuilding': 'Ychwanegu adeilad',
        'buildings.tableLabel': 'Adeiladau',
        'buildings.tableCaption':
            "Safleoedd ffisegol sy'n dal ystafelloedd cofnodion",
        'buildings.colRooms': 'Ystafelloedd',
        'buildings.noBuildings': 'Dim adeiladau eto.',
        'buildingNew.backToBuildings': 'Yn ôl i adeiladau',
        'buildingNew.heading': 'Ychwanegu adeilad',
        'buildingNew.intro':
            'Gall un adeilad gael llawer o ystafelloedd; gall pob ystafell ddal llawer o gabinetau.',
        'buildingNew.cannotSave': 'Methu cadw adeilad',
        'buildingNew.nameRequired': 'Mae angen enw adeilad.',
        'buildingNew.formLabel': 'Ychwanegu adeilad',
        'buildingNew.nameLabel': 'Enw adeilad',
        'buildingNew.saveBuilding': 'Cadw adeilad',
        'buildingDetail.backToBuildings': 'Yn ôl i adeiladau',
        'buildingDetail.rooms': 'Ystafelloedd ({n})',
        'buildingDetail.roomsTable': 'Ystafelloedd',
        'buildingDetail.colCabinets': 'Cabinetau',
        'buildingDetail.noRooms': 'Dim ystafelloedd eto.',
        'buildingDetail.addRoom': 'Ychwanegu ystafell',
        'buildingDetail.addRoomLabel': 'Ychwanegu ystafell',
        'buildingDetail.roomNameRequired': 'Mae angen enw ystafell.',
        'buildingDetail.roomName': 'Enw ystafell',
        'buildingDetail.saveRoom': 'Cadw ystafell',
        'buildingDetail.presenceHistory': 'Hanes presenoldeb ffolderi',
        'buildingDetail.presenceIntro':
            'Ffolderi sydd wedi bod mewn unrhyw gabinet yn yr adeilad hwn, y diweddaraf yn gyntaf.',
        'buildingDetail.presenceTable': "Hanes presenoldeb ffolderi'r adeilad",
        'buildingDetail.presenceCaption':
            "Wedi'i gyfuno ar draws cabinetau'r adeilad hwn",
        'buildingDetail.noPresence':
            "Dim presenoldeb ffolderi wedi'i gofnodi yn yr adeilad hwn eto.",
        'cabinets.heading': 'Cabinetau ffeilio',
        'cabinets.addCabinet': 'Ychwanegu cabinet',
        'cabinets.tableLabel': 'Cabinetau',
        'cabinets.tableCaption':
            "Cabinetau ffeilio ffisegol, eu hadeilad/ystafell, a'u meddiannaeth",
        'cabinets.colLabel': 'Label',
        'cabinets.colBuilding': 'Adeilad',
        'cabinets.colRoom': 'Ystafell',
        'cabinets.colCapacity': 'Capasiti',
        'cabinets.colFolders': 'Ffolderi',
        'cabinets.colUtilisation': 'Defnydd',
        'cabinetNew.backToCabinets': 'Yn ôl i gabinetau',
        'cabinetNew.heading': 'Ychwanegu cabinet ffeilio',
        'cabinetNew.intro':
            "Mae cabinet yn byw y tu mewn i ystafell (sy'n byw y tu mewn i adeilad).",
        'cabinetNew.noRoomsHeading': 'Dim ystafelloedd yn bodoli eto',
        'cabinetNew.noRoomsBody':
            'yn gyntaf, yna ychwanegwch ystafell o dudalen yr adeilad.',
        'cabinetNew.noRoomsBuilding': 'adeilad',
        'cabinetNew.noRoomsCreate': 'Crëwch',
        'cabinetNew.cannotSave': 'Methu cadw cabinet',
        'cabinetNew.labelRequired': 'Mae angen label cabinet.',
        'cabinetNew.roomRequired': 'Dewiswch ystafell.',
        'cabinetNew.formLabel': 'Ychwanegu cabinet',
        'cabinetNew.labelLabel': 'Label cabinet',
        'cabinetNew.labelPlaceholder': 'e.e. Cabinet D3',
        'cabinetNew.roomLabel': 'Ystafell',
        'cabinetNew.selectRoomOption': '— Dewis ystafell —',
        'cabinetNew.capacityLabel': 'Capasiti',
        'cabinetNew.capacityDescription':
            'Nifer bras o ffolderi y gall y cabinet eu dal. Gadewch yn wag os yn anhysbys.',
        'cabinetNew.saveCabinet': 'Cadw cabinet',
        'cabinetDetail.backToCabinets': 'Yn ôl i gabinetau',
        'cabinetDetail.currentFolders':
            'Ffolderi sydd yn y cabinet hwn ar hyn o bryd ({n})',
        'cabinetDetail.currentFoldersTable': 'Ffolderi cyfredol',
        'cabinetDetail.empty': "Mae'r cabinet hwn yn wag ar hyn o bryd.",
        'cabinetDetail.presenceHistory': 'Hanes presenoldeb ffolderi',
        'cabinetDetail.presenceIntro':
            'Pa ffolderi sydd wedi bod yn y cabinet hwn, a phryd. Y diweddaraf yn gyntaf.',
        'cabinetDetail.presenceTable': 'Hanes presenoldeb ffolderi',
        'cabinetDetail.presenceCaption': "Deilliwyd o'r log archwilio symud",
        'cabinetDetail.colReasonLeft': 'Rheswm dros adael',
        'cabinetDetail.noPresence':
            "Nid oes ffolder wedi'i gofnodi yn y cabinet hwn eto.",
        'roomDetail.backToBuildings': 'Yn ôl i adeiladau',
        'roomDetail.presenceHistory': 'Hanes presenoldeb ffolderi',
        'roomDetail.presenceIntro':
            'Ffolderi sydd wedi bod mewn unrhyw gabinet yn yr ystafell hon, y diweddaraf yn gyntaf.',
        'roomDetail.presenceTable': "Hanes presenoldeb ffolderi'r ystafell",
        'roomDetail.presenceCaption':
            "Wedi'i gyfuno ar draws cabinetau'r ystafell hon",
        'roomDetail.noPresence':
            "Dim presenoldeb ffolderi wedi'i gofnodi yn yr ystafell hon eto.",
        'volumes.heading': 'Cyfrolau',
        'volumes.printLabels': 'Argraffu labeli',
        'volumes.newVolume': 'Cyfrol newydd',
        'volumes.intro':
            "Mae cyfrol yn fwndel symudol o ffolderi un claf. Symudwch y gyfrol a bydd pob ffolder ynddi'n symud gyda'i gilydd.",
        'volumes.tableLabel': 'Cyfrolau',
        'volumes.tableCaption':
            'Bwndeli o ffolderi, pob un yn perthyn i un claf',
        'volumes.colFolders': 'Ffolderi',
        'volumes.colLocation': 'Lleoliad',
        'volumes.noVolumes':
            'Dim cyfrolau eto. Crëwch un i fwndelu ffolderi claf.',
        'volumes.noLabelsSelected': "Dim labeli wedi'u dewis.",
        'volumes.queued.one':
            "Ciwiwyd {n} label × {copies} {copy} i'w argraffu.",
        'volumes.queued.other':
            "Ciwiwyd {n} label × {copies} {copy} i'w argraffu.",
        'volumes.copy': 'copi',
        'volumes.copies': 'copi',
        'volumeNew.backToVolumes': 'Yn ôl i gyfrolau',
        'volumeNew.heading': 'Cyfrol newydd',
        'volumeNew.intro':
            "Mae cyfrol yn bwndelu ffolderi ar gyfer un claf. Rhaid i'r claf fod wedi'i gofrestru eisoes (crëwch ffolder iddo yn gyntaf). Gallwch ychwanegu ffolderi at y gyfrol unwaith y bydd yn bodoli.",
        'volumeNew.cannotCreate': 'Methu creu cyfrol',
        'volumeNew.invalidNhs':
            'Rhowch Rif GIG dilys 10 digid (methodd gwiriad Modulus 11).',
        'volumeNew.titleRequired': 'Mae angen teitl cyfrol.',
        'volumeNew.formLabel': 'Cyfrol newydd',
        'volumeNew.patientNhs': 'Rhif GIG y claf',
        'volumeNew.nhsDescription': '10 digid, ar ffurf XXX XXX XXXX.',
        'volumeNew.titleLabel': 'Teitl cyfrol',
        'volumeNew.titleDescription': 'e.e. Alice Johnson — Cyf 1',
        'volumeNew.initialCabinet': 'Cabinet cychwynnol',
        'volumeNew.initialCabinetDescription':
            "Lle mae'r gyfrol yn byw. Gadewch yn wag os ar daith.",
        'volumeNew.createVolume': 'Creu cyfrol',
        'volumeDetail.backToVolumes': 'Yn ôl i gyfrolau',
        'volumeDetail.somethingWrong': "Aeth rhywbeth o'i le",
        'volumeDetail.foldersIn': 'Ffolderi yn y gyfrol hon ({n})',
        'volumeDetail.foldersTable': "Ffolderi'r gyfrol",
        'volumeDetail.noFolders': 'Dim ffolderi yn y gyfrol hon eto.',
        'volumeDetail.addFolderLabel': 'Ychwanegu ffolder',
        'volumeDetail.addFolderFor': 'Ychwanegu ffolder ar gyfer {patient}',
        'volumeDetail.chooseFolder': '— Dewis ffolder —',
        'volumeDetail.addToVolume': 'Ychwanegu at y gyfrol',
        'volumeDetail.renameVolume': 'Ailenwi cyfrol',
        'volumeDetail.rename': 'Ailenwi',
        'volumeDetail.moveVolume': 'Symud y gyfrol hon',
        'volumeDetail.moveIntro':
            "Yn adleoli pob ffolder yn y gyfrol gyda'i gilydd.",
        'volumeDetail.moveFormLabel': 'Symud cyfrol',
        'volumeDetail.destinationCabinet': 'Cabinet cyrchfan',
        'volumeDetail.moveReasonPlaceholder': 'e.e. Clinig cleifion allanol',
        'volumeDetail.moveVolumeButton': 'Symud cyfrol',
        'volumeDetail.moveHistory': 'Hanes symud',
        'workers.heading': 'Gweithwyr',
        'workers.intro':
            "Staff sy'n symud ffolderi. Agorwch weithiwr i weld y ffolderi maent wedi'u symud a phob ffolder sy'n perthyn i'w cleifion.",
        'workers.tableLabel': 'Gweithwyr',
        'workers.tableCaption': "Gweithwyr o'r Prif Wasanaeth Gweithwyr",
        'workers.noWorkers': "Dim gweithwyr wedi'u canfod.",
        'workerDetail.backToWorkers': 'Yn ôl i weithwyr',
        'workerDetail.foldersMoved':
            'Ffolderi a symudwyd gan y gweithiwr hwn ({n})',
        'workerDetail.foldersMovedTable':
            'Ffolderi a symudwyd gan y gweithiwr hwn',
        'workerDetail.noMovedFolders':
            "Nid yw'r gweithiwr hwn wedi symud unrhyw ffolderi eto.",
        'workerDetail.patientsFolders': 'Holl ffolderi eu cleifion ({n})',
        'workerDetail.patientsFoldersIntro':
            "Pob ffolder sy'n perthyn i glaf y mae'r gweithiwr hwn wedi delio ag ef.",
        'workerDetail.patientsFoldersTable':
            'Ffolderi cleifion y gweithiwr hwn',
        'workerDetail.noPatientFolders':
            "Dim ffolderi cleifion i'w dangos eto.",
        'workerDetail.movesByWorker': 'Symudiadau gan y gweithiwr hwn',
        'history.backToDashboard': "Yn ôl i'r dangosfwrdd",
        'history.heading': 'Hanes symud (log archwilio)',
        'history.filterPlaceholder':
            'Hidlo yn ôl claf, rhif GIG, cabinet, neu borthor',
        'history.filterLabel': "Hidlo'r log archwilio",
        'history.tableLabel': 'Log archwilio symud',
        'history.tableCaption':
            'Pob symudiad ffolder papur a gofnodwyd, y diweddaraf yn gyntaf',
        'history.noMatch': "Dim symudiadau'n cyfateb i'ch hidlydd.",
        'historyDetail.backToHistory': 'Yn ôl i hanes symud',
        'historyDetail.heading': 'Digwyddiad symud',
        'historyDetail.folderInvolved': 'Ffolder dan sylw',
        'historyDetail.detailsLabel': 'Manylion y digwyddiad symud',
        'historyDetail.otherFolders': 'Ffolderi eraill ar gyfer {patient}',
        'historyDetail.noOtherFolders':
            'Dim ffolderi eraill ar gyfer y claf hwn.',
        'reports.backToDashboard': "Yn ôl i'r dangosfwrdd",
        'reports.heading': 'Adroddiadau',
        'reports.atAGlance': 'Cipolwg',
        'reports.kpiPatients': 'Cleifion:',
        'reports.kpiFolders': 'Ffolderi:',
        'reports.kpiFoldersDetail':
            '({inCabinet} yn y cabinet, {inTransit} ar daith)',
        'reports.kpiVolumes': 'Cyfrolau:',
        'reports.kpiCabinets': 'Cabinetau:',
        'reports.kpiCabinetsDetail': 'mewn {n} adeilad',
        'reports.kpiMoves': 'Symudiadau — 24awr diwethaf:',
        'reports.kpiMoves7d': '7 diwrnod diwethaf:',
        'reports.cabinetUtilisation': 'Defnydd cabinetau',
        'reports.colCabinet': 'Cabinet',
        'reports.colFolders': 'Ffolderi',
        'reports.colCapacity': 'Capasiti',
        'reports.colUtilisation': 'Defnydd',
        'reports.inTransit': 'Ar daith ({n})',
        'reports.noInTransit': 'Nid oes ffolderi ar daith.',
        'reports.activityByWorker': 'Gweithgaredd yn ôl gweithiwr',
        'reports.colWorker': 'Gweithiwr',
        'reports.colMoves': 'Symudiadau',
        'reports.noMovesYet': "Dim symudiadau wedi'u cofnodi eto.",
        'reports.derivedLive':
            "Deillir adroddiadau'n fyw o'r API — dim storfa adrodd ar wahân.",
        'alerts.backToDashboard': "Yn ôl i'r dangosfwrdd",
        'alerts.heading': 'Rhybuddion geoffin',
        'alerts.intro':
            "Cofnodion achos a groesodd ffin adeilad. Mae torri geoffin yn unrhyw symudiad lle mae'r cabinetau tarddiad a chyrchfan mewn gwahanol adeiladau.",
        'alerts.tableLabel': 'Rhybuddion geoffin',
        'alerts.tableCaption': 'Symudiadau croesi ffin, y diweddaraf yn gyntaf',
        'alerts.colCrossed': 'Croesodd',
        'alerts.none':
            'Dim toriadau geoffin — mae pob ffolder wedi aros o fewn ei adeilad.',
        'grid.nhsNumber': 'Rhif GIG',
        'grid.patient': 'Claf',
        'grid.folder': 'Ffolder',
        'grid.cabinet': 'Cabinet',
        'grid.status': 'Statws',
        'grid.lastMoved': 'Symudwyd ddiwethaf',
        'addressograph.label': 'Addressograff claf',
        'addressograph.nhsNo': 'Rhif GIG',
        'addressograph.dob': 'D.G.',
        'addressograph.sex': 'Rhyw',
        'addressograph.address': 'Cyfeiriad',
        'buttonBar.label': 'Gweithredoedd',
        'buttonBar.patient': 'Claf',
        'buttonBar.referrals': 'Atgyfeiriadau',
        'buttonBar.activate': 'Actifadu',
        'buttonBar.caseNotes': 'Cofnodion Achos',
        'buttonBar.pathways': 'Llwybrau',
        'buttonBar.legalStatus': 'Statws Cyfreithiol',
        'buttonBar.documents': 'Dogfennau',
        'buttonBar.wrapper': 'Amlen',
        'buttonBar.audit': 'Archwilio',
        'buttonBar.quickReports': 'Adroddiadau Cyflym',
        'labels.title': 'Labeli',
        'labels.searchLabel': 'Chwilio labeli',
        'labels.searchPlaceholder': 'Rhowch destun i chwilio...',
        'labels.find': 'Canfod',
        'labels.clear': 'Clirio',
        'labels.volumeTitles': 'Teitlau cyfrolau',
        'labels.noMatching': "Dim cyfrolau'n cyfateb.",
        'labels.numberOfCopies': 'Nifer y Copïau:',
        'labels.copiesLabel': 'Nifer y copïau',
        'labels.print': 'Argraffu',
        'labels.close': 'Cau',
        'auth.signin': 'Mewngofnodi',
        'auth.signout': 'Allgofnodi',
        'share.email': "E-bostio'r ddolen",
        'share.linkedin': 'Rhannu ar LinkedIn',
        'share.reddit': 'Rhannu ar Reddit',
        'share.bluesky': 'Rhannu ar Bluesky',
        'share.mastodon': 'Rhannu ar Mastodon',
        'splash.hero.secondary': 'Gweld beth sydd y tu mewn',
        'splash.benefits.title': "Pam mae timau'n ei ddewis",
        'splash.features.title': 'Beth allwch chi ei wneud',
        'splash.trust.title': "Wedi'i adeiladu ar gyfer ymddiriedaeth",
        'splash.trust.1.title': 'Mewngofnodi heb gyfrinair',
        'splash.trust.1.body':
            "Dolen hud a anfonir i'ch e-bost: dim cyfrinair i'w ollwng na'i ailddefnyddio.",
        'splash.trust.2.title': 'Caniatâd yn seiliedig ar briodoleddau',
        'splash.trust.2.body':
            "Mae rheolau manwl yn penderfynu pwy sy'n cael darllen, ysgrifennu, uno neu ddileu.",
        'splash.trust.3.title': "Llwybr archwilio sy'n dangos ymyrryd",
        'splash.trust.3.body':
            "Mae hanes atodi-yn-unig yn cofnodi pob newid a phwy a'i gwnaeth.",
        'splash.trust.4.title': 'Rheolaethau preifatrwydd',
        'splash.trust.4.body':
            "Mae manylion sensitif wedi'u cuddio oni bai bod hawl gennych i'w gweld.",
        'splash.trust.5.title': 'Safonau agored',
        'splash.trust.5.body':
            'REST gydag OpenAPI, a HL7 FHIR lle mae systemau iechyd ei angen.',
        'splash.trust.6.title': 'Yn siarad eich iaith',
        'splash.trust.6.body':
            'Arabeg, Tsieinëeg, Saesneg, Ffrangeg, Hindi, Sbaeneg a Chymraeg.',
        'splash.cta.title': 'Barod i ddechrau?',
        'splash.cta.body':
            "Mewngofnodwch gyda dolen hud a anfonir i'ch e-bost. Dim angen cyfrinair.",
        'splash.hero.title': 'Gwybod ble mae pob ffolder',
        'splash.hero.subtitle':
            'Olrheiniwch ffolderi nodiadau achos papur yn ôl Rhif GIG ar draws adeiladau, ystafelloedd a chabinetau, gyda llwybr archwilio llawn o bob symudiad.',
        'splash.benefits.1.title': 'Dod o hyd i unrhyw ffolder yn gyflym',
        'splash.benefits.1.body':
            'Chwiliwch am ffolder nodiadau achos papur yn ôl Rhif GIG a gweld ble mae ar hyn o bryd.',
        'splash.benefits.2.title': 'Llai o ffolderi ar goll',
        'splash.benefits.2.body':
            'Cofnodir pob symudiad, felly nid yw ffolder byth yn rhywle yn yr adeilad yn unig.',
        'splash.benefits.3.title': 'Llai o amser yn chwilio',
        'splash.benefits.3.body':
            'Mae porthorion a chlinigwyr yn treulio eu hamser ar ofal, nid yn chwilio drwy gabinetau.',
        'splash.benefits.4.title': 'Llwybr dibynadwy',
        'splash.benefits.4.body':
            "Mae hanes cyflawn, wedi'i ddyddio, yn dangos pwy symudodd bob ffolder, pryd a pham.",
        'splash.benefits.5.title': 'Hunaniaeth claf ddiogel',
        'splash.benefits.5.body':
            "Gwirir Rhifau GIG cyn eu cadw, felly nid yw camgymeriadau teipio byth yn troi'n gofnodion.",
        'splash.benefits.6.title': "Golwg glir o'r ystad",
        'splash.benefits.6.body':
            'Gwelwch pa mor llawn yw pob cabinet a faint o ffolderi sydd ar daith.',
        'splash.features.1.title': 'Cofrestr ffolderi',
        'splash.features.1.body':
            'Chwiliwch bob ffolder yn ôl claf, teitl neu Rif GIG.',
        'splash.features.2.title': 'Symud a sganio',
        'splash.features.2.body':
            "Cofnodwch symudiad mewn eiliadau drwy sganio neu deipio'r Rhif GIG.",
        'splash.features.3.title': 'Cyfrolau',
        'splash.features.3.body':
            "Grwpiwch ffolderi claf yn gyfrol a'u symud gyda'i gilydd.",
        'splash.features.4.title': 'Adeiladau, ystafelloedd, cabinetau',
        'splash.features.4.body':
            'Modelwch yr hierarchaeth ffisegol o bob adeilad i lawr at ei gabinetau.',
        'splash.features.5.title': 'Hanes symud',
        'splash.features.5.body':
            "Chwiliwch drwy'r log archwilio llawn o bob symudiad ffolder.",
        'splash.features.6.title': 'Rhybuddion ac adroddiadau',
        'splash.features.6.body':
            'Adolygwch rybuddion rhwng adeiladau, defnydd cabinetau a ffigurau allweddol.',
        'nav.tour': 'Taith',
        'splash.hero.tour': 'Cymerwch y daith',
        'tour.head': 'Cymerwch y daith',
        'tour.toc': 'Ar y dudalen hon',
        'tour.open': 'Agor y sgrin hon',
        'tour.top': "Yn ôl i'r brig",
        'tour.start.title': 'Cyn i chi ddechrau',
        'tour.start.summary':
            "Mae angen cyfrif arnoch i weithio gyda data go iawn. Mae mewngofnodi'n cymryd llai na munud ac nid oes angen cyfrinair.",
        'tour.start.step.1':
            'Dewiswch Mewngofnodi ar y brig ar y dde a rhowch eich cyfeiriad e-bost.',
        'tour.start.step.2':
            "Agorwch y ddolen hud a anfonwn atoch. Mae'n gweithio unwaith ac yn dod i ben yn gyflym.",
        'tour.start.step.3':
            "Byddwch yn dychwelyd i'r ap wedi mewngofnodi, heb ddim i'w gofio na'i ailosod.",
        'tour.start.step.4':
            "Defnyddiwch y botymau wrth ymyl Mewngofnodi i newid y thema, yr iaith a maint y testun, neu i rannu'r dudalen.",
        'tour.intro':
            "Taith dywys drwy Olrhain Achosion: beth mae pob sgrin yn ei wneud a'r camau i'w defnyddio, o ychwanegu ffolder i wirio'r log archwilio symud.",
        'tour.s1.title': 'Ychwanegu ffolder at y gofrestr',
        'tour.s1.summary':
            "Mae pob ffolder papur o nodiadau achos yn perthyn i un claf ac yn cael ei olrhain o'r eiliad y byddwch yn ei ychwanegu.",
        'tour.s1.step.1':
            "Agorwch Ffolderi a dewiswch Ychwanegu ffolder, yna rhowch Rhif GIG 10 digid y claf, sy'n gorfod pasio'r prawf Modwlws 11.",
        'tour.s1.step.2':
            'Teipiwch Teitl ffolder, fel Cyfrol 1 neu Cardioleg 2023.',
        'tour.s1.step.3':
            "Ar gyfer claf nad yw wedi'i gofrestru eto, llenwch Enw'r claf a Dyddiad geni hefyd; nid oes eu hangen ar gyfer claf sy'n bodoli eisoes.",
        'tour.s1.step.4':
            "Dewiswch y Cabinet cychwynnol, neu gadewch ef yn wag os yw'r ffolder ar daith, yna dewiswch Cadw ffolder.",
        'tour.s2.title': 'Dod o hyd i ffolder',
        'tour.s2.summary':
            'Mae\'r Gofrestr ffolderi yn ateb "ble mae\'r ffolder hwn ar hyn o bryd?" o un blwch chwilio.',
        'tour.s2.step.1':
            'Agorwch Ffolderi a theipiwch Rhif GIG, enw claf, teitl ffolder neu gabinet yn Chwilio ffolderi.',
        'tour.s2.step.2':
            'Darllenwch Cabinet, Statws (mewn cabinet neu ar daith) a Symudwyd ddiwethaf pob rhes.',
        'tour.s2.step.3':
            "Agorwch ffolder i weld ei manylion a'i Hanes symud, neu agorwch y claf o Gleifion i weld ei holl ffolderi.",
        'tour.s2.step.4':
            'Dewiswch Symud y ffolder hwn i fynd yn syth at gofnodi symudiad ar ei gyfer.',
        'tour.s3.title': 'Symud ffolder',
        'tour.s3.summary':
            'Cofnodir pob lleoliad, felly mae lleoliad ffolder bob amser yn gyfredol.',
        'tour.s3.step.1':
            "Agorwch Symud ffolder, rhowch Rhif GIG y claf, yna dewiswch pa un o ffolderi'r claf hwnnw rydych yn ei symud.",
        'tour.s3.step.2':
            'Dewiswch y Cabinet cyrchfan, neu Ar daith (porthor yn cario) os yw ar ei ffordd.',
        'tour.s3.step.3':
            "Dewiswch Weithiwr o'r rhestr, neu teipiwch enw o dan Symudwyd gan, ac ychwanegwch Reswm.",
        'tour.s3.step.4':
            "Dewiswch Cofnodi symudiad; mae'r dudalen yn cadarnhau Symudiad wedi'i gofnodi ac mae'r symudiad yn ymuno â'r log archwilio.",
        'tour.s4.title': 'Sganio ffolder',
        'tour.s4.summary':
            "Y llwybr cyflym i'r ddesg cofnodion: nid oes angen sganiwr caledwedd, er bod sganiwr sy'n teipio drwy'r bysellfwrdd yn gweithio hefyd.",
        'tour.s4.step.1':
            'Agorwch Sganio a chliciwch yn y blwch Sganio neu chwilio.',
        'tour.s4.step.2':
            'Sganiwch god bar, neu teipiwch Rhif GIG neu id ffolder.',
        'tour.s4.step.3':
            'Adolygwch y rhestr Cydweddiadau ac agorwch ffolder i weld ei manylion.',
        'tour.s4.step.4':
            "Dewiswch Symud y ffolder hwn i gofnodi ei symudiad gyda'r ffolder eisoes wedi'i ddewis; os nad oes dim yn cydweddu, mae'r dudalen yn dweud na cheir hyd i ffolder.",
        'tour.s5.title': 'Bwndelu ffolderi yn gyfrolau',
        'tour.s5.summary':
            "Bwndel symudol o ffolderi un claf yw cyfrol, felly maen nhw'n teithio gyda'i gilydd.",
        'tour.s5.step.1':
            "Agorwch Cyfrolau a dewiswch Cyfrol newydd, yna rhowch Rhif GIG y claf a Theitl cyfrol; rhaid i'r claf fod â ffolder eisoes.",
        'tour.s5.step.2':
            "Agorwch y gyfrol a defnyddiwch Ychwanegu ffolder i fwndelu ffolderi'r claf hwnnw ynddi.",
        'tour.s5.step.3':
            'Defnyddiwch Symud y gyfrol hon i symud pob ffolder ynddi i un cabinet cyrchfan mewn un cam.',
        'tour.s5.step.4':
            "Yn ôl ar Gyfrolau, dewiswch Argraffu labeli, dewiswch y cyfrolau, gosodwch Nifer y Copïau a dewiswch Argraffu i'w rhoi yn y ciw.",
        'tour.s6.title': 'Adolygu hanes, rhybuddion ac adroddiadau',
        'tour.s6.summary':
            'Cedwir pob symudiad, felly gallwch ateb pwy symudodd beth, pryd a pham.',
        'tour.s6.step.1':
            "Agorwch Hanes symud a defnyddiwch Hidlo'r log archwilio i gyfyngu yn ôl claf, rhif GIG, cabinet neu borthor; daw'r symudiadau diweddaraf gyntaf.",
        'tour.s6.step.2':
            'Agorwch res i weld y digwyddiad symud llawn, y ffolder dan sylw a ffolderi eraill y claf.',
        'tour.s6.step.3':
            "Agorwch Rhybuddion i weld rhybuddion geoffin: symudiadau lle mae'r cabinetau tarddiad a chyrchfan mewn adeiladau gwahanol.",
        'tour.s6.step.4':
            "Agorwch Adroddiadau i weld y cyfrifon cryno, defnydd cabinetau, ffolderi ar daith a gweithgarwch fesul gweithiwr, i gyd wedi'u deillio'n fyw.",
        'signin.sso': 'Mewngofnodi gydag SSO',
    },
    'en-001': {
        // Brand / chrome
        'brand.name': 'Case Tracking',
        'brand.tagline': 'NHS paper records',
        'chrome.language': 'Language',
        'chrome.theme': 'Theme',
        'nav.share': 'Share',
        'nav.text_size': 'Text size',
        'share.copy_link': 'Copy Link',
        'share.copied': 'Link copied',
        'share.copy_failed': 'Could not copy — copy it from the address bar',
        'theme.default': 'Default',
        'theme.highContrast': 'High contrast',
        'nav.toggle': 'Toggle navigation',
        'auth.signedInAs': 'Signed in as',
        'auth.signOut': 'Sign out',
        // Nav links
        'nav.dashboard': 'Dashboard',
        'nav.patients': 'Patients',
        'nav.folders': 'Folders',
        'nav.volumes': 'Volumes',
        'nav.workers': 'Workers',
        'nav.buildings': 'Buildings',
        'nav.cabinets': 'Cabinets',
        'nav.move': 'Move folder',
        'nav.scan': 'Scan',
        'nav.history': 'Move history',
        'nav.alerts': 'Alerts',
        'nav.reports': 'Reports',
        // Layout regions / footer / skip link
        'layout.skipToContent': 'Skip to main content',
        'layout.siteHeader': 'Site header',
        'layout.siteFooter': 'Site footer',
        'layout.primaryNavigation': 'Primary navigation',
        'footer.text':
            'Case Tracking — built with the Lily Design System (NHS theme) and SVAR Svelte. Demo data only; not a regulated medical record.',
        // Shared / common
        'common.backToDashboard': 'Back to dashboard',
        'common.cancel': 'Cancel',
        'common.move': 'Move',
        'common.view': 'View',
        'common.remove': 'Remove',
        'common.action': 'Action',
        'common.status': 'Status',
        'common.patient': 'Patient',
        'common.folder': 'Folder',
        'common.cabinet': 'Cabinet',
        'common.title': 'Title',
        'common.name': 'Name',
        'common.role': 'Role',
        'common.reason': 'Reason',
        'common.movedBy': 'Moved by',
        'common.lastMoved': 'Last moved',
        'common.nhsNumber': 'NHS Number',
        'common.dateOfBirth': 'Date of birth',
        'common.description': 'Description',
        'common.notes': 'Notes',
        'common.entered': 'Entered',
        'common.left': 'Left',
        'common.when': 'When',
        'common.from': 'From',
        'common.to': 'To',
        'common.source': 'Source',
        'common.volume': 'Volume',
        'common.stillHere': 'Still here',
        'common.noMovesYet': 'No moves recorded yet.',
        'common.inTransitPorter': 'In transit (porter carrying)',
        'common.selectCabinetOption': '— Select cabinet —',
        'common.inTransitOption': '— In transit —',
        'status.inCabinet': 'in-cabinet',
        'status.inTransit': 'in-transit',
        // Folder status badges (display labels)
        'badge.located': 'Located',
        'badge.porterInMotion': 'Porter in motion',
        // Dashboard
        'dashboard.welcomePrefix': 'Welcome.',
        'dashboard.inTransit.one':
            'You have {n} folder currently in transit. Use the',
        'dashboard.inTransit.other':
            'You have {n} folders currently in transit. Use the',
        'dashboard.moveFolderPage': 'Move folder',
        'dashboard.pageToRecord': 'page to record a placement.',
        'dashboard.folderSummary': 'Folder summary',
        'dashboard.patients': 'Patients',
        'dashboard.foldersTracked': '{n} folders tracked',
        'dashboard.inCabinet': 'In cabinet',
        'dashboard.inTransitCard': 'In transit',
        'dashboard.buildings': 'Buildings',
        'dashboard.roomsCabinets': '{rooms} rooms · {cabinets} cabinets',
        'dashboard.moves24h': 'Moves (24h)',
        'dashboard.auditedPlacements': 'Audited folder placements',
        'dashboard.folderRegister': 'Folder register',
        'dashboard.viewAll': 'View all',
        'dashboard.addFolder': 'Add folder',
        'dashboard.recentMoves': 'Recent moves',
        'dashboard.seeFullHistory': 'See full audit history →',
        'dashboard.cabinetUtilisation': 'Cabinet utilisation',
        // Error boundary
        'error.backToDashboard': 'Back to dashboard',
        'error.heading': 'Case Tracking API error',
        'error.unknown': 'Unknown error',
        'error.apiHint':
            'The app talks to the Loco JSON API through /api (proxied to the Loco server in dev). Make sure that API is running. See README.md § "Quick start".',
        // Login
        'login.title': 'Sign in',
        'login.intro':
            "Enter your work email address. If it is recognised, we'll send you a one-time sign-in link. No password required.",
        'login.sendError': 'Could not send sign-in link',
        'login.enterEmail': 'Enter your email address.',
        'login.checkEmail': 'Check your email',
        'login.sentBody':
            'matches a known account, a sign-in link is on its way. The link expires in 10 minutes.',
        'login.sentPrefix': 'If',
        'login.devShortcut': 'Development shortcut:',
        'login.openLink': 'open your sign-in link',
        'login.formLabel': 'Sign in',
        'login.emailLabel': 'Email address',
        'login.submit': 'Email me a sign-in link',
        // Auth callback
        'callback.title': 'Signing you in',
        'callback.error': 'Sign-in link could not be used',
        'callback.backToSignIn': 'Back to sign in',
        'callback.completing': 'One moment — completing your sign-in…',
        // Scan
        'scan.heading': 'Scan a folder',
        'scan.intro':
            'Scan a barcode or type an NHS Number (or folder id) to jump straight to a folder and record its move — the Scan4Safety fast path. No hardware scanner needed; a keyboard-wedge scanner types into the box below.',
        'scan.failed': 'Scan failed',
        'scan.formLabel': 'Scan',
        'scan.fieldLabel': 'Scan or search',
        'scan.fieldDescription':
            'NHS Number (e.g. 943 476 5919) or a folder id.',
        'scan.placeholder': 'Scan or type…',
        'scan.matches': 'Matches ({n})',
        'scan.moveThisFolder': 'Move this folder',
        'scan.noFolderFound': 'No folder found for “{term}”.',
        // Move folder
        'move.heading': 'Move a folder',
        'move.intro':
            "Enter a patient's NHS Number, pick the folder you're moving, then pick the destination cabinet (or mark it in transit).",
        'move.recorded': 'Move recorded',
        'move.formLabel': 'Move folder',
        'move.patientNhs': 'Patient NHS Number',
        'move.invalidNhs': 'Enter a valid 10-digit NHS Number.',
        'move.folder': 'Folder',
        'move.selectFolderError': 'Select which folder to move.',
        'move.pickFolderDescription':
            "Pick which of this patient's folders to move.",
        'move.enterNhsDescription': 'Enter an NHS Number to see folders.',
        'move.selectFolderOption': '— Select folder —',
        'move.destination': 'Destination',
        'move.workerLabel': 'Worker (from Main Worker Service)',
        'move.workerDescription':
            'Pick a registered worker, or leave blank to use the free-text field below.',
        'move.freeTextOnly': '— Free-text only —',
        'move.movedByLabel': 'Moved by (free text)',
        'move.movedByDescription': 'Used when no worker is selected.',
        'move.movedByPlaceholder': 'e.g. Alice (porter)',
        'move.reasonPlaceholder': 'e.g. Outpatient appointment',
        'move.recordMove': 'Record move',
        'move.patientFolders': 'Patient folders',
        'move.enterValidNhs':
            "Enter a valid NHS Number to see this patient's folders.",
        'move.folderNotFound': 'Folder not found.',
        'move.recordedSummary':
            'Recorded move of {patient} — {folder} from {from} to {to}.',
        // Folders index
        'folders.register': 'Folder register',
        'folders.searchPlaceholder':
            'Search by NHS Number, patient, folder title, or cabinet',
        'folders.searchLabel': 'Search folders',
        'folders.addFolder': 'Add folder',
        'folders.tableLabel': 'Folders',
        'folders.tableCaption':
            'All paper case-note folders tracked by the system',
        'folders.colNhsNumber': 'NHS Number',
        'folders.colPatient': 'Patient',
        'folders.colFolder': 'Folder',
        'folders.colCabinet': 'Cabinet',
        'folders.colStatus': 'Status',
        'folders.colLastMoved': 'Last moved',
        'folders.colAction': 'Action',
        'folders.noMatch': 'No folders match',
        // New folder
        'folderNew.backToFolders': 'Back to folders',
        'folderNew.heading': 'Add a new folder',
        'folderNew.intro':
            "A folder belongs to one patient. If the patient is not yet registered with the Main Patient Service, we'll create them; otherwise the new folder is attached to the existing patient record.",
        'folderNew.cannotSave': 'Cannot save folder',
        'folderNew.invalidNhs':
            'Enter a valid 10-digit NHS Number (Modulus 11 check failed).',
        'folderNew.titleRequired': 'Folder title is required.',
        'folderNew.formLabel': 'Add folder',
        'folderNew.nhsDescription': '10 digits, formatted XXX XXX XXXX.',
        'folderNew.titleLabel': 'Folder title',
        'folderNew.titleDescription': 'e.g. Volume 1, Cardiology 2023',
        'folderNew.patientName': 'Patient name',
        'folderNew.patientNameDescription': 'Only needed for a new patient.',
        'folderNew.dobDescription': 'Only needed for a new patient.',
        'folderNew.initialCabinet': 'Initial cabinet',
        'folderNew.initialCabinetDescription':
            'Leave blank if the folder is in transit.',
        'folderNew.saveFolder': 'Save folder',
        // Folder detail
        'folderDetail.backToFolders': 'Back to folders',
        'folderDetail.patientPrefix': 'Patient:',
        'folderDetail.detailsLabel': 'Folder details',
        'folderDetail.folderTitle': 'Folder title',
        'folderDetail.currentCabinet': 'Current cabinet',
        'folderDetail.moveThisFolder': 'Move this folder',
        'folderDetail.moveHistory': 'Move history',
        // Patients index
        'patients.heading': 'Patients',
        'patients.searchPlaceholder': 'Search by NHS Number or name',
        'patients.searchLabel': 'Search patients',
        'patients.tableLabel': 'Patients',
        'patients.tableCaption':
            'All patients with one or more registered folders',
        'patients.colFolders': 'Folders',
        'patients.noMatch': 'No patients match',
        // Patient detail
        'patientDetail.backToPatients': 'Back to patients',
        'patientDetail.notFoundHeading':
            'Patient not found in Main Patient Service',
        'patientDetail.notFoundBody':
            'No patient record exists for NHS Number {nhs}. The folders below are reconstructed from local snapshots written when each folder was created.',
        'patientDetail.sourcePrefix': 'Source:',
        'patientDetail.recordActions': 'Patient record actions',
        'patientDetail.nhsNumberHeading': 'NHS Number {nhs}',
        'patientDetail.foldersForPatient': 'Folders for this patient ({n})',
        'patientDetail.patientFoldersTable': 'Patient folders',
        'patientDetail.colVolume': 'Volume',
        'patientDetail.noFoldersYet': 'No folders yet.',
        'patientDetail.addFolderForPatient': 'Add a folder for this patient',
        'patientDetail.moveHistoryForPatient': 'Move history for this patient',
        'patientDetail.demoUnavailable':
            "“{action}” isn't available in this demo.",
        // Buildings index
        'buildings.heading': 'Buildings',
        'buildings.addBuilding': 'Add building',
        'buildings.tableLabel': 'Buildings',
        'buildings.tableCaption': 'Physical sites holding records rooms',
        'buildings.colRooms': 'Rooms',
        'buildings.noBuildings': 'No buildings yet.',
        // New building
        'buildingNew.backToBuildings': 'Back to buildings',
        'buildingNew.heading': 'Add a building',
        'buildingNew.intro':
            'One building can have many rooms; each room can hold many cabinets.',
        'buildingNew.cannotSave': 'Cannot save building',
        'buildingNew.nameRequired': 'Building name is required.',
        'buildingNew.formLabel': 'Add building',
        'buildingNew.nameLabel': 'Building name',
        'buildingNew.saveBuilding': 'Save building',
        // Building detail
        'buildingDetail.backToBuildings': 'Back to buildings',
        'buildingDetail.rooms': 'Rooms ({n})',
        'buildingDetail.roomsTable': 'Rooms',
        'buildingDetail.colCabinets': 'Cabinets',
        'buildingDetail.noRooms': 'No rooms yet.',
        'buildingDetail.addRoom': 'Add a room',
        'buildingDetail.addRoomLabel': 'Add room',
        'buildingDetail.roomNameRequired': 'Room name is required.',
        'buildingDetail.roomName': 'Room name',
        'buildingDetail.saveRoom': 'Save room',
        'buildingDetail.presenceHistory': 'Folder presence history',
        'buildingDetail.presenceIntro':
            'Folders that have been in any cabinet in this building, newest first.',
        'buildingDetail.presenceTable': 'Building folder presence history',
        'buildingDetail.presenceCaption':
            "Aggregated across this building's cabinets",
        'buildingDetail.noPresence':
            'No folder presence recorded in this building yet.',
        // Cabinets index
        'cabinets.heading': 'File cabinets',
        'cabinets.addCabinet': 'Add cabinet',
        'cabinets.tableLabel': 'Cabinets',
        'cabinets.tableCaption':
            'Physical file cabinets, their building/room, and occupancy',
        'cabinets.colLabel': 'Label',
        'cabinets.colBuilding': 'Building',
        'cabinets.colRoom': 'Room',
        'cabinets.colCapacity': 'Capacity',
        'cabinets.colFolders': 'Folders',
        'cabinets.colUtilisation': 'Utilisation',
        // New cabinet
        'cabinetNew.backToCabinets': 'Back to cabinets',
        'cabinetNew.heading': 'Add a file cabinet',
        'cabinetNew.intro':
            'A cabinet lives inside a room (which lives inside a building).',
        'cabinetNew.noRoomsHeading': 'No rooms exist yet',
        'cabinetNew.noRoomsBody':
            "first, then add a room from the building's page.",
        'cabinetNew.noRoomsBuilding': 'building',
        'cabinetNew.noRoomsCreate': 'Create a',
        'cabinetNew.cannotSave': 'Cannot save cabinet',
        'cabinetNew.labelRequired': 'Cabinet label is required.',
        'cabinetNew.roomRequired': 'Select a room.',
        'cabinetNew.formLabel': 'Add cabinet',
        'cabinetNew.labelLabel': 'Cabinet label',
        'cabinetNew.labelPlaceholder': 'e.g. Cabinet D3',
        'cabinetNew.roomLabel': 'Room',
        'cabinetNew.selectRoomOption': '— Select a room —',
        'cabinetNew.capacityLabel': 'Capacity',
        'cabinetNew.capacityDescription':
            'Approximate number of folders the cabinet holds. Leave blank for unknown.',
        'cabinetNew.saveCabinet': 'Save cabinet',
        // Cabinet detail
        'cabinetDetail.backToCabinets': 'Back to cabinets',
        'cabinetDetail.currentFolders':
            'Folders currently in this cabinet ({n})',
        'cabinetDetail.currentFoldersTable': 'Current folders',
        'cabinetDetail.empty': 'This cabinet is currently empty.',
        'cabinetDetail.presenceHistory': 'Folder presence history',
        'cabinetDetail.presenceIntro':
            'Which folders have been in this cabinet, and when. Newest first.',
        'cabinetDetail.presenceTable': 'Folder presence history',
        'cabinetDetail.presenceCaption': 'Derived from the move audit log',
        'cabinetDetail.colReasonLeft': 'Reason left',
        'cabinetDetail.noPresence':
            'No folder has been recorded in this cabinet yet.',
        // Room detail
        'roomDetail.backToBuildings': 'Back to buildings',
        'roomDetail.presenceHistory': 'Folder presence history',
        'roomDetail.presenceIntro':
            'Folders that have been in any cabinet in this room, newest first.',
        'roomDetail.presenceTable': 'Room folder presence history',
        'roomDetail.presenceCaption': "Aggregated across this room's cabinets",
        'roomDetail.noPresence':
            'No folder presence recorded in this room yet.',
        // Volumes index
        'volumes.heading': 'Volumes',
        'volumes.printLabels': 'Print labels',
        'volumes.newVolume': 'New volume',
        'volumes.intro':
            "A volume is a movable bundle of one patient's folders. Move the volume and every folder inside it moves together.",
        'volumes.tableLabel': 'Volumes',
        'volumes.tableCaption':
            'Bundles of folders, each belonging to one patient',
        'volumes.colFolders': 'Folders',
        'volumes.colLocation': 'Location',
        'volumes.noVolumes':
            "No volumes yet. Create one to bundle a patient's folders.",
        'volumes.noLabelsSelected': 'No labels selected.',
        'volumes.queued.one':
            'Queued {n} label × {copies} {copy} for printing.',
        'volumes.queued.other':
            'Queued {n} labels × {copies} {copy} for printing.',
        'volumes.copy': 'copy',
        'volumes.copies': 'copies',
        // New volume
        'volumeNew.backToVolumes': 'Back to volumes',
        'volumeNew.heading': 'New volume',
        'volumeNew.intro':
            'A volume bundles folders for one patient. The patient must already be registered (create a folder for them first). You can add folders to the volume once it exists.',
        'volumeNew.cannotCreate': 'Cannot create volume',
        'volumeNew.invalidNhs':
            'Enter a valid 10-digit NHS Number (Modulus 11 check failed).',
        'volumeNew.titleRequired': 'Volume title is required.',
        'volumeNew.formLabel': 'New volume',
        'volumeNew.patientNhs': 'Patient NHS Number',
        'volumeNew.nhsDescription': '10 digits, formatted XXX XXX XXXX.',
        'volumeNew.titleLabel': 'Volume title',
        'volumeNew.titleDescription': 'e.g. Alice Johnson — Vol 1',
        'volumeNew.initialCabinet': 'Initial cabinet',
        'volumeNew.initialCabinetDescription':
            'Where the volume lives. Leave blank if in transit.',
        'volumeNew.createVolume': 'Create volume',
        // Volume detail
        'volumeDetail.backToVolumes': 'Back to volumes',
        'volumeDetail.somethingWrong': 'Something went wrong',
        'volumeDetail.foldersIn': 'Folders in this volume ({n})',
        'volumeDetail.foldersTable': 'Volume folders',
        'volumeDetail.noFolders': 'No folders in this volume yet.',
        'volumeDetail.addFolderLabel': 'Add a folder',
        'volumeDetail.addFolderFor': 'Add a folder for {patient}',
        'volumeDetail.chooseFolder': '— Choose a folder —',
        'volumeDetail.addToVolume': 'Add to volume',
        'volumeDetail.renameVolume': 'Rename volume',
        'volumeDetail.rename': 'Rename',
        'volumeDetail.moveVolume': 'Move this volume',
        'volumeDetail.moveIntro':
            'Relocates every folder in the volume together.',
        'volumeDetail.moveFormLabel': 'Move volume',
        'volumeDetail.destinationCabinet': 'Destination cabinet',
        'volumeDetail.moveReasonPlaceholder': 'e.g. Outpatient clinic',
        'volumeDetail.moveVolumeButton': 'Move volume',
        'volumeDetail.moveHistory': 'Move history',
        // Workers index
        'workers.heading': 'Workers',
        'workers.intro':
            "Staff who move folders. Open a worker to see the folders they've moved and every folder belonging to their patients.",
        'workers.tableLabel': 'Workers',
        'workers.tableCaption': 'Workers from the Main Worker Service',
        'workers.noWorkers': 'No workers found.',
        // Worker detail
        'workerDetail.backToWorkers': 'Back to workers',
        'workerDetail.foldersMoved': 'Folders moved by this worker ({n})',
        'workerDetail.foldersMovedTable': 'Folders moved by this worker',
        'workerDetail.noMovedFolders':
            "This worker hasn't moved any folders yet.",
        'workerDetail.patientsFolders': "All their patients' folders ({n})",
        'workerDetail.patientsFoldersIntro':
            'Every folder belonging to a patient this worker has handled.',
        'workerDetail.patientsFoldersTable': "This worker's patients' folders",
        'workerDetail.noPatientFolders': 'No patient folders to show yet.',
        'workerDetail.movesByWorker': 'Moves by this worker',
        // History index
        'history.backToDashboard': 'Back to dashboard',
        'history.heading': 'Move history (audit log)',
        'history.filterPlaceholder':
            'Filter by patient, NHS number, cabinet, or porter',
        'history.filterLabel': 'Filter audit log',
        'history.tableLabel': 'Move audit log',
        'history.tableCaption':
            'Every recorded movement of a paper folder, newest first',
        'history.noMatch': 'No moves match your filter.',
        // History detail
        'historyDetail.backToHistory': 'Back to move history',
        'historyDetail.heading': 'Move event',
        'historyDetail.folderInvolved': 'Folder involved',
        'historyDetail.detailsLabel': 'Move event details',
        'historyDetail.otherFolders': 'Other folders for {patient}',
        'historyDetail.noOtherFolders': 'No other folders for this patient.',
        // Reports
        'reports.backToDashboard': 'Back to dashboard',
        'reports.heading': 'Reports',
        'reports.atAGlance': 'At a glance',
        'reports.kpiPatients': 'Patients:',
        'reports.kpiFolders': 'Folders:',
        'reports.kpiFoldersDetail':
            '({inCabinet} in cabinet, {inTransit} in transit)',
        'reports.kpiVolumes': 'Volumes:',
        'reports.kpiCabinets': 'Cabinets:',
        'reports.kpiCabinetsDetail': 'in {n} buildings',
        'reports.kpiMoves': 'Moves — last 24h:',
        'reports.kpiMoves7d': 'last 7d:',
        'reports.cabinetUtilisation': 'Cabinet utilisation',
        'reports.colCabinet': 'Cabinet',
        'reports.colFolders': 'Folders',
        'reports.colCapacity': 'Capacity',
        'reports.colUtilisation': 'Utilisation',
        'reports.inTransit': 'In transit ({n})',
        'reports.noInTransit': 'No folders are in transit.',
        'reports.activityByWorker': 'Activity by worker',
        'reports.colWorker': 'Worker',
        'reports.colMoves': 'Moves',
        'reports.noMovesYet': 'No moves recorded yet.',
        'reports.derivedLive':
            'Reports are derived live from the API — no separate reporting store.',
        // Alerts
        'alerts.backToDashboard': 'Back to dashboard',
        'alerts.heading': 'Geofence alerts',
        'alerts.intro':
            'Case notes that crossed a building boundary. A geofence breach is any move whose origin and destination cabinets are in different buildings.',
        'alerts.tableLabel': 'Geofence alerts',
        'alerts.tableCaption': 'Boundary-crossing moves, newest first',
        'alerts.colCrossed': 'Crossed',
        'alerts.none':
            'No geofence breaches — every folder has stayed within its building.',
        // FolderGrid (SVAR) column headers
        'grid.nhsNumber': 'NHS Number',
        'grid.patient': 'Patient',
        'grid.folder': 'Folder',
        'grid.cabinet': 'Cabinet',
        'grid.status': 'Status',
        'grid.lastMoved': 'Last moved',
        // Addressograph box
        'addressograph.label': 'Patient addressograph',
        'addressograph.nhsNo': 'NHS No.',
        'addressograph.dob': 'D.O.B.',
        'addressograph.sex': 'Sex',
        'addressograph.address': 'Address',
        // ButtonBar
        'buttonBar.label': 'Actions',
        'buttonBar.patient': 'Patient',
        'buttonBar.referrals': 'Referrals',
        'buttonBar.activate': 'Activate',
        'buttonBar.caseNotes': 'Case Notes',
        'buttonBar.pathways': 'Pathways',
        'buttonBar.legalStatus': 'Legal Status',
        'buttonBar.documents': 'Documents',
        'buttonBar.wrapper': 'Wrapper',
        'buttonBar.audit': 'Audit',
        'buttonBar.quickReports': 'Quick Reports',
        // Labels dialog
        'labels.title': 'Labels',
        'labels.searchLabel': 'Search labels',
        'labels.searchPlaceholder': 'Enter text to search...',
        'labels.find': 'Find',
        'labels.clear': 'Clear',
        'labels.volumeTitles': 'Volume titles',
        'labels.noMatching': 'No matching volumes.',
        'labels.numberOfCopies': 'Number of Copies:',
        'labels.copiesLabel': 'Number of copies',
        'labels.print': 'Print',
        'labels.close': 'Close',
        'auth.signin': 'Sign in',
        'auth.signout': 'Sign out',
        'share.email': 'Email Link',
        'share.linkedin': 'Share on LinkedIn',
        'share.reddit': 'Share on Reddit',
        'share.bluesky': 'Share on Bluesky',
        'share.mastodon': 'Share on Mastodon',
        'splash.hero.secondary': "See what's inside",
        'splash.benefits.title': 'Why teams choose it',
        'splash.features.title': 'What you can do',
        'splash.trust.title': 'Built for trust',
        'splash.trust.1.title': 'Passwordless sign-in',
        'splash.trust.1.body':
            'A magic link sent to your email: no password to leak or reuse.',
        'splash.trust.2.title': 'Attribute-based permissions',
        'splash.trust.2.body':
            'Fine-grained rules decide who may read, write, merge or delete.',
        'splash.trust.3.title': 'Tamper-evident audit trail',
        'splash.trust.3.body':
            'An append-only history records every change and who made it.',
        'splash.trust.4.title': 'Privacy controls',
        'splash.trust.4.body':
            'Sensitive details are masked unless you are entitled to see them.',
        'splash.trust.5.title': 'Open standards',
        'splash.trust.5.body':
            'REST with OpenAPI, and HL7 FHIR where health systems need it.',
        'splash.trust.6.title': 'Speaks your language',
        'splash.trust.6.body':
            'Arabic, Chinese, English, French, Hindi, Spanish and Welsh.',
        'splash.cta.title': 'Ready to get started?',
        'splash.cta.body':
            'Sign in with a magic link sent to your email. No password needed.',
        'splash.hero.title': 'Know where every folder is',
        'splash.hero.subtitle':
            'Track paper case-note folders by NHS Number across buildings, rooms and cabinets, with a complete audit trail of every move.',
        'splash.benefits.1.title': 'Find any folder fast',
        'splash.benefits.1.body':
            'Look up a paper case-note folder by NHS Number and see where it is right now.',
        'splash.benefits.2.title': 'Fewer lost folders',
        'splash.benefits.2.body':
            'Every move is recorded, so a folder is never just somewhere in the building.',
        'splash.benefits.3.title': 'Less time searching',
        'splash.benefits.3.body':
            'Porters and clinicians spend their time on care, not hunting through cabinets.',
        'splash.benefits.4.title': 'A trustworthy trail',
        'splash.benefits.4.body':
            'A complete, dated history shows who moved each folder, when and why.',
        'splash.benefits.5.title': 'Safe patient identity',
        'splash.benefits.5.body':
            'NHS Numbers are checked before they are saved, so typing mistakes never become records.',
        'splash.benefits.6.title': 'A clear view of the estate',
        'splash.benefits.6.body':
            'See how full each cabinet is and how many folders are in transit.',
        'splash.features.1.title': 'Folder register',
        'splash.features.1.body':
            'Search every folder by patient, title or NHS Number.',
        'splash.features.2.title': 'Move and scan',
        'splash.features.2.body':
            'Record a move in seconds by scanning or entering the NHS Number.',
        'splash.features.3.title': 'Volumes',
        'splash.features.3.body':
            "Group a patient's folders into a volume and move them together.",
        'splash.features.4.title': 'Buildings, rooms, cabinets',
        'splash.features.4.body':
            'Model the physical hierarchy from each building down to its cabinets.',
        'splash.features.5.title': 'Move history',
        'splash.features.5.body':
            'Search the full audit log of every folder movement.',
        'splash.features.6.title': 'Alerts and reports',
        'splash.features.6.body':
            'Review cross-building alerts, cabinet utilisation and headline figures.',
        'nav.tour': 'Tour',
        'splash.hero.tour': 'Take the tour',
        'tour.head': 'Take the tour',
        'tour.toc': 'On this page',
        'tour.open': 'Open this screen',
        'tour.top': 'Back to top',
        'tour.start.title': 'Before you begin',
        'tour.start.summary':
            'You need an account to work with real data. Signing in takes under a minute and needs no password.',
        'tour.start.step.1':
            'Choose Sign in at the top right and enter your email address.',
        'tour.start.step.2':
            'Open the magic link we email you. It works once and expires quickly.',
        'tour.start.step.3':
            'You return to the app signed in, with nothing to remember or reset.',
        'tour.start.step.4':
            'Use the buttons beside Sign in to change the theme, language and text size, or to share the page.',
        'tour.intro':
            'A guided walkthrough of Case Tracking: what each screen does and the steps to use it, from adding a folder to checking the move audit log.',
        'tour.s1.title': 'Add a folder to the register',
        'tour.s1.summary':
            'Every paper case-note folder belongs to one patient and is tracked from the moment you add it.',
        'tour.s1.step.1':
            "Open Folders and choose Add folder, then enter the patient's 10-digit NHS Number, which must pass the Modulus 11 check.",
        'tour.s1.step.2':
            'Type a Folder title such as Volume 1 or Cardiology 2023.',
        'tour.s1.step.3':
            'For a patient who is not yet registered, also fill in Patient name and Date of birth; for an existing patient they are not needed.',
        'tour.s1.step.4':
            'Pick the Initial cabinet, or leave it blank if the folder is in transit, then choose Save folder.',
        'tour.s2.title': 'Find a folder',
        'tour.s2.summary':
            'The Folder register answers "where is this folder right now?" from one search box.',
        'tour.s2.step.1':
            'Open Folders and type an NHS Number, patient name, folder title or cabinet into Search folders.',
        'tour.s2.step.2':
            "Read each row's Cabinet, Status (in cabinet or in transit) and Last moved.",
        'tour.s2.step.3':
            'Open a folder to see its details and Move history, or open the patient from Patients to see all of their folders.',
        'tour.s2.step.4':
            'Choose Move this folder to go straight to recording a move for it.',
        'tour.s3.title': 'Move a folder',
        'tour.s3.summary':
            "Every placement is recorded, so a folder's location is always current.",
        'tour.s3.step.1':
            "Open Move folder, enter the Patient NHS Number, then pick which of that patient's folders you are moving.",
        'tour.s3.step.2':
            'Choose the Destination cabinet, or In transit (porter carrying) if it is on its way.',
        'tour.s3.step.3':
            'Pick a Worker from the list, or type a name under Moved by, and add a Reason.',
        'tour.s3.step.4':
            'Choose Record move; the page confirms Move recorded and the move joins the audit log.',
        'tour.s4.title': 'Scan a folder',
        'tour.s4.summary':
            'The fast path for the records desk: no hardware scanner is needed, though a keyboard-wedge scanner works too.',
        'tour.s4.step.1': 'Open Scan and click into the Scan or search box.',
        'tour.s4.step.2':
            'Scan a barcode, or type an NHS Number or a folder id.',
        'tour.s4.step.3':
            'Review the Matches list and open a folder to see its details.',
        'tour.s4.step.4':
            'Choose Move this folder to record its move with the folder already selected; if nothing matches, the page tells you no folder was found.',
        'tour.s5.title': 'Bundle folders into volumes',
        'tour.s5.summary':
            "A volume is a movable bundle of one patient's folders, so they travel together.",
        'tour.s5.step.1':
            "Open Volumes and choose New volume, then enter the patient's NHS Number and a Volume title; the patient must already have a folder.",
        'tour.s5.step.2':
            "Open the volume and use Add a folder to bundle that patient's folders into it.",
        'tour.s5.step.3':
            'Use Move this volume to relocate every folder inside it to one Destination cabinet in a single step.',
        'tour.s5.step.4':
            'Back on Volumes, choose Print labels, select the volumes, set the Number of Copies and choose Print to queue them.',
        'tour.s6.title': 'Review history, alerts and reports',
        'tour.s6.summary':
            'Every move is kept, so you can answer who moved what, when and why.',
        'tour.s6.step.1':
            'Open Move history and use Filter audit log to narrow it by patient, NHS number, cabinet or porter; the newest moves come first.',
        'tour.s6.step.2':
            "Open a row to see the full move event, the folder involved and the patient's other folders.",
        'tour.s6.step.3':
            'Open Alerts to see geofence alerts: moves whose origin and destination cabinets are in different buildings.',
        'tour.s6.step.4':
            'Open Reports for the at-a-glance counts, cabinet utilisation, folders in transit and activity by worker, all derived live.',
        'signin.sso': 'Sign in with SSO',
    },
    'es-001': {
        'brand.name': 'Seguimiento de casos',
        'brand.tagline': 'Registros en papel del NHS',
        'chrome.language': 'Idioma',
        'chrome.theme': 'Tema',
        'nav.share': 'Compartir',
        'nav.text_size': 'Tamaño del texto',
        'share.copy_link': 'Copiar enlace',
        'share.copied': 'Enlace copiado',
        'share.copy_failed':
            'No se pudo copiar — cópielo desde la barra de direcciones',
        'theme.default': 'Predeterminado',
        'theme.highContrast': 'Alto contraste',
        'nav.toggle': 'Alternar navegación',
        'auth.signedInAs': 'Sesión iniciada como',
        'auth.signOut': 'Cerrar sesión',
        'nav.dashboard': 'Panel',
        'nav.patients': 'Pacientes',
        'nav.folders': 'Carpetas',
        'nav.volumes': 'Volúmenes',
        'nav.workers': 'Trabajadores',
        'nav.buildings': 'Edificios',
        'nav.cabinets': 'Archivadores',
        'nav.move': 'Mover carpeta',
        'nav.scan': 'Escanear',
        'nav.history': 'Historial de movimientos',
        'nav.alerts': 'Alertas',
        'nav.reports': 'Informes',
        'layout.skipToContent': 'Saltar al contenido principal',
        'layout.siteHeader': 'Encabezado del sitio',
        'layout.siteFooter': 'Pie del sitio',
        'layout.primaryNavigation': 'Navegación principal',
        'footer.text':
            'Seguimiento de casos — construido con el Sistema de Diseño Lily (tema NHS) y SVAR Svelte. Solo datos de demostración; no es un registro médico regulado.',
        'common.backToDashboard': 'Volver al panel',
        'common.cancel': 'Cancelar',
        'common.move': 'Mover',
        'common.view': 'Ver',
        'common.remove': 'Eliminar',
        'common.action': 'Acción',
        'common.status': 'Estado',
        'common.patient': 'Paciente',
        'common.folder': 'Carpeta',
        'common.cabinet': 'Archivador',
        'common.title': 'Título',
        'common.name': 'Nombre',
        'common.role': 'Función',
        'common.reason': 'Motivo',
        'common.movedBy': 'Movido por',
        'common.lastMoved': 'Último movimiento',
        'common.nhsNumber': 'Número del NHS',
        'common.dateOfBirth': 'Fecha de nacimiento',
        'common.description': 'Descripción',
        'common.notes': 'Notas',
        'common.entered': 'Entró',
        'common.left': 'Salió',
        'common.when': 'Cuándo',
        'common.from': 'Desde',
        'common.to': 'Hasta',
        'common.source': 'Fuente',
        'common.volume': 'Volumen',
        'common.stillHere': 'Todavía aquí',
        'common.noMovesYet': 'Aún no se han registrado movimientos.',
        'common.inTransitPorter': 'En tránsito (transportado por celador)',
        'common.selectCabinetOption': '— Seleccionar archivador —',
        'common.inTransitOption': '— En tránsito —',
        'status.inCabinet': 'en-archivador',
        'status.inTransit': 'en-tránsito',
        'badge.located': 'Localizado',
        'badge.porterInMotion': 'Celador en movimiento',
        'dashboard.welcomePrefix': 'Bienvenido.',
        'dashboard.inTransit.one':
            'Tiene {n} carpeta en tránsito actualmente. Use la página',
        'dashboard.inTransit.other':
            'Tiene {n} carpetas en tránsito actualmente. Use la página',
        'dashboard.moveFolderPage': 'Mover carpeta',
        'dashboard.pageToRecord': 'para registrar una ubicación.',
        'dashboard.folderSummary': 'Resumen de carpetas',
        'dashboard.patients': 'Pacientes',
        'dashboard.foldersTracked': '{n} carpetas rastreadas',
        'dashboard.inCabinet': 'En archivador',
        'dashboard.inTransitCard': 'En tránsito',
        'dashboard.buildings': 'Edificios',
        'dashboard.roomsCabinets': '{rooms} salas · {cabinets} archivadores',
        'dashboard.moves24h': 'Movimientos (24h)',
        'dashboard.auditedPlacements': 'Ubicaciones de carpetas auditadas',
        'dashboard.folderRegister': 'Registro de carpetas',
        'dashboard.viewAll': 'Ver todo',
        'dashboard.addFolder': 'Añadir carpeta',
        'dashboard.recentMoves': 'Movimientos recientes',
        'dashboard.seeFullHistory': 'Ver historial de auditoría completo →',
        'dashboard.cabinetUtilisation': 'Uso de archivadores',
        'error.backToDashboard': 'Volver al panel',
        'error.heading': 'Error de la API de Seguimiento de casos',
        'error.unknown': 'Error desconocido',
        'error.apiHint':
            'La aplicación se comunica con la API JSON de Loco a través de /api (con proxy al servidor Loco en desarrollo). Asegúrese de que esa API esté en ejecución. Consulte README.md § "Quick start".',
        'login.title': 'Iniciar sesión',
        'login.intro':
            'Introduzca su correo electrónico de trabajo. Si se reconoce, le enviaremos un enlace de inicio de sesión de un solo uso. No se requiere contraseña.',
        'login.sendError': 'No se pudo enviar el enlace de inicio de sesión',
        'login.enterEmail': 'Introduzca su correo electrónico.',
        'login.checkEmail': 'Revise su correo electrónico',
        'login.sentBody':
            'coincide con una cuenta conocida, un enlace de inicio de sesión está en camino. El enlace caduca en 10 minutos.',
        'login.sentPrefix': 'Si',
        'login.devShortcut': 'Atajo de desarrollo:',
        'login.openLink': 'abra su enlace de inicio de sesión',
        'login.formLabel': 'Iniciar sesión',
        'login.emailLabel': 'Correo electrónico',
        'login.submit': 'Envíenme un enlace de inicio de sesión',
        'callback.title': 'Iniciando su sesión',
        'callback.error': 'No se pudo usar el enlace de inicio de sesión',
        'callback.backToSignIn': 'Volver a iniciar sesión',
        'callback.completing': 'Un momento — completando su inicio de sesión…',
        'scan.heading': 'Escanear una carpeta',
        'scan.intro':
            'Escanee un código de barras o escriba un Número del NHS (o id de carpeta) para ir directamente a una carpeta y registrar su movimiento — la vía rápida Scan4Safety. No se necesita escáner de hardware; un escáner tipo teclado escribe en el cuadro de abajo.',
        'scan.failed': 'Error al escanear',
        'scan.formLabel': 'Escanear',
        'scan.fieldLabel': 'Escanear o buscar',
        'scan.fieldDescription':
            'Número del NHS (p. ej. 943 476 5919) o un id de carpeta.',
        'scan.placeholder': 'Escanear o escribir…',
        'scan.matches': 'Coincidencias ({n})',
        'scan.moveThisFolder': 'Mover esta carpeta',
        'scan.noFolderFound': 'No se encontró ninguna carpeta para “{term}”.',
        'move.heading': 'Mover una carpeta',
        'move.intro':
            'Introduzca el Número del NHS de un paciente, elija la carpeta que mueve, luego elija el archivador de destino (o márquela como en tránsito).',
        'move.recorded': 'Movimiento registrado',
        'move.formLabel': 'Mover carpeta',
        'move.patientNhs': 'Número del NHS del paciente',
        'move.invalidNhs': 'Introduzca un Número del NHS válido de 10 dígitos.',
        'move.folder': 'Carpeta',
        'move.selectFolderError': 'Seleccione qué carpeta mover.',
        'move.pickFolderDescription':
            'Elija cuál de las carpetas de este paciente mover.',
        'move.enterNhsDescription':
            'Introduzca un Número del NHS para ver carpetas.',
        'move.selectFolderOption': '— Seleccionar carpeta —',
        'move.destination': 'Destino',
        'move.workerLabel':
            'Trabajador (del Servicio Principal de Trabajadores)',
        'move.workerDescription':
            'Elija un trabajador registrado, o déjelo en blanco para usar el campo de texto libre de abajo.',
        'move.freeTextOnly': '— Solo texto libre —',
        'move.movedByLabel': 'Movido por (texto libre)',
        'move.movedByDescription':
            'Se usa cuando no se selecciona ningún trabajador.',
        'move.movedByPlaceholder': 'p. ej. Alice (celador)',
        'move.reasonPlaceholder': 'p. ej. Cita ambulatoria',
        'move.recordMove': 'Registrar movimiento',
        'move.patientFolders': 'Carpetas del paciente',
        'move.enterValidNhs':
            'Introduzca un Número del NHS válido para ver las carpetas de este paciente.',
        'move.folderNotFound': 'Carpeta no encontrada.',
        'move.recordedSummary':
            'Movimiento registrado de {patient} — {folder} de {from} a {to}.',
        'folders.register': 'Registro de carpetas',
        'folders.searchPlaceholder':
            'Buscar por Número del NHS, paciente, título de carpeta o archivador',
        'folders.searchLabel': 'Buscar carpetas',
        'folders.addFolder': 'Añadir carpeta',
        'folders.tableLabel': 'Carpetas',
        'folders.tableCaption':
            'Todas las carpetas de notas de caso en papel rastreadas por el sistema',
        'folders.colNhsNumber': 'Número del NHS',
        'folders.colPatient': 'Paciente',
        'folders.colFolder': 'Carpeta',
        'folders.colCabinet': 'Archivador',
        'folders.colStatus': 'Estado',
        'folders.colLastMoved': 'Último movimiento',
        'folders.colAction': 'Acción',
        'folders.noMatch': 'Ninguna carpeta coincide con',
        'folderNew.backToFolders': 'Volver a carpetas',
        'folderNew.heading': 'Añadir una carpeta nueva',
        'folderNew.intro':
            'Una carpeta pertenece a un paciente. Si el paciente aún no está registrado en el Servicio Principal de Pacientes, lo crearemos; de lo contrario, la nueva carpeta se adjunta al registro del paciente existente.',
        'folderNew.cannotSave': 'No se puede guardar la carpeta',
        'folderNew.invalidNhs':
            'Introduzca un Número del NHS válido de 10 dígitos (falló la comprobación Módulo 11).',
        'folderNew.titleRequired': 'El título de la carpeta es obligatorio.',
        'folderNew.formLabel': 'Añadir carpeta',
        'folderNew.nhsDescription': '10 dígitos, con formato XXX XXX XXXX.',
        'folderNew.titleLabel': 'Título de la carpeta',
        'folderNew.titleDescription': 'p. ej. Volumen 1, Cardiología 2023',
        'folderNew.patientName': 'Nombre del paciente',
        'folderNew.patientNameDescription':
            'Solo necesario para un paciente nuevo.',
        'folderNew.dobDescription': 'Solo necesario para un paciente nuevo.',
        'folderNew.initialCabinet': 'Archivador inicial',
        'folderNew.initialCabinetDescription':
            'Deje en blanco si la carpeta está en tránsito.',
        'folderNew.saveFolder': 'Guardar carpeta',
        'folderDetail.backToFolders': 'Volver a carpetas',
        'folderDetail.patientPrefix': 'Paciente:',
        'folderDetail.detailsLabel': 'Detalles de la carpeta',
        'folderDetail.folderTitle': 'Título de la carpeta',
        'folderDetail.currentCabinet': 'Archivador actual',
        'folderDetail.moveThisFolder': 'Mover esta carpeta',
        'folderDetail.moveHistory': 'Historial de movimientos',
        'patients.heading': 'Pacientes',
        'patients.searchPlaceholder': 'Buscar por Número del NHS o nombre',
        'patients.searchLabel': 'Buscar pacientes',
        'patients.tableLabel': 'Pacientes',
        'patients.tableCaption':
            'Todos los pacientes con una o más carpetas registradas',
        'patients.colFolders': 'Carpetas',
        'patients.noMatch': 'Ningún paciente coincide con',
        'patientDetail.backToPatients': 'Volver a pacientes',
        'patientDetail.notFoundHeading':
            'Paciente no encontrado en el Servicio Principal de Pacientes',
        'patientDetail.notFoundBody':
            'No existe ningún registro de paciente para el Número del NHS {nhs}. Las carpetas de abajo se reconstruyen a partir de instantáneas locales escritas cuando se creó cada carpeta.',
        'patientDetail.sourcePrefix': 'Fuente:',
        'patientDetail.recordActions': 'Acciones del registro del paciente',
        'patientDetail.nhsNumberHeading': 'Número del NHS {nhs}',
        'patientDetail.foldersForPatient': 'Carpetas de este paciente ({n})',
        'patientDetail.patientFoldersTable': 'Carpetas del paciente',
        'patientDetail.colVolume': 'Volumen',
        'patientDetail.noFoldersYet': 'Aún no hay carpetas.',
        'patientDetail.addFolderForPatient':
            'Añadir una carpeta para este paciente',
        'patientDetail.moveHistoryForPatient':
            'Historial de movimientos de este paciente',
        'patientDetail.demoUnavailable':
            '“{action}” no está disponible en esta demostración.',
        'buildings.heading': 'Edificios',
        'buildings.addBuilding': 'Añadir edificio',
        'buildings.tableLabel': 'Edificios',
        'buildings.tableCaption':
            'Sitios físicos que albergan salas de registros',
        'buildings.colRooms': 'Salas',
        'buildings.noBuildings': 'Aún no hay edificios.',
        'buildingNew.backToBuildings': 'Volver a edificios',
        'buildingNew.heading': 'Añadir un edificio',
        'buildingNew.intro':
            'Un edificio puede tener muchas salas; cada sala puede albergar muchos archivadores.',
        'buildingNew.cannotSave': 'No se puede guardar el edificio',
        'buildingNew.nameRequired': 'El nombre del edificio es obligatorio.',
        'buildingNew.formLabel': 'Añadir edificio',
        'buildingNew.nameLabel': 'Nombre del edificio',
        'buildingNew.saveBuilding': 'Guardar edificio',
        'buildingDetail.backToBuildings': 'Volver a edificios',
        'buildingDetail.rooms': 'Salas ({n})',
        'buildingDetail.roomsTable': 'Salas',
        'buildingDetail.colCabinets': 'Archivadores',
        'buildingDetail.noRooms': 'Aún no hay salas.',
        'buildingDetail.addRoom': 'Añadir una sala',
        'buildingDetail.addRoomLabel': 'Añadir sala',
        'buildingDetail.roomNameRequired':
            'El nombre de la sala es obligatorio.',
        'buildingDetail.roomName': 'Nombre de la sala',
        'buildingDetail.saveRoom': 'Guardar sala',
        'buildingDetail.presenceHistory': 'Historial de presencia de carpetas',
        'buildingDetail.presenceIntro':
            'Carpetas que han estado en cualquier archivador de este edificio, las más recientes primero.',
        'buildingDetail.presenceTable':
            'Historial de presencia de carpetas del edificio',
        'buildingDetail.presenceCaption':
            'Agregado entre los archivadores de este edificio',
        'buildingDetail.noPresence':
            'Aún no se ha registrado presencia de carpetas en este edificio.',
        'cabinets.heading': 'Archivadores',
        'cabinets.addCabinet': 'Añadir archivador',
        'cabinets.tableLabel': 'Archivadores',
        'cabinets.tableCaption':
            'Archivadores físicos, su edificio/sala y ocupación',
        'cabinets.colLabel': 'Etiqueta',
        'cabinets.colBuilding': 'Edificio',
        'cabinets.colRoom': 'Sala',
        'cabinets.colCapacity': 'Capacidad',
        'cabinets.colFolders': 'Carpetas',
        'cabinets.colUtilisation': 'Uso',
        'cabinetNew.backToCabinets': 'Volver a archivadores',
        'cabinetNew.heading': 'Añadir un archivador',
        'cabinetNew.intro':
            'Un archivador vive dentro de una sala (que vive dentro de un edificio).',
        'cabinetNew.noRoomsHeading': 'Aún no existen salas',
        'cabinetNew.noRoomsBody':
            'primero, luego añada una sala desde la página del edificio.',
        'cabinetNew.noRoomsBuilding': 'edificio',
        'cabinetNew.noRoomsCreate': 'Cree un',
        'cabinetNew.cannotSave': 'No se puede guardar el archivador',
        'cabinetNew.labelRequired':
            'La etiqueta del archivador es obligatoria.',
        'cabinetNew.roomRequired': 'Seleccione una sala.',
        'cabinetNew.formLabel': 'Añadir archivador',
        'cabinetNew.labelLabel': 'Etiqueta del archivador',
        'cabinetNew.labelPlaceholder': 'p. ej. Archivador D3',
        'cabinetNew.roomLabel': 'Sala',
        'cabinetNew.selectRoomOption': '— Seleccionar una sala —',
        'cabinetNew.capacityLabel': 'Capacidad',
        'cabinetNew.capacityDescription':
            'Número aproximado de carpetas que contiene el archivador. Deje en blanco si se desconoce.',
        'cabinetNew.saveCabinet': 'Guardar archivador',
        'cabinetDetail.backToCabinets': 'Volver a archivadores',
        'cabinetDetail.currentFolders':
            'Carpetas actualmente en este archivador ({n})',
        'cabinetDetail.currentFoldersTable': 'Carpetas actuales',
        'cabinetDetail.empty': 'Este archivador está actualmente vacío.',
        'cabinetDetail.presenceHistory': 'Historial de presencia de carpetas',
        'cabinetDetail.presenceIntro':
            'Qué carpetas han estado en este archivador, y cuándo. Las más recientes primero.',
        'cabinetDetail.presenceTable': 'Historial de presencia de carpetas',
        'cabinetDetail.presenceCaption':
            'Derivado del registro de auditoría de movimientos',
        'cabinetDetail.colReasonLeft': 'Motivo de salida',
        'cabinetDetail.noPresence':
            'Aún no se ha registrado ninguna carpeta en este archivador.',
        'roomDetail.backToBuildings': 'Volver a edificios',
        'roomDetail.presenceHistory': 'Historial de presencia de carpetas',
        'roomDetail.presenceIntro':
            'Carpetas que han estado en cualquier archivador de esta sala, las más recientes primero.',
        'roomDetail.presenceTable':
            'Historial de presencia de carpetas de la sala',
        'roomDetail.presenceCaption':
            'Agregado entre los archivadores de esta sala',
        'roomDetail.noPresence':
            'Aún no se ha registrado presencia de carpetas en esta sala.',
        'volumes.heading': 'Volúmenes',
        'volumes.printLabels': 'Imprimir etiquetas',
        'volumes.newVolume': 'Nuevo volumen',
        'volumes.intro':
            'Un volumen es un paquete móvil de las carpetas de un paciente. Mueva el volumen y todas las carpetas dentro de él se mueven juntas.',
        'volumes.tableLabel': 'Volúmenes',
        'volumes.tableCaption':
            'Paquetes de carpetas, cada uno perteneciente a un paciente',
        'volumes.colFolders': 'Carpetas',
        'volumes.colLocation': 'Ubicación',
        'volumes.noVolumes':
            'Aún no hay volúmenes. Cree uno para agrupar las carpetas de un paciente.',
        'volumes.noLabelsSelected': 'No se han seleccionado etiquetas.',
        'volumes.queued.one':
            '{n} etiqueta × {copies} {copy} en cola para imprimir.',
        'volumes.queued.other':
            '{n} etiquetas × {copies} {copy} en cola para imprimir.',
        'volumes.copy': 'copia',
        'volumes.copies': 'copias',
        'volumeNew.backToVolumes': 'Volver a volúmenes',
        'volumeNew.heading': 'Nuevo volumen',
        'volumeNew.intro':
            'Un volumen agrupa carpetas de un paciente. El paciente ya debe estar registrado (cree una carpeta para él primero). Puede añadir carpetas al volumen una vez que exista.',
        'volumeNew.cannotCreate': 'No se puede crear el volumen',
        'volumeNew.invalidNhs':
            'Introduzca un Número del NHS válido de 10 dígitos (falló la comprobación Módulo 11).',
        'volumeNew.titleRequired': 'El título del volumen es obligatorio.',
        'volumeNew.formLabel': 'Nuevo volumen',
        'volumeNew.patientNhs': 'Número del NHS del paciente',
        'volumeNew.nhsDescription': '10 dígitos, con formato XXX XXX XXXX.',
        'volumeNew.titleLabel': 'Título del volumen',
        'volumeNew.titleDescription': 'p. ej. Alice Johnson — Vol 1',
        'volumeNew.initialCabinet': 'Archivador inicial',
        'volumeNew.initialCabinetDescription':
            'Dónde vive el volumen. Deje en blanco si está en tránsito.',
        'volumeNew.createVolume': 'Crear volumen',
        'volumeDetail.backToVolumes': 'Volver a volúmenes',
        'volumeDetail.somethingWrong': 'Algo salió mal',
        'volumeDetail.foldersIn': 'Carpetas en este volumen ({n})',
        'volumeDetail.foldersTable': 'Carpetas del volumen',
        'volumeDetail.noFolders': 'Aún no hay carpetas en este volumen.',
        'volumeDetail.addFolderLabel': 'Añadir una carpeta',
        'volumeDetail.addFolderFor': 'Añadir una carpeta para {patient}',
        'volumeDetail.chooseFolder': '— Elegir una carpeta —',
        'volumeDetail.addToVolume': 'Añadir al volumen',
        'volumeDetail.renameVolume': 'Renombrar volumen',
        'volumeDetail.rename': 'Renombrar',
        'volumeDetail.moveVolume': 'Mover este volumen',
        'volumeDetail.moveIntro':
            'Reubica todas las carpetas del volumen juntas.',
        'volumeDetail.moveFormLabel': 'Mover volumen',
        'volumeDetail.destinationCabinet': 'Archivador de destino',
        'volumeDetail.moveReasonPlaceholder': 'p. ej. Clínica ambulatoria',
        'volumeDetail.moveVolumeButton': 'Mover volumen',
        'volumeDetail.moveHistory': 'Historial de movimientos',
        'workers.heading': 'Trabajadores',
        'workers.intro':
            'Personal que mueve carpetas. Abra un trabajador para ver las carpetas que ha movido y todas las carpetas de sus pacientes.',
        'workers.tableLabel': 'Trabajadores',
        'workers.tableCaption':
            'Trabajadores del Servicio Principal de Trabajadores',
        'workers.noWorkers': 'No se encontraron trabajadores.',
        'workerDetail.backToWorkers': 'Volver a trabajadores',
        'workerDetail.foldersMoved':
            'Carpetas movidas por este trabajador ({n})',
        'workerDetail.foldersMovedTable':
            'Carpetas movidas por este trabajador',
        'workerDetail.noMovedFolders':
            'Este trabajador aún no ha movido ninguna carpeta.',
        'workerDetail.patientsFolders':
            'Todas las carpetas de sus pacientes ({n})',
        'workerDetail.patientsFoldersIntro':
            'Todas las carpetas pertenecientes a un paciente que este trabajador ha gestionado.',
        'workerDetail.patientsFoldersTable':
            'Carpetas de los pacientes de este trabajador',
        'workerDetail.noPatientFolders':
            'Aún no hay carpetas de pacientes para mostrar.',
        'workerDetail.movesByWorker': 'Movimientos de este trabajador',
        'history.backToDashboard': 'Volver al panel',
        'history.heading': 'Historial de movimientos (registro de auditoría)',
        'history.filterPlaceholder':
            'Filtrar por paciente, número del NHS, archivador o celador',
        'history.filterLabel': 'Filtrar registro de auditoría',
        'history.tableLabel': 'Registro de auditoría de movimientos',
        'history.tableCaption':
            'Cada movimiento registrado de una carpeta de papel, los más recientes primero',
        'history.noMatch': 'Ningún movimiento coincide con su filtro.',
        'historyDetail.backToHistory': 'Volver al historial de movimientos',
        'historyDetail.heading': 'Evento de movimiento',
        'historyDetail.folderInvolved': 'Carpeta implicada',
        'historyDetail.detailsLabel': 'Detalles del evento de movimiento',
        'historyDetail.otherFolders': 'Otras carpetas de {patient}',
        'historyDetail.noOtherFolders':
            'No hay otras carpetas para este paciente.',
        'reports.backToDashboard': 'Volver al panel',
        'reports.heading': 'Informes',
        'reports.atAGlance': 'De un vistazo',
        'reports.kpiPatients': 'Pacientes:',
        'reports.kpiFolders': 'Carpetas:',
        'reports.kpiFoldersDetail':
            '({inCabinet} en archivador, {inTransit} en tránsito)',
        'reports.kpiVolumes': 'Volúmenes:',
        'reports.kpiCabinets': 'Archivadores:',
        'reports.kpiCabinetsDetail': 'en {n} edificios',
        'reports.kpiMoves': 'Movimientos — últimas 24h:',
        'reports.kpiMoves7d': 'últimos 7d:',
        'reports.cabinetUtilisation': 'Uso de archivadores',
        'reports.colCabinet': 'Archivador',
        'reports.colFolders': 'Carpetas',
        'reports.colCapacity': 'Capacidad',
        'reports.colUtilisation': 'Uso',
        'reports.inTransit': 'En tránsito ({n})',
        'reports.noInTransit': 'No hay carpetas en tránsito.',
        'reports.activityByWorker': 'Actividad por trabajador',
        'reports.colWorker': 'Trabajador',
        'reports.colMoves': 'Movimientos',
        'reports.noMovesYet': 'Aún no se han registrado movimientos.',
        'reports.derivedLive':
            'Los informes se derivan en vivo de la API — sin almacén de informes separado.',
        'alerts.backToDashboard': 'Volver al panel',
        'alerts.heading': 'Alertas de geocerca',
        'alerts.intro':
            'Notas de caso que cruzaron un límite de edificio. Una infracción de geocerca es cualquier movimiento cuyos archivadores de origen y destino están en edificios diferentes.',
        'alerts.tableLabel': 'Alertas de geocerca',
        'alerts.tableCaption':
            'Movimientos que cruzan límites, los más recientes primero',
        'alerts.colCrossed': 'Cruzó',
        'alerts.none':
            'Sin infracciones de geocerca — cada carpeta ha permanecido dentro de su edificio.',
        'grid.nhsNumber': 'Número del NHS',
        'grid.patient': 'Paciente',
        'grid.folder': 'Carpeta',
        'grid.cabinet': 'Archivador',
        'grid.status': 'Estado',
        'grid.lastMoved': 'Último movimiento',
        'addressograph.label': 'Etiqueta del paciente',
        'addressograph.nhsNo': 'N.º NHS',
        'addressograph.dob': 'F. nac.',
        'addressograph.sex': 'Sexo',
        'addressograph.address': 'Dirección',
        'buttonBar.label': 'Acciones',
        'buttonBar.patient': 'Paciente',
        'buttonBar.referrals': 'Derivaciones',
        'buttonBar.activate': 'Activar',
        'buttonBar.caseNotes': 'Notas de caso',
        'buttonBar.pathways': 'Vías',
        'buttonBar.legalStatus': 'Estado legal',
        'buttonBar.documents': 'Documentos',
        'buttonBar.wrapper': 'Envoltorio',
        'buttonBar.audit': 'Auditoría',
        'buttonBar.quickReports': 'Informes rápidos',
        'labels.title': 'Etiquetas',
        'labels.searchLabel': 'Buscar etiquetas',
        'labels.searchPlaceholder': 'Introduzca texto para buscar...',
        'labels.find': 'Buscar',
        'labels.clear': 'Limpiar',
        'labels.volumeTitles': 'Títulos de volúmenes',
        'labels.noMatching': 'No hay volúmenes coincidentes.',
        'labels.numberOfCopies': 'Número de copias:',
        'labels.copiesLabel': 'Número de copias',
        'labels.print': 'Imprimir',
        'labels.close': 'Cerrar',
        'auth.signin': 'Iniciar sesión',
        'auth.signout': 'Cerrar sesión',
        'share.email': 'Enviar enlace por correo',
        'share.linkedin': 'Compartir en LinkedIn',
        'share.reddit': 'Compartir en Reddit',
        'share.bluesky': 'Compartir en Bluesky',
        'share.mastodon': 'Compartir en Mastodon',
        'splash.hero.secondary': 'Descubre qué incluye',
        'splash.benefits.title': 'Por qué la eligen los equipos',
        'splash.features.title': 'Qué puedes hacer',
        'splash.trust.title': 'Diseñado para la confianza',
        'splash.trust.1.title': 'Acceso sin contraseña',
        'splash.trust.1.body':
            'Un enlace mágico enviado a tu correo: sin contraseñas que filtrar ni reutilizar.',
        'splash.trust.2.title': 'Permisos basados en atributos',
        'splash.trust.2.body':
            'Reglas detalladas deciden quién puede leer, escribir, fusionar o eliminar.',
        'splash.trust.3.title': 'Auditoría a prueba de manipulación',
        'splash.trust.3.body':
            'Un historial de solo anexado registra cada cambio y quién lo hizo.',
        'splash.trust.4.title': 'Controles de privacidad',
        'splash.trust.4.body':
            'Los datos sensibles se ocultan salvo que tengas derecho a verlos.',
        'splash.trust.5.title': 'Estándares abiertos',
        'splash.trust.5.body':
            'REST con OpenAPI y HL7 FHIR donde lo necesitan los sistemas de salud.',
        'splash.trust.6.title': 'Habla tu idioma',
        'splash.trust.6.body':
            'Árabe, chino, español, francés, galés, hindi e inglés.',
        'splash.cta.title': '¿Listo para empezar?',
        'splash.cta.body':
            'Inicia sesión con un enlace mágico enviado a tu correo. No necesitas contraseña.',
        'splash.hero.title': 'Sepa dónde está cada carpeta',
        'splash.hero.subtitle':
            'Sigue las carpetas de notas de casos en papel por número del NHS entre edificios, salas y archivadores, con un registro de auditoría completo de cada movimiento.',
        'splash.benefits.1.title': 'Encuentra cualquier carpeta al instante',
        'splash.benefits.1.body':
            'Busca una carpeta de notas en papel por número del NHS y ve dónde está ahora mismo.',
        'splash.benefits.2.title': 'Menos carpetas perdidas',
        'splash.benefits.2.body':
            'Cada movimiento queda registrado, así que una carpeta nunca está simplemente en algún lugar del edificio.',
        'splash.benefits.3.title': 'Menos tiempo buscando',
        'splash.benefits.3.body':
            'Camilleros y clínicos dedican su tiempo a la atención, no a rebuscar entre archivadores.',
        'splash.benefits.4.title': 'Un rastro fiable',
        'splash.benefits.4.body':
            'Un historial completo y fechado muestra quién movió cada carpeta, cuándo y por qué.',
        'splash.benefits.5.title': 'Identidad del paciente segura',
        'splash.benefits.5.body':
            'Los números del NHS se validan antes de guardarse, así que los errores de tecleo nunca se convierten en registros.',
        'splash.benefits.6.title': 'Una visión clara de las instalaciones',
        'splash.benefits.6.body':
            'Consulta cuán lleno está cada archivador y cuántas carpetas están en tránsito.',
        'splash.features.1.title': 'Registro de carpetas',
        'splash.features.1.body':
            'Busca cualquier carpeta por paciente, título o número del NHS.',
        'splash.features.2.title': 'Mover y escanear',
        'splash.features.2.body':
            'Registra un movimiento en segundos escaneando o escribiendo el número del NHS.',
        'splash.features.3.title': 'Volúmenes',
        'splash.features.3.body':
            'Agrupa las carpetas de un paciente en un volumen y muévelas juntas.',
        'splash.features.4.title': 'Edificios, salas, archivadores',
        'splash.features.4.body':
            'Modela la jerarquía física desde cada edificio hasta sus archivadores.',
        'splash.features.5.title': 'Historial de movimientos',
        'splash.features.5.body':
            'Busca en el registro de auditoría completo de cada movimiento de carpetas.',
        'splash.features.6.title': 'Alertas e informes',
        'splash.features.6.body':
            'Revisa las alertas entre edificios, el uso de los archivadores y las cifras clave.',
        'nav.tour': 'Recorrido',
        'splash.hero.tour': 'Haz el recorrido',
        'tour.head': 'Haz el recorrido',
        'tour.toc': 'En esta página',
        'tour.open': 'Abrir esta pantalla',
        'tour.top': 'Volver arriba',
        'tour.start.title': 'Antes de empezar',
        'tour.start.summary':
            'Necesitas una cuenta para trabajar con datos reales. Iniciar sesión lleva menos de un minuto y no requiere contraseña.',
        'tour.start.step.1':
            'Elige Iniciar sesión arriba a la derecha e introduce tu correo electrónico.',
        'tour.start.step.2':
            'Abre el enlace mágico que te enviamos por correo. Funciona una sola vez y caduca pronto.',
        'tour.start.step.3':
            'Vuelves a la aplicación con la sesión iniciada, sin nada que recordar ni restablecer.',
        'tour.start.step.4':
            'Usa los botones junto a Iniciar sesión para cambiar el tema, el idioma y el tamaño del texto, o para compartir la página.',
        'tour.intro':
            'Un recorrido guiado por Seguimiento de casos: qué hace cada pantalla y los pasos para usarla, desde añadir una carpeta hasta consultar el registro de auditoría de movimientos.',
        'tour.s1.title': 'Añadir una carpeta al registro',
        'tour.s1.summary':
            'Cada carpeta de papel con notas del caso pertenece a un paciente y se rastrea desde el momento en que la añade.',
        'tour.s1.step.1':
            'Abra Carpetas y elija Añadir carpeta; después introduzca el Número del NHS de 10 dígitos del paciente, que debe superar la comprobación de módulo 11.',
        'tour.s1.step.2':
            'Escriba un Título de la carpeta, como Volumen 1 o Cardiología 2023.',
        'tour.s1.step.3':
            'Si el paciente aún no está registrado, rellene también Nombre del paciente y Fecha de nacimiento; para un paciente existente no hacen falta.',
        'tour.s1.step.4':
            'Elija el Archivador inicial, o déjelo en blanco si la carpeta está en tránsito, y pulse Guardar carpeta.',
        'tour.s2.title': 'Encontrar una carpeta',
        'tour.s2.summary':
            'El Registro de carpetas responde a «¿dónde está esta carpeta ahora?» desde un único cuadro de búsqueda.',
        'tour.s2.step.1':
            'Abra Carpetas y escriba un Número del NHS, nombre de paciente, título de carpeta o archivador en Buscar carpetas.',
        'tour.s2.step.2':
            'Lea el Archivador, el Estado (en archivador o en tránsito) y el Último movimiento de cada fila.',
        'tour.s2.step.3':
            'Abra una carpeta para ver sus detalles y su Historial de movimientos, o abra al paciente desde Pacientes para ver todas sus carpetas.',
        'tour.s2.step.4':
            'Pulse Mover esta carpeta para ir directamente a registrar un movimiento.',
        'tour.s3.title': 'Mover una carpeta',
        'tour.s3.summary':
            'Cada ubicación queda registrada, de modo que la ubicación de una carpeta siempre está al día.',
        'tour.s3.step.1':
            'Abra Mover carpeta, introduzca el Número del NHS del paciente y elija cuál de sus carpetas va a mover.',
        'tour.s3.step.2':
            'Elija el Archivador de destino, o En tránsito (transportado por celador) si va de camino.',
        'tour.s3.step.3':
            'Elija un Trabajador de la lista, o escriba un nombre en Movido por, y añada un Motivo.',
        'tour.s3.step.4':
            'Pulse Registrar movimiento; la página confirma Movimiento registrado y el movimiento pasa al registro de auditoría.',
        'tour.s4.title': 'Escanear una carpeta',
        'tour.s4.summary':
            'La vía rápida para el servicio de historias: no hace falta un escáner físico, aunque un escáner que escribe como teclado también funciona.',
        'tour.s4.step.1':
            'Abra Escanear y haga clic en el cuadro Escanear o buscar.',
        'tour.s4.step.2':
            'Escanee un código de barras, o escriba un Número del NHS o un id de carpeta.',
        'tour.s4.step.3':
            'Revise la lista de Coincidencias y abra una carpeta para ver sus detalles.',
        'tour.s4.step.4':
            'Pulse Mover esta carpeta para registrar su movimiento con la carpeta ya seleccionada; si no hay coincidencias, la página indica que no se encontró ninguna carpeta.',
        'tour.s5.title': 'Agrupar carpetas en volúmenes',
        'tour.s5.summary':
            'Un volumen es un conjunto movible de las carpetas de un paciente, de modo que viajan juntas.',
        'tour.s5.step.1':
            'Abra Volúmenes y elija Nuevo volumen; introduzca el Número del NHS del paciente y un Título del volumen. El paciente ya debe tener una carpeta.',
        'tour.s5.step.2':
            'Abra el volumen y use Añadir una carpeta para agrupar en él las carpetas de ese paciente.',
        'tour.s5.step.3':
            'Use Mover este volumen para trasladar todas las carpetas que contiene a un único archivador de destino en un solo paso.',
        'tour.s5.step.4':
            'De vuelta en Volúmenes, elija Imprimir etiquetas, seleccione los volúmenes, indique el Número de copias y pulse Imprimir para ponerlas en cola.',
        'tour.s6.title': 'Revisar historial, alertas e informes',
        'tour.s6.summary':
            'Se conserva cada movimiento, así que puede saber quién movió qué, cuándo y por qué.',
        'tour.s6.step.1':
            'Abra Historial de movimientos y use Filtrar registro de auditoría para acotar por paciente, número del NHS, archivador o celador; los movimientos más recientes van primero.',
        'tour.s6.step.2':
            'Abra una fila para ver el evento de movimiento completo, la carpeta implicada y las demás carpetas del paciente.',
        'tour.s6.step.3':
            'Abra Alertas para ver las alertas de geocerca: movimientos cuyo archivador de origen y de destino están en edificios distintos.',
        'tour.s6.step.4':
            'Abra Informes para ver los recuentos de un vistazo, la utilización de archivadores, las carpetas en tránsito y la actividad por trabajador, todo derivado en vivo.',
        'signin.sso': 'Iniciar sesión con SSO',
    },
    'fr-001': {
        'brand.name': 'Suivi des dossiers',
        'brand.tagline': 'Dossiers papier du NHS',
        'chrome.language': 'Langue',
        'chrome.theme': 'Thème',
        'nav.share': 'Partager',
        'nav.text_size': 'Taille du texte',
        'share.copy_link': 'Copier le lien',
        'share.copied': 'Lien copié',
        'share.copy_failed':
            "Impossible de copier — copiez-le depuis la barre d'adresse",
        'theme.default': 'Par défaut',
        'theme.highContrast': 'Contraste élevé',
        'nav.toggle': 'Basculer la navigation',
        'auth.signedInAs': 'Connecté en tant que',
        'auth.signOut': 'Se déconnecter',
        'nav.dashboard': 'Tableau de bord',
        'nav.patients': 'Patients',
        'nav.folders': 'Dossiers',
        'nav.volumes': 'Volumes',
        'nav.workers': 'Personnel',
        'nav.buildings': 'Bâtiments',
        'nav.cabinets': 'Armoires',
        'nav.move': 'Déplacer un dossier',
        'nav.scan': 'Scanner',
        'nav.history': 'Historique des déplacements',
        'nav.alerts': 'Alertes',
        'nav.reports': 'Rapports',
        'layout.skipToContent': 'Aller au contenu principal',
        'layout.siteHeader': 'En-tête du site',
        'layout.siteFooter': 'Pied de page du site',
        'layout.primaryNavigation': 'Navigation principale',
        'footer.text':
            'Suivi des dossiers — construit avec le Lily Design System (thème NHS) et SVAR Svelte. Données de démonstration uniquement ; pas un dossier médical réglementé.',
        'common.backToDashboard': 'Retour au tableau de bord',
        'common.cancel': 'Annuler',
        'common.move': 'Déplacer',
        'common.view': 'Voir',
        'common.remove': 'Retirer',
        'common.action': 'Action',
        'common.status': 'Statut',
        'common.patient': 'Patient',
        'common.folder': 'Dossier',
        'common.cabinet': 'Armoire',
        'common.title': 'Titre',
        'common.name': 'Nom',
        'common.role': 'Rôle',
        'common.reason': 'Motif',
        'common.movedBy': 'Déplacé par',
        'common.lastMoved': 'Dernier déplacement',
        'common.nhsNumber': 'Numéro NHS',
        'common.dateOfBirth': 'Date de naissance',
        'common.description': 'Description',
        'common.notes': 'Notes',
        'common.entered': 'Entré',
        'common.left': 'Sorti',
        'common.when': 'Quand',
        'common.from': 'De',
        'common.to': 'À',
        'common.source': 'Source',
        'common.volume': 'Volume',
        'common.stillHere': 'Toujours ici',
        'common.noMovesYet': 'Aucun déplacement enregistré pour le moment.',
        'common.inTransitPorter': 'En transit (porté par un brancardier)',
        'common.selectCabinetOption': '— Sélectionner une armoire —',
        'common.inTransitOption': '— En transit —',
        'status.inCabinet': 'en-armoire',
        'status.inTransit': 'en-transit',
        'badge.located': 'Localisé',
        'badge.porterInMotion': 'Brancardier en mouvement',
        'dashboard.welcomePrefix': 'Bienvenue.',
        'dashboard.inTransit.one':
            'Vous avez {n} dossier actuellement en transit. Utilisez la page',
        'dashboard.inTransit.other':
            'Vous avez {n} dossiers actuellement en transit. Utilisez la page',
        'dashboard.moveFolderPage': 'Déplacer un dossier',
        'dashboard.pageToRecord': 'pour enregistrer un emplacement.',
        'dashboard.folderSummary': 'Résumé des dossiers',
        'dashboard.patients': 'Patients',
        'dashboard.foldersTracked': '{n} dossiers suivis',
        'dashboard.inCabinet': 'En armoire',
        'dashboard.inTransitCard': 'En transit',
        'dashboard.buildings': 'Bâtiments',
        'dashboard.roomsCabinets': '{rooms} salles · {cabinets} armoires',
        'dashboard.moves24h': 'Déplacements (24 h)',
        'dashboard.auditedPlacements': 'Emplacements de dossiers audités',
        'dashboard.folderRegister': 'Registre des dossiers',
        'dashboard.viewAll': 'Tout voir',
        'dashboard.addFolder': 'Ajouter un dossier',
        'dashboard.recentMoves': 'Déplacements récents',
        'dashboard.seeFullHistory': "Voir l'historique d'audit complet →",
        'dashboard.cabinetUtilisation': 'Utilisation des armoires',
        'error.backToDashboard': 'Retour au tableau de bord',
        'error.heading': "Erreur de l'API Suivi des dossiers",
        'error.unknown': 'Erreur inconnue',
        'error.apiHint':
            "L'application communique avec l'API JSON Loco via /api (proxy vers le serveur Loco en développement). Assurez-vous que cette API est en cours d'exécution. Voir README.md § \"Quick start\".",
        'login.title': 'Se connecter',
        'login.intro':
            'Saisissez votre adresse e-mail professionnelle. Si elle est reconnue, nous vous enverrons un lien de connexion à usage unique. Aucun mot de passe requis.',
        'login.sendError': "Impossible d'envoyer le lien de connexion",
        'login.enterEmail': 'Saisissez votre adresse e-mail.',
        'login.checkEmail': 'Vérifiez votre e-mail',
        'login.sentBody':
            'correspond à un compte connu, un lien de connexion est en route. Le lien expire dans 10 minutes.',
        'login.sentPrefix': 'Si',
        'login.devShortcut': 'Raccourci de développement :',
        'login.openLink': 'ouvrez votre lien de connexion',
        'login.formLabel': 'Se connecter',
        'login.emailLabel': 'Adresse e-mail',
        'login.submit': 'Envoyez-moi un lien de connexion',
        'callback.title': 'Connexion en cours',
        'callback.error': "Le lien de connexion n'a pas pu être utilisé",
        'callback.backToSignIn': 'Retour à la connexion',
        'callback.completing': 'Un instant — finalisation de votre connexion…',
        'scan.heading': 'Scanner un dossier',
        'scan.intro':
            'Scannez un code-barres ou saisissez un Numéro NHS (ou un id de dossier) pour accéder directement à un dossier et enregistrer son déplacement — la voie rapide Scan4Safety. Aucun scanner matériel nécessaire ; un scanner clavier saisit dans le champ ci-dessous.',
        'scan.failed': 'Échec du scan',
        'scan.formLabel': 'Scanner',
        'scan.fieldLabel': 'Scanner ou rechercher',
        'scan.fieldDescription':
            'Numéro NHS (p. ex. 943 476 5919) ou un id de dossier.',
        'scan.placeholder': 'Scanner ou saisir…',
        'scan.matches': 'Correspondances ({n})',
        'scan.moveThisFolder': 'Déplacer ce dossier',
        'scan.noFolderFound': 'Aucun dossier trouvé pour « {term} ».',
        'move.heading': 'Déplacer un dossier',
        'move.intro':
            "Saisissez le Numéro NHS d'un patient, choisissez le dossier que vous déplacez, puis choisissez l'armoire de destination (ou marquez-le en transit).",
        'move.recorded': 'Déplacement enregistré',
        'move.formLabel': 'Déplacer un dossier',
        'move.patientNhs': 'Numéro NHS du patient',
        'move.invalidNhs': 'Saisissez un Numéro NHS valide à 10 chiffres.',
        'move.folder': 'Dossier',
        'move.selectFolderError': 'Sélectionnez le dossier à déplacer.',
        'move.pickFolderDescription':
            'Choisissez lequel des dossiers de ce patient déplacer.',
        'move.enterNhsDescription':
            'Saisissez un Numéro NHS pour voir les dossiers.',
        'move.selectFolderOption': '— Sélectionner un dossier —',
        'move.destination': 'Destination',
        'move.workerLabel': 'Personnel (du Service principal du personnel)',
        'move.workerDescription':
            'Choisissez un membre du personnel enregistré, ou laissez vide pour utiliser le champ de texte libre ci-dessous.',
        'move.freeTextOnly': '— Texte libre uniquement —',
        'move.movedByLabel': 'Déplacé par (texte libre)',
        'move.movedByDescription':
            "Utilisé lorsqu'aucun membre du personnel n'est sélectionné.",
        'move.movedByPlaceholder': 'p. ex. Alice (brancardier)',
        'move.reasonPlaceholder': 'p. ex. Consultation externe',
        'move.recordMove': 'Enregistrer le déplacement',
        'move.patientFolders': 'Dossiers du patient',
        'move.enterValidNhs':
            'Saisissez un Numéro NHS valide pour voir les dossiers de ce patient.',
        'move.folderNotFound': 'Dossier introuvable.',
        'move.recordedSummary':
            'Déplacement enregistré de {patient} — {folder} de {from} vers {to}.',
        'folders.register': 'Registre des dossiers',
        'folders.searchPlaceholder':
            'Rechercher par Numéro NHS, patient, titre de dossier ou armoire',
        'folders.searchLabel': 'Rechercher des dossiers',
        'folders.addFolder': 'Ajouter un dossier',
        'folders.tableLabel': 'Dossiers',
        'folders.tableCaption':
            'Tous les dossiers papier de notes de cas suivis par le système',
        'folders.colNhsNumber': 'Numéro NHS',
        'folders.colPatient': 'Patient',
        'folders.colFolder': 'Dossier',
        'folders.colCabinet': 'Armoire',
        'folders.colStatus': 'Statut',
        'folders.colLastMoved': 'Dernier déplacement',
        'folders.colAction': 'Action',
        'folders.noMatch': 'Aucun dossier ne correspond à',
        'folderNew.backToFolders': 'Retour aux dossiers',
        'folderNew.heading': 'Ajouter un nouveau dossier',
        'folderNew.intro':
            "Un dossier appartient à un seul patient. Si le patient n'est pas encore enregistré auprès du Service principal des patients, nous le créerons ; sinon, le nouveau dossier est rattaché au dossier patient existant.",
        'folderNew.cannotSave': "Impossible d'enregistrer le dossier",
        'folderNew.invalidNhs':
            'Saisissez un Numéro NHS valide à 10 chiffres (échec du contrôle Modulo 11).',
        'folderNew.titleRequired': 'Le titre du dossier est obligatoire.',
        'folderNew.formLabel': 'Ajouter un dossier',
        'folderNew.nhsDescription': '10 chiffres, au format XXX XXX XXXX.',
        'folderNew.titleLabel': 'Titre du dossier',
        'folderNew.titleDescription': 'p. ex. Volume 1, Cardiologie 2023',
        'folderNew.patientName': 'Nom du patient',
        'folderNew.patientNameDescription':
            'Nécessaire uniquement pour un nouveau patient.',
        'folderNew.dobDescription':
            'Nécessaire uniquement pour un nouveau patient.',
        'folderNew.initialCabinet': 'Armoire initiale',
        'folderNew.initialCabinetDescription':
            'Laissez vide si le dossier est en transit.',
        'folderNew.saveFolder': 'Enregistrer le dossier',
        'folderDetail.backToFolders': 'Retour aux dossiers',
        'folderDetail.patientPrefix': 'Patient :',
        'folderDetail.detailsLabel': 'Détails du dossier',
        'folderDetail.folderTitle': 'Titre du dossier',
        'folderDetail.currentCabinet': 'Armoire actuelle',
        'folderDetail.moveThisFolder': 'Déplacer ce dossier',
        'folderDetail.moveHistory': 'Historique des déplacements',
        'patients.heading': 'Patients',
        'patients.searchPlaceholder': 'Rechercher par Numéro NHS ou nom',
        'patients.searchLabel': 'Rechercher des patients',
        'patients.tableLabel': 'Patients',
        'patients.tableCaption':
            'Tous les patients avec un ou plusieurs dossiers enregistrés',
        'patients.colFolders': 'Dossiers',
        'patients.noMatch': 'Aucun patient ne correspond à',
        'patientDetail.backToPatients': 'Retour aux patients',
        'patientDetail.notFoundHeading':
            'Patient introuvable dans le Service principal des patients',
        'patientDetail.notFoundBody':
            "Aucun dossier patient n'existe pour le Numéro NHS {nhs}. Les dossiers ci-dessous sont reconstruits à partir d'instantanés locaux écrits lors de la création de chaque dossier.",
        'patientDetail.sourcePrefix': 'Source :',
        'patientDetail.recordActions': 'Actions sur le dossier patient',
        'patientDetail.nhsNumberHeading': 'Numéro NHS {nhs}',
        'patientDetail.foldersForPatient': 'Dossiers de ce patient ({n})',
        'patientDetail.patientFoldersTable': 'Dossiers du patient',
        'patientDetail.colVolume': 'Volume',
        'patientDetail.noFoldersYet': 'Aucun dossier pour le moment.',
        'patientDetail.addFolderForPatient':
            'Ajouter un dossier pour ce patient',
        'patientDetail.moveHistoryForPatient':
            'Historique des déplacements de ce patient',
        'patientDetail.demoUnavailable':
            "« {action} » n'est pas disponible dans cette démonstration.",
        'buildings.heading': 'Bâtiments',
        'buildings.addBuilding': 'Ajouter un bâtiment',
        'buildings.tableLabel': 'Bâtiments',
        'buildings.tableCaption':
            "Sites physiques abritant des salles d'archives",
        'buildings.colRooms': 'Salles',
        'buildings.noBuildings': 'Aucun bâtiment pour le moment.',
        'buildingNew.backToBuildings': 'Retour aux bâtiments',
        'buildingNew.heading': 'Ajouter un bâtiment',
        'buildingNew.intro':
            'Un bâtiment peut avoir plusieurs salles ; chaque salle peut contenir plusieurs armoires.',
        'buildingNew.cannotSave': "Impossible d'enregistrer le bâtiment",
        'buildingNew.nameRequired': 'Le nom du bâtiment est obligatoire.',
        'buildingNew.formLabel': 'Ajouter un bâtiment',
        'buildingNew.nameLabel': 'Nom du bâtiment',
        'buildingNew.saveBuilding': 'Enregistrer le bâtiment',
        'buildingDetail.backToBuildings': 'Retour aux bâtiments',
        'buildingDetail.rooms': 'Salles ({n})',
        'buildingDetail.roomsTable': 'Salles',
        'buildingDetail.colCabinets': 'Armoires',
        'buildingDetail.noRooms': 'Aucune salle pour le moment.',
        'buildingDetail.addRoom': 'Ajouter une salle',
        'buildingDetail.addRoomLabel': 'Ajouter une salle',
        'buildingDetail.roomNameRequired':
            'Le nom de la salle est obligatoire.',
        'buildingDetail.roomName': 'Nom de la salle',
        'buildingDetail.saveRoom': 'Enregistrer la salle',
        'buildingDetail.presenceHistory': 'Historique de présence des dossiers',
        'buildingDetail.presenceIntro':
            'Dossiers ayant été dans une armoire de ce bâtiment, les plus récents en premier.',
        'buildingDetail.presenceTable':
            'Historique de présence des dossiers du bâtiment',
        'buildingDetail.presenceCaption':
            'Agrégé sur les armoires de ce bâtiment',
        'buildingDetail.noPresence':
            'Aucune présence de dossier enregistrée dans ce bâtiment pour le moment.',
        'cabinets.heading': 'Armoires de classement',
        'cabinets.addCabinet': 'Ajouter une armoire',
        'cabinets.tableLabel': 'Armoires',
        'cabinets.tableCaption':
            'Armoires de classement physiques, leur bâtiment/salle et leur occupation',
        'cabinets.colLabel': 'Étiquette',
        'cabinets.colBuilding': 'Bâtiment',
        'cabinets.colRoom': 'Salle',
        'cabinets.colCapacity': 'Capacité',
        'cabinets.colFolders': 'Dossiers',
        'cabinets.colUtilisation': 'Utilisation',
        'cabinetNew.backToCabinets': 'Retour aux armoires',
        'cabinetNew.heading': 'Ajouter une armoire de classement',
        'cabinetNew.intro':
            'Une armoire se trouve dans une salle (qui se trouve dans un bâtiment).',
        'cabinetNew.noRoomsHeading': "Aucune salle n'existe encore",
        'cabinetNew.noRoomsBody':
            "d'abord, puis ajoutez une salle depuis la page du bâtiment.",
        'cabinetNew.noRoomsBuilding': 'bâtiment',
        'cabinetNew.noRoomsCreate': 'Créez un',
        'cabinetNew.cannotSave': "Impossible d'enregistrer l'armoire",
        'cabinetNew.labelRequired': "L'étiquette de l'armoire est obligatoire.",
        'cabinetNew.roomRequired': 'Sélectionnez une salle.',
        'cabinetNew.formLabel': 'Ajouter une armoire',
        'cabinetNew.labelLabel': "Étiquette de l'armoire",
        'cabinetNew.labelPlaceholder': 'p. ex. Armoire D3',
        'cabinetNew.roomLabel': 'Salle',
        'cabinetNew.selectRoomOption': '— Sélectionner une salle —',
        'cabinetNew.capacityLabel': 'Capacité',
        'cabinetNew.capacityDescription':
            "Nombre approximatif de dossiers que l'armoire contient. Laissez vide si inconnu.",
        'cabinetNew.saveCabinet': "Enregistrer l'armoire",
        'cabinetDetail.backToCabinets': 'Retour aux armoires',
        'cabinetDetail.currentFolders':
            'Dossiers actuellement dans cette armoire ({n})',
        'cabinetDetail.currentFoldersTable': 'Dossiers actuels',
        'cabinetDetail.empty': 'Cette armoire est actuellement vide.',
        'cabinetDetail.presenceHistory': 'Historique de présence des dossiers',
        'cabinetDetail.presenceIntro':
            'Quels dossiers ont été dans cette armoire, et quand. Les plus récents en premier.',
        'cabinetDetail.presenceTable': 'Historique de présence des dossiers',
        'cabinetDetail.presenceCaption':
            "Dérivé du journal d'audit des déplacements",
        'cabinetDetail.colReasonLeft': 'Motif du départ',
        'cabinetDetail.noPresence':
            "Aucun dossier n'a encore été enregistré dans cette armoire.",
        'roomDetail.backToBuildings': 'Retour aux bâtiments',
        'roomDetail.presenceHistory': 'Historique de présence des dossiers',
        'roomDetail.presenceIntro':
            'Dossiers ayant été dans une armoire de cette salle, les plus récents en premier.',
        'roomDetail.presenceTable':
            'Historique de présence des dossiers de la salle',
        'roomDetail.presenceCaption': 'Agrégé sur les armoires de cette salle',
        'roomDetail.noPresence':
            'Aucune présence de dossier enregistrée dans cette salle pour le moment.',
        'volumes.heading': 'Volumes',
        'volumes.printLabels': 'Imprimer les étiquettes',
        'volumes.newVolume': 'Nouveau volume',
        'volumes.intro':
            "Un volume est un lot mobile des dossiers d'un patient. Déplacez le volume et tous les dossiers qu'il contient se déplacent ensemble.",
        'volumes.tableLabel': 'Volumes',
        'volumes.tableCaption':
            'Lots de dossiers, chacun appartenant à un patient',
        'volumes.colFolders': 'Dossiers',
        'volumes.colLocation': 'Emplacement',
        'volumes.noVolumes':
            "Aucun volume pour le moment. Créez-en un pour regrouper les dossiers d'un patient.",
        'volumes.noLabelsSelected': 'Aucune étiquette sélectionnée.',
        'volumes.queued.one':
            "{n} étiquette × {copies} {copy} mise en file d'attente pour impression.",
        'volumes.queued.other':
            "{n} étiquettes × {copies} {copy} mises en file d'attente pour impression.",
        'volumes.copy': 'copie',
        'volumes.copies': 'copies',
        'volumeNew.backToVolumes': 'Retour aux volumes',
        'volumeNew.heading': 'Nouveau volume',
        'volumeNew.intro':
            "Un volume regroupe les dossiers d'un patient. Le patient doit déjà être enregistré (créez d'abord un dossier pour lui). Vous pouvez ajouter des dossiers au volume une fois qu'il existe.",
        'volumeNew.cannotCreate': 'Impossible de créer le volume',
        'volumeNew.invalidNhs':
            'Saisissez un Numéro NHS valide à 10 chiffres (échec du contrôle Modulo 11).',
        'volumeNew.titleRequired': 'Le titre du volume est obligatoire.',
        'volumeNew.formLabel': 'Nouveau volume',
        'volumeNew.patientNhs': 'Numéro NHS du patient',
        'volumeNew.nhsDescription': '10 chiffres, au format XXX XXX XXXX.',
        'volumeNew.titleLabel': 'Titre du volume',
        'volumeNew.titleDescription': 'p. ex. Alice Johnson — Vol 1',
        'volumeNew.initialCabinet': 'Armoire initiale',
        'volumeNew.initialCabinetDescription':
            'Où le volume se trouve. Laissez vide si en transit.',
        'volumeNew.createVolume': 'Créer le volume',
        'volumeDetail.backToVolumes': 'Retour aux volumes',
        'volumeDetail.somethingWrong': 'Une erreur est survenue',
        'volumeDetail.foldersIn': 'Dossiers dans ce volume ({n})',
        'volumeDetail.foldersTable': 'Dossiers du volume',
        'volumeDetail.noFolders':
            'Aucun dossier dans ce volume pour le moment.',
        'volumeDetail.addFolderLabel': 'Ajouter un dossier',
        'volumeDetail.addFolderFor': 'Ajouter un dossier pour {patient}',
        'volumeDetail.chooseFolder': '— Choisir un dossier —',
        'volumeDetail.addToVolume': 'Ajouter au volume',
        'volumeDetail.renameVolume': 'Renommer le volume',
        'volumeDetail.rename': 'Renommer',
        'volumeDetail.moveVolume': 'Déplacer ce volume',
        'volumeDetail.moveIntro':
            'Relocalise tous les dossiers du volume ensemble.',
        'volumeDetail.moveFormLabel': 'Déplacer le volume',
        'volumeDetail.destinationCabinet': 'Armoire de destination',
        'volumeDetail.moveReasonPlaceholder': 'p. ex. Clinique ambulatoire',
        'volumeDetail.moveVolumeButton': 'Déplacer le volume',
        'volumeDetail.moveHistory': 'Historique des déplacements',
        'workers.heading': 'Personnel',
        'workers.intro':
            "Personnel qui déplace les dossiers. Ouvrez un membre du personnel pour voir les dossiers qu'il a déplacés et tous les dossiers appartenant à ses patients.",
        'workers.tableLabel': 'Personnel',
        'workers.tableCaption': 'Personnel du Service principal du personnel',
        'workers.noWorkers': 'Aucun membre du personnel trouvé.',
        'workerDetail.backToWorkers': 'Retour au personnel',
        'workerDetail.foldersMoved':
            'Dossiers déplacés par ce membre du personnel ({n})',
        'workerDetail.foldersMovedTable':
            'Dossiers déplacés par ce membre du personnel',
        'workerDetail.noMovedFolders':
            "Ce membre du personnel n'a encore déplacé aucun dossier.",
        'workerDetail.patientsFolders':
            'Tous les dossiers de leurs patients ({n})',
        'workerDetail.patientsFoldersIntro':
            'Tous les dossiers appartenant à un patient géré par ce membre du personnel.',
        'workerDetail.patientsFoldersTable':
            'Dossiers des patients de ce membre du personnel',
        'workerDetail.noPatientFolders':
            'Aucun dossier patient à afficher pour le moment.',
        'workerDetail.movesByWorker': 'Déplacements par ce membre du personnel',
        'history.backToDashboard': 'Retour au tableau de bord',
        'history.heading': "Historique des déplacements (journal d'audit)",
        'history.filterPlaceholder':
            'Filtrer par patient, numéro NHS, armoire ou brancardier',
        'history.filterLabel': "Filtrer le journal d'audit",
        'history.tableLabel': "Journal d'audit des déplacements",
        'history.tableCaption':
            "Chaque déplacement enregistré d'un dossier papier, les plus récents en premier",
        'history.noMatch': 'Aucun déplacement ne correspond à votre filtre.',
        'historyDetail.backToHistory': "Retour à l'historique des déplacements",
        'historyDetail.heading': 'Événement de déplacement',
        'historyDetail.folderInvolved': 'Dossier concerné',
        'historyDetail.detailsLabel': "Détails de l'événement de déplacement",
        'historyDetail.otherFolders': 'Autres dossiers pour {patient}',
        'historyDetail.noOtherFolders': 'Aucun autre dossier pour ce patient.',
        'reports.backToDashboard': 'Retour au tableau de bord',
        'reports.heading': 'Rapports',
        'reports.atAGlance': "En un coup d'œil",
        'reports.kpiPatients': 'Patients :',
        'reports.kpiFolders': 'Dossiers :',
        'reports.kpiFoldersDetail':
            '({inCabinet} en armoire, {inTransit} en transit)',
        'reports.kpiVolumes': 'Volumes :',
        'reports.kpiCabinets': 'Armoires :',
        'reports.kpiCabinetsDetail': 'dans {n} bâtiments',
        'reports.kpiMoves': 'Déplacements — dernières 24 h :',
        'reports.kpiMoves7d': '7 derniers jours :',
        'reports.cabinetUtilisation': 'Utilisation des armoires',
        'reports.colCabinet': 'Armoire',
        'reports.colFolders': 'Dossiers',
        'reports.colCapacity': 'Capacité',
        'reports.colUtilisation': 'Utilisation',
        'reports.inTransit': 'En transit ({n})',
        'reports.noInTransit': 'Aucun dossier en transit.',
        'reports.activityByWorker': 'Activité par membre du personnel',
        'reports.colWorker': 'Personnel',
        'reports.colMoves': 'Déplacements',
        'reports.noMovesYet': 'Aucun déplacement enregistré pour le moment.',
        'reports.derivedLive':
            "Les rapports sont dérivés en direct de l'API — pas de stockage de rapports séparé.",
        'alerts.backToDashboard': 'Retour au tableau de bord',
        'alerts.heading': 'Alertes de géorepérage',
        'alerts.intro':
            "Notes de cas ayant franchi une limite de bâtiment. Une violation de géorepérage est tout déplacement dont les armoires d'origine et de destination sont dans des bâtiments différents.",
        'alerts.tableLabel': 'Alertes de géorepérage',
        'alerts.tableCaption':
            'Déplacements franchissant les limites, les plus récents en premier',
        'alerts.colCrossed': 'Franchi',
        'alerts.none':
            'Aucune violation de géorepérage — chaque dossier est resté dans son bâtiment.',
        'grid.nhsNumber': 'Numéro NHS',
        'grid.patient': 'Patient',
        'grid.folder': 'Dossier',
        'grid.cabinet': 'Armoire',
        'grid.status': 'Statut',
        'grid.lastMoved': 'Dernier déplacement',
        'addressograph.label': 'Étiquette patient',
        'addressograph.nhsNo': 'N° NHS',
        'addressograph.dob': 'D.D.N.',
        'addressograph.sex': 'Sexe',
        'addressograph.address': 'Adresse',
        'buttonBar.label': 'Actions',
        'buttonBar.patient': 'Patient',
        'buttonBar.referrals': 'Orientations',
        'buttonBar.activate': 'Activer',
        'buttonBar.caseNotes': 'Notes de cas',
        'buttonBar.pathways': 'Parcours',
        'buttonBar.legalStatus': 'Statut juridique',
        'buttonBar.documents': 'Documents',
        'buttonBar.wrapper': 'Enveloppe',
        'buttonBar.audit': 'Audit',
        'buttonBar.quickReports': 'Rapports rapides',
        'labels.title': 'Étiquettes',
        'labels.searchLabel': 'Rechercher des étiquettes',
        'labels.searchPlaceholder': 'Saisissez du texte à rechercher...',
        'labels.find': 'Rechercher',
        'labels.clear': 'Effacer',
        'labels.volumeTitles': 'Titres des volumes',
        'labels.noMatching': 'Aucun volume correspondant.',
        'labels.numberOfCopies': 'Nombre de copies :',
        'labels.copiesLabel': 'Nombre de copies',
        'labels.print': 'Imprimer',
        'labels.close': 'Fermer',
        'auth.signin': 'Se connecter',
        'auth.signout': 'Se déconnecter',
        'share.email': 'Envoyer le lien par e-mail',
        'share.linkedin': 'Partager sur LinkedIn',
        'share.reddit': 'Partager sur Reddit',
        'share.bluesky': 'Partager sur Bluesky',
        'share.mastodon': 'Partager sur Mastodon',
        'splash.hero.secondary': 'Découvrir le contenu',
        'splash.benefits.title': 'Pourquoi les équipes la choisissent',
        'splash.features.title': 'Ce que vous pouvez faire',
        'splash.trust.title': 'Conçu pour la confiance',
        'splash.trust.1.title': 'Connexion sans mot de passe',
        'splash.trust.1.body':
            'Un lien magique envoyé par e-mail : aucun mot de passe à divulguer ni à réutiliser.',
        'splash.trust.2.title': 'Autorisations par attributs',
        'splash.trust.2.body':
            'Des règles fines déterminent qui peut lire, écrire, fusionner ou supprimer.',
        'splash.trust.3.title': "Piste d'audit inviolable",
        'splash.trust.3.body':
            'Un historique en ajout seul consigne chaque modification et son auteur.',
        'splash.trust.4.title': 'Contrôles de confidentialité',
        'splash.trust.4.body':
            "Les données sensibles sont masquées, sauf droit d'accès.",
        'splash.trust.5.title': 'Standards ouverts',
        'splash.trust.5.body':
            'REST avec OpenAPI, et HL7 FHIR là où les systèmes de santé en ont besoin.',
        'splash.trust.6.title': 'Parle votre langue',
        'splash.trust.6.body':
            'Anglais, arabe, chinois, espagnol, français, gallois et hindi.',
        'splash.cta.title': 'Prêt à commencer ?',
        'splash.cta.body':
            'Connectez-vous avec un lien magique envoyé par e-mail. Aucun mot de passe requis.',
        'splash.hero.title': 'Sachez où se trouve chaque dossier',
        'splash.hero.subtitle':
            "Suivez les dossiers papier des notes de cas par numéro NHS entre bâtiments, salles et armoires, avec une piste d'audit complète de chaque déplacement.",
        'splash.benefits.1.title': "Retrouvez n'importe quel dossier vite",
        'splash.benefits.1.body':
            'Recherchez un dossier papier par numéro NHS et voyez où il se trouve en ce moment.',
        'splash.benefits.2.title': 'Moins de dossiers perdus',
        'splash.benefits.2.body':
            "Chaque déplacement est enregistré : un dossier n'est jamais simplement quelque part dans le bâtiment.",
        'splash.benefits.3.title': 'Moins de temps à chercher',
        'splash.benefits.3.body':
            'Brancardiers et cliniciens consacrent leur temps aux soins, pas à fouiller les armoires.',
        'splash.benefits.4.title': 'Une piste fiable',
        'splash.benefits.4.body':
            'Un historique complet et daté montre qui a déplacé chaque dossier, quand et pourquoi.',
        'splash.benefits.5.title': 'Identité du patient sécurisée',
        'splash.benefits.5.body':
            "Les numéros NHS sont vérifiés avant l'enregistrement : une faute de frappe ne devient jamais un dossier.",
        'splash.benefits.6.title': 'Une vue claire des locaux',
        'splash.benefits.6.body':
            'Voyez le taux de remplissage de chaque armoire et combien de dossiers sont en transit.',
        'splash.features.1.title': 'Registre des dossiers',
        'splash.features.1.body':
            'Recherchez chaque dossier par patient, titre ou numéro NHS.',
        'splash.features.2.title': 'Déplacer et scanner',
        'splash.features.2.body':
            'Enregistrez un déplacement en quelques secondes en scannant ou en saisissant le numéro NHS.',
        'splash.features.3.title': 'Volumes',
        'splash.features.3.body':
            "Regroupez les dossiers d'un patient en un volume et déplacez-les ensemble.",
        'splash.features.4.title': 'Bâtiments, salles, armoires',
        'splash.features.4.body':
            "Modélisez la hiérarchie physique de chaque bâtiment jusqu'à ses armoires.",
        'splash.features.5.title': 'Historique des déplacements',
        'splash.features.5.body':
            "Recherchez dans le journal d'audit complet de chaque déplacement de dossier.",
        'splash.features.6.title': 'Alertes et rapports',
        'splash.features.6.body':
            "Consultez les alertes entre bâtiments, l'utilisation des armoires et les chiffres clés.",
        'nav.tour': 'Visite guidée',
        'splash.hero.tour': 'Faire la visite guidée',
        'tour.head': 'Faire la visite guidée',
        'tour.toc': 'Sur cette page',
        'tour.open': 'Ouvrir cet écran',
        'tour.top': 'Retour en haut',
        'tour.start.title': 'Avant de commencer',
        'tour.start.summary':
            "Un compte est nécessaire pour travailler avec des données réelles. La connexion prend moins d'une minute et n'exige aucun mot de passe.",
        'tour.start.step.1':
            'Choisissez Se connecter en haut à droite et saisissez votre adresse e-mail.',
        'tour.start.step.2':
            "Ouvrez le lien magique reçu par e-mail. Il ne fonctionne qu'une fois et expire vite.",
        'tour.start.step.3':
            "Vous revenez dans l'application connecté, sans rien à retenir ni à réinitialiser.",
        'tour.start.step.4':
            'Utilisez les boutons à côté de Se connecter pour changer le thème, la langue et la taille du texte, ou pour partager la page.',
        'tour.intro':
            "Une visite guidée de Suivi des dossiers : ce que fait chaque écran et les étapes pour l'utiliser, de l'ajout d'un dossier à la consultation du journal d'audit des déplacements.",
        'tour.s1.title': 'Ajouter un dossier au registre',
        'tour.s1.summary':
            "Chaque dossier papier de notes de cas appartient à un patient et est suivi dès l'instant où vous l'ajoutez.",
        'tour.s1.step.1':
            'Ouvrez Dossiers et choisissez Ajouter un dossier, puis saisissez le numéro NHS à 10 chiffres du patient, qui doit réussir le contrôle modulo 11.',
        'tour.s1.step.2':
            'Saisissez un Titre du dossier, par exemple Volume 1 ou Cardiologie 2023.',
        'tour.s1.step.3':
            "Pour un patient pas encore enregistré, renseignez aussi Nom du patient et Date de naissance ; pour un patient existant, ce n'est pas nécessaire.",
        'tour.s1.step.4':
            "Choisissez l'Armoire initiale, ou laissez-la vide si le dossier est en transit, puis choisissez Enregistrer le dossier.",
        'tour.s2.title': 'Retrouver un dossier',
        'tour.s2.summary':
            'Le Registre des dossiers répond à « où est ce dossier en ce moment ? » depuis une seule zone de recherche.',
        'tour.s2.step.1':
            'Ouvrez Dossiers et saisissez un numéro NHS, un nom de patient, un titre de dossier ou une armoire dans Rechercher des dossiers.',
        'tour.s2.step.2':
            "Lisez l'Armoire, le Statut (en armoire ou en transit) et le Dernier déplacement de chaque ligne.",
        'tour.s2.step.3':
            'Ouvrez un dossier pour voir ses détails et son Historique des déplacements, ou ouvrez le patient depuis Patients pour voir tous ses dossiers.',
        'tour.s2.step.4':
            'Choisissez Déplacer ce dossier pour enregistrer directement un déplacement.',
        'tour.s3.title': 'Déplacer un dossier',
        'tour.s3.summary':
            "Chaque emplacement est enregistré, de sorte que la position d'un dossier est toujours à jour.",
        'tour.s3.step.1':
            'Ouvrez Déplacer un dossier, saisissez le Numéro NHS du patient, puis choisissez lequel de ses dossiers vous déplacez.',
        'tour.s3.step.2':
            "Choisissez l'armoire de Destination, ou En transit (porté par un brancardier) si le dossier est en route.",
        'tour.s3.step.3':
            'Choisissez un membre du personnel dans la liste, ou saisissez un nom sous Déplacé par, et ajoutez un Motif.',
        'tour.s3.step.4':
            "Choisissez Enregistrer le déplacement ; la page confirme Déplacement enregistré et le déplacement rejoint le journal d'audit.",
        'tour.s4.title': 'Scanner un dossier',
        'tour.s4.summary':
            "La voie rapide pour le service des dossiers : aucun lecteur matériel n'est nécessaire, même si une douchette qui simule le clavier fonctionne aussi.",
        'tour.s4.step.1':
            'Ouvrez Scanner et cliquez dans la zone Scanner ou rechercher.',
        'tour.s4.step.2':
            'Scannez un code-barres, ou saisissez un numéro NHS ou un identifiant de dossier.',
        'tour.s4.step.3':
            'Consultez la liste des Correspondances et ouvrez un dossier pour voir ses détails.',
        'tour.s4.step.4':
            "Choisissez Déplacer ce dossier pour enregistrer son déplacement avec le dossier déjà sélectionné ; s'il n'y a aucune correspondance, la page indique qu'aucun dossier n'a été trouvé.",
        'tour.s5.title': 'Regrouper des dossiers en volumes',
        'tour.s5.summary':
            "Un volume est un ensemble déplaçable des dossiers d'un patient, qui voyagent donc ensemble.",
        'tour.s5.step.1':
            'Ouvrez Volumes et choisissez Nouveau volume, puis saisissez le Numéro NHS du patient et un Titre du volume ; le patient doit déjà avoir un dossier.',
        'tour.s5.step.2':
            'Ouvrez le volume et utilisez Ajouter un dossier pour y regrouper les dossiers de ce patient.',
        'tour.s5.step.3':
            "Utilisez Déplacer ce volume pour transférer tous les dossiers qu'il contient vers une seule armoire de destination en une seule étape.",
        'tour.s5.step.4':
            'De retour sur Volumes, choisissez Imprimer les étiquettes, sélectionnez les volumes, définissez le Nombre de copies et choisissez Imprimer pour les mettre en file.',
        'tour.s6.title': "Consulter l'historique, les alertes et les rapports",
        'tour.s6.summary':
            'Chaque déplacement est conservé, de sorte que vous pouvez savoir qui a déplacé quoi, quand et pourquoi.',
        'tour.s6.step.1':
            "Ouvrez Historique des déplacements et utilisez Filtrer le journal d'audit pour restreindre par patient, numéro NHS, armoire ou brancardier ; les déplacements les plus récents viennent en premier.",
        'tour.s6.step.2':
            "Ouvrez une ligne pour voir l'événement de déplacement complet, le dossier concerné et les autres dossiers du patient.",
        'tour.s6.step.3':
            "Ouvrez Alertes pour voir les alertes de géorepérage : des déplacements dont les armoires d'origine et de destination sont dans des bâtiments différents.",
        'tour.s6.step.4':
            "Ouvrez Rapports pour les chiffres clés, l'utilisation des armoires, les dossiers en transit et l'activité par membre du personnel, le tout calculé en direct.",
        'signin.sso': 'Se connecter avec SSO',
    },
    'hi-001': {
        'brand.name': 'केस ट्रैकिंग',
        'brand.tagline': 'NHS कागज़ी रिकॉर्ड',
        'chrome.language': 'भाषा',
        'chrome.theme': 'थीम',
        'nav.share': 'साझा करें',
        'nav.text_size': 'टेक्स्ट का आकार',
        'share.copy_link': 'लिंक कॉपी करें',
        'share.copied': 'लिंक कॉपी हो गया',
        'share.copy_failed': 'कॉपी नहीं हो सका — इसे एड्रेस बार से कॉपी करें',
        'theme.default': 'डिफ़ॉल्ट',
        'theme.highContrast': 'उच्च कंट्रास्ट',
        'nav.toggle': 'नेविगेशन टॉगल करें',
        'auth.signedInAs': 'के रूप में साइन इन',
        'auth.signOut': 'साइन आउट',
        'nav.dashboard': 'डैशबोर्ड',
        'nav.patients': 'मरीज़',
        'nav.folders': 'फ़ोल्डर',
        'nav.volumes': 'खंड',
        'nav.workers': 'कर्मचारी',
        'nav.buildings': 'इमारतें',
        'nav.cabinets': 'अलमारियाँ',
        'nav.move': 'फ़ोल्डर स्थानांतरित करें',
        'nav.scan': 'स्कैन',
        'nav.history': 'स्थानांतरण इतिहास',
        'nav.alerts': 'अलर्ट',
        'nav.reports': 'रिपोर्ट',
        'layout.skipToContent': 'मुख्य सामग्री पर जाएँ',
        'layout.siteHeader': 'साइट हेडर',
        'layout.siteFooter': 'साइट फ़ुटर',
        'layout.primaryNavigation': 'प्राथमिक नेविगेशन',
        'footer.text':
            'केस ट्रैकिंग — Lily डिज़ाइन सिस्टम (NHS थीम) और SVAR Svelte के साथ बनाया गया। केवल डेमो डेटा; यह विनियमित चिकित्सा रिकॉर्ड नहीं है।',
        'common.backToDashboard': 'डैशबोर्ड पर वापस',
        'common.cancel': 'रद्द करें',
        'common.move': 'स्थानांतरित करें',
        'common.view': 'देखें',
        'common.remove': 'हटाएँ',
        'common.action': 'क्रिया',
        'common.status': 'स्थिति',
        'common.patient': 'मरीज़',
        'common.folder': 'फ़ोल्डर',
        'common.cabinet': 'अलमारी',
        'common.title': 'शीर्षक',
        'common.name': 'नाम',
        'common.role': 'भूमिका',
        'common.reason': 'कारण',
        'common.movedBy': 'द्वारा स्थानांतरित',
        'common.lastMoved': 'अंतिम स्थानांतरण',
        'common.nhsNumber': 'NHS नंबर',
        'common.dateOfBirth': 'जन्म तिथि',
        'common.description': 'विवरण',
        'common.notes': 'टिप्पणियाँ',
        'common.entered': 'प्रवेश किया',
        'common.left': 'छोड़ा',
        'common.when': 'कब',
        'common.from': 'से',
        'common.to': 'तक',
        'common.source': 'स्रोत',
        'common.volume': 'खंड',
        'common.stillHere': 'अभी भी यहाँ',
        'common.noMovesYet': 'अभी तक कोई स्थानांतरण दर्ज नहीं किया गया।',
        'common.inTransitPorter': 'स्थानांतरण में (पोर्टर ले जा रहा है)',
        'common.selectCabinetOption': '— अलमारी चुनें —',
        'common.inTransitOption': '— स्थानांतरण में —',
        'status.inCabinet': 'अलमारी-में',
        'status.inTransit': 'स्थानांतरण-में',
        'badge.located': 'स्थित',
        'badge.porterInMotion': 'पोर्टर गति में',
        'dashboard.welcomePrefix': 'स्वागत है।',
        'dashboard.inTransit.one':
            'आपके पास इस समय {n} फ़ोल्डर स्थानांतरण में है। पृष्ठ का उपयोग करें',
        'dashboard.inTransit.other':
            'आपके पास इस समय {n} फ़ोल्डर स्थानांतरण में हैं। पृष्ठ का उपयोग करें',
        'dashboard.moveFolderPage': 'फ़ोल्डर स्थानांतरित करें',
        'dashboard.pageToRecord': 'एक स्थान दर्ज करने के लिए।',
        'dashboard.folderSummary': 'फ़ोल्डर सारांश',
        'dashboard.patients': 'मरीज़',
        'dashboard.foldersTracked': '{n} फ़ोल्डर ट्रैक किए गए',
        'dashboard.inCabinet': 'अलमारी में',
        'dashboard.inTransitCard': 'स्थानांतरण में',
        'dashboard.buildings': 'इमारतें',
        'dashboard.roomsCabinets': '{rooms} कमरे · {cabinets} अलमारियाँ',
        'dashboard.moves24h': 'स्थानांतरण (24घं)',
        'dashboard.auditedPlacements': 'ऑडिट किए गए फ़ोल्डर स्थान',
        'dashboard.folderRegister': 'फ़ोल्डर रजिस्टर',
        'dashboard.viewAll': 'सभी देखें',
        'dashboard.addFolder': 'फ़ोल्डर जोड़ें',
        'dashboard.recentMoves': 'हाल के स्थानांतरण',
        'dashboard.seeFullHistory': 'पूरा ऑडिट इतिहास देखें →',
        'dashboard.cabinetUtilisation': 'अलमारी उपयोग',
        'error.backToDashboard': 'डैशबोर्ड पर वापस',
        'error.heading': 'केस ट्रैकिंग API त्रुटि',
        'error.unknown': 'अज्ञात त्रुटि',
        'error.apiHint':
            'ऐप /api के माध्यम से Loco JSON API से बात करता है (विकास में Loco सर्वर पर प्रॉक्सी)। सुनिश्चित करें कि वह API चल रहा है। README.md § "Quick start" देखें।',
        'login.title': 'साइन इन',
        'login.intro':
            'अपना कार्य ईमेल पता दर्ज करें। यदि इसे पहचाना जाता है, तो हम आपको एक बार उपयोग होने वाला साइन-इन लिंक भेजेंगे। कोई पासवर्ड आवश्यक नहीं।',
        'login.sendError': 'साइन-इन लिंक नहीं भेजा जा सका',
        'login.enterEmail': 'अपना ईमेल पता दर्ज करें।',
        'login.checkEmail': 'अपना ईमेल जाँचें',
        'login.sentBody':
            'किसी ज्ञात खाते से मेल खाता है, तो साइन-इन लिंक रास्ते में है। लिंक 10 मिनट में समाप्त हो जाता है।',
        'login.sentPrefix': 'यदि',
        'login.devShortcut': 'विकास शॉर्टकट:',
        'login.openLink': 'अपना साइन-इन लिंक खोलें',
        'login.formLabel': 'साइन इन',
        'login.emailLabel': 'ईमेल पता',
        'login.submit': 'मुझे साइन-इन लिंक ईमेल करें',
        'callback.title': 'आपको साइन इन किया जा रहा है',
        'callback.error': 'साइन-इन लिंक का उपयोग नहीं किया जा सका',
        'callback.backToSignIn': 'साइन इन पर वापस',
        'callback.completing': 'एक क्षण — आपका साइन-इन पूरा हो रहा है…',
        'scan.heading': 'फ़ोल्डर स्कैन करें',
        'scan.intro':
            'बारकोड स्कैन करें या NHS नंबर (या फ़ोल्डर id) टाइप करें ताकि सीधे किसी फ़ोल्डर पर पहुँचें और उसका स्थानांतरण दर्ज करें — Scan4Safety तेज़ रास्ता। किसी हार्डवेयर स्कैनर की आवश्यकता नहीं; एक कीबोर्ड-वेज स्कैनर नीचे के बॉक्स में टाइप करता है।',
        'scan.failed': 'स्कैन विफल',
        'scan.formLabel': 'स्कैन',
        'scan.fieldLabel': 'स्कैन या खोजें',
        'scan.fieldDescription': 'NHS नंबर (उदा. 943 476 5919) या फ़ोल्डर id।',
        'scan.placeholder': 'स्कैन करें या टाइप करें…',
        'scan.matches': 'मिलान ({n})',
        'scan.moveThisFolder': 'इस फ़ोल्डर को स्थानांतरित करें',
        'scan.noFolderFound': '“{term}” के लिए कोई फ़ोल्डर नहीं मिला।',
        'move.heading': 'एक फ़ोल्डर स्थानांतरित करें',
        'move.intro':
            'किसी मरीज़ का NHS नंबर दर्ज करें, वह फ़ोल्डर चुनें जिसे आप स्थानांतरित कर रहे हैं, फिर गंतव्य अलमारी चुनें (या इसे स्थानांतरण में चिह्नित करें)।',
        'move.recorded': 'स्थानांतरण दर्ज किया गया',
        'move.formLabel': 'फ़ोल्डर स्थानांतरित करें',
        'move.patientNhs': 'मरीज़ का NHS नंबर',
        'move.invalidNhs': 'एक मान्य 10-अंकीय NHS नंबर दर्ज करें।',
        'move.folder': 'फ़ोल्डर',
        'move.selectFolderError':
            'चुनें कि कौन-सा फ़ोल्डर स्थानांतरित करना है।',
        'move.pickFolderDescription':
            'चुनें कि इस मरीज़ के किस फ़ोल्डर को स्थानांतरित करना है।',
        'move.enterNhsDescription': 'फ़ोल्डर देखने के लिए NHS नंबर दर्ज करें।',
        'move.selectFolderOption': '— फ़ोल्डर चुनें —',
        'move.destination': 'गंतव्य',
        'move.workerLabel': 'कर्मचारी (मुख्य कर्मचारी सेवा से)',
        'move.workerDescription':
            'एक पंजीकृत कर्मचारी चुनें, या नीचे के मुक्त-पाठ फ़ील्ड का उपयोग करने के लिए खाली छोड़ें।',
        'move.freeTextOnly': '— केवल मुक्त-पाठ —',
        'move.movedByLabel': 'द्वारा स्थानांतरित (मुक्त-पाठ)',
        'move.movedByDescription':
            'जब कोई कर्मचारी चयनित न हो तब उपयोग किया जाता है।',
        'move.movedByPlaceholder': 'उदा. ऐलिस (पोर्टर)',
        'move.reasonPlaceholder': 'उदा. बाह्य रोगी अपॉइंटमेंट',
        'move.recordMove': 'स्थानांतरण दर्ज करें',
        'move.patientFolders': 'मरीज़ के फ़ोल्डर',
        'move.enterValidNhs':
            'इस मरीज़ के फ़ोल्डर देखने के लिए एक मान्य NHS नंबर दर्ज करें।',
        'move.folderNotFound': 'फ़ोल्डर नहीं मिला।',
        'move.recordedSummary':
            '{patient} का स्थानांतरण दर्ज किया गया — {folder} {from} से {to} तक।',
        'folders.register': 'फ़ोल्डर रजिस्टर',
        'folders.searchPlaceholder':
            'NHS नंबर, मरीज़, फ़ोल्डर शीर्षक, या अलमारी से खोजें',
        'folders.searchLabel': 'फ़ोल्डर खोजें',
        'folders.addFolder': 'फ़ोल्डर जोड़ें',
        'folders.tableLabel': 'फ़ोल्डर',
        'folders.tableCaption':
            'सिस्टम द्वारा ट्रैक किए गए सभी कागज़ी केस-नोट फ़ोल्डर',
        'folders.colNhsNumber': 'NHS नंबर',
        'folders.colPatient': 'मरीज़',
        'folders.colFolder': 'फ़ोल्डर',
        'folders.colCabinet': 'अलमारी',
        'folders.colStatus': 'स्थिति',
        'folders.colLastMoved': 'अंतिम स्थानांतरण',
        'folders.colAction': 'क्रिया',
        'folders.noMatch': 'कोई फ़ोल्डर मेल नहीं खाता',
        'folderNew.backToFolders': 'फ़ोल्डर पर वापस',
        'folderNew.heading': 'एक नया फ़ोल्डर जोड़ें',
        'folderNew.intro':
            'एक फ़ोल्डर एक मरीज़ का होता है। यदि मरीज़ अभी तक मुख्य मरीज़ सेवा में पंजीकृत नहीं है, तो हम उसे बना देंगे; अन्यथा नया फ़ोल्डर मौजूदा मरीज़ रिकॉर्ड से जुड़ जाएगा।',
        'folderNew.cannotSave': 'फ़ोल्डर सहेजा नहीं जा सकता',
        'folderNew.invalidNhs':
            'एक मान्य 10-अंकीय NHS नंबर दर्ज करें (Modulus 11 जाँच विफल)।',
        'folderNew.titleRequired': 'फ़ोल्डर शीर्षक आवश्यक है।',
        'folderNew.formLabel': 'फ़ोल्डर जोड़ें',
        'folderNew.nhsDescription': '10 अंक, प्रारूप XXX XXX XXXX।',
        'folderNew.titleLabel': 'फ़ोल्डर शीर्षक',
        'folderNew.titleDescription': 'उदा. खंड 1, कार्डियोलॉजी 2023',
        'folderNew.patientName': 'मरीज़ का नाम',
        'folderNew.patientNameDescription': 'केवल नए मरीज़ के लिए आवश्यक।',
        'folderNew.dobDescription': 'केवल नए मरीज़ के लिए आवश्यक।',
        'folderNew.initialCabinet': 'प्रारंभिक अलमारी',
        'folderNew.initialCabinetDescription':
            'यदि फ़ोल्डर स्थानांतरण में है तो खाली छोड़ें।',
        'folderNew.saveFolder': 'फ़ोल्डर सहेजें',
        'folderDetail.backToFolders': 'फ़ोल्डर पर वापस',
        'folderDetail.patientPrefix': 'मरीज़:',
        'folderDetail.detailsLabel': 'फ़ोल्डर विवरण',
        'folderDetail.folderTitle': 'फ़ोल्डर शीर्षक',
        'folderDetail.currentCabinet': 'वर्तमान अलमारी',
        'folderDetail.moveThisFolder': 'इस फ़ोल्डर को स्थानांतरित करें',
        'folderDetail.moveHistory': 'स्थानांतरण इतिहास',
        'patients.heading': 'मरीज़',
        'patients.searchPlaceholder': 'NHS नंबर या नाम से खोजें',
        'patients.searchLabel': 'मरीज़ खोजें',
        'patients.tableLabel': 'मरीज़',
        'patients.tableCaption': 'एक या अधिक पंजीकृत फ़ोल्डर वाले सभी मरीज़',
        'patients.colFolders': 'फ़ोल्डर',
        'patients.noMatch': 'कोई मरीज़ मेल नहीं खाता',
        'patientDetail.backToPatients': 'मरीज़ों पर वापस',
        'patientDetail.notFoundHeading': 'मुख्य मरीज़ सेवा में मरीज़ नहीं मिला',
        'patientDetail.notFoundBody':
            'NHS नंबर {nhs} के लिए कोई मरीज़ रिकॉर्ड मौजूद नहीं है। नीचे के फ़ोल्डर प्रत्येक फ़ोल्डर बनाते समय लिखे गए स्थानीय स्नैपशॉट से पुनर्निर्मित किए गए हैं।',
        'patientDetail.sourcePrefix': 'स्रोत:',
        'patientDetail.recordActions': 'मरीज़ रिकॉर्ड क्रियाएँ',
        'patientDetail.nhsNumberHeading': 'NHS नंबर {nhs}',
        'patientDetail.foldersForPatient': 'इस मरीज़ के फ़ोल्डर ({n})',
        'patientDetail.patientFoldersTable': 'मरीज़ के फ़ोल्डर',
        'patientDetail.colVolume': 'खंड',
        'patientDetail.noFoldersYet': 'अभी तक कोई फ़ोल्डर नहीं।',
        'patientDetail.addFolderForPatient':
            'इस मरीज़ के लिए एक फ़ोल्डर जोड़ें',
        'patientDetail.moveHistoryForPatient': 'इस मरीज़ का स्थानांतरण इतिहास',
        'patientDetail.demoUnavailable':
            '“{action}” इस डेमो में उपलब्ध नहीं है।',
        'buildings.heading': 'इमारतें',
        'buildings.addBuilding': 'इमारत जोड़ें',
        'buildings.tableLabel': 'इमारतें',
        'buildings.tableCaption': 'रिकॉर्ड कमरों वाली भौतिक साइटें',
        'buildings.colRooms': 'कमरे',
        'buildings.noBuildings': 'अभी तक कोई इमारत नहीं।',
        'buildingNew.backToBuildings': 'इमारतों पर वापस',
        'buildingNew.heading': 'एक इमारत जोड़ें',
        'buildingNew.intro':
            'एक इमारत में कई कमरे हो सकते हैं; प्रत्येक कमरे में कई अलमारियाँ हो सकती हैं।',
        'buildingNew.cannotSave': 'इमारत सहेजी नहीं जा सकती',
        'buildingNew.nameRequired': 'इमारत का नाम आवश्यक है।',
        'buildingNew.formLabel': 'इमारत जोड़ें',
        'buildingNew.nameLabel': 'इमारत का नाम',
        'buildingNew.saveBuilding': 'इमारत सहेजें',
        'buildingDetail.backToBuildings': 'इमारतों पर वापस',
        'buildingDetail.rooms': 'कमरे ({n})',
        'buildingDetail.roomsTable': 'कमरे',
        'buildingDetail.colCabinets': 'अलमारियाँ',
        'buildingDetail.noRooms': 'अभी तक कोई कमरा नहीं।',
        'buildingDetail.addRoom': 'एक कमरा जोड़ें',
        'buildingDetail.addRoomLabel': 'कमरा जोड़ें',
        'buildingDetail.roomNameRequired': 'कमरे का नाम आवश्यक है।',
        'buildingDetail.roomName': 'कमरे का नाम',
        'buildingDetail.saveRoom': 'कमरा सहेजें',
        'buildingDetail.presenceHistory': 'फ़ोल्डर उपस्थिति इतिहास',
        'buildingDetail.presenceIntro':
            'इस इमारत की किसी भी अलमारी में रहे फ़ोल्डर, नवीनतम पहले।',
        'buildingDetail.presenceTable': 'इमारत फ़ोल्डर उपस्थिति इतिहास',
        'buildingDetail.presenceCaption': 'इस इमारत की अलमारियों में समेकित',
        'buildingDetail.noPresence':
            'इस इमारत में अभी तक कोई फ़ोल्डर उपस्थिति दर्ज नहीं की गई।',
        'cabinets.heading': 'फ़ाइल अलमारियाँ',
        'cabinets.addCabinet': 'अलमारी जोड़ें',
        'cabinets.tableLabel': 'अलमारियाँ',
        'cabinets.tableCaption':
            'भौतिक फ़ाइल अलमारियाँ, उनकी इमारत/कमरा, और उपयोग',
        'cabinets.colLabel': 'लेबल',
        'cabinets.colBuilding': 'इमारत',
        'cabinets.colRoom': 'कमरा',
        'cabinets.colCapacity': 'क्षमता',
        'cabinets.colFolders': 'फ़ोल्डर',
        'cabinets.colUtilisation': 'उपयोग',
        'cabinetNew.backToCabinets': 'अलमारियों पर वापस',
        'cabinetNew.heading': 'एक फ़ाइल अलमारी जोड़ें',
        'cabinetNew.intro':
            'एक अलमारी एक कमरे के अंदर रहती है (जो एक इमारत के अंदर रहता है)।',
        'cabinetNew.noRoomsHeading': 'अभी तक कोई कमरा मौजूद नहीं',
        'cabinetNew.noRoomsBody': 'पहले, फिर इमारत के पृष्ठ से एक कमरा जोड़ें।',
        'cabinetNew.noRoomsBuilding': 'इमारत',
        'cabinetNew.noRoomsCreate': 'एक',
        'cabinetNew.cannotSave': 'अलमारी सहेजी नहीं जा सकती',
        'cabinetNew.labelRequired': 'अलमारी लेबल आवश्यक है।',
        'cabinetNew.roomRequired': 'एक कमरा चुनें।',
        'cabinetNew.formLabel': 'अलमारी जोड़ें',
        'cabinetNew.labelLabel': 'अलमारी लेबल',
        'cabinetNew.labelPlaceholder': 'उदा. अलमारी D3',
        'cabinetNew.roomLabel': 'कमरा',
        'cabinetNew.selectRoomOption': '— एक कमरा चुनें —',
        'cabinetNew.capacityLabel': 'क्षमता',
        'cabinetNew.capacityDescription':
            'अलमारी जितने फ़ोल्डर रखती है उसकी अनुमानित संख्या। अज्ञात होने पर खाली छोड़ें।',
        'cabinetNew.saveCabinet': 'अलमारी सहेजें',
        'cabinetDetail.backToCabinets': 'अलमारियों पर वापस',
        'cabinetDetail.currentFolders': 'इस अलमारी में वर्तमान फ़ोल्डर ({n})',
        'cabinetDetail.currentFoldersTable': 'वर्तमान फ़ोल्डर',
        'cabinetDetail.empty': 'यह अलमारी वर्तमान में खाली है।',
        'cabinetDetail.presenceHistory': 'फ़ोल्डर उपस्थिति इतिहास',
        'cabinetDetail.presenceIntro':
            'इस अलमारी में कौन-से फ़ोल्डर रहे हैं, और कब। नवीनतम पहले।',
        'cabinetDetail.presenceTable': 'फ़ोल्डर उपस्थिति इतिहास',
        'cabinetDetail.presenceCaption': 'स्थानांतरण ऑडिट लॉग से व्युत्पन्न',
        'cabinetDetail.colReasonLeft': 'छोड़ने का कारण',
        'cabinetDetail.noPresence':
            'इस अलमारी में अभी तक कोई फ़ोल्डर दर्ज नहीं किया गया।',
        'roomDetail.backToBuildings': 'इमारतों पर वापस',
        'roomDetail.presenceHistory': 'फ़ोल्डर उपस्थिति इतिहास',
        'roomDetail.presenceIntro':
            'इस कमरे की किसी भी अलमारी में रहे फ़ोल्डर, नवीनतम पहले।',
        'roomDetail.presenceTable': 'कमरा फ़ोल्डर उपस्थिति इतिहास',
        'roomDetail.presenceCaption': 'इस कमरे की अलमारियों में समेकित',
        'roomDetail.noPresence':
            'इस कमरे में अभी तक कोई फ़ोल्डर उपस्थिति दर्ज नहीं की गई।',
        'volumes.heading': 'खंड',
        'volumes.printLabels': 'लेबल प्रिंट करें',
        'volumes.newVolume': 'नया खंड',
        'volumes.intro':
            'एक खंड एक मरीज़ के फ़ोल्डरों का एक स्थानांतरणीय बंडल है। खंड को स्थानांतरित करें और उसके अंदर का हर फ़ोल्डर साथ-साथ स्थानांतरित हो जाता है।',
        'volumes.tableLabel': 'खंड',
        'volumes.tableCaption': 'फ़ोल्डरों के बंडल, प्रत्येक एक मरीज़ का',
        'volumes.colFolders': 'फ़ोल्डर',
        'volumes.colLocation': 'स्थान',
        'volumes.noVolumes':
            'अभी तक कोई खंड नहीं। मरीज़ के फ़ोल्डर बंडल करने के लिए एक बनाएँ।',
        'volumes.noLabelsSelected': 'कोई लेबल चयनित नहीं।',
        'volumes.queued.one':
            'प्रिंट के लिए {n} लेबल × {copies} {copy} कतार में।',
        'volumes.queued.other':
            'प्रिंट के लिए {n} लेबल × {copies} {copy} कतार में।',
        'volumes.copy': 'प्रति',
        'volumes.copies': 'प्रतियाँ',
        'volumeNew.backToVolumes': 'खंडों पर वापस',
        'volumeNew.heading': 'नया खंड',
        'volumeNew.intro':
            'एक खंड एक मरीज़ के फ़ोल्डरों को बंडल करता है। मरीज़ का पहले से पंजीकृत होना आवश्यक है (पहले उसके लिए एक फ़ोल्डर बनाएँ)। खंड के मौजूद होने पर आप उसमें फ़ोल्डर जोड़ सकते हैं।',
        'volumeNew.cannotCreate': 'खंड बनाया नहीं जा सका',
        'volumeNew.invalidNhs':
            'एक मान्य 10-अंकीय NHS नंबर दर्ज करें (Modulus 11 जाँच विफल)।',
        'volumeNew.titleRequired': 'खंड शीर्षक आवश्यक है।',
        'volumeNew.formLabel': 'नया खंड',
        'volumeNew.patientNhs': 'मरीज़ का NHS नंबर',
        'volumeNew.nhsDescription': '10 अंक, प्रारूप XXX XXX XXXX।',
        'volumeNew.titleLabel': 'खंड शीर्षक',
        'volumeNew.titleDescription': 'उदा. ऐलिस जॉनसन — खंड 1',
        'volumeNew.initialCabinet': 'प्रारंभिक अलमारी',
        'volumeNew.initialCabinetDescription':
            'जहाँ खंड रहता है। स्थानांतरण में होने पर खाली छोड़ें।',
        'volumeNew.createVolume': 'खंड बनाएँ',
        'volumeDetail.backToVolumes': 'खंडों पर वापस',
        'volumeDetail.somethingWrong': 'कुछ गलत हो गया',
        'volumeDetail.foldersIn': 'इस खंड में फ़ोल्डर ({n})',
        'volumeDetail.foldersTable': 'खंड फ़ोल्डर',
        'volumeDetail.noFolders': 'इस खंड में अभी तक कोई फ़ोल्डर नहीं।',
        'volumeDetail.addFolderLabel': 'एक फ़ोल्डर जोड़ें',
        'volumeDetail.addFolderFor': '{patient} के लिए एक फ़ोल्डर जोड़ें',
        'volumeDetail.chooseFolder': '— एक फ़ोल्डर चुनें —',
        'volumeDetail.addToVolume': 'खंड में जोड़ें',
        'volumeDetail.renameVolume': 'खंड का नाम बदलें',
        'volumeDetail.rename': 'नाम बदलें',
        'volumeDetail.moveVolume': 'इस खंड को स्थानांतरित करें',
        'volumeDetail.moveIntro':
            'खंड के सभी फ़ोल्डरों को एक साथ स्थानांतरित करता है।',
        'volumeDetail.moveFormLabel': 'खंड स्थानांतरित करें',
        'volumeDetail.destinationCabinet': 'गंतव्य अलमारी',
        'volumeDetail.moveReasonPlaceholder': 'उदा. बाह्य रोगी क्लिनिक',
        'volumeDetail.moveVolumeButton': 'खंड स्थानांतरित करें',
        'volumeDetail.moveHistory': 'स्थानांतरण इतिहास',
        'workers.heading': 'कर्मचारी',
        'workers.intro':
            'वह स्टाफ़ जो फ़ोल्डर स्थानांतरित करता है। किसी कर्मचारी को खोलें ताकि उसके द्वारा स्थानांतरित फ़ोल्डर और उसके मरीज़ों के सभी फ़ोल्डर देख सकें।',
        'workers.tableLabel': 'कर्मचारी',
        'workers.tableCaption': 'मुख्य कर्मचारी सेवा से कर्मचारी',
        'workers.noWorkers': 'कोई कर्मचारी नहीं मिला।',
        'workerDetail.backToWorkers': 'कर्मचारियों पर वापस',
        'workerDetail.foldersMoved':
            'इस कर्मचारी द्वारा स्थानांतरित फ़ोल्डर ({n})',
        'workerDetail.foldersMovedTable':
            'इस कर्मचारी द्वारा स्थानांतरित फ़ोल्डर',
        'workerDetail.noMovedFolders':
            'इस कर्मचारी ने अभी तक कोई फ़ोल्डर स्थानांतरित नहीं किया।',
        'workerDetail.patientsFolders': 'उनके मरीज़ों के सभी फ़ोल्डर ({n})',
        'workerDetail.patientsFoldersIntro':
            'इस कर्मचारी द्वारा संभाले गए किसी मरीज़ का प्रत्येक फ़ोल्डर।',
        'workerDetail.patientsFoldersTable':
            'इस कर्मचारी के मरीज़ों के फ़ोल्डर',
        'workerDetail.noPatientFolders':
            'दिखाने के लिए अभी तक कोई मरीज़ फ़ोल्डर नहीं।',
        'workerDetail.movesByWorker': 'इस कर्मचारी द्वारा स्थानांतरण',
        'history.backToDashboard': 'डैशबोर्ड पर वापस',
        'history.heading': 'स्थानांतरण इतिहास (ऑडिट लॉग)',
        'history.filterPlaceholder':
            'मरीज़, NHS नंबर, अलमारी, या पोर्टर से फ़िल्टर करें',
        'history.filterLabel': 'ऑडिट लॉग फ़िल्टर करें',
        'history.tableLabel': 'स्थानांतरण ऑडिट लॉग',
        'history.tableCaption':
            'कागज़ी फ़ोल्डर का प्रत्येक दर्ज स्थानांतरण, नवीनतम पहले',
        'history.noMatch': 'कोई स्थानांतरण आपके फ़िल्टर से मेल नहीं खाता।',
        'historyDetail.backToHistory': 'स्थानांतरण इतिहास पर वापस',
        'historyDetail.heading': 'स्थानांतरण घटना',
        'historyDetail.folderInvolved': 'संबंधित फ़ोल्डर',
        'historyDetail.detailsLabel': 'स्थानांतरण घटना विवरण',
        'historyDetail.otherFolders': '{patient} के लिए अन्य फ़ोल्डर',
        'historyDetail.noOtherFolders':
            'इस मरीज़ के लिए कोई अन्य फ़ोल्डर नहीं।',
        'reports.backToDashboard': 'डैशबोर्ड पर वापस',
        'reports.heading': 'रिपोर्ट',
        'reports.atAGlance': 'एक नज़र में',
        'reports.kpiPatients': 'मरीज़:',
        'reports.kpiFolders': 'फ़ोल्डर:',
        'reports.kpiFoldersDetail':
            '({inCabinet} अलमारी में, {inTransit} स्थानांतरण में)',
        'reports.kpiVolumes': 'खंड:',
        'reports.kpiCabinets': 'अलमारियाँ:',
        'reports.kpiCabinetsDetail': '{n} इमारतों में',
        'reports.kpiMoves': 'स्थानांतरण — पिछले 24घं:',
        'reports.kpiMoves7d': 'पिछले 7द:',
        'reports.cabinetUtilisation': 'अलमारी उपयोग',
        'reports.colCabinet': 'अलमारी',
        'reports.colFolders': 'फ़ोल्डर',
        'reports.colCapacity': 'क्षमता',
        'reports.colUtilisation': 'उपयोग',
        'reports.inTransit': 'स्थानांतरण में ({n})',
        'reports.noInTransit': 'कोई फ़ोल्डर स्थानांतरण में नहीं।',
        'reports.activityByWorker': 'कर्मचारी द्वारा गतिविधि',
        'reports.colWorker': 'कर्मचारी',
        'reports.colMoves': 'स्थानांतरण',
        'reports.noMovesYet': 'अभी तक कोई स्थानांतरण दर्ज नहीं किया गया।',
        'reports.derivedLive':
            'रिपोर्ट API से लाइव व्युत्पन्न होती हैं — कोई अलग रिपोर्टिंग स्टोर नहीं।',
        'alerts.backToDashboard': 'डैशबोर्ड पर वापस',
        'alerts.heading': 'जियोफ़ेंस अलर्ट',
        'alerts.intro':
            'केस नोट्स जिन्होंने किसी इमारत की सीमा पार की। जियोफ़ेंस उल्लंघन कोई भी ऐसा स्थानांतरण है जिसकी मूल और गंतव्य अलमारियाँ अलग-अलग इमारतों में हैं।',
        'alerts.tableLabel': 'जियोफ़ेंस अलर्ट',
        'alerts.tableCaption': 'सीमा पार करने वाले स्थानांतरण, नवीनतम पहले',
        'alerts.colCrossed': 'पार किया',
        'alerts.none':
            'कोई जियोफ़ेंस उल्लंघन नहीं — प्रत्येक फ़ोल्डर अपनी इमारत के भीतर रहा है।',
        'grid.nhsNumber': 'NHS नंबर',
        'grid.patient': 'मरीज़',
        'grid.folder': 'फ़ोल्डर',
        'grid.cabinet': 'अलमारी',
        'grid.status': 'स्थिति',
        'grid.lastMoved': 'अंतिम स्थानांतरण',
        'addressograph.label': 'मरीज़ लेबल',
        'addressograph.nhsNo': 'NHS सं.',
        'addressograph.dob': 'ज.ति.',
        'addressograph.sex': 'लिंग',
        'addressograph.address': 'पता',
        'buttonBar.label': 'क्रियाएँ',
        'buttonBar.patient': 'मरीज़',
        'buttonBar.referrals': 'रेफ़रल',
        'buttonBar.activate': 'सक्रिय करें',
        'buttonBar.caseNotes': 'केस नोट्स',
        'buttonBar.pathways': 'मार्ग',
        'buttonBar.legalStatus': 'कानूनी स्थिति',
        'buttonBar.documents': 'दस्तावेज़',
        'buttonBar.wrapper': 'रैपर',
        'buttonBar.audit': 'ऑडिट',
        'buttonBar.quickReports': 'त्वरित रिपोर्ट',
        'labels.title': 'लेबल',
        'labels.searchLabel': 'लेबल खोजें',
        'labels.searchPlaceholder': 'खोजने के लिए पाठ दर्ज करें...',
        'labels.find': 'खोजें',
        'labels.clear': 'साफ़ करें',
        'labels.volumeTitles': 'खंड शीर्षक',
        'labels.noMatching': 'कोई मेल खाने वाला खंड नहीं।',
        'labels.numberOfCopies': 'प्रतियों की संख्या:',
        'labels.copiesLabel': 'प्रतियों की संख्या',
        'labels.print': 'प्रिंट',
        'labels.close': 'बंद करें',
        'auth.signin': 'साइन इन करें',
        'auth.signout': 'साइन आउट करें',
        'share.email': 'लिंक ईमेल करें',
        'share.linkedin': 'LinkedIn पर साझा करें',
        'share.reddit': 'Reddit पर साझा करें',
        'share.bluesky': 'Bluesky पर साझा करें',
        'share.mastodon': 'Mastodon पर साझा करें',
        'splash.hero.secondary': 'अंदर क्या है, देखें',
        'splash.benefits.title': 'टीमें इसे क्यों चुनती हैं',
        'splash.features.title': 'आप क्या कर सकते हैं',
        'splash.trust.title': 'भरोसे के लिए बनाया गया',
        'splash.trust.1.title': 'बिना पासवर्ड साइन-इन',
        'splash.trust.1.body':
            'आपके ईमेल पर भेजा गया मैजिक लिंक: न लीक होने वाला, न दोबारा इस्तेमाल होने वाला पासवर्ड।',
        'splash.trust.2.title': 'विशेषता-आधारित अनुमतियाँ',
        'splash.trust.2.body':
            'सूक्ष्म नियम तय करते हैं कि कौन पढ़, लिख, मर्ज या हटा सकता है।',
        'splash.trust.3.title': 'छेड़छाड़-प्रमाण ऑडिट ट्रेल',
        'splash.trust.3.body':
            'केवल-जोड़ने वाला इतिहास हर बदलाव और उसे करने वाले को दर्ज करता है।',
        'splash.trust.4.title': 'गोपनीयता नियंत्रण',
        'splash.trust.4.body':
            'संवेदनशील विवरण तब तक छिपे रहते हैं जब तक आपको उन्हें देखने का अधिकार न हो।',
        'splash.trust.5.title': 'खुले मानक',
        'splash.trust.5.body':
            'OpenAPI के साथ REST, और जहाँ स्वास्थ्य प्रणालियों को ज़रूरत हो वहाँ HL7 FHIR।',
        'splash.trust.6.title': 'आपकी भाषा में',
        'splash.trust.6.body':
            'अरबी, चीनी, अंग्रेज़ी, फ़्रेंच, हिन्दी, स्पेनिश और वेल्श।',
        'splash.cta.title': 'शुरू करने के लिए तैयार हैं?',
        'splash.cta.body':
            'अपने ईमेल पर भेजे गए मैजिक लिंक से साइन इन करें। पासवर्ड की ज़रूरत नहीं।',
        'splash.hero.title': 'जानें कि हर फ़ोल्डर कहाँ है',
        'splash.hero.subtitle':
            'NHS नंबर से कागज़ी केस-नोट फ़ोल्डरों को इमारतों, कमरों और अलमारियों में ट्रैक करें, हर स्थानांतरण के पूरे ऑडिट रिकॉर्ड के साथ।',
        'splash.benefits.1.title': 'कोई भी फ़ोल्डर जल्दी खोजें',
        'splash.benefits.1.body':
            'NHS नंबर से कागज़ी केस-नोट फ़ोल्डर खोजें और देखें कि वह अभी कहाँ है।',
        'splash.benefits.2.title': 'कम खोए हुए फ़ोल्डर',
        'splash.benefits.2.body':
            'हर स्थानांतरण दर्ज होता है, इसलिए कोई फ़ोल्डर कभी सिर्फ़ इमारत में कहीं नहीं होता।',
        'splash.benefits.3.title': 'खोजने में कम समय',
        'splash.benefits.3.body':
            'पोर्टर और चिकित्सक अलमारियाँ खंगालने के बजाय देखभाल में समय लगाते हैं।',
        'splash.benefits.4.title': 'भरोसेमंद रिकॉर्ड',
        'splash.benefits.4.body':
            'पूरा, तारीख़ वाला इतिहास दिखाता है कि हर फ़ोल्डर किसने, कब और क्यों स्थानांतरित किया।',
        'splash.benefits.5.title': 'मरीज़ की पहचान सुरक्षित',
        'splash.benefits.5.body':
            'NHS नंबर सहेजने से पहले जाँचे जाते हैं, इसलिए टाइपिंग की गलतियाँ कभी रिकॉर्ड नहीं बनतीं।',
        'splash.benefits.6.title': 'पूरे परिसर की साफ़ तस्वीर',
        'splash.benefits.6.body':
            'देखें कि हर अलमारी कितनी भरी है और कितने फ़ोल्डर स्थानांतरण में हैं।',
        'splash.features.1.title': 'फ़ोल्डर रजिस्टर',
        'splash.features.1.body':
            'मरीज़, शीर्षक या NHS नंबर से हर फ़ोल्डर खोजें।',
        'splash.features.2.title': 'स्थानांतरण और स्कैन',
        'splash.features.2.body':
            'NHS नंबर स्कैन करके या दर्ज करके कुछ ही सेकंड में स्थानांतरण दर्ज करें।',
        'splash.features.3.title': 'खंड',
        'splash.features.3.body':
            'किसी मरीज़ के फ़ोल्डरों को एक खंड में समूहित करें और उन्हें साथ स्थानांतरित करें।',
        'splash.features.4.title': 'इमारतें, कमरे, अलमारियाँ',
        'splash.features.4.body':
            'हर इमारत से लेकर उसकी अलमारियों तक भौतिक क्रम को दर्ज करें।',
        'splash.features.5.title': 'स्थानांतरण इतिहास',
        'splash.features.5.body':
            'हर फ़ोल्डर स्थानांतरण के पूरे ऑडिट लॉग में खोजें।',
        'splash.features.6.title': 'अलर्ट और रिपोर्ट',
        'splash.features.6.body':
            'इमारतों के बीच अलर्ट, अलमारियों का उपयोग और मुख्य आँकड़े देखें।',
        'nav.tour': 'टूर',
        'splash.hero.tour': 'टूर देखें',
        'tour.head': 'टूर देखें',
        'tour.toc': 'इस पृष्ठ पर',
        'tour.open': 'यह स्क्रीन खोलें',
        'tour.top': 'ऊपर लौटें',
        'tour.start.title': 'शुरू करने से पहले',
        'tour.start.summary':
            'वास्तविक डेटा के साथ काम करने के लिए खाता चाहिए। साइन इन में एक मिनट से कम लगता है और पासवर्ड की ज़रूरत नहीं।',
        'tour.start.step.1':
            'ऊपर दाईं ओर साइन इन चुनें और अपना ईमेल पता दर्ज करें।',
        'tour.start.step.2':
            'हमारे भेजे मैजिक लिंक को खोलें। यह एक ही बार काम करता है और जल्दी समाप्त हो जाता है।',
        'tour.start.step.3':
            'आप साइन इन होकर ऐप में लौटते हैं, न कुछ याद रखना, न रीसेट करना।',
        'tour.start.step.4':
            'थीम, भाषा और टेक्स्ट का आकार बदलने या पृष्ठ साझा करने के लिए साइन इन के पास के बटन इस्तेमाल करें।',
        'tour.intro':
            'केस ट्रैकिंग का मार्गदर्शित परिचय: हर स्क्रीन क्या करती है और उसे इस्तेमाल करने के चरण, फ़ोल्डर जोड़ने से लेकर स्थानांतरण ऑडिट लॉग देखने तक।',
        'tour.s1.title': 'रजिस्टर में फ़ोल्डर जोड़ना',
        'tour.s1.summary':
            'केस-नोट का हर काग़ज़ी फ़ोल्डर एक मरीज़ का होता है और जोड़े जाने के क्षण से ही उसे ट्रैक किया जाता है।',
        'tour.s1.step.1':
            'फ़ोल्डर खोलें और फ़ोल्डर जोड़ें चुनें, फिर मरीज़ का 10 अंकों का NHS नंबर डालें, जो मॉड्यूलस 11 जाँच में सफल होना चाहिए।',
        'tour.s1.step.2':
            'फ़ोल्डर शीर्षक लिखें, जैसे खंड 1 या कार्डियोलॉजी 2023।',
        'tour.s1.step.3':
            'जो मरीज़ अभी पंजीकृत नहीं है, उसके लिए मरीज़ का नाम और जन्म तिथि भी भरें; मौजूदा मरीज़ के लिए इनकी ज़रूरत नहीं है।',
        'tour.s1.step.4':
            'प्रारंभिक अलमारी चुनें, या फ़ोल्डर स्थानांतरण में हो तो उसे खाली छोड़ें, फिर फ़ोल्डर सहेजें चुनें।',
        'tour.s2.title': 'फ़ोल्डर ढूँढना',
        'tour.s2.summary':
            'फ़ोल्डर रजिस्टर एक ही खोज बॉक्स से बताता है कि "यह फ़ोल्डर अभी कहाँ है?"',
        'tour.s2.step.1':
            'फ़ोल्डर खोलें और फ़ोल्डर खोजें में NHS नंबर, मरीज़ का नाम, फ़ोल्डर शीर्षक या अलमारी लिखें।',
        'tour.s2.step.2':
            'हर पंक्ति की अलमारी, स्थिति (अलमारी में या स्थानांतरण में) और अंतिम स्थानांतरण पढ़ें।',
        'tour.s2.step.3':
            'फ़ोल्डर खोलकर उसका ब्योरा और स्थानांतरण इतिहास देखें, या मरीज़ खोलकर उसके सभी फ़ोल्डर देखें।',
        'tour.s2.step.4':
            'इस फ़ोल्डर को स्थानांतरित करें चुनकर सीधे उसका स्थानांतरण दर्ज करें।',
        'tour.s3.title': 'फ़ोल्डर स्थानांतरित करना',
        'tour.s3.summary':
            'हर स्थान दर्ज होता है, इसलिए फ़ोल्डर की जगह की जानकारी हमेशा ताज़ा रहती है।',
        'tour.s3.step.1':
            'फ़ोल्डर स्थानांतरित करें खोलें, मरीज़ का NHS नंबर डालें, फिर चुनें कि उस मरीज़ का कौन-सा फ़ोल्डर स्थानांतरित कर रहे हैं।',
        'tour.s3.step.2':
            'गंतव्य अलमारी चुनें, या रास्ते में हो तो स्थानांतरण में (पोर्टर ले जा रहा है) चुनें।',
        'tour.s3.step.3':
            'सूची से कर्मचारी चुनें, या द्वारा स्थानांतरित के अंतर्गत नाम लिखें, और कारण जोड़ें।',
        'tour.s3.step.4':
            'स्थानांतरण दर्ज करें चुनें; पेज स्थानांतरण दर्ज किया गया दिखाता है और यह स्थानांतरण ऑडिट लॉग में जुड़ जाता है।',
        'tour.s4.title': 'फ़ोल्डर स्कैन करना',
        'tour.s4.summary':
            'रिकॉर्ड डेस्क के लिए तेज़ रास्ता: हार्डवेयर स्कैनर ज़रूरी नहीं है, हालाँकि कीबोर्ड की तरह टाइप करने वाला स्कैनर भी चलता है।',
        'tour.s4.step.1': 'स्कैन खोलें और स्कैन या खोजें बॉक्स में क्लिक करें।',
        'tour.s4.step.2':
            'बारकोड स्कैन करें, या NHS नंबर या फ़ोल्डर आईडी लिखें।',
        'tour.s4.step.3':
            'मिलान सूची देखें और फ़ोल्डर खोलकर उसका ब्योरा देखें।',
        'tour.s4.step.4':
            'इस फ़ोल्डर को स्थानांतरित करें चुनें ताकि फ़ोल्डर पहले से चुने हुए के साथ स्थानांतरण दर्ज हो; कुछ न मिले तो पेज बताता है कि कोई फ़ोल्डर नहीं मिला।',
        'tour.s5.title': 'फ़ोल्डरों को खंडों में बाँधना',
        'tour.s5.summary':
            'खंड एक मरीज़ के फ़ोल्डरों का ऐसा समूह है जिसे साथ ले जाया जा सकता है, इसलिए वे एक साथ चलते हैं।',
        'tour.s5.step.1':
            'खंड खोलें और नया खंड चुनें, फिर मरीज़ का NHS नंबर और खंड शीर्षक डालें; मरीज़ का पहले से कोई फ़ोल्डर होना चाहिए।',
        'tour.s5.step.2':
            'खंड खोलें और एक फ़ोल्डर जोड़ें से उस मरीज़ के फ़ोल्डर उसमें बाँधें।',
        'tour.s5.step.3':
            'इस खंड को स्थानांतरित करें से उसके भीतर के सभी फ़ोल्डरों को एक ही चरण में एक गंतव्य अलमारी में भेजें।',
        'tour.s5.step.4':
            'खंड पर लौटकर लेबल प्रिंट करें चुनें, खंड चुनें, प्रतियों की संख्या तय करें और प्रिंट चुनकर उन्हें कतार में लगाएँ।',
        'tour.s6.title': 'इतिहास, अलर्ट और रिपोर्ट देखना',
        'tour.s6.summary':
            'हर स्थानांतरण सुरक्षित रहता है, इसलिए आप जान सकते हैं कि किसने क्या, कब और क्यों स्थानांतरित किया।',
        'tour.s6.step.1':
            'स्थानांतरण इतिहास खोलें और ऑडिट लॉग फ़िल्टर करें से मरीज़, NHS नंबर, अलमारी या पोर्टर के आधार पर छाँटें; सबसे नए स्थानांतरण पहले आते हैं।',
        'tour.s6.step.2':
            'किसी पंक्ति को खोलकर पूरा स्थानांतरण ब्योरा, संबंधित फ़ोल्डर और मरीज़ के अन्य फ़ोल्डर देखें।',
        'tour.s6.step.3':
            'अलर्ट खोलकर जियोफ़ेंस अलर्ट देखें: ऐसे स्थानांतरण जिनकी मूल और गंतव्य अलमारियाँ अलग-अलग इमारतों में हैं।',
        'tour.s6.step.4':
            'रिपोर्ट खोलकर एक नज़र में गिनती, अलमारी उपयोग, स्थानांतरण में फ़ोल्डर और कर्मचारी के अनुसार गतिविधि देखें, सब कुछ लाइव तैयार होता है।',
        'signin.sso': 'SSO से साइन इन करें',
    },
    'zh-cn': {
        'brand.name': '病案追踪',
        'brand.tagline': 'NHS 纸质记录',
        'chrome.language': '语言',
        'chrome.theme': '主题',
        'nav.share': '分享',
        'nav.text_size': '文字大小',
        'share.copy_link': '复制链接',
        'share.copied': '链接已复制',
        'share.copy_failed': '无法复制 — 请从地址栏复制',
        'theme.default': '默认',
        'theme.highContrast': '高对比度',
        'nav.toggle': '切换导航',
        'auth.signedInAs': '登录身份',
        'auth.signOut': '退出登录',
        'nav.dashboard': '仪表板',
        'nav.patients': '患者',
        'nav.folders': '文件夹',
        'nav.volumes': '卷册',
        'nav.workers': '员工',
        'nav.buildings': '建筑',
        'nav.cabinets': '档案柜',
        'nav.move': '移动文件夹',
        'nav.scan': '扫描',
        'nav.history': '移动历史',
        'nav.alerts': '警报',
        'nav.reports': '报告',
        'layout.skipToContent': '跳到主要内容',
        'layout.siteHeader': '网站页眉',
        'layout.siteFooter': '网站页脚',
        'layout.primaryNavigation': '主导航',
        'footer.text':
            '病案追踪 — 使用 Lily 设计系统（NHS 主题）和 SVAR Svelte 构建。仅为演示数据；并非受监管的医疗记录。',
        'common.backToDashboard': '返回仪表板',
        'common.cancel': '取消',
        'common.move': '移动',
        'common.view': '查看',
        'common.remove': '移除',
        'common.action': '操作',
        'common.status': '状态',
        'common.patient': '患者',
        'common.folder': '文件夹',
        'common.cabinet': '档案柜',
        'common.title': '标题',
        'common.name': '姓名',
        'common.role': '角色',
        'common.reason': '原因',
        'common.movedBy': '移动者',
        'common.lastMoved': '最近移动',
        'common.nhsNumber': 'NHS 号码',
        'common.dateOfBirth': '出生日期',
        'common.description': '描述',
        'common.notes': '备注',
        'common.entered': '进入',
        'common.left': '离开',
        'common.when': '时间',
        'common.from': '从',
        'common.to': '到',
        'common.source': '来源',
        'common.volume': '卷册',
        'common.stillHere': '仍在此处',
        'common.noMovesYet': '尚未记录任何移动。',
        'common.inTransitPorter': '运送中（搬运员携带）',
        'common.selectCabinetOption': '— 选择档案柜 —',
        'common.inTransitOption': '— 运送中 —',
        'status.inCabinet': '在柜中',
        'status.inTransit': '运送中',
        'badge.located': '已定位',
        'badge.porterInMotion': '搬运员移动中',
        'dashboard.welcomePrefix': '欢迎。',
        'dashboard.inTransit.one':
            '您当前有 {n} 个文件夹正在运送中。请使用页面',
        'dashboard.inTransit.other':
            '您当前有 {n} 个文件夹正在运送中。请使用页面',
        'dashboard.moveFolderPage': '移动文件夹',
        'dashboard.pageToRecord': '来记录一次放置。',
        'dashboard.folderSummary': '文件夹摘要',
        'dashboard.patients': '患者',
        'dashboard.foldersTracked': '已追踪 {n} 个文件夹',
        'dashboard.inCabinet': '在柜中',
        'dashboard.inTransitCard': '运送中',
        'dashboard.buildings': '建筑',
        'dashboard.roomsCabinets': '{rooms} 个房间 · {cabinets} 个档案柜',
        'dashboard.moves24h': '移动（24 小时）',
        'dashboard.auditedPlacements': '已审计的文件夹放置',
        'dashboard.folderRegister': '文件夹登记册',
        'dashboard.viewAll': '查看全部',
        'dashboard.addFolder': '添加文件夹',
        'dashboard.recentMoves': '最近移动',
        'dashboard.seeFullHistory': '查看完整审计历史 →',
        'dashboard.cabinetUtilisation': '档案柜使用率',
        'error.backToDashboard': '返回仪表板',
        'error.heading': '病案追踪 API 错误',
        'error.unknown': '未知错误',
        'error.apiHint':
            '该应用通过 /api 与 Loco JSON API 通信（开发时代理到 Loco 服务器）。请确保该 API 正在运行。参见 README.md § "Quick start"。',
        'login.title': '登录',
        'login.intro':
            '请输入您的工作电子邮件地址。如被识别，我们将向您发送一次性登录链接。无需密码。',
        'login.sendError': '无法发送登录链接',
        'login.enterEmail': '请输入您的电子邮件地址。',
        'login.checkEmail': '请检查您的电子邮件',
        'login.sentBody':
            '与已知帐户匹配，登录链接即将送达。链接将在 10 分钟后失效。',
        'login.sentPrefix': '如果',
        'login.devShortcut': '开发快捷方式：',
        'login.openLink': '打开您的登录链接',
        'login.formLabel': '登录',
        'login.emailLabel': '电子邮件地址',
        'login.submit': '向我发送登录链接',
        'callback.title': '正在为您登录',
        'callback.error': '无法使用登录链接',
        'callback.backToSignIn': '返回登录',
        'callback.completing': '请稍候 — 正在完成您的登录…',
        'scan.heading': '扫描文件夹',
        'scan.intro':
            '扫描条形码或输入 NHS 号码（或文件夹 id）以直接跳转到某个文件夹并记录其移动 — Scan4Safety 快速通道。无需硬件扫描仪；键盘楔式扫描仪会在下方框中输入。',
        'scan.failed': '扫描失败',
        'scan.formLabel': '扫描',
        'scan.fieldLabel': '扫描或搜索',
        'scan.fieldDescription': 'NHS 号码（如 943 476 5919）或文件夹 id。',
        'scan.placeholder': '扫描或输入…',
        'scan.matches': '匹配项（{n}）',
        'scan.moveThisFolder': '移动此文件夹',
        'scan.noFolderFound': '未找到与“{term}”对应的文件夹。',
        'move.heading': '移动文件夹',
        'move.intro':
            '输入患者的 NHS 号码，选择您要移动的文件夹，然后选择目标档案柜（或标记为运送中）。',
        'move.recorded': '移动已记录',
        'move.formLabel': '移动文件夹',
        'move.patientNhs': '患者 NHS 号码',
        'move.invalidNhs': '请输入有效的 10 位 NHS 号码。',
        'move.folder': '文件夹',
        'move.selectFolderError': '请选择要移动的文件夹。',
        'move.pickFolderDescription': '选择要移动该患者的哪个文件夹。',
        'move.enterNhsDescription': '输入 NHS 号码以查看文件夹。',
        'move.selectFolderOption': '— 选择文件夹 —',
        'move.destination': '目的地',
        'move.workerLabel': '员工（来自主员工服务）',
        'move.workerDescription':
            '选择一名已注册员工，或留空以使用下方的自由文本字段。',
        'move.freeTextOnly': '— 仅自由文本 —',
        'move.movedByLabel': '移动者（自由文本）',
        'move.movedByDescription': '未选择员工时使用。',
        'move.movedByPlaceholder': '如 Alice（搬运员）',
        'move.reasonPlaceholder': '如 门诊预约',
        'move.recordMove': '记录移动',
        'move.patientFolders': '患者文件夹',
        'move.enterValidNhs': '输入有效的 NHS 号码以查看该患者的文件夹。',
        'move.folderNotFound': '未找到文件夹。',
        'move.recordedSummary':
            '已记录 {patient} 的移动 — {folder} 从 {from} 到 {to}。',
        'folders.register': '文件夹登记册',
        'folders.searchPlaceholder':
            '按 NHS 号码、患者、文件夹标题或档案柜搜索',
        'folders.searchLabel': '搜索文件夹',
        'folders.addFolder': '添加文件夹',
        'folders.tableLabel': '文件夹',
        'folders.tableCaption': '系统追踪的所有纸质病案文件夹',
        'folders.colNhsNumber': 'NHS 号码',
        'folders.colPatient': '患者',
        'folders.colFolder': '文件夹',
        'folders.colCabinet': '档案柜',
        'folders.colStatus': '状态',
        'folders.colLastMoved': '最近移动',
        'folders.colAction': '操作',
        'folders.noMatch': '没有文件夹匹配',
        'folderNew.backToFolders': '返回文件夹',
        'folderNew.heading': '添加新文件夹',
        'folderNew.intro':
            '一个文件夹属于一名患者。如果该患者尚未在主患者服务中注册，我们将创建它；否则新文件夹将附加到现有患者记录。',
        'folderNew.cannotSave': '无法保存文件夹',
        'folderNew.invalidNhs':
            '请输入有效的 10 位 NHS 号码（模 11 校验失败）。',
        'folderNew.titleRequired': '文件夹标题为必填项。',
        'folderNew.formLabel': '添加文件夹',
        'folderNew.nhsDescription': '10 位数字，格式为 XXX XXX XXXX。',
        'folderNew.titleLabel': '文件夹标题',
        'folderNew.titleDescription': '如 卷 1、心脏病学 2023',
        'folderNew.patientName': '患者姓名',
        'folderNew.patientNameDescription': '仅新患者需要。',
        'folderNew.dobDescription': '仅新患者需要。',
        'folderNew.initialCabinet': '初始档案柜',
        'folderNew.initialCabinetDescription': '如果文件夹正在运送中，请留空。',
        'folderNew.saveFolder': '保存文件夹',
        'folderDetail.backToFolders': '返回文件夹',
        'folderDetail.patientPrefix': '患者：',
        'folderDetail.detailsLabel': '文件夹详情',
        'folderDetail.folderTitle': '文件夹标题',
        'folderDetail.currentCabinet': '当前档案柜',
        'folderDetail.moveThisFolder': '移动此文件夹',
        'folderDetail.moveHistory': '移动历史',
        'patients.heading': '患者',
        'patients.searchPlaceholder': '按 NHS 号码或姓名搜索',
        'patients.searchLabel': '搜索患者',
        'patients.tableLabel': '患者',
        'patients.tableCaption': '拥有一个或多个已注册文件夹的所有患者',
        'patients.colFolders': '文件夹',
        'patients.noMatch': '没有患者匹配',
        'patientDetail.backToPatients': '返回患者',
        'patientDetail.notFoundHeading': '在主患者服务中未找到患者',
        'patientDetail.notFoundBody':
            'NHS 号码 {nhs} 不存在患者记录。下方的文件夹根据创建每个文件夹时写入的本地快照重建。',
        'patientDetail.sourcePrefix': '来源：',
        'patientDetail.recordActions': '患者记录操作',
        'patientDetail.nhsNumberHeading': 'NHS 号码 {nhs}',
        'patientDetail.foldersForPatient': '该患者的文件夹（{n}）',
        'patientDetail.patientFoldersTable': '患者文件夹',
        'patientDetail.colVolume': '卷册',
        'patientDetail.noFoldersYet': '尚无文件夹。',
        'patientDetail.addFolderForPatient': '为该患者添加文件夹',
        'patientDetail.moveHistoryForPatient': '该患者的移动历史',
        'patientDetail.demoUnavailable': '“{action}”在此演示中不可用。',
        'buildings.heading': '建筑',
        'buildings.addBuilding': '添加建筑',
        'buildings.tableLabel': '建筑',
        'buildings.tableCaption': '存放记录房间的实体场所',
        'buildings.colRooms': '房间',
        'buildings.noBuildings': '尚无建筑。',
        'buildingNew.backToBuildings': '返回建筑',
        'buildingNew.heading': '添加建筑',
        'buildingNew.intro':
            '一座建筑可以有多个房间；每个房间可以容纳多个档案柜。',
        'buildingNew.cannotSave': '无法保存建筑',
        'buildingNew.nameRequired': '建筑名称为必填项。',
        'buildingNew.formLabel': '添加建筑',
        'buildingNew.nameLabel': '建筑名称',
        'buildingNew.saveBuilding': '保存建筑',
        'buildingDetail.backToBuildings': '返回建筑',
        'buildingDetail.rooms': '房间（{n}）',
        'buildingDetail.roomsTable': '房间',
        'buildingDetail.colCabinets': '档案柜',
        'buildingDetail.noRooms': '尚无房间。',
        'buildingDetail.addRoom': '添加房间',
        'buildingDetail.addRoomLabel': '添加房间',
        'buildingDetail.roomNameRequired': '房间名称为必填项。',
        'buildingDetail.roomName': '房间名称',
        'buildingDetail.saveRoom': '保存房间',
        'buildingDetail.presenceHistory': '文件夹存放历史',
        'buildingDetail.presenceIntro':
            '曾在该建筑任一档案柜中的文件夹，最新在前。',
        'buildingDetail.presenceTable': '建筑文件夹存放历史',
        'buildingDetail.presenceCaption': '按该建筑的档案柜汇总',
        'buildingDetail.noPresence': '该建筑中尚未记录任何文件夹存放。',
        'cabinets.heading': '档案柜',
        'cabinets.addCabinet': '添加档案柜',
        'cabinets.tableLabel': '档案柜',
        'cabinets.tableCaption': '实体档案柜及其建筑/房间和占用情况',
        'cabinets.colLabel': '标签',
        'cabinets.colBuilding': '建筑',
        'cabinets.colRoom': '房间',
        'cabinets.colCapacity': '容量',
        'cabinets.colFolders': '文件夹',
        'cabinets.colUtilisation': '使用率',
        'cabinetNew.backToCabinets': '返回档案柜',
        'cabinetNew.heading': '添加档案柜',
        'cabinetNew.intro': '档案柜位于房间内（房间位于建筑内）。',
        'cabinetNew.noRoomsHeading': '尚无房间',
        'cabinetNew.noRoomsBody': '然后从建筑页面添加房间。',
        'cabinetNew.noRoomsBuilding': '建筑',
        'cabinetNew.noRoomsCreate': '请先创建一座',
        'cabinetNew.cannotSave': '无法保存档案柜',
        'cabinetNew.labelRequired': '档案柜标签为必填项。',
        'cabinetNew.roomRequired': '请选择一个房间。',
        'cabinetNew.formLabel': '添加档案柜',
        'cabinetNew.labelLabel': '档案柜标签',
        'cabinetNew.labelPlaceholder': '如 档案柜 D3',
        'cabinetNew.roomLabel': '房间',
        'cabinetNew.selectRoomOption': '— 选择一个房间 —',
        'cabinetNew.capacityLabel': '容量',
        'cabinetNew.capacityDescription':
            '档案柜容纳的文件夹大致数量。未知时请留空。',
        'cabinetNew.saveCabinet': '保存档案柜',
        'cabinetDetail.backToCabinets': '返回档案柜',
        'cabinetDetail.currentFolders': '当前在此档案柜中的文件夹（{n}）',
        'cabinetDetail.currentFoldersTable': '当前文件夹',
        'cabinetDetail.empty': '此档案柜当前为空。',
        'cabinetDetail.presenceHistory': '文件夹存放历史',
        'cabinetDetail.presenceIntro':
            '哪些文件夹曾在此档案柜中，以及何时。最新在前。',
        'cabinetDetail.presenceTable': '文件夹存放历史',
        'cabinetDetail.presenceCaption': '源自移动审计日志',
        'cabinetDetail.colReasonLeft': '离开原因',
        'cabinetDetail.noPresence': '此档案柜中尚未记录任何文件夹。',
        'roomDetail.backToBuildings': '返回建筑',
        'roomDetail.presenceHistory': '文件夹存放历史',
        'roomDetail.presenceIntro':
            '曾在该房间任一档案柜中的文件夹，最新在前。',
        'roomDetail.presenceTable': '房间文件夹存放历史',
        'roomDetail.presenceCaption': '按该房间的档案柜汇总',
        'roomDetail.noPresence': '该房间中尚未记录任何文件夹存放。',
        'volumes.heading': '卷册',
        'volumes.printLabels': '打印标签',
        'volumes.newVolume': '新建卷册',
        'volumes.intro':
            '卷册是一名患者文件夹的可移动捆束。移动卷册，其中的每个文件夹都会一起移动。',
        'volumes.tableLabel': '卷册',
        'volumes.tableCaption': '文件夹的捆束，每个属于一名患者',
        'volumes.colFolders': '文件夹',
        'volumes.colLocation': '位置',
        'volumes.noVolumes': '尚无卷册。创建一个以捆绑患者的文件夹。',
        'volumes.noLabelsSelected': '未选择标签。',
        'volumes.queued.one':
            '已将 {n} 个标签 × {copies} {copy} 加入打印队列。',
        'volumes.queued.other':
            '已将 {n} 个标签 × {copies} {copy} 加入打印队列。',
        'volumes.copy': '份',
        'volumes.copies': '份',
        'volumeNew.backToVolumes': '返回卷册',
        'volumeNew.heading': '新建卷册',
        'volumeNew.intro':
            '卷册捆绑一名患者的文件夹。该患者必须已注册（请先为其创建一个文件夹）。卷册存在后即可向其添加文件夹。',
        'volumeNew.cannotCreate': '无法创建卷册',
        'volumeNew.invalidNhs':
            '请输入有效的 10 位 NHS 号码（模 11 校验失败）。',
        'volumeNew.titleRequired': '卷册标题为必填项。',
        'volumeNew.formLabel': '新建卷册',
        'volumeNew.patientNhs': '患者 NHS 号码',
        'volumeNew.nhsDescription': '10 位数字，格式为 XXX XXX XXXX。',
        'volumeNew.titleLabel': '卷册标题',
        'volumeNew.titleDescription': '如 Alice Johnson — 卷 1',
        'volumeNew.initialCabinet': '初始档案柜',
        'volumeNew.initialCabinetDescription': '卷册所在位置。运送中时请留空。',
        'volumeNew.createVolume': '创建卷册',
        'volumeDetail.backToVolumes': '返回卷册',
        'volumeDetail.somethingWrong': '出错了',
        'volumeDetail.foldersIn': '此卷册中的文件夹（{n}）',
        'volumeDetail.foldersTable': '卷册文件夹',
        'volumeDetail.noFolders': '此卷册中尚无文件夹。',
        'volumeDetail.addFolderLabel': '添加文件夹',
        'volumeDetail.addFolderFor': '为 {patient} 添加文件夹',
        'volumeDetail.chooseFolder': '— 选择一个文件夹 —',
        'volumeDetail.addToVolume': '添加到卷册',
        'volumeDetail.renameVolume': '重命名卷册',
        'volumeDetail.rename': '重命名',
        'volumeDetail.moveVolume': '移动此卷册',
        'volumeDetail.moveIntro': '将卷册中的所有文件夹一起重新放置。',
        'volumeDetail.moveFormLabel': '移动卷册',
        'volumeDetail.destinationCabinet': '目标档案柜',
        'volumeDetail.moveReasonPlaceholder': '如 门诊诊所',
        'volumeDetail.moveVolumeButton': '移动卷册',
        'volumeDetail.moveHistory': '移动历史',
        'workers.heading': '员工',
        'workers.intro':
            '移动文件夹的工作人员。打开某员工以查看其移动过的文件夹以及其患者的所有文件夹。',
        'workers.tableLabel': '员工',
        'workers.tableCaption': '来自主员工服务的员工',
        'workers.noWorkers': '未找到员工。',
        'workerDetail.backToWorkers': '返回员工',
        'workerDetail.foldersMoved': '该员工移动的文件夹（{n}）',
        'workerDetail.foldersMovedTable': '该员工移动的文件夹',
        'workerDetail.noMovedFolders': '该员工尚未移动任何文件夹。',
        'workerDetail.patientsFolders': '其所有患者的文件夹（{n}）',
        'workerDetail.patientsFoldersIntro': '该员工经手的患者的每个文件夹。',
        'workerDetail.patientsFoldersTable': '该员工患者的文件夹',
        'workerDetail.noPatientFolders': '暂无患者文件夹可显示。',
        'workerDetail.movesByWorker': '该员工的移动',
        'history.backToDashboard': '返回仪表板',
        'history.heading': '移动历史（审计日志）',
        'history.filterPlaceholder': '按患者、NHS 号码、档案柜或搬运员筛选',
        'history.filterLabel': '筛选审计日志',
        'history.tableLabel': '移动审计日志',
        'history.tableCaption': '纸质文件夹的每次记录移动，最新在前',
        'history.noMatch': '没有移动符合您的筛选。',
        'historyDetail.backToHistory': '返回移动历史',
        'historyDetail.heading': '移动事件',
        'historyDetail.folderInvolved': '涉及的文件夹',
        'historyDetail.detailsLabel': '移动事件详情',
        'historyDetail.otherFolders': '{patient} 的其他文件夹',
        'historyDetail.noOtherFolders': '该患者没有其他文件夹。',
        'reports.backToDashboard': '返回仪表板',
        'reports.heading': '报告',
        'reports.atAGlance': '概览',
        'reports.kpiPatients': '患者：',
        'reports.kpiFolders': '文件夹：',
        'reports.kpiFoldersDetail':
            '（{inCabinet} 在柜中，{inTransit} 运送中）',
        'reports.kpiVolumes': '卷册：',
        'reports.kpiCabinets': '档案柜：',
        'reports.kpiCabinetsDetail': '位于 {n} 座建筑',
        'reports.kpiMoves': '移动 — 最近 24 小时：',
        'reports.kpiMoves7d': '最近 7 天：',
        'reports.cabinetUtilisation': '档案柜使用率',
        'reports.colCabinet': '档案柜',
        'reports.colFolders': '文件夹',
        'reports.colCapacity': '容量',
        'reports.colUtilisation': '使用率',
        'reports.inTransit': '运送中（{n}）',
        'reports.noInTransit': '没有文件夹在运送中。',
        'reports.activityByWorker': '按员工统计的活动',
        'reports.colWorker': '员工',
        'reports.colMoves': '移动',
        'reports.noMovesYet': '尚未记录任何移动。',
        'reports.derivedLive': '报告从 API 实时派生 — 无单独的报告存储。',
        'alerts.backToDashboard': '返回仪表板',
        'alerts.heading': '地理围栏警报',
        'alerts.intro':
            '跨越建筑边界的病案。地理围栏违规是指起点和目标档案柜位于不同建筑的任何移动。',
        'alerts.tableLabel': '地理围栏警报',
        'alerts.tableCaption': '跨越边界的移动，最新在前',
        'alerts.colCrossed': '已跨越',
        'alerts.none': '没有地理围栏违规 — 每个文件夹都留在其所属建筑内。',
        'grid.nhsNumber': 'NHS 号码',
        'grid.patient': '患者',
        'grid.folder': '文件夹',
        'grid.cabinet': '档案柜',
        'grid.status': '状态',
        'grid.lastMoved': '最近移动',
        'addressograph.label': '患者标签',
        'addressograph.nhsNo': 'NHS 号',
        'addressograph.dob': '出生日期',
        'addressograph.sex': '性别',
        'addressograph.address': '地址',
        'buttonBar.label': '操作',
        'buttonBar.patient': '患者',
        'buttonBar.referrals': '转诊',
        'buttonBar.activate': '激活',
        'buttonBar.caseNotes': '病案记录',
        'buttonBar.pathways': '诊疗路径',
        'buttonBar.legalStatus': '法律状态',
        'buttonBar.documents': '文档',
        'buttonBar.wrapper': '封套',
        'buttonBar.audit': '审计',
        'buttonBar.quickReports': '快速报告',
        'labels.title': '标签',
        'labels.searchLabel': '搜索标签',
        'labels.searchPlaceholder': '输入要搜索的文本...',
        'labels.find': '查找',
        'labels.clear': '清除',
        'labels.volumeTitles': '卷册标题',
        'labels.noMatching': '没有匹配的卷册。',
        'labels.numberOfCopies': '份数：',
        'labels.copiesLabel': '份数',
        'labels.print': '打印',
        'labels.close': '关闭',
        'auth.signin': '登录',
        'auth.signout': '退出登录',
        'share.email': '通过邮件发送链接',
        'share.linkedin': '分享到 LinkedIn',
        'share.reddit': '分享到 Reddit',
        'share.bluesky': '分享到 Bluesky',
        'share.mastodon': '分享到 Mastodon',
        'splash.hero.secondary': '了解详情',
        'splash.benefits.title': '团队为何选择它',
        'splash.features.title': '你可以做什么',
        'splash.trust.title': '以信任为本',
        'splash.trust.1.title': '免密码登录',
        'splash.trust.1.body':
            '魔法链接发送到你的邮箱：没有密码可泄露或重复使用。',
        'splash.trust.2.title': '基于属性的权限',
        'splash.trust.2.body': '细粒度规则决定谁可以读取、写入、合并或删除。',
        'splash.trust.3.title': '防篡改审计记录',
        'splash.trust.3.body': '仅追加的历史记录会记下每一次更改及操作者。',
        'splash.trust.4.title': '隐私控制',
        'splash.trust.4.body': '除非你有权查看，否则敏感信息会被屏蔽。',
        'splash.trust.5.title': '开放标准',
        'splash.trust.5.body':
            'REST 搭配 OpenAPI，并在医疗系统需要时支持 HL7 FHIR。',
        'splash.trust.6.title': '支持你的语言',
        'splash.trust.6.body':
            '阿拉伯语、中文、英语、法语、印地语、西班牙语和威尔士语。',
        'splash.cta.title': '准备好开始了吗？',
        'splash.cta.body': '通过发送到邮箱的魔法链接登录，无需密码。',
        'splash.hero.title': '随时知道每份文件夹在哪里',
        'splash.hero.subtitle':
            '按 NHS 编号追踪纸质病案文件夹在楼宇、房间和档案柜之间的流转，并完整记录每一次移动以备审计。',
        'splash.benefits.1.title': '快速找到任何文件夹',
        'splash.benefits.1.body':
            '按 NHS 编号查找纸质病案文件夹，立即看到它现在的位置。',
        'splash.benefits.2.title': '减少丢失的文件夹',
        'splash.benefits.2.body':
            '每次移动都有记录，文件夹不会再只是“在楼里某个地方”。',
        'splash.benefits.3.title': '减少查找时间',
        'splash.benefits.3.body':
            '护工和临床医生把时间用在护理上，而不是翻找档案柜。',
        'splash.benefits.4.title': '可信的记录',
        'splash.benefits.4.body':
            '完整且带日期的历史显示谁在何时、因何移动了每份文件夹。',
        'splash.benefits.5.title': '患者身份更安全',
        'splash.benefits.5.body':
            'NHS 编号在保存前会经过校验，输入错误不会变成记录。',
        'splash.benefits.6.title': '清晰掌握全局',
        'splash.benefits.6.body':
            '查看每个档案柜有多满，以及有多少文件夹正在运送中。',
        'splash.features.1.title': '文件夹登记册',
        'splash.features.1.body': '按患者、标题或 NHS 编号搜索每份文件夹。',
        'splash.features.2.title': '移动与扫描',
        'splash.features.2.body':
            '扫描或输入 NHS 编号，几秒钟即可记录一次移动。',
        'splash.features.3.title': '卷册',
        'splash.features.3.body': '把患者的多份文件夹归为一卷，一起移动。',
        'splash.features.4.title': '楼宇、房间、档案柜',
        'splash.features.4.body':
            '按实际层级建模，从每栋楼一直到其中的档案柜。',
        'splash.features.5.title': '移动历史',
        'splash.features.5.body': '搜索每一次文件夹移动的完整审计日志。',
        'splash.features.6.title': '警报与报告',
        'splash.features.6.body': '查看跨楼宇警报、档案柜使用情况和关键数据。',
        'nav.tour': '导览',
        'splash.hero.tour': '开始导览',
        'tour.head': '开始导览',
        'tour.toc': '本页内容',
        'tour.open': '打开此页面',
        'tour.top': '返回顶部',
        'tour.start.title': '开始之前',
        'tour.start.summary':
            '处理真实数据需要账号。登录不到一分钟，也不需要密码。',
        'tour.start.step.1': '点击右上角的“登录”，输入你的邮箱地址。',
        'tour.start.step.2':
            '打开我们发到邮箱的魔法链接。它只能使用一次，且很快过期。',
        'tour.start.step.3':
            '你会以已登录状态回到应用，无需记忆或重置任何内容。',
        'tour.start.step.4':
            '使用“登录”旁边的按钮切换主题、语言和文字大小，或分享此页面。',
        'tour.intro':
            '病例追踪的导览：每个界面的作用及使用步骤，从添加文件夹到查看移动审计日志。',
        'tour.s1.title': '向登记册添加文件夹',
        'tour.s1.summary':
            '每个纸质病例记录文件夹都属于一位患者，从添加的那一刻起就会被追踪。',
        'tour.s1.step.1':
            '打开“文件夹”并选择“添加文件夹”，然后输入患者 10 位的 NHS 号码，该号码必须通过模 11 校验。',
        'tour.s1.step.2': '输入文件夹标题，例如“卷册 1”或“心脏科 2023”。',
        'tour.s1.step.3':
            '如果患者尚未登记，还需填写患者姓名和出生日期；已有患者则无需填写。',
        'tour.s1.step.4':
            '选择初始档案柜；如果文件夹正在运送中则留空，然后选择“保存文件夹”。',
        'tour.s2.title': '查找文件夹',
        'tour.s2.summary':
            '文件夹登记册只需一个搜索框，就能回答“这个文件夹现在在哪里？”。',
        'tour.s2.step.1':
            '打开“文件夹”，在“搜索文件夹”中输入 NHS 号码、患者姓名、文件夹标题或档案柜。',
        'tour.s2.step.2':
            '查看每一行的档案柜、状态（在档案柜中或运送中）和最近移动。',
        'tour.s2.step.3':
            '打开文件夹可查看详情和移动历史，或从“患者”打开患者以查看其所有文件夹。',
        'tour.s2.step.4': '选择“移动此文件夹”，可直接为它记录一次移动。',
        'tour.s3.title': '移动文件夹',
        'tour.s3.summary':
            '每一次存放都会被记录，因此文件夹的位置始终是最新的。',
        'tour.s3.step.1':
            '打开“移动文件夹”，输入患者 NHS 号码，然后选择要移动该患者的哪个文件夹。',
        'tour.s3.step.2':
            '选择目的地档案柜；如果文件夹正在途中，则选择“运送中（搬运员携带）”。',
        'tour.s3.step.3':
            '从列表中选择员工，或在“移动者”下输入姓名，并填写原因。',
        'tour.s3.step.4':
            '选择“记录移动”；页面会显示“移动已记录”，该次移动随即加入审计日志。',
        'tour.s4.title': '扫描文件夹',
        'tour.s4.summary':
            '记录室的快捷通道：无需专用扫描设备，不过模拟键盘输入的扫码枪同样可用。',
        'tour.s4.step.1': '打开“扫描”，点击“扫描或搜索”框。',
        'tour.s4.step.2': '扫描条形码，或输入 NHS 号码或文件夹编号。',
        'tour.s4.step.3': '查看“匹配项”列表，并打开文件夹查看详情。',
        'tour.s4.step.4':
            '选择“移动此文件夹”，即可在已选定该文件夹的情况下记录移动；如果没有匹配项，页面会提示未找到文件夹。',
        'tour.s5.title': '将文件夹归入卷册',
        'tour.s5.summary':
            '卷册是同一位患者文件夹的可移动集合，因此它们会一起移动。',
        'tour.s5.step.1':
            '打开“卷册”并选择“新建卷册”，然后输入患者 NHS 号码和卷册标题；该患者必须已有文件夹。',
        'tour.s5.step.2':
            '打开该卷册，使用“添加文件夹”把该患者的文件夹归入其中。',
        'tour.s5.step.3':
            '使用“移动此卷册”，一步就把其中所有文件夹移到同一个目的地档案柜。',
        'tour.s5.step.4':
            '回到“卷册”，选择“打印标签”，选中卷册，设置份数，然后选择“打印”加入打印队列。',
        'tour.s6.title': '查看历史、警报和报告',
        'tour.s6.summary':
            '每一次移动都会保留，因此你可以查清是谁在何时、为何移动了什么。',
        'tour.s6.step.1':
            '打开“移动历史”，用“筛选审计日志”按患者、NHS 号码、档案柜或搬运员缩小范围；最新的移动排在最前。',
        'tour.s6.step.2':
            '打开某一行，可查看完整的移动事件、相关文件夹以及该患者的其他文件夹。',
        'tour.s6.step.3':
            '打开“警报”查看地理围栏警报：起点和终点档案柜位于不同楼宇的移动。',
        'tour.s6.step.4':
            '打开“报告”查看概览数据、档案柜使用情况、运送中的文件夹和按员工统计的活动，全部实时生成。',
        'signin.sso': '使用 SSO 登录',
    },
} as const;

/** The set of valid translation keys (derived from the English catalog). */
export type StringKey = keyof (typeof STRINGS)['en-001'];

/**
 * Every translatable key (the English catalog's key set). Exported so a
 * coverage test can assert that each locale defines every key.
 */
export const STRING_KEYS = Object.keys(STRINGS['en-001']) as StringKey[];

/** Raw per-locale strings table, exposed for coverage testing. */
export const STRINGS_BY_LOCALE: Record<
    Locale,
    Record<string, string>
> = STRINGS;

// Normalise raw input to a supported locale, or null if unsupported.
// Hyphen/underscore- and case-insensitive; an exact match wins, otherwise
// the primary subtag resolves to the supported locale for that language
// (`es-MX` -> `es-001`, `zh` -> `zh-cn`).
function normaliseLocale(raw: string | null | undefined): Locale | null {
    if (!raw) return null;
    const normalized = raw.trim().replace(/_/g, '-').toLowerCase();
    const exact = LOCALES.find((l) => l === normalized);
    if (exact) return exact;
    const primary = normalized.split('-')[0] ?? '';
    return LOCALES.find((l) => l.split('-')[0] === primary) ?? null;
}

// Seed the reactive locale from localStorage (default off the browser).
function readStoredLocale(): Locale {
    // Guard on `localStorage` availability, not just `browser`: a jsdom test
    // run can set the browser resolve condition while leaving `localStorage`
    // undefined.
    if (!browser || typeof localStorage === 'undefined') return DEFAULT_LOCALE;
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
        if (browser && typeof localStorage !== 'undefined')
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
 * Interpolate `{name}` placeholders in a translated string.
 *
 * @param key - A valid {@link StringKey}.
 * @param vars - Map of placeholder name → replacement value.
 * @returns The translated string with placeholders substituted.
 */
export function tf(
    key: StringKey,
    vars: Record<string, string | number>,
): string {
    return translate(key, current).replace(/\{(\w+)\}/g, (whole, name) =>
        name in vars ? String(vars[name]) : whole,
    );
}

/**
 * Reactive translation accessor for components: `t("folders.register")`.
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

/**
 * Translate a folder status string from the API (`in-cabinet` /
 * `in-transit`) to a localized display label. Reactive (reads the current
 * locale); unknown statuses pass through unchanged.
 *
 * @param status - Raw status from the API.
 * @returns The localized status label.
 */
export function statusLabel(status: string): string {
    if (status === 'in-cabinet') return t('status.inCabinet');
    if (status === 'in-transit') return t('status.inTransit');
    return status;
}
