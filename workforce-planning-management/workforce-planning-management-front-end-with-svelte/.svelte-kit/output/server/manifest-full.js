export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set(["assets/themes/abyss.css","assets/themes/acid.css","assets/themes/adobe-spectrum.css","assets/themes/aqua.css","assets/themes/autumn.css","assets/themes/black.css","assets/themes/bumblebee.css","assets/themes/business.css","assets/themes/caramellatte.css","assets/themes/cmyk.css","assets/themes/coffee.css","assets/themes/corporate.css","assets/themes/cupcake.css","assets/themes/cyberpunk.css","assets/themes/dark.css","assets/themes/dim.css","assets/themes/dracula.css","assets/themes/emerald.css","assets/themes/fantasy.css","assets/themes/forest.css","assets/themes/garden.css","assets/themes/halloween.css","assets/themes/lemonade.css","assets/themes/light.css","assets/themes/lofi.css","assets/themes/luxury.css","assets/themes/mozilla-protocol.css","assets/themes/night.css","assets/themes/nord.css","assets/themes/pastel.css","assets/themes/retro.css","assets/themes/silk.css","assets/themes/sunset.css","assets/themes/synthwave.css","assets/themes/united-kingdom-government-digital-service.css","assets/themes/united-kingdom-national-health-service-england-for-patients.css","assets/themes/united-kingdom-national-health-service-england-for-practitioners.css","assets/themes/united-kingdom-national-health-service-scotland-for-patients.css","assets/themes/united-kingdom-national-health-service-scotland-for-practitioners.css","assets/themes/united-kingdom-national-health-service-wales-for-patients.css","assets/themes/united-kingdom-national-health-service-wales-for-practitioners.css","assets/themes/united-states-web-design-system.css","assets/themes/valentine.css","assets/themes/winter.css","assets/themes/wireframe.css"]),
	mimeTypes: {".css":"text/css"},
	_: {
		client: {start:"_app/immutable/entry/start.BkVstH1M.js",app:"_app/immutable/entry/app.BBEWIgaX.js",imports:["_app/immutable/entry/start.BkVstH1M.js","_app/immutable/chunks/wLbeR-Q4.js","_app/immutable/chunks/Yqg4C_aI.js","_app/immutable/chunks/BM-gBzY3.js","_app/immutable/chunks/CoXfTjQT.js","_app/immutable/entry/app.BBEWIgaX.js","_app/immutable/chunks/BM-gBzY3.js","_app/immutable/chunks/Bzak7iHL.js","_app/immutable/chunks/Yqg4C_aI.js","_app/immutable/chunks/CIEYxCLc.js","_app/immutable/chunks/Bvo-zJOP.js","_app/immutable/chunks/CkfGNuyk.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:true},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js')),
			__memo(() => import('./nodes/3.js')),
			__memo(() => import('./nodes/4.js')),
			__memo(() => import('./nodes/5.js')),
			__memo(() => import('./nodes/6.js')),
			__memo(() => import('./nodes/7.js')),
			__memo(() => import('./nodes/8.js')),
			__memo(() => import('./nodes/9.js')),
			__memo(() => import('./nodes/10.js')),
			__memo(() => import('./nodes/11.js')),
			__memo(() => import('./nodes/12.js')),
			__memo(() => import('./nodes/13.js')),
			__memo(() => import('./nodes/14.js')),
			__memo(() => import('./nodes/15.js')),
			__memo(() => import('./nodes/16.js')),
			__memo(() => import('./nodes/17.js')),
			__memo(() => import('./nodes/18.js')),
			__memo(() => import('./nodes/19.js'))
		],
		remotes: {
			
		},
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			},
			{
				id: "/api/proxy/[...path]",
				pattern: /^\/api\/proxy(?:\/([^]*))?\/?$/,
				params: [{"name":"path","optional":false,"rest":true,"chained":true}],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/api/proxy/_...path_/_server.ts.js'))
			},
			{
				id: "/benchmarks",
				pattern: /^\/benchmarks\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 3 },
				endpoint: null
			},
			{
				id: "/development",
				pattern: /^\/development\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 4 },
				endpoint: null
			},
			{
				id: "/employees",
				pattern: /^\/employees\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 5 },
				endpoint: null
			},
			{
				id: "/employees/[pid]",
				pattern: /^\/employees\/([^/]+?)\/?$/,
				params: [{"name":"pid","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 6 },
				endpoint: null
			},
			{
				id: "/learning",
				pattern: /^\/learning\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 7 },
				endpoint: null
			},
			{
				id: "/mentorship",
				pattern: /^\/mentorship\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 8 },
				endpoint: null
			},
			{
				id: "/org-chart",
				pattern: /^\/org-chart\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 9 },
				endpoint: null
			},
			{
				id: "/payroll",
				pattern: /^\/payroll\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 10 },
				endpoint: null
			},
			{
				id: "/payroll/[pid]",
				pattern: /^\/payroll\/([^/]+?)\/?$/,
				params: [{"name":"pid","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 11 },
				endpoint: null
			},
			{
				id: "/privacy",
				pattern: /^\/privacy\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 12 },
				endpoint: null
			},
			{
				id: "/requisitions",
				pattern: /^\/requisitions\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 13 },
				endpoint: null
			},
			{
				id: "/requisitions/[pid]",
				pattern: /^\/requisitions\/([^/]+?)\/?$/,
				params: [{"name":"pid","optional":false,"rest":false,"chained":false}],
				page: { layouts: [0,], errors: [1,], leaf: 14 },
				endpoint: null
			},
			{
				id: "/signin",
				pattern: /^\/signin\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 15 },
				endpoint: null
			},
			{
				id: "/signin/sso",
				pattern: /^\/signin\/sso\/?$/,
				params: [],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/signin/sso/_server.ts.js'))
			},
			{
				id: "/signout",
				pattern: /^\/signout\/?$/,
				params: [],
				page: null,
				endpoint: __memo(() => import('./entries/endpoints/signout/_server.ts.js'))
			},
			{
				id: "/tour",
				pattern: /^\/tour\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 16 },
				endpoint: null
			},
			{
				id: "/verify",
				pattern: /^\/verify\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 17 },
				endpoint: null
			},
			{
				id: "/wellbeing",
				pattern: /^\/wellbeing\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 18 },
				endpoint: null
			},
			{
				id: "/workforce",
				pattern: /^\/workforce\/?$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 19 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
