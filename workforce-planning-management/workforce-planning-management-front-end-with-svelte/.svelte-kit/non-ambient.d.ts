
// this file is generated — do not edit it


declare module "svelte/elements" {
	export interface HTMLAttributes<T> {
		'data-sveltekit-keepfocus'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-noscroll'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-preload-code'?:
			| true
			| ''
			| 'eager'
			| 'viewport'
			| 'hover'
			| 'tap'
			| 'off'
			| undefined
			| null;
		'data-sveltekit-preload-data'?: true | '' | 'hover' | 'tap' | 'off' | undefined | null;
		'data-sveltekit-reload'?: true | '' | 'off' | undefined | null;
		'data-sveltekit-replacestate'?: true | '' | 'off' | undefined | null;
	}
}

export {};


declare module "$app/types" {
	type MatcherParam<M> = M extends (param : string) => param is (infer U extends string) ? U : string;

	export interface AppTypes {
		RouteId(): "/" | "/api" | "/api/proxy" | "/api/proxy/[...path]" | "/benchmarks" | "/development" | "/employees" | "/employees/[pid]" | "/learning" | "/mentorship" | "/org-chart" | "/payroll" | "/payroll/[pid]" | "/privacy" | "/requisitions" | "/requisitions/[pid]" | "/signin" | "/signin/sso" | "/signout" | "/tour" | "/verify" | "/wellbeing" | "/workforce";
		RouteParams(): {
			"/api/proxy/[...path]": { path: string };
			"/employees/[pid]": { pid: string };
			"/payroll/[pid]": { pid: string };
			"/requisitions/[pid]": { pid: string }
		};
		LayoutParams(): {
			"/": { path?: string | undefined; pid?: string | undefined };
			"/api": { path?: string | undefined };
			"/api/proxy": { path?: string | undefined };
			"/api/proxy/[...path]": { path: string };
			"/benchmarks": Record<string, never>;
			"/development": Record<string, never>;
			"/employees": { pid?: string | undefined };
			"/employees/[pid]": { pid: string };
			"/learning": Record<string, never>;
			"/mentorship": Record<string, never>;
			"/org-chart": Record<string, never>;
			"/payroll": { pid?: string | undefined };
			"/payroll/[pid]": { pid: string };
			"/privacy": Record<string, never>;
			"/requisitions": { pid?: string | undefined };
			"/requisitions/[pid]": { pid: string };
			"/signin": Record<string, never>;
			"/signin/sso": Record<string, never>;
			"/signout": Record<string, never>;
			"/tour": Record<string, never>;
			"/verify": Record<string, never>;
			"/wellbeing": Record<string, never>;
			"/workforce": Record<string, never>
		};
		Pathname(): "/" | `/api/proxy/${string}` & {} | "/benchmarks" | "/development" | "/employees" | `/employees/${string}` & {} | "/learning" | "/mentorship" | "/org-chart" | "/payroll" | `/payroll/${string}` & {} | "/privacy" | "/requisitions" | `/requisitions/${string}` & {} | "/signin" | "/signin/sso" | "/signout" | "/tour" | "/verify" | "/wellbeing" | "/workforce";
		ResolvedPathname(): `${"" | `/${string}`}${ReturnType<AppTypes['Pathname']>}`;
		Asset(): "/assets/themes/abyss.css" | "/assets/themes/acid.css" | "/assets/themes/adobe-spectrum.css" | "/assets/themes/aqua.css" | "/assets/themes/autumn.css" | "/assets/themes/black.css" | "/assets/themes/bumblebee.css" | "/assets/themes/business.css" | "/assets/themes/caramellatte.css" | "/assets/themes/cmyk.css" | "/assets/themes/coffee.css" | "/assets/themes/corporate.css" | "/assets/themes/cupcake.css" | "/assets/themes/cyberpunk.css" | "/assets/themes/dark.css" | "/assets/themes/dim.css" | "/assets/themes/dracula.css" | "/assets/themes/emerald.css" | "/assets/themes/fantasy.css" | "/assets/themes/forest.css" | "/assets/themes/garden.css" | "/assets/themes/halloween.css" | "/assets/themes/lemonade.css" | "/assets/themes/light.css" | "/assets/themes/lofi.css" | "/assets/themes/luxury.css" | "/assets/themes/mozilla-protocol.css" | "/assets/themes/night.css" | "/assets/themes/nord.css" | "/assets/themes/pastel.css" | "/assets/themes/retro.css" | "/assets/themes/silk.css" | "/assets/themes/sunset.css" | "/assets/themes/synthwave.css" | "/assets/themes/united-kingdom-government-digital-service.css" | "/assets/themes/united-kingdom-national-health-service-england-for-patients.css" | "/assets/themes/united-kingdom-national-health-service-england-for-practitioners.css" | "/assets/themes/united-kingdom-national-health-service-scotland-for-patients.css" | "/assets/themes/united-kingdom-national-health-service-scotland-for-practitioners.css" | "/assets/themes/united-kingdom-national-health-service-wales-for-patients.css" | "/assets/themes/united-kingdom-national-health-service-wales-for-practitioners.css" | "/assets/themes/united-states-web-design-system.css" | "/assets/themes/valentine.css" | "/assets/themes/winter.css" | "/assets/themes/wireframe.css" | string & {};
	}
}